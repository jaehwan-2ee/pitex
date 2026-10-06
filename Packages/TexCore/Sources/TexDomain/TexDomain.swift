public enum TexDomainValidationError: Error, Equatable, Sendable {
    case emptyIdentifier
    case identifierTooLong(maximum: Int)
    case invalidIdentifierCharacter
    case emptyPath
    case absolutePath
    case pathEscapesRoot
    case invalidPathCharacter
    case pathComponentTooLong(maximum: Int)
}

private func validateStableIdentifier(_ value: String) throws {
    guard !value.isEmpty else {
        throw TexDomainValidationError.emptyIdentifier
    }
    guard value.utf8.count <= 128 else {
        throw TexDomainValidationError.identifierTooLong(maximum: 128)
    }
    guard value.utf8.allSatisfy({ byte in
        (byte >= 48 && byte <= 57)
            || (byte >= 65 && byte <= 90)
            || (byte >= 97 && byte <= 122)
            || byte == 45
            || byte == 95
    }) else {
        throw TexDomainValidationError.invalidIdentifierCharacter
    }
}

public struct StableProjectID: Hashable, Codable, Sendable, Comparable {
    public let rawValue: String

    public init(rawValue: String) throws {
        try validateStableIdentifier(rawValue)
        self.rawValue = rawValue
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.singleValueContainer()
        try self.init(rawValue: container.decode(String.self))
    }

    public func encode(to encoder: any Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(rawValue)
    }

    public static func < (lhs: Self, rhs: Self) -> Bool {
        lhs.rawValue < rhs.rawValue
    }
}

public struct StableDocumentID: Hashable, Codable, Sendable, Comparable {
    public let rawValue: String

    public init(rawValue: String) throws {
        try validateStableIdentifier(rawValue)
        self.rawValue = rawValue
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.singleValueContainer()
        try self.init(rawValue: container.decode(String.self))
    }

    public func encode(to encoder: any Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(rawValue)
    }

    public static func < (lhs: Self, rhs: Self) -> Bool {
        lhs.rawValue < rhs.rawValue
    }
}

public struct NormalizedRelativePath: Hashable, Codable, Sendable, Comparable {
    public let rawValue: String

    public init(rawValue: String) throws {
        guard !rawValue.isEmpty else {
            throw TexDomainValidationError.emptyPath
        }
        guard !rawValue.hasPrefix("/"), !rawValue.hasPrefix("\\") else {
            throw TexDomainValidationError.absolutePath
        }

        let originalComponents = rawValue.split(
            separator: "/",
            omittingEmptySubsequences: false
        )
        if let first = originalComponents.first,
           first.count >= 2,
           first[first.index(after: first.startIndex)] == ":" {
            throw TexDomainValidationError.absolutePath
        }

        var normalizedComponents: [Substring] = []
        for component in originalComponents {
            if component.isEmpty || component == "." {
                continue
            }
            if component == ".." {
                guard !normalizedComponents.isEmpty else {
                    throw TexDomainValidationError.pathEscapesRoot
                }
                normalizedComponents.removeLast()
                continue
            }
            guard component.utf8.count <= 255 else {
                throw TexDomainValidationError.pathComponentTooLong(maximum: 255)
            }
            guard !component.contains("\\"), !component.contains("\0") else {
                throw TexDomainValidationError.invalidPathCharacter
            }
            normalizedComponents.append(component)
        }

        guard !normalizedComponents.isEmpty else {
            throw TexDomainValidationError.emptyPath
        }
        self.rawValue = normalizedComponents.joined(separator: "/")
    }

    public init(from decoder: any Decoder) throws {
        let container = try decoder.singleValueContainer()
        try self.init(rawValue: container.decode(String.self))
    }

    public func encode(to encoder: any Encoder) throws {
        var container = encoder.singleValueContainer()
        try container.encode(rawValue)
    }

    public static func < (lhs: Self, rhs: Self) -> Bool {
        lhs.rawValue < rhs.rawValue
    }
}
