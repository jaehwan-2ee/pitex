import Foundation

public struct AgentStdoutAssembler: Equatable, Sendable {
    public static let defaultMaxLineBytes = 4 * 1024 * 1024

    public let maxLineBytes: Int
    public private(set) var buffered = Data()
    private var skippingToNewline = false

    public init(maxLineBytes: Int = defaultMaxLineBytes) {
        self.maxLineBytes = maxLineBytes
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
                if buffered.count > maxLineBytes {
                    buffered.removeAll(keepingCapacity: true)
                    skippingToNewline = true
                }
                break
            }
            buffered.append(chunk[index..<newline])
            index = chunk.index(after: newline)
            if buffered.count > maxLineBytes {
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
