import Foundation

/// One concrete `Host` alias from an OpenSSH client config — the entries the
/// Settings "Add SSH Connection" sheet offers. Wildcard/negated patterns
/// (`Host *`, `Host !foo`) and `Match` blocks are defaults, not devices, so
/// they never become entries.
public struct SSHHostEntry: Hashable, Codable, Sendable {
    public let alias: String
    public var hostName: String?
    public var user: String?
    public var port: Int?

    public init(alias: String, hostName: String? = nil, user: String? = nil, port: Int? = nil) {
        self.alias = alias
        self.hostName = hostName
        self.user = user
        self.port = port
    }

    /// "user@hostname:port" as ssh would resolve it — the sheet's subtitle.
    public var summary: String {
        var text = hostName ?? alias
        if let user { text = "\(user)@\(text)" }
        if let port, port != 22 { text += ":\(port)" }
        return text
    }
}

public enum SSHConfigParser {
    /// `~/.ssh/config` of the current user.
    public static var defaultConfigURL: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".ssh/config")
    }

    /// Hosts from `url`, following `Include` like ssh does (relative paths
    /// resolve against `~/.ssh`, `*`/`?` globs expand, depth-limited so an
    /// include cycle cannot loop). A missing file yields no hosts.
    public static func loadHosts(from url: URL = defaultConfigURL) -> [SSHHostEntry] {
        var hosts: [SSHHostEntry] = []
        var seen = Set<String>()
        collect(url: url, depth: 0, into: &hosts, seen: &seen)
        return hosts
    }

    /// Hosts declared in one config text; `include` resolves `Include`
    /// arguments to further config texts.
    public static func hosts(
        in text: String,
        include: (String) -> [String] = { _ in [] }
    ) -> [SSHHostEntry] {
        var hosts: [SSHHostEntry] = []
        var seen = Set<String>()
        parse(text, include: { argument in include(argument) }, into: &hosts, seen: &seen)
        return hosts
    }

    private static func collect(url: URL, depth: Int, into hosts: inout [SSHHostEntry], seen: inout Set<String>) {
        guard depth < 8, let text = try? String(contentsOf: url, encoding: .utf8) else { return }
        parse(text, include: { argument in
            expandInclude(argument).compactMap { try? String(contentsOf: $0, encoding: .utf8) }
        }, into: &hosts, seen: &seen, depth: depth)
    }

    private static func parse(
        _ text: String,
        include: (String) -> [String],
        into hosts: inout [SSHHostEntry],
        seen: inout Set<String>,
        depth: Int = 0
    ) {
        // Indexes into `hosts` the current Host block applies to; nil while
        // inside a Match block (or before any Host line: global defaults).
        var current: [Int]? = nil
        for rawLine in text.split(omittingEmptySubsequences: false, whereSeparator: \.isNewline) {
            let (keyword, arguments) = split(String(rawLine))
            guard let keyword else { continue }
            switch keyword {
            case "host":
                var indexes: [Int] = []
                for alias in arguments where isConcreteAlias(alias) {
                    if seen.insert(alias).inserted {
                        hosts.append(SSHHostEntry(alias: alias))
                        indexes.append(hosts.count - 1)
                    }
                }
                current = indexes
            case "match":
                current = nil
            case "include":
                // ssh splices the included files right here; host entries
                // they declare are hosts too.
                guard depth < 8 else { continue }
                for argument in arguments {
                    for included in include(argument) {
                        parse(included, include: include, into: &hosts, seen: &seen, depth: depth + 1)
                    }
                }
            case "hostname", "user", "port":
                // First value wins, as in ssh.
                guard let indexes = current, let value = arguments.first else { continue }
                for index in indexes {
                    switch keyword {
                    case "hostname" where hosts[index].hostName == nil: hosts[index].hostName = value
                    case "user" where hosts[index].user == nil: hosts[index].user = value
                    case "port" where hosts[index].port == nil: hosts[index].port = Int(value)
                    default: break
                    }
                }
            default:
                continue
            }
        }
    }

    /// Keyword (lowercased) and arguments of one config line. Keywords are
    /// separated from values by whitespace or `=`; `"quoted values"` keep
    /// their spaces; `#` starts a comment outside quotes.
    static func split(_ line: String) -> (String?, [String]) {
        var tokens: [String] = []
        var token = ""
        var inQuotes = false
        var hasToken = false
        for character in line {
            if inQuotes {
                if character == "\"" { inQuotes = false } else { token.append(character) }
                continue
            }
            switch character {
            case "\"":
                inQuotes = true
                hasToken = true
            case "#":
                if hasToken { tokens.append(token) }
                return finish(tokens)
            case " ", "\t":
                if hasToken { tokens.append(token); token = ""; hasToken = false }
            case "=" where (tokens.isEmpty && hasToken) || (tokens.count == 1 && !hasToken):
                // `Keyword=value` / `Keyword = value`: only the first `=`
                // after the keyword separates; later ones are literal.
                if hasToken { tokens.append(token); token = ""; hasToken = false }
            default:
                token.append(character)
                hasToken = true
            }
        }
        if hasToken { tokens.append(token) }
        return finish(tokens)
    }

    private static func finish(_ tokens: [String]) -> (String?, [String]) {
        guard let first = tokens.first else { return (nil, []) }
        return (first.lowercased(), Array(tokens.dropFirst()))
    }

    private static func isConcreteAlias(_ alias: String) -> Bool {
        !alias.isEmpty && !alias.contains(where: { "*?!".contains($0) })
    }

    /// Include paths: `~` expands, relative paths are under `~/.ssh`, and a
    /// `*`/`?` glob in the last component matches files in that directory.
    static func expandInclude(_ argument: String) -> [URL] {
        let home = FileManager.default.homeDirectoryForCurrentUser
        var path = argument
        if path.hasPrefix("~/") { path = home.path + String(path.dropFirst(1)) }
        if !path.hasPrefix("/") { path = home.appendingPathComponent(".ssh").path + "/" + path }
        let url = URL(fileURLWithPath: path)
        let pattern = url.lastPathComponent
        guard pattern.contains(where: { "*?".contains($0) }) else { return [url] }
        let directory = url.deletingLastPathComponent()
        let names = (try? FileManager.default.contentsOfDirectory(atPath: directory.path)) ?? []
        return names.sorted().filter { matches(pattern, $0) }.map { directory.appendingPathComponent($0) }
    }

    /// `*` / `?` wildcard match (no character classes — ssh configs rarely
    /// use them in Include).
    static func matches(_ pattern: String, _ name: String) -> Bool {
        let p = Array(pattern), n = Array(name)
        var memo = [[Bool?]](repeating: [Bool?](repeating: nil, count: n.count + 1), count: p.count + 1)
        func match(_ i: Int, _ j: Int) -> Bool {
            if let known = memo[i][j] { return known }
            let result: Bool
            if i == p.count {
                result = j == n.count
            } else if p[i] == "*" {
                result = match(i + 1, j) || (j < n.count && match(i, j + 1))
            } else {
                result = j < n.count && (p[i] == "?" || p[i] == n[j]) && match(i + 1, j + 1)
            }
            memo[i][j] = result
            return result
        }
        return match(0, 0)
    }
}
