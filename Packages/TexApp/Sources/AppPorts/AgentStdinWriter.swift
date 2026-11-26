import Foundation

/// Writes agent commands to a child's stdin, in order, on a private queue, so a
/// child that stops reading blocks that queue and never the caller's thread.
public final class AgentStdinWriter: @unchecked Sendable {
    private let handle: FileHandle
    private let queue = DispatchQueue(label: "dev.pitex.agent-stdin")
    private let onFailure: @Sendable (Error) -> Void
    private var failed = false

    public init(_ handle: FileHandle, onFailure: @escaping @Sendable (Error) -> Void) {
        self.handle = handle
        self.onFailure = onFailure
        #if canImport(Darwin)
        // A child that exits mid-write must surface as EPIPE, not kill the app.
        _ = fcntl(handle.fileDescriptor, F_SETNOSIGPIPE, 1)
        #endif
    }

    public func write(_ data: Data) {
        queue.async { [self] in
            guard !failed else { return }
            do {
                try handle.write(contentsOf: data)
            } catch {
                failed = true
                onFailure(error)
            }
        }
    }
}
