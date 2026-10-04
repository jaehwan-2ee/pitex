import Dispatch
import Foundation

#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

/// Outgoing requests of one `pitex-preview` session (PROTOCOL.md), bounded
/// no matter how slowly the helper reads its stdin:
/// - one request in flight;
/// - the newest update only — a newer snapshot replaces a queued one;
/// - every release not yet written, deduplicated (never dropped: the
///   helper holds at most 4 unreleased publications, so this stays small);
/// - quit, which supersedes everything still queued.
///
/// Updates are queued as full snapshots of the dirty buffers. Their delta
/// is computed only when one is written, against the override set the
/// helper received in full — so a replaced snapshot can never take a
/// `closed` path (or a changed text) with it.
public struct EmbeddedPreviewOutbox: Equatable, Sendable {
    /// A dirty buffer: content hash and unsaved text.
    public struct Buffer: Equatable, Sendable {
        public let hash: UInt64
        public let text: String

        public init(hash: UInt64, text: String) {
            self.hash = hash
            self.text = text
        }
    }

    public struct File: Equatable, Sendable {
        public let path: String
        public let text: String

        public init(path: String, text: String) {
            self.path = path
            self.text = text
        }
    }

    public enum Request: Equatable, Sendable {
        case releases([UInt64])
        /// `files`: overrides whose text the helper does not hold yet;
        /// `closed`: overrides it holds that the snapshot no longer has.
        case update(generation: UInt64, files: [File], closed: [String])
        case quit
    }

    private struct Snapshot: Equatable, Sendable {
        let generation: UInt64
        let buffers: [String: Buffer]
    }

    public private(set) var inFlight: Request?
    /// Overrides the helper received in full: path → hash.
    public private(set) var delivered: [String: UInt64] = [:]
    private var inFlightOverrides: [String: UInt64]?
    private var pendingUpdate: Snapshot?
    private var pendingReleases: [UInt64] = []
    private var quitRequested = false
    private var quitWritten = false

    public init() {}

    public var hasPendingUpdate: Bool { pendingUpdate != nil }
    public var pendingReleaseCount: Int { pendingReleases.count }

    public mutating func enqueue(update generation: UInt64, buffers: [String: Buffer]) {
        guard !quitRequested else { return }
        pendingUpdate = Snapshot(generation: generation, buffers: buffers)
    }

    public mutating func enqueue(release seq: UInt64) {
        guard !quitRequested, !pendingReleases.contains(seq) else { return }
        if case let .releases(written) = inFlight, written.contains(seq) { return }
        pendingReleases.append(seq)
    }

    /// Session end: queued updates and releases die with its directory.
    public mutating func enqueueQuit() {
        quitRequested = true
        pendingUpdate = nil
        pendingReleases = []
    }

    /// The next request to write, if none is in flight. Releases go first:
    /// they unblock the helper's publication backpressure.
    public mutating func next() -> Request? {
        guard inFlight == nil, !quitWritten else { return nil }
        let request: Request
        if quitRequested {
            request = .quit
        } else if !pendingReleases.isEmpty {
            request = .releases(pendingReleases)
            pendingReleases = []
        } else if let snapshot = pendingUpdate {
            pendingUpdate = nil
            let files = snapshot.buffers
                .filter { delivered[$0.key] != $0.value.hash }
                .map { File(path: $0.key, text: $0.value.text) }
                .sorted { $0.path < $1.path }
            let closed = delivered.keys.filter { snapshot.buffers[$0] == nil }.sorted()
            inFlightOverrides = snapshot.buffers.mapValues(\.hash)
            request = .update(generation: snapshot.generation, files: files, closed: closed)
        } else {
            return nil
        }
        inFlight = request
        return request
    }

    /// The in-flight request reached the pipe completely.
    public mutating func written() {
        if let overrides = inFlightOverrides { delivered = overrides }
        inFlightOverrides = nil
        if inFlight == .quit { quitWritten = true }
        inFlight = nil
    }

    /// The stream was cancelled: nothing more is written.
    public mutating func abandon() {
        enqueueQuit()
        inFlight = nil
        inFlightOverrides = nil
        quitWritten = true
    }
}

/// Writes an `EmbeddedPreviewOutbox` to the helper's stdin asynchronously
/// (DispatchIO, non-blocking). No caller ever waits on the pipe: a stalled
/// reader only keeps one write pending, and `finish()` cancels it instead
/// of queueing behind it. Owns and closes the descriptor.
public final class EmbeddedPreviewWriter: @unchecked Sendable {
    /// Serializes all state below; handlers run here and never block.
    private let queue = DispatchQueue(label: "app.pitex.preview-engine.writer")
    private var channel: DispatchIO?
    private var outbox = EmbeddedPreviewOutbox()

    public init(fileDescriptor: Int32) {
        channel = DispatchIO(type: .stream, fileDescriptor: fileDescriptor, queue: queue) { _ in
            close(fileDescriptor)
        }
    }

    public func update(generation: UInt64, buffers: [String: EmbeddedPreviewOutbox.Buffer]) {
        queue.async { [self] in
            outbox.enqueue(update: generation, buffers: buffers)
            pump()
        }
    }

    public func release(_ seq: UInt64) {
        queue.async { [self] in
            outbox.enqueue(release: seq)
            pump()
        }
    }

    /// Ends the stream without waiting: quit then EOF when the pipe is
    /// free, otherwise the stuck write is cancelled and the descriptor
    /// closed at once (the helper sees EOF either way).
    public func finish() {
        queue.async { [self] in
            outbox.enqueueQuit()
            if outbox.inFlight == nil { pump() } else { stop() }
        }
    }

    /// Queued work right now (after every earlier call was applied).
    public func backlog() -> (inFlight: EmbeddedPreviewOutbox.Request?, pendingUpdate: Bool, pendingReleases: Int) {
        queue.sync { (outbox.inFlight, outbox.hasPendingUpdate, outbox.pendingReleaseCount) }
    }

    private func pump() {
        guard let channel, let request = outbox.next() else { return }
        let bytes = Self.encode(request)
        let data = bytes.withUnsafeBytes { DispatchData(bytes: $0) }
        channel.write(offset: 0, data: data, queue: queue) { [self] done, _, error in
            guard done else { return }
            guard error == 0 else {
                stop()
                return
            }
            outbox.written()
            if request == .quit {
                self.channel?.close()
                self.channel = nil
            } else {
                pump()
            }
        }
    }

    private func stop() {
        outbox.abandon()
        channel?.close(flags: .stop)
        channel = nil
    }

    static func encode(_ request: EmbeddedPreviewOutbox.Request) -> Data {
        switch request {
        case let .releases(seqs):
            return Data(seqs.map { #"{"op":"release","seq":\#($0)}"# + "\n" }.joined().utf8)
        case .quit:
            return Data(#"{"op":"quit"}"#.utf8) + Data([0x0A])
        case let .update(generation, files, closed):
            struct Wire: Encodable {
                struct File: Encodable {
                    let path: String
                    let text: String
                }
                let op: String
                let generation: UInt64
                let files: [File]
                let closed: [String]
            }
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.withoutEscapingSlashes]
            let wire = Wire(op: "update", generation: generation,
                            files: files.map { Wire.File(path: $0.path, text: $0.text) }, closed: closed)
            // Strings and integers always encode.
            return ((try? encoder.encode(wire)) ?? Data()) + Data([0x0A])
        }
    }
}
