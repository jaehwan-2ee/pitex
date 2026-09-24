import Foundation
#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

/// Minimal subprocess plumbing for ssh/tar with stdin — BuildCore's
/// ProcessRunner has no stdin, which uploads and file lists need. Input,
/// stdout and stderr are pumped on separate threads so a large transfer
/// can never deadlock on a full pipe; task cancellation terminates the
/// child.
enum ProcessPipe {
    /// Writing to a pipe whose reader exited raises SIGPIPE, which would
    /// kill the app; ignore it so the write fails with EPIPE instead.
    private static let ignoreSigpipe: Void = { signal(SIGPIPE, SIG_IGN) }()

    private final class Box: @unchecked Sendable {
        let lock = NSLock()
        var output = Data()
        var error = Data()
        var process: Process?
        var cancelled = false

        func withLock<T>(_ body: (Box) -> T) -> T {
            lock.lock(); defer { lock.unlock() }
            return body(self)
        }
    }

    static func run(
        executable: URL,
        arguments: [String],
        input: Data?,
        outputFile: URL?,
        currentDirectory: URL? = nil,
        environment: [String: String] = [:]
    ) async throws -> SSHCommandResult {
        _ = ignoreSigpipe
        let box = Box()
        return try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<SSHCommandResult, Error>) in
                DispatchQueue.global(qos: .userInitiated).async {
                    let process = Process()
                    process.executableURL = executable
                    process.arguments = arguments
                    if let currentDirectory { process.currentDirectoryURL = currentDirectory }
                    if !environment.isEmpty {
                        process.environment = ProcessInfo.processInfo.environment.merging(environment) { $1 }
                    }
                    let stdout = Pipe(), stderr = Pipe()
                    var outputHandle: FileHandle?
                    if let outputFile {
                        FileManager.default.createFile(atPath: outputFile.path, contents: nil)
                        guard let handle = try? FileHandle(forWritingTo: outputFile) else {
                            continuation.resume(throwing: SSHError.remote(status: -1, message: "Cannot write \(outputFile.path)."))
                            return
                        }
                        outputHandle = handle
                        process.standardOutput = handle
                    } else {
                        process.standardOutput = stdout
                    }
                    process.standardError = stderr
                    let stdin = input == nil ? nil : Pipe()
                    process.standardInput = stdin ?? FileHandle.nullDevice
                    let started: Bool = box.withLock { box in
                        guard !box.cancelled else { return false }
                        box.process = process
                        do { try process.run(); return true } catch { return false }
                    }
                    guard started else {
                        try? outputHandle?.close()
                        continuation.resume(throwing: box.withLock { $0.cancelled } ? SSHError.cancelled
                                            : SSHError.connection("\(executable.path) could not be started."))
                        return
                    }
                    let group = DispatchGroup()
                    if let input, let stdin {
                        group.enter()
                        DispatchQueue.global().async {
                            try? stdin.fileHandleForWriting.write(contentsOf: input)
                            try? stdin.fileHandleForWriting.close()
                            group.leave()
                        }
                    }
                    if outputHandle == nil {
                        group.enter()
                        DispatchQueue.global().async {
                            let data = stdout.fileHandleForReading.readDataToEndOfFile()
                            box.withLock { $0.output = data }
                            group.leave()
                        }
                    }
                    group.enter()
                    DispatchQueue.global().async {
                        let data = stderr.fileHandleForReading.readDataToEndOfFile()
                        box.withLock { $0.error = data }
                        group.leave()
                    }
                    process.waitUntilExit()
                    group.wait()
                    try? outputHandle?.close()
                    let (output, error, cancelled) = box.withLock { ($0.output, $0.error, $0.cancelled) }
                    if cancelled {
                        continuation.resume(throwing: SSHError.cancelled)
                    } else {
                        continuation.resume(returning: SSHCommandResult(
                            status: process.terminationStatus, standardOutput: output, standardError: error))
                    }
                }
            }
        } onCancel: {
            box.withLock { box in
                box.cancelled = true
                if box.process?.isRunning == true { box.process?.terminate() }
            }
        }
    }

    /// Runs to completion, handing each stdout (`false`) / stderr (`true`)
    /// chunk to `output` in arrival order.
    static func stream(
        executable: URL,
        arguments: [String],
        output: @escaping @Sendable (Data, Bool) async -> Void
    ) async throws -> Int32 {
        _ = ignoreSigpipe
        let box = Box()
        let (chunks, sink) = AsyncStream<(Data, Bool)>.makeStream()
        let process = Process()
        process.executableURL = executable
        process.arguments = arguments
        let stdout = Pipe(), stderr = Pipe()
        process.standardOutput = stdout
        process.standardError = stderr
        process.standardInput = FileHandle.nullDevice
        let started: Bool = box.withLock { box in
            box.process = process
            do { try process.run(); return true } catch { return false }
        }
        guard started else { throw SSHError.connection("\(executable.path) could not be started.") }
        let group = DispatchGroup()
        for (pipe, isError) in [(stdout, false), (stderr, true)] {
            group.enter()
            DispatchQueue.global().async {
                while true {
                    let data = pipe.fileHandleForReading.availableData
                    if data.isEmpty { break }
                    sink.yield((data, isError))
                }
                group.leave()
            }
        }
        DispatchQueue.global().async {
            group.wait()
            process.waitUntilExit()
            sink.finish()
        }
        return try await withTaskCancellationHandler {
            for await (data, isError) in chunks {
                await output(data, isError)
            }
            if box.withLock({ $0.cancelled }) { throw SSHError.cancelled }
            return process.terminationStatus
        } onCancel: {
            box.withLock { box in
                box.cancelled = true
                if box.process?.isRunning == true { box.process?.terminate() }
            }
        }
    }
}
