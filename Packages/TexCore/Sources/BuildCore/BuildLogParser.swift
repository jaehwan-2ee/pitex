import Foundation

public struct BuildIssueRecord: Hashable, Codable, Sendable {
    public let sequence: UInt64
    public let severity: BuildIssueSeverity
    public let message: String
    public let file: String?
    public let line: Int?
    public let column: Int?

    public init(
        sequence: UInt64,
        severity: BuildIssueSeverity,
        message: String,
        file: String? = nil,
        line: Int? = nil,
        column: Int? = nil
    ) {
        self.sequence = sequence
        self.severity = severity
        self.message = message
        self.file = file
        self.line = line
        self.column = column
    }

    public var isClickable: Bool { file != nil && line != nil }

    public var issue: BuildIssue {
        get throws {
            try BuildIssue(severity: severity, message: message, file: file, line: line)
        }
    }
}

public struct BuildLogParser: Sendable {
    private let projectRoot: URL
    private var buffers: [BuildLogChannel: Data] = [:]
    private var fileStack: [String] = []
    private var parenthesisStack: [Bool] = []
    private var pendingClassicError: PendingIssue?
    private var nextSequence: UInt64 = 0

    public init(projectRoot: URL) {
        self.projectRoot = projectRoot.standardizedFileURL
    }

    public mutating func consume(
        _ bytes: Data,
        channel: BuildLogChannel
    ) -> [BuildIssueRecord] {
        // Taking the buffer out of the dictionary leaves it uniquely owned
        // here, so neither the append nor the one prefix removal copies it
        // — the per-line removeSubrange made each chunk O(buffer²).
        var buffer = buffers.removeValue(forKey: channel) ?? Data()
        buffer.append(bytes)
        var records: [BuildIssueRecord] = []
        var start = buffer.startIndex

        while let newline = buffer[start...].firstIndex(of: 0x0A) {
            var lineData = buffer[start..<newline]
            if lineData.last == 0x0D { lineData = lineData.dropLast() }
            records.append(contentsOf: parseLine(String(decoding: lineData, as: UTF8.self)))
            start = buffer.index(after: newline)
        }
        if start > buffer.startIndex { buffer.removeSubrange(..<start) }
        buffers[channel] = buffer
        return records
    }

    public mutating func consume(
        _ text: String,
        channel: BuildLogChannel
    ) -> [BuildIssueRecord] {
        consume(Data(text.utf8), channel: channel)
    }

    public mutating func finish() -> [BuildIssueRecord] {
        var records: [BuildIssueRecord] = []
        for channel in [BuildLogChannel.standardOutput, .standardError, .system] {
            if let data = buffers[channel], !data.isEmpty {
                records.append(contentsOf: parseLine(String(decoding: data, as: UTF8.self)))
            }
            buffers[channel] = nil
        }
        if let pendingClassicError {
            records.append(makeRecord(pendingClassicError))
            self.pendingClassicError = nil
        }
        return records.sorted { $0.sequence < $1.sequence }
    }

    private mutating func parseLine(_ rawLine: String) -> [BuildIssueRecord] {
        let line = escapeControls(rawLine)
        let locationFile = fileStack.last
        updateFileStack(from: line)

        if let pendingClassicError {
            if let sourceLine = classicLineNumber(in: line) {
                self.pendingClassicError = nil
                return [makeRecord(PendingIssue(
                    severity: pendingClassicError.severity,
                    message: pendingClassicError.message,
                    file: pendingClassicError.file,
                    line: sourceLine,
                    column: nil
                ))]
            }
            if isDiagnosticStart(line) {
                self.pendingClassicError = nil
                var result = [makeRecord(pendingClassicError)]
                result.append(contentsOf: parseFreshLine(line, locationFile: locationFile))
                return result
            }
            return []
        }
        return parseFreshLine(line, locationFile: locationFile)
    }

    private mutating func parseFreshLine(
        _ line: String,
        locationFile: String?
    ) -> [BuildIssueRecord] {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return [] }

        if let parsed = parseFileLineDiagnostic(trimmed) {
            return [makeRecord(parsed)]
        }
        if trimmed.hasPrefix("!") {
            let message = String(trimmed.dropFirst()).trimmingCharacters(in: .whitespaces)
            guard !message.isEmpty else { return [] }
            pendingClassicError = PendingIssue(
                severity: .error,
                message: message,
                file: normalizedDisplayPath(locationFile),
                line: nil,
                column: nil
            )
            return []
        }
        if let diagnostic = parseToolDiagnostic(trimmed, locationFile: locationFile) {
            return [makeRecord(diagnostic)]
        }
        return []
    }

    private func parseFileLineDiagnostic(_ line: String) -> PendingIssue? {
        var search = line.startIndex
        while let colon = line[search...].firstIndex(of: ":") {
            let afterColon = line.index(after: colon)
            var digitsEnd = afterColon
            while digitsEnd < line.endIndex, line[digitsEnd].isNumber {
                digitsEnd = line.index(after: digitsEnd)
            }
            guard digitsEnd > afterColon, digitsEnd < line.endIndex, line[digitsEnd] == ":" else {
                search = afterColon
                continue
            }
            let file = String(line[..<colon])
            guard !file.isEmpty, let lineNumber = Int(line[afterColon..<digitsEnd]), lineNumber > 0 else {
                search = afterColon
                continue
            }

            var messageStart = line.index(after: digitsEnd)
            var column: Int?
            var columnEnd = messageStart
            while columnEnd < line.endIndex, line[columnEnd].isNumber {
                columnEnd = line.index(after: columnEnd)
            }
            if columnEnd > messageStart, columnEnd < line.endIndex, line[columnEnd] == ":" {
                column = Int(line[messageStart..<columnEnd])
                messageStart = line.index(after: columnEnd)
            }
            let message = String(line[messageStart...]).trimmingCharacters(in: .whitespaces)
            guard !message.isEmpty else { return nil }
            return PendingIssue(
                severity: severity(for: message),
                message: message,
                file: normalizedDisplayPath(file),
                line: lineNumber,
                column: column.flatMap { $0 > 0 ? $0 : nil }
            )
        }
        return nil
    }

    private func parseToolDiagnostic(
        _ line: String,
        locationFile: String?
    ) -> PendingIssue? {
        let lower = line.lowercased()
        let severity: BuildIssueSeverity?
        if lower.hasPrefix("warning--") || lower.contains(" warning:") || lower.hasPrefix("warning:") || lower.contains("warning (file") || lower.contains("overfull \\hbox") || lower.contains("underfull \\hbox") {
            severity = .warning
        } else if lower.hasPrefix("error:") || lower.hasPrefix("error--") || lower.hasPrefix("!!") || lower.contains("error message") || lower.contains("not found") || lower.contains("couldn't open") || lower.contains("cannot open") {
            severity = .error
        } else {
            severity = nil
        }
        guard let severity else { return nil }

        let boxLine = firstPositiveInteger(after: "at lines ", in: lower)
            ?? firstPositiveInteger(after: "at line ", in: lower)
        return PendingIssue(
            severity: severity,
            message: line,
            file: normalizedDisplayPath(locationFile),
            line: boxLine,
            column: nil
        )
    }

    private func severity(for message: String) -> BuildIssueSeverity {
        let lower = message.lowercased()
        return lower.contains("warning") || lower.contains("overfull") || lower.contains("underfull") ? .warning : .error
    }

    private func firstPositiveInteger(after marker: String, in text: String) -> Int? {
        guard let range = text.range(of: marker) else { return nil }
        let suffix = text[range.upperBound...]
        let digits = suffix.prefix(while: { $0.isNumber })
        guard let value = Int(digits), value > 0 else { return nil }
        return value
    }

    private func classicLineNumber(in line: String) -> Int? {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        guard trimmed.hasPrefix("l.") else { return nil }
        let digits = trimmed.dropFirst(2).prefix(while: { $0.isNumber })
        guard let value = Int(digits), value > 0 else { return nil }
        return value
    }

    private func isDiagnosticStart(_ line: String) -> Bool {
        let trimmed = line.trimmingCharacters(in: .whitespaces)
        return trimmed.hasPrefix("!") || parseFileLineDiagnostic(trimmed) != nil || parseToolDiagnostic(trimmed, locationFile: nil) != nil
    }

    private mutating func makeRecord(_ pending: PendingIssue) -> BuildIssueRecord {
        defer { nextSequence += 1 }
        return BuildIssueRecord(
            sequence: nextSequence,
            severity: pending.severity,
            message: pending.message,
            file: pending.file,
            line: pending.line,
            column: pending.column
        )
    }

    private mutating func updateFileStack(from line: String) {
        var index = line.startIndex
        while index < line.endIndex {
            let character = line[index]
            if character == "(" {
                let start = line.index(after: index)
                var cursor = start
                var fileEnd: String.Index?
                while cursor < line.endIndex, line[cursor] != ")", line[cursor] != "(" {
                    cursor = line.index(after: cursor)
                    if isTeXFileToken(String(line[start..<cursor])) {
                        fileEnd = cursor
                        break
                    }
                }
                let token = fileEnd.map { String(line[start..<$0]) }
                let isFile = token != nil
                parenthesisStack.append(isFile)
                if let token {
                    fileStack.append(normalizedDisplayPath(token) ?? token)
                }
                index = fileEnd ?? start
                continue
            }
            if character == ")", let openedFile = parenthesisStack.popLast(), openedFile {
                _ = fileStack.popLast()
            }
            index = line.index(after: index)
        }
    }

    private func isTeXFileToken(_ token: String) -> Bool {
        let lower = token.lowercased()
        return lower.hasSuffix(".tex") || lower.hasSuffix(".sty") || lower.hasSuffix(".cls") || lower.hasSuffix(".bib") || lower.hasSuffix(".idx")
    }

    private func normalizedDisplayPath(_ path: String?) -> String? {
        guard let path, !path.isEmpty else { return nil }
        let candidate: URL
        if path.hasPrefix("/") {
            candidate = URL(fileURLWithPath: path).standardizedFileURL
        } else {
            candidate = projectRoot.appendingPathComponent(path).standardizedFileURL
        }
        let rootPath = projectRoot.path == "/" ? "/" : projectRoot.path + "/"
        guard candidate.path == projectRoot.path || candidate.path.hasPrefix(rootPath) else {
            return path
        }
        if candidate.path == projectRoot.path { return "." }
        return String(candidate.path.dropFirst(rootPath.count))
    }

    private func escapeControls(_ text: String) -> String {
        var escaped = ""
        for scalar in text.unicodeScalars {
            if scalar.value < 0x20 && scalar != "\t" || scalar.value == 0x7F {
                escaped += "\\u{" + String(scalar.value, radix: 16, uppercase: true) + "}"
            } else {
                escaped.unicodeScalars.append(scalar)
            }
        }
        return escaped
    }

    private struct PendingIssue: Sendable {
        let severity: BuildIssueSeverity
        let message: String
        let file: String?
        let line: Int?
        let column: Int?
    }
}
