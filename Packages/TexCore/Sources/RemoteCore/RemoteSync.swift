import Foundation
#if canImport(Darwin)
import Darwin
#elseif canImport(Glibc)
import Glibc
#endif

/// A folder on a remote device, as opened through "Open via SSH".
public struct RemoteProject: Codable, Hashable, Sendable {
    public var connection: SSHConnection
    /// Absolute, symlink-resolved remote path (`pwd -P`).
    public var remoteRoot: String

    public init(connection: SSHConnection, remoteRoot: String) {
        self.connection = connection
        self.remoteRoot = remoteRoot
    }

    public var folderName: String {
        let name = (remoteRoot as NSString).lastPathComponent
        return name.isEmpty || name == "/" ? "root" : name
    }
}

/// The local working copy of a remote folder. Pitex edits, previews and
/// indexes this directory like any local project; `RemoteSync` keeps it
/// and the remote folder in step. Layout under the store — the project
/// sits alone under `project/`, so no folder name can collide with the
/// bookkeeping beside it:
///
///     <store>/<slug>/remote.json              — which device/folder this mirrors
///     <store>/<slug>/manifest.json            — content hashes at the last sync
///     <store>/<slug>/work/                    — transfer staging
///     <store>/<slug>/project/<folder name>/   — the project root the editor opens
public struct RemoteMirror: Sendable, Hashable {
    public let directory: URL
    public let project: RemoteProject

    public var root: URL {
        directory.appendingPathComponent("project", isDirectory: true)
            .appendingPathComponent(project.folderName, isDirectory: true)
    }
    var metadataURL: URL { directory.appendingPathComponent("remote.json") }
    var manifestURL: URL { directory.appendingPathComponent("manifest.json") }
    var workURL: URL { directory.appendingPathComponent("work", isDirectory: true) }
    var stagingURL: URL { workURL.appendingPathComponent("staging", isDirectory: true) }

    /// `~/Library/Application Support/Pitex/Remote` on macOS.
    public static var defaultStore: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("Pitex/Remote", isDirectory: true)
    }

    /// The mirror for `project`, created (with its metadata) on first use.
    public static func prepare(for project: RemoteProject, store: URL = defaultStore) throws -> RemoteMirror {
        let key = [project.connection.destination, project.connection.user ?? "",
                   String(project.connection.port ?? 22), project.remoteRoot].joined(separator: "\u{0}")
        let digest = String(SHA256.hex(Data(key.utf8)).prefix(12))
        let slug = "\(sanitized(project.connection.name))-\(sanitized(project.folderName))-\(digest)"
        let mirror = RemoteMirror(directory: store.appendingPathComponent(slug, isDirectory: true), project: project)
        try FileManager.default.createDirectory(at: mirror.root, withIntermediateDirectories: true)
        let data = try JSONEncoder().encode(project)
        try data.write(to: mirror.metadataURL, options: .atomic)
        return mirror
    }

    /// The mirror containing `url`, if `url` lies inside one — how a recent
    /// or re-opened path is recognized as a remote project.
    public static func containing(_ url: URL, store: URL = defaultStore) -> RemoteMirror? {
        let storePath = store.standardizedFileURL.resolvingSymlinksInPath().path
        let path = url.standardizedFileURL.resolvingSymlinksInPath().path
        guard path.hasPrefix(storePath + "/") else { return nil }
        let rest = path.dropFirst(storePath.count + 1)
        guard let slug = rest.split(separator: "/").first else { return nil }
        let directory = URL(fileURLWithPath: storePath).appendingPathComponent(String(slug), isDirectory: true)
        guard let data = try? Data(contentsOf: directory.appendingPathComponent("remote.json")),
              let project = try? JSONDecoder().decode(RemoteProject.self, from: data) else { return nil }
        let mirror = RemoteMirror(directory: directory, project: project)
        let rootPath = mirror.root.path
        guard path == rootPath || path.hasPrefix(rootPath + "/") else { return nil }
        return mirror
    }

    private static func sanitized(_ text: String) -> String {
        let allowed = CharacterSet.alphanumerics.union(CharacterSet(charactersIn: "._-"))
        let mapped = String(text.unicodeScalars.map { allowed.contains($0) ? Character($0) : "-" })
        return String(mapped.prefix(40))
    }

    /// Reverse of `remotePath(for:)` for git output: the device path
    /// `toplevel` + "/" + `path` (a repo root from `rev-parse
    /// --show-toplevel` plus a repo-relative porcelain path) mapped into
    /// this mirror — `root / relative(joined, from: remoteRoot)`. `nil`
    /// when the repo extends above `remoteRoot` (the opened project is a
    /// subfolder of the repo) and the change points outside the mirror —
    /// the file still lists in the pane but has no local copy to open.
    public func localURL(toplevel: String, path: String) -> URL? {
        func stripTrailingSlashes(_ value: String) -> String {
            var value = value
            while value.hasSuffix("/") { value.removeLast() }
            return value
        }
        let remote = stripTrailingSlashes(Self.normalizedGitDevicePath(project.remoteRoot))
        let top = stripTrailingSlashes(Self.normalizedGitDevicePath(toplevel))
        let joined = Self.normalizedGitDevicePath(top + "/" + path)
        // Component-boundary check — a plain `hasPrefix` would let
        // `/repo-other/…` pass for remoteRoot `/repo`.
        guard joined.hasPrefix(remote) else { return nil }
        let rest = joined.dropFirst(remote.count)
        guard rest.hasPrefix("/") else { return nil }
        let relative = rest.dropFirst()
        guard !relative.isEmpty, RemoteSyncRules.isSafeRelativePath(String(relative)) else { return nil }
        var url = root
        for component in relative.split(separator: "/") {
            url.appendPathComponent(String(component), isDirectory: false)
        }
        return url
    }

    /// Lexical POSIX normalization for device paths git reports
    /// (`rev-parse --show-toplevel`, `remoteRoot`): collapses `//`,
    /// resolves `.` and `..`, strips the trailing `/`. git always emits
    /// `/`-separated paths; the remote filesystem is never touched.
    static func normalizedGitDevicePath(_ path: String) -> String {
        let absolute = path.hasPrefix("/")
        var out: [Substring] = []
        for component in path.split(separator: "/", omittingEmptySubsequences: true) {
            if component == "." { continue }
            if component == ".." {
                _ = out.popLast()
                continue
            }
            out.append(component)
        }
        return (absolute ? "/" : "") + out.joined(separator: "/")
    }
}

/// Which files are mirrored, applied identically on both sides: VCS and
/// dependency trees, macOS litter, Pitex's own atomic-save temp files and
/// anything ≥ 50 MB stay out.
public enum RemoteSyncRules {
    public static let prunedNames: Set<String> = [
        ".git", ".svn", ".hg", "node_modules", ".venv", "venv", "__pycache__", ".DS_Store", ".Trash",
    ]
    public static let maximumFileSize = 50 * 1024 * 1024

    public static func isExcluded(name: String) -> Bool {
        prunedNames.contains(name) || name.hasPrefix(".pitex-upload.")
            || (name.hasPrefix(".") && name.contains(".texspark-") && name.hasSuffix(".tmp"))
    }

    /// A relative path from the remote is only ever used inside the mirror:
    /// no absolute paths, `..`, empty or control-character components.
    public static func isSafeRelativePath(_ path: String) -> Bool {
        guard !path.isEmpty, !path.hasPrefix("/") else { return false }
        return path.split(separator: "/", omittingEmptySubsequences: false).allSatisfy { component in
            !component.isEmpty && component != "." && component != ".."
                && !component.unicodeScalars.contains { CharacterSet.controlCharacters.contains($0) }
        }
    }
}

/// Pure pull decisions over three hash maps (path → SHA-256): the remote
/// folder, the manifest (both sides at the last sync) and the local mirror.
/// Push needs no plan here: the device decides per file at the moment of
/// replacement (`RemoteScripts.commitUpload`).
public enum SyncPlanner {
    public struct PullPlan: Equatable, Sendable {
        /// Remote changed, local untouched → download.
        public var download: [String] = []
        /// Both sides already equal → record in the manifest only.
        public var adopt: [String: String] = [:]
        /// Missing from the remote listing, local untouched → delete locally
        /// once the device confirms the file is really gone.
        public var delete: [String] = []
        /// Changed on both sides → keep local, report.
        public var conflicts: [String] = []
    }

    public static func pull(remote: [String: String], manifest: [String: String], local: [String: String]) -> PullPlan {
        var plan = PullPlan()
        for (path, remoteHash) in remote {
            let base = manifest[path], mine = local[path]
            if remoteHash == base { continue }
            if mine == remoteHash {
                plan.adopt[path] = remoteHash
            } else if mine == base {
                // Untouched locally (or absent both locally and in the
                // manifest: a brand-new remote file).
                plan.download.append(path)
            } else {
                plan.conflicts.append(path)
            }
        }
        for (path, base) in manifest where remote[path] == nil {
            if local[path] == nil || local[path] == base {
                plan.delete.append(path)
            } else {
                plan.conflicts.append(path)
            }
        }
        plan.download.sort(); plan.delete.sort(); plan.conflicts.sort()
        return plan
    }
}

public struct SyncReport: Equatable, Sendable {
    public var downloaded: [String] = []
    public var uploaded: [String] = []
    public var deleted: [String] = []
    /// Every path of this mirror currently changed on both sides (not just
    /// those found by this operation).
    public var conflicts: [String] = []

    public init() {}
    public var changedLocally: Bool { !downloaded.isEmpty || !deleted.isEmpty }
}

/// What the device holds at a path, as `RemoteScripts.probe` reports it.
enum RemoteFileState: Equatable, Sendable {
    case file(String)
    /// Confirmed absent: the nearest existing ancestor is a searchable
    /// directory.
    case absent
    /// Exists but cannot be hashed (unreadable, a directory, a symlink) or
    /// its absence cannot be confirmed.
    case unavailable
}

/// Lets the app hold its own editor saves back while sync replaces or
/// deletes mirror files, so a save can never land between sync's "is this
/// file still untouched?" check and the replacement.
public protocol MirrorWriteGate: Sendable {
    /// Returns once no app write is in flight; new ones wait for `endSyncCommit`.
    func beginSyncCommit() async
    func endSyncCommit() async
}

/// The POSIX sh scripts run on the remote. Single-line so a csh/tcsh login
/// shell can pass them through; every path arrives as a positional
/// parameter (`$1`…) or on stdin, never as script text.
enum RemoteScripts {
    private static let hashCommand =
        #"if command -v sha256sum >/dev/null 2>&1; then h=sha256sum; else h='shasum -a 256'; fi"#

    private static var prune: String {
        let names = RemoteSyncRules.prunedNames.sorted().map { "-name \(SSHClient.quote($0))" }
            + ["-name '.*.texspark-*.tmp'", "-name '.pitex-upload.*'"]
        return #"\( "# + names.joined(separator: " -o ") + #" \) -prune"#
    }

    /// `a PATH` checks PATH's parent components top-down: 0 = all real,
    /// searchable directories; 2 = one is missing below real ones (so PATH
    /// is absent); 1 = a symlink, non-directory or unsearchable directory
    /// is in the way — nothing about PATH can be concluded or written.
    private static let ancestry =
        #"a() { q=.; z=$1; while :; do case $z in */*) q=$q/${z%%/*}; z=${z#*/} ;; *) return 0 ;; esac; if [ -L "$q" ]; then return 1; elif [ -d "$q" ]; then [ -x "$q" ] || return 1; elif [ -e "$q" ]; then return 1; else return 2; fi; done; }"#

    /// `hash  ./path` for every mirrored file under $1. A file missing here
    /// is only a deletion candidate (`probe` decides).
    static var hashTree: String {
        #"cd -- "$1" || exit 3; "# + hashCommand
            + "; find . \(prune) -o -type f -size -\(RemoteSyncRules.maximumFileSize)c -print0 | xargs -0 $h"
    }

    /// For each newline-separated relative path on stdin: `H<hash>/path`,
    /// `A/path` (confirmed absent) or `E/path` (not a plain file below real
    /// directories, unreadable, or absence not confirmable).
    static var probe: String {
        #"cd -- "$1" || exit 3; "# + hashCommand + "; " + ancestry
            + #"; while IFS= read -r p; do a "$p"; v=$?; if [ $v = 2 ]; then printf 'A/%s\n' "$p"; elif [ $v = 1 ] || [ -L "$p" ] || [ -d "$p" ]; then printf 'E/%s\n' "$p"; elif [ -e "$p" ]; then if c=$($h < "$p"); then printf 'H%s/%s\n' "${c%% *}" "$p"; else printf 'E/%s\n' "$p"; fi; else printf 'A/%s\n' "$p"; fi; done"#
    }

    /// Commits an upload: the archive on stdin holds `expect` (lines of
    /// `BASE NEW path`, `-` = absent) and `data/<path>`. Prints per path
    /// `U/` (placed), `S/` (already NEW), `C/` (conflict), `E/` (not
    /// placeable) or `L/` (a hard link was refused in a writable folder —
    /// no hard links there, or no space/quota — so it cannot be done
    /// safely). The live path is never missing or partially written:
    ///
    /// - the archive is unpacked into a private directory first, so nothing
    ///   changes until it arrived complete;
    /// - a lock directory (owner PID inside; stale once that process is
    ///   gone) serializes Pitex clients;
    /// - `a` must find only real directories above the path;
    /// - an existing file gets a backup hard link and is hashed; it is
    ///   replaced by one rename(2), only if it is still BASE and still that
    ///   same file. A write into the old file during that instant survives
    ///   as `<path>.pitex-conflict-<pid>`;
    /// - a new file is published complete by link(2), which never replaces
    ///   anything that appeared meanwhile (a directory that appeared is
    ///   caught by the `-ef` check and our stray link removed).
    ///
    /// ponytail: a writer that renames its own file over the path between
    /// the `-ef` check and our rename (microseconds) is still replaced —
    /// closing that needs renameat2/renamex_np, which sh cannot reach.
    static var commitUpload: String {
        let setup = [
            #"cd -- "$1" || exit 3"#, hashCommand, ancestry,
            #"t=$(mktemp -d ./.pitex-upload.XXXXXX) || exit 4"#,
            "w=; o=",
            #"trap 'rm -f -- ${w:+"$w"} ${o:+"$o"}; rm -rf "$t"; [ "$(cat ./.pitex-upload.lock/pid 2>/dev/null)" = "$$" ] && rm -rf ./.pitex-upload.lock' EXIT"#,
            "trap 'exit 1' HUP INT TERM",
            #"tar -xf - -C "$t" || exit 5"#,
            #"[ -f "$t/expect" ] || exit 5"#,
            #"k=0; until mkdir ./.pitex-upload.lock 2>/dev/null; do x=$(cat ./.pitex-upload.lock/pid 2>/dev/null); if [ -n "$x" ]; then ps -p "$x" >/dev/null 2>&1 || { rm -rf ./.pitex-upload.lock; continue; }; fi; k=$((k+1)); [ $k -lt 60 ] || exit 6; sleep 1; done"#,
            #"echo $$ > ./.pitex-upload.lock/pid"#,
        ]
        let perFile = [
            #"b=${l%% *}; r=${l#* }; n=${r%% *}; p=${r#* }"#,
            #"a "$p"; v=$?"#,
            #"if [ $v = 1 ]; then printf 'E/%s\n' "$p"; continue; fi"#,
            #"if [ -L "$p" ] || [ -d "$p" ]; then printf 'C/%s\n' "$p"; continue; fi"#,
            #"case $p in */*) d=${p%/*} ;; *) d=. ;; esac"#,
            #"if [ $v = 2 ]; then mkdir -p -- "$d" || { printf 'E/%s\n' "$p"; continue; }; fi"#,
            // Beside the target, so the final rename stays on one filesystem.
            #"w=$d/.pitex-upload.new.$$; o=$d/.pitex-upload.old.$$"#,
            #"mv -f -- "$t/data/$p" "$w" || { w=; printf 'E/%s\n' "$p"; continue; }"#,
            // `f`: why a link could not be made — no write access (E), or
            // refused in a writable folder (L: no hard links, space, quota).
            #"f() { if [ -w "$d" ]; then printf 'L/%s\n' "$p"; else printf 'E/%s\n' "$p"; fi; }"#,
            #"if [ -e "$p" ]; then ln -f -- "$p" "$o" 2>/dev/null || { f; o=; rm -f -- "$w"; w=; continue; }; c=$($h < "$o") || c=x; c=${c%% *}; if [ "$c" = "$n" ]; then printf 'S/%s\n' "$p"; elif [ "$c" != "$b" ]; then printf 'C/%s\n' "$p"; elif [ ! "$p" -ef "$o" ]; then printf 'C/%s\n' "$p"; elif mv -f -- "$w" "$p"; then e=$($h < "$o") || e=x; if [ "${e%% *}" != "$c" ]; then mv -f -- "$o" "$p.pitex-conflict-$$"; o=; printf 'C/%s\n' "$p"; else printf 'U/%s\n' "$p"; fi; else printf 'E/%s\n' "$p"; fi"#,
            #"elif [ "$b" != - ]; then printf 'C/%s\n' "$p"; elif ln -- "$w" "$p" 2>/dev/null; then if [ "$p" -ef "$w" ]; then printf 'U/%s\n' "$p"; else rm -f -- "$p/${w##*/}"; printf 'C/%s\n' "$p"; fi; elif [ -e "$p" ] || [ -L "$p" ]; then printf 'C/%s\n' "$p"; else f; fi"#,
            #"rm -f -- ${w:+"$w"} ${o:+"$o"}; w=; o="#,
        ]
        return setup.joined(separator: "; ")
            + #"; while IFS= read -r l; do "# + perFile.joined(separator: "; ")
            + #"; done < "$t/expect""#
    }

    /// A tar of the NUL-separated relative paths on stdin that exist.
    static var tarOut: String {
        #"cd -- "$1" || exit 3; tar -cf - --null -T -"#
    }

    /// `pwd -P`, then `D/name` / `F/name` for each visible entry of $1
    /// (empty = home).
    static var listDirectory: String {
        #"if [ -n "$1" ]; then cd -- "$1" || exit 3; else cd || exit 3; fi; pwd -P; for f in *; do [ -e "$f" ] || continue; if [ -d "$f" ]; then printf 'D/%s\n' "$f"; else printf 'F/%s\n' "$f"; fi; done"#
    }

    /// Runs "$2" "$3"… in $1 through the user's login shell, so PATH comes
    /// from their profile (TeX installs usually add themselves there), with
    /// the standard TeX locations appended as a fallback. The fallback is
    /// computed here in /bin/sh and handed over in a variable: the login
    /// shell may be zsh, which aborts on an unmatched glob.
    static var loginExec: String {
        #"cd -- "$1" || exit 127; shift; t=/Library/TeX/texbin:/usr/texbin:/opt/homebrew/bin:/usr/local/bin; for d in /usr/local/texlive/*/bin/*; do [ -d "$d" ] && t="$t:$d"; done; PITEX_TEX_PATH=$t; export PITEX_TEX_PATH; PATH="$PATH:$t"; export PATH; case "${SHELL##*/}" in bash|zsh|sh|dash|ksh) s=$SHELL ;; *) s=/bin/sh ;; esac; exec "$s" -lc 'PATH="$PATH:$PITEX_TEX_PATH"; export PATH; exec "$@"' pitex "$@""#
    }

    /// Parses `sha256sum`/`shasum` lines into relative path → hash. Lines
    /// for escaped names (leading `\`), stdin (`-`) and unsafe paths drop.
    static func parseHashes(_ output: String) -> [String: String] {
        var hashes: [String: String] = [:]
        for line in output.split(separator: "\n") {
            guard line.count > 66, !line.hasPrefix("\\") else { continue }
            let hash = line.prefix(64)
            guard hash.allSatisfy(\.isHexDigit) else { continue }
            var path = String(line.dropFirst(66))
            if path.hasPrefix("./") { path.removeFirst(2) }
            // `-` is the hasher reading an empty stdin (xargs with no input).
            guard path != "-", RemoteSyncRules.isSafeRelativePath(path) else { continue }
            hashes[path] = hash.lowercased()
        }
        return hashes
    }

    static func parseProbe(_ output: String) -> [String: RemoteFileState] {
        var states: [String: RemoteFileState] = [:]
        for line in output.split(separator: "\n") {
            if line.hasPrefix("A/") {
                states[String(line.dropFirst(2))] = .absent
            } else if line.hasPrefix("E/") {
                states[String(line.dropFirst(2))] = .unavailable
            } else if line.hasPrefix("H"), line.count > 66, line.dropFirst(65).first == "/" {
                let hash = line.dropFirst().prefix(64)
                guard hash.allSatisfy(\.isHexDigit) else { continue }
                states[String(line.dropFirst(66))] = .file(hash.lowercased())
            }
        }
        return states
    }
}

/// One remote directory listing for the folder browser.
public struct RemoteDirectoryListing: Equatable, Sendable {
    public let path: String
    public let folders: [String]
    public let files: [String]

    static func parse(_ output: String) -> RemoteDirectoryListing? {
        var lines = output.split(separator: "\n", omittingEmptySubsequences: false).map(String.init)
        guard let path = lines.first, path.hasPrefix("/") else { return nil }
        lines.removeFirst()
        var folders: [String] = [], files: [String] = []
        for line in lines where line.count > 2 {
            let name = String(line.dropFirst(2))
            if line.hasPrefix("D/") { folders.append(name) } else if line.hasPrefix("F/") { files.append(name) }
        }
        let order: (String, String) -> Bool = { $0.localizedStandardCompare($1) == .orderedAscending }
        return RemoteDirectoryListing(path: path, folders: folders.sorted(by: order), files: files.sorted(by: order))
    }
}

extension SSHClient {
    /// Lists `path` on the remote; empty or `~` means the home directory,
    /// `~/x` is relative to it.
    public func listDirectory(_ path: String) async throws -> RemoteDirectoryListing {
        var target = path.trimmingCharacters(in: .whitespaces)
        if target == "~" { target = "" } else if target.hasPrefix("~/") { target.removeFirst(2) }
        let result = try await run(RemoteScripts.listDirectory, arguments: [target])
        guard result.status == 0, let listing = RemoteDirectoryListing.parse(result.stdoutText) else {
            throw SSHError.remote(status: result.status, message: "\(path) is not a readable folder on \(connection.name).")
        }
        return listing
    }

    /// The login-shell path of `tool` on the remote, or nil.
    public func which(_ tool: String) async throws -> String? {
        let result = try await run(RemoteScripts.loginExec, arguments: [".", "/bin/sh", "-c", #"command -v "$1""#, "sh", tool])
        let path = result.stdoutText.trimmingCharacters(in: .whitespacesAndNewlines)
        return result.status == 0 && !path.isEmpty ? path.split(separator: "\n").last.map(String.init) : nil
    }
}

/// Keeps one mirror and its remote folder in step. Pull brings remote
/// changes down without touching files edited locally since the last sync;
/// push uploads local edits only where the remote is still what was last
/// synced. Anything changed on both sides is reported, never overwritten.
public actor RemoteSync {
    /// Immutable and Sendable — explicitly nonisolated so other modules
    /// (the app targets `Mac/`) can read them synchronously; inside the
    /// module `let` already is.
    public nonisolated let mirror: RemoteMirror
    public nonisolated let client: SSHClient
    private let gate: (any MirrorWriteGate)?
    private var manifest: [String: String]
    /// Paths changed on both sides, kept here so every window sharing the
    /// engine sees one list, and entries leave it once the sides agree.
    private var conflicts: Set<String> = []
    /// Local hash cache keyed by (size, mtime) so a push does not re-hash
    /// every file in the mirror.
    private var localCache: [String: (size: Int, modified: Date, hash: String)] = [:]
    /// Pull/push/fetch/resolve run one at a time — they read and write the
    /// manifest and the staging area across suspension points.
    private var busy = false
    private var waiters: [CheckedContinuation<Void, Never>] = []

    private func acquire() async {
        if busy {
            await withCheckedContinuation { waiters.append($0) }
        } else {
            busy = true
        }
    }

    private func release() {
        if waiters.isEmpty { busy = false } else { waiters.removeFirst().resume() }
    }

    public init(mirror: RemoteMirror, client: SSHClient, gate: (any MirrorWriteGate)? = nil) {
        self.mirror = mirror
        self.client = client
        self.gate = gate
        manifest = (try? Data(contentsOf: mirror.manifestURL))
            .flatMap { try? JSONDecoder().decode([String: String].self, from: $0) } ?? [:]
    }

    private final class Registry: @unchecked Sendable {
        let lock = NSLock()
        var engines: [String: RemoteSync] = [:]
    }
    private static let registry = Registry()

    /// The one engine for `mirror` in this process: windows showing the
    /// same remote folder share its manifest, staging area and ordering.
    public static func shared(for mirror: RemoteMirror, gate: (any MirrorWriteGate)? = nil) -> RemoteSync {
        let key = mirror.directory.standardizedFileURL.path
        registry.lock.lock()
        defer { registry.lock.unlock() }
        if let engine = registry.engines[key] { return engine }
        let engine = RemoteSync(mirror: mirror, client: SSHClient(connection: mirror.project.connection), gate: gate)
        registry.engines[key] = engine
        return engine
    }

    private var remoteRoot: String { mirror.project.remoteRoot }
    private var deviceName: String { mirror.project.connection.name }

    /// Remote → local.
    @discardableResult
    public func pull() async throws -> SyncReport {
        await acquire()
        defer { release() }
        let listing = try await client.runChecked(RemoteScripts.hashTree, arguments: [remoteRoot])
        let remote = RemoteScripts.parseHashes(listing.stdoutText)
        var local: [String: String] = [:]
        for path in Set(remote.keys).union(manifest.keys).union(conflicts) {
            if let hash = localHash(path) { local[path] = hash }
        }
        let plan = SyncPlanner.pull(remote: remote, manifest: manifest, local: local)
        var report = SyncReport()
        // Sides that agree again (even back on the old baseline) are settled.
        for path in conflicts where local[path] == remote[path] { conflicts.remove(path) }
        conflicts.formUnion(plan.conflicts)
        for (path, hash) in plan.adopt {
            manifest[path] = hash
            conflicts.remove(path)
        }
        defer { try? FileManager.default.removeItem(at: mirror.stagingURL) }
        let staged = plan.download.isEmpty ? [:] : try await download(plan.download)
        // Missing from the listing is not proof of deletion (too large,
        // unreadable directory…): only a confirmed absence deletes.
        let gone = plan.delete.isEmpty ? [] : try await probe(plan.delete).filter { $0.value == .absent }.keys.sorted()
        var failed: [String] = []
        try await commitLocally {
            for path in plan.download {
                guard let file = staged[path] else { continue }
                // Re-check under the gate: an edit saved while the download
                // ran makes this a conflict instead of a loss.
                guard localHash(path) == local[path] else {
                    conflicts.insert(path)
                    continue
                }
                do { try place(file.url, at: path) } catch { failed.append(path); continue }
                manifest[path] = file.hash
                conflicts.remove(path)
                report.downloaded.append(path)
            }
            for path in gone {
                if let url = containedURL(path, create: false), itemExists(url) {
                    guard localHash(path) == manifest[path] else { conflicts.insert(path); continue }
                    do { try FileManager.default.removeItem(at: url) } catch { failed.append(path); continue }
                }
                manifest[path] = nil
                localCache[path] = nil
                conflicts.remove(path)
                report.deleted.append(path)
            }
            try saveManifest()
        }
        report.conflicts = conflicts.sorted()
        if !failed.isEmpty {
            throw SSHError.remote(status: -1, message: "Could not update \(failed.joined(separator: ", ")) in the local copy.")
        }
        return report
    }

    /// Local → remote. Each changed file replaces the remote copy only if
    /// that is still what the manifest recorded, checked on the device at
    /// the moment of the move; otherwise it is a conflict.
    @discardableResult
    public func push() async throws -> SyncReport {
        await acquire()
        defer { release() }
        let changed = localHashes().filter { manifest[$0.key] != $0.value }.keys.sorted()
        var report = SyncReport()
        report.conflicts = conflicts.sorted()
        guard !changed.isEmpty else { return report }
        let outcomes = try await upload(changed, expected: manifest)
        var failed: [String] = [], unsupported: [String] = []
        for path in changed {
            switch outcomes[path] {
            case let .stored(hash, replaced)?:
                manifest[path] = hash
                conflicts.remove(path)
                if replaced { report.uploaded.append(path) }
            case .conflict?:
                conflicts.insert(path)
            case .failed?:
                failed.append(path)
            case .unsupported?:
                unsupported.append(path)
            case nil:
                // Vanished or became unreadable since the scan.
                continue
            }
        }
        report.conflicts = conflicts.sorted()
        try saveManifest()
        if !unsupported.isEmpty {
            throw SSHError.remote(status: -1, message: "Could not upload \(unsupported.joined(separator: ", ")) to \(deviceName): its folder refused a hard link (a filesystem without them, or out of space or quota), and Pitex only replaces files that way.")
        }
        if !failed.isEmpty {
            throw SSHError.remote(status: -1, message: "Could not upload \(failed.joined(separator: ", ")) to \(deviceName).")
        }
        return report
    }

    /// Build outputs (PDF, SyncTeX, log…) from the remote, overwriting the
    /// local copies: they are generated, never edited, so no conflict rule.
    /// Missing paths are skipped. Returns the paths fetched.
    @discardableResult
    public func fetch(_ paths: [String]) async throws -> [String] {
        await acquire()
        defer { release() }
        let safe = paths.filter(RemoteSyncRules.isSafeRelativePath)
        guard !safe.isEmpty else { return [] }
        defer { try? FileManager.default.removeItem(at: mirror.stagingURL) }
        let staged = try await download(safe)
        var fetched: [String] = []
        try await commitLocally {
            for path in safe {
                guard let file = staged[path] else { continue }
                try place(file.url, at: path)
                manifest[path] = file.hash
                conflicts.remove(path)
                fetched.append(path)
            }
            try saveManifest()
        }
        return fetched
    }

    /// Settles a conflict: `keepLocal` uploads this Mac's copy over the
    /// remote one, otherwise the remote copy replaces the local file (or
    /// removes it when the remote file is confirmed gone). Neither side is
    /// overwritten if it changed again after the choice was made (the call
    /// throws and the conflict stays). Returns the remaining conflicts.
    @discardableResult
    public func resolve(_ path: String, keepLocal: Bool) async throws -> [String] {
        guard RemoteSyncRules.isSafeRelativePath(path) else { return conflicts.sorted() }
        // The local version the user chose against — taken before queueing
        // behind another transfer, so a save made meanwhile is not lost.
        let mine = localHash(path)
        await acquire()
        defer { release() }
        let state = try await probe([path])[path] ?? .unavailable
        if state == .unavailable {
            throw SSHError.remote(status: -1, message: "\(path) cannot be read on \(deviceName).")
        }
        if keepLocal {
            if mine == nil {
                // Deleted here. Deletions are not sent to the device, so
                // the device's copy becomes the baseline and stays put.
                if case let .file(hash) = state { manifest[path] = hash } else { manifest[path] = nil }
            } else {
                var expected: [String: String] = [:]
                if case let .file(hash) = state { expected[path] = hash }
                switch try await upload([path], expected: expected)[path] {
                case let .stored(hash, _)?:
                    manifest[path] = hash
                case .conflict?:
                    throw SSHError.remote(status: -1, message: "\(path) changed on \(deviceName) again — try again.")
                case .unsupported?:
                    throw SSHError.remote(status: -1, message: "Could not upload \(path) to \(deviceName): its folder refused a hard link (a filesystem without them, or out of space or quota), and Pitex only replaces files that way.")
                case .failed?, nil:
                    throw SSHError.remote(status: -1, message: "Could not upload \(path) to \(deviceName).")
                }
            }
            conflicts.remove(path)
            try saveManifest()
            return conflicts.sorted()
        }
        let changedHere = SSHError.remote(status: -1, message: "\(path) was saved here meanwhile — try again.")
        if state == .absent {
            try await commitLocally {
                guard localHash(path) == mine else { throw changedHere }
                if let url = containedURL(path, create: false), itemExists(url) {
                    try FileManager.default.removeItem(at: url)
                }
                manifest[path] = nil
                localCache[path] = nil
                conflicts.remove(path)
                try saveManifest()
            }
        } else {
            defer { try? FileManager.default.removeItem(at: mirror.stagingURL) }
            guard let file = try await download([path])[path] else {
                throw SSHError.remote(status: -1, message: "\(path) could not be downloaded.")
            }
            try await commitLocally {
                guard localHash(path) == mine else { throw changedHere }
                try place(file.url, at: path)
                manifest[path] = file.hash
                conflicts.remove(path)
                try saveManifest()
            }
        }
        return conflicts.sorted()
    }

    /// The remote counterpart of a local path inside the mirror.
    public func remotePath(for url: URL) -> String? {
        let root = mirror.root.standardizedFileURL.path
        let path = url.standardizedFileURL.path
        if path == root { return remoteRoot }
        guard path.hasPrefix(root + "/") else { return nil }
        return (remoteRoot as NSString).appendingPathComponent(String(path.dropFirst(root.count + 1)))
    }

    // MARK: - Transfers

    private struct StagedFile {
        let url: URL
        /// Of the bytes actually received — what the manifest records.
        let hash: String
    }

    private enum UploadOutcome {
        /// The device holds these bytes now (`replaced`: by this upload).
        case stored(String, replaced: Bool)
        case conflict
        case failed
        /// A hard link was refused (no support, space or quota): no safe replacement.
        case unsupported
    }

    private func download(_ paths: [String]) async throws -> [String: StagedFile] {
        let fileManager = FileManager.default
        try? fileManager.removeItem(at: mirror.stagingURL)
        try fileManager.createDirectory(at: mirror.stagingURL, withIntermediateDirectories: true)
        try fileManager.createDirectory(at: mirror.workURL, withIntermediateDirectories: true)
        let archive = mirror.workURL.appendingPathComponent("download.tar")
        defer { try? fileManager.removeItem(at: archive) }
        let list = Data(paths.joined(separator: "\u{0}").utf8)
        // tar exits non-zero when a listed file vanished meanwhile; what it
        // did archive is still usable, so only an empty archive is an error.
        let result = try await client.run(RemoteScripts.tarOut, arguments: [remoteRoot], input: list, outputFile: archive)
        let size = (try? fileManager.attributesOfItem(atPath: archive.path)[.size] as? Int) ?? 0
        guard size > 0 else {
            throw SSHError.remote(status: result.status, message: SSHClient.firstLine(result.stderrText) ?? "Download failed.")
        }
        // Local bsdtar/GNU tar refuse absolute and `..` members by default;
        // only regular files at expected paths are taken from staging.
        let untar = try await ProcessPipe.run(executable: URL(fileURLWithPath: "/usr/bin/tar"),
                                              arguments: ["-xf", archive.path, "-C", mirror.stagingURL.path],
                                              input: nil, outputFile: nil)
        guard untar.status == 0 || untar.status == 1 else {
            throw SSHError.remote(status: untar.status, message: SSHClient.firstLine(untar.stderrText) ?? "The download could not be unpacked.")
        }
        var staged: [String: StagedFile] = [:]
        let stagingRoot = mirror.stagingURL.resolvingSymlinksInPath().standardizedFileURL.path
        for path in paths where RemoteSyncRules.isSafeRelativePath(path) {
            let url = URL(fileURLWithPath: stagingRoot).appendingPathComponent(path)
            // A symlink anywhere on the way (a hostile archive's `dir ->
            // /Users/me` followed by `dir/.ssh/id_ed25519`) would make the
            // move into the mirror pull an arbitrary local file along.
            guard url.resolvingSymlinksInPath().standardizedFileURL.path == url.standardizedFileURL.path,
                  isRegularFile(url), let hash = SHA256.hex(ofFile: url) else { continue }
            staged[path] = StagedFile(url: url, hash: hash)
        }
        return staged
    }

    /// Uploads snapshots of `paths`; on the device each replaces the remote
    /// file only if that still hashes to `expected[path]` (missing = the
    /// file must not exist). Paths that could not be snapshotted are left
    /// out of the result.
    private func upload(_ paths: [String], expected: [String: String]) async throws -> [String: UploadOutcome] {
        let fileManager = FileManager.default
        // `upload/expect` + `upload/data/<path>`: the archive's metadata
        // never shares a namespace with project paths.
        let snapshot = mirror.workURL.appendingPathComponent("upload", isDirectory: true)
        let data = snapshot.appendingPathComponent("data", isDirectory: true)
        let list = mirror.workURL.appendingPathComponent("upload.list")
        let archive = mirror.workURL.appendingPathComponent("upload.tar")
        defer {
            try? fileManager.removeItem(at: snapshot)
            try? fileManager.removeItem(at: list)
            try? fileManager.removeItem(at: archive)
        }
        try? fileManager.removeItem(at: snapshot)
        try fileManager.createDirectory(at: data, withIntermediateDirectories: true)
        // Copies are what travel and what the manifest records: a save
        // landing mid-upload changes the mirror, not these bytes.
        var sent: [String: String] = [:]
        var expectations = ""
        for path in paths {
            guard let source = containedURL(path, create: false), isRegularFile(source),
                  let bytes = try? Data(contentsOf: source) else { continue }
            let copy = data.appendingPathComponent(path)
            try fileManager.createDirectory(at: copy.deletingLastPathComponent(), withIntermediateDirectories: true)
            try bytes.write(to: copy)
            if let mode = try? fileManager.attributesOfItem(atPath: source.path)[.posixPermissions] {
                try? fileManager.setAttributes([.posixPermissions: mode], ofItemAtPath: copy.path)
            }
            let hash = SHA256.hex(bytes)
            sent[path] = hash
            expectations += "\(expected[path] ?? "-") \(hash) \(path)\n"
        }
        guard !sent.isEmpty else { return [:] }
        try Data(expectations.utf8).write(to: snapshot.appendingPathComponent("expect"))
        let members = ["expect"] + sent.keys.sorted().map { "data/" + $0 }
        try Data(members.joined(separator: "\u{0}").utf8).write(to: list)
        // COPYFILE_DISABLE keeps macOS tar from adding `._` metadata files.
        let pack = try await ProcessPipe.run(executable: URL(fileURLWithPath: "/usr/bin/tar"),
                                             arguments: ["-cf", archive.path, "--null", "-T", list.path],
                                             input: nil, outputFile: nil, currentDirectory: snapshot,
                                             environment: ["COPYFILE_DISABLE": "1"])
        guard pack.status == 0 else {
            throw SSHError.remote(status: pack.status, message: SSHClient.firstLine(pack.stderrText) ?? "The upload could not be packed.")
        }
        let result = try await client.runChecked(RemoteScripts.commitUpload, arguments: [remoteRoot],
                                                 input: try Data(contentsOf: archive))
        var outcomes = sent.mapValues { _ in UploadOutcome.failed }
        for line in result.stdoutText.split(separator: "\n") where line.count > 2 {
            let path = String(line.dropFirst(2))
            guard let hash = sent[path] else { continue }
            switch line.prefix(2) {
            case "U/": outcomes[path] = .stored(hash, replaced: true)
            case "S/": outcomes[path] = .stored(hash, replaced: false)
            case "C/": outcomes[path] = .conflict
            case "L/": outcomes[path] = .unsupported
            default: outcomes[path] = .failed
            }
        }
        return outcomes
    }

    private func probe(_ paths: [String]) async throws -> [String: RemoteFileState] {
        let list = Data(paths.map { $0 + "\n" }.joined().utf8)
        let result = try await client.runChecked(RemoteScripts.probe, arguments: [remoteRoot], input: list)
        return RemoteScripts.parseProbe(result.stdoutText)
    }

    // MARK: - Local side

    /// Runs local replacements/deletions with the app's own saves held
    /// back (see `MirrorWriteGate`).
    private func commitLocally(_ body: () throws -> Void) async throws {
        await gate?.beginSyncCommit()
        let outcome = Result { try body() }
        await gate?.endSyncCommit()
        try outcome.get()
    }

    /// The mirror URL for `path`, or nil when an existing component on the
    /// way is anything but a real directory — a symlink there (made by an
    /// agent or tool) would send a write or delete outside the mirror.
    /// `create` makes missing directories; otherwise a missing one is nil.
    private func containedURL(_ path: String, create: Bool) -> URL? {
        guard RemoteSyncRules.isSafeRelativePath(path) else { return nil }
        let fileManager = FileManager.default
        let components = path.split(separator: "/").map(String.init)
        var url = mirror.root
        for component in components.dropLast() {
            url.appendPathComponent(component, isDirectory: true)
            // attributesOfItem does not follow a symlink.
            if let type = (try? fileManager.attributesOfItem(atPath: url.path))?[.type] as? FileAttributeType {
                guard type == .typeDirectory else { return nil }
            } else if !create || (try? fileManager.createDirectory(at: url, withIntermediateDirectories: false)) == nil {
                return nil
            }
        }
        return url.appendingPathComponent(components[components.count - 1])
    }

    private func itemExists(_ url: URL) -> Bool {
        (try? FileManager.default.attributesOfItem(atPath: url.path)) != nil
    }

    /// A regular file itself, not a symlink to one.
    private func isRegularFile(_ url: URL) -> Bool {
        (try? FileManager.default.attributesOfItem(atPath: url.path))?[.type] as? FileAttributeType == .typeRegular
    }

    private func localHash(_ path: String) -> String? {
        guard let url = containedURL(path, create: false), isRegularFile(url),
              let values = try? url.resourceValues(forKeys: [.fileSizeKey, .contentModificationDateKey]),
              let size = values.fileSize, let modified = values.contentModificationDate else { return nil }
        if let cached = localCache[path], cached.size == size, cached.modified == modified { return cached.hash }
        guard let hash = SHA256.hex(ofFile: url) else { return nil }
        localCache[path] = (size, modified, hash)
        return hash
    }

    /// Every mirrored local file (same exclusions as the remote listing).
    private func localHashes() -> [String: String] {
        var hashes: [String: String] = [:]
        let root = mirror.root
        guard let enumerator = FileManager.default.enumerator(
            at: root, includingPropertiesForKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey],
            options: [], errorHandler: nil) else { return [:] }
        let rootPath = root.standardizedFileURL.path
        while let url = enumerator.nextObject() as? URL {
            let name = url.lastPathComponent
            if RemoteSyncRules.isExcluded(name: name) {
                enumerator.skipDescendants()
                continue
            }
            guard let values = try? url.resourceValues(forKeys: [.isRegularFileKey, .isSymbolicLinkKey, .fileSizeKey]),
                  values.isRegularFile == true, values.isSymbolicLink != true,
                  (values.fileSize ?? 0) < RemoteSyncRules.maximumFileSize else { continue }
            let path = String(url.standardizedFileURL.path.dropFirst(rootPath.count + 1))
            if let hash = localHash(path) { hashes[path] = hash }
        }
        return hashes
    }

    /// Moves a staged regular file into the mirror, replacing the old one.
    private func place(_ staged: URL, at path: String) throws {
        guard let destination = containedURL(path, create: true) else {
            throw SSHError.remote(status: -1, message: "\(path) is not inside the project folder.")
        }
        // rename(2) replaces atomically (a symlink at the final component
        // is replaced itself, not followed; a directory there fails);
        // staging sits beside the mirror on the same volume.
        guard rename(staged.path, destination.path) == 0 else {
            throw SSHError.remote(status: -1, message: "Could not update \(path): \(String(cString: strerror(errno)))")
        }
        localCache[path] = nil
    }

    private func saveManifest() throws {
        try JSONEncoder().encode(manifest).write(to: mirror.manifestURL, options: .atomic)
    }
}
