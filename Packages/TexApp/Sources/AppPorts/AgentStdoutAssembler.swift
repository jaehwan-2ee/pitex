import Foundation

/// Splits an agent stdout stream into LF-delimited lines.
/// A line with no newline is kept only up to `maxUnterminatedBytes`.
/// Past that, the partial line is dropped and bytes are ignored until the next newline.
public struct AgentStdoutAssembler: Equatable, Sendable {
    public static let defaultMaxUnterminatedBytes = 4 * 1024 * 1024

    public let maxUnterminatedBytes: Int
    public private(set) var buffered = Data()
    private var skippingToNewline = false

    public init(maxUnterminatedBytes: Int = defaultMaxUnterminatedBytes) {
        self.maxUnterminatedBytes = maxUnterminatedBytes
    }

    public mutating func append(_ chunk: Data) -> [Data] {
        var lines: [Data] = []
        var index = chunk.startIndex
        while index < chunk.endIndex {
            if skippingToNewline {
                guard let newline = chunk[index...].firstIndex(of: 0x0a) else { break }
                skippingToNewline = false
                index = chunk.index(after: newline)
                continue
            }
            guard let newline = chunk[index...].firstIndex(of: 0x0a) else {
                buffered.append(chunk[index...])
                if buffered.count > maxUnterminatedBytes {
                    buffered.removeAll(keepingCapacity: true)
                    skippingToNewline = true
                }
                break
            }
            buffered.append(chunk[index..<newline])
            index = chunk.index(after: newline)
            if buffered.count > maxUnterminatedBytes {
                buffered.removeAll(keepingCapacity: true)
                continue
            }
            var line = buffered
            buffered.removeAll(keepingCapacity: true)
            if line.last == 0x0d { line.removeLast() }
            if !line.isEmpty { lines.append(line) }
        }
        return lines
    }
}
