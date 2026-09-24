import BuildCore
import Foundation
@testable import RemoteCore
import XCTest

private actor RecordingGate: MirrorWriteGate {
    var events: [String] = []
    func beginSyncCommit() async { events.append("begin") }
    func endSyncCommit() async { events.append("end") }
}

final class RemoteCoreTests: XCTestCase {
    // MARK: - ssh config

    func testConfigHostsSkipPatternsMatchBlocksAndKeepFirstValues() {
        let config = """
        # personal machines
        Host *
            ServerAliveInterval 30
        Host mini studio
            HostName mini.local
            User "jae hwan"
            Port=2222
            HostName ignored.local
        Host !blocked *.corp
            User nobody
        Match host mini
            User matched
        Host=lab
            Hostname = 10.0.0.5 # trailing comment
        Include extra
        """
        let hosts = SSHConfigParser.hosts(in: config) { argument in
            argument == "extra" ? ["Host included\n  User inc\nHost mini\n  User late"] : []
        }
        XCTAssertEqual(hosts.map(\.alias), ["mini", "studio", "lab", "included"])
        XCTAssertEqual(hosts[0], SSHHostEntry(alias: "mini", hostName: "mini.local", user: "jae hwan", port: 2222))
        XCTAssertEqual(hosts[1].hostName, "mini.local")
        XCTAssertEqual(hosts[2].hostName, "10.0.0.5")
        XCTAssertNil(hosts[2].user, "values under Match must not leak into later hosts")
        XCTAssertEqual(hosts[3].user, "inc")
        XCTAssertEqual(hosts[0].summary, "jae hwan@mini.local:2222")
    }

    func testIncludeGlobAndMissingConfig() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent("pitex-ssh-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        try "Host a\n".write(to: directory.appendingPathComponent("one.conf"), atomically: true, encoding: .utf8)
        try "Host b\n".write(to: directory.appendingPathComponent("two.conf"), atomically: true, encoding: .utf8)
        try "Host c\n".write(to: directory.appendingPathComponent("skip.txt"), atomically: true, encoding: .utf8)
        let main = directory.appendingPathComponent("config")
        try "Include \(directory.path)/*.conf\nHost main\n".write(to: main, atomically: true, encoding: .utf8)
        XCTAssertEqual(SSHConfigParser.loadHosts(from: main).map(\.alias), ["a", "b", "main"])
        XCTAssertEqual(SSHConfigParser.loadHosts(from: directory.appendingPathComponent("missing")), [])
        XCTAssertTrue(SSHConfigParser.matches("*.conf", "x.conf"))
        XCTAssertFalse(SSHConfigParser.matches("*.conf", "x.txt"))
        XCTAssertTrue(SSHConfigParser.matches("h?st", "host"))
    }

    // MARK: - connection and argv

    func testConnectionValidationRejectsOptionInjection() {
        XCTAssertNil(SSHConnection(name: "mini", destination: "mini").validationError)
        XCTAssertNotNil(SSHConnection(name: "x", destination: "-oProxyCommand=evil").validationError)
        XCTAssertNotNil(SSHConnection(name: "x", destination: "host name").validationError)
        XCTAssertNotNil(SSHConnection(name: "x", destination: "").validationError)
        XCTAssertNotNil(SSHConnection(name: "x", destination: "h", user: "-l").validationError)
        XCTAssertNotNil(SSHConnection(name: "x", destination: "h", user: "a@b").validationError)
        XCTAssertNotNil(SSHConnection(name: "x", destination: "h", port: 70000).validationError)
    }

    func testArgumentsQuoteTheScriptAndKeepDestinationAfterDoubleDash() throws {
        let connection = SSHConnection(name: "lab", destination: "lab.example", user: "me", port: 2200, identityFile: "/k/id")
        let client = SSHClient(connection: connection, controlDirectory: nil)
        let argv = try client.arguments(script: #"echo "$1""#, arguments: ["it's here"], tty: false)
        XCTAssertTrue(argv.contains("BatchMode=yes"))
        XCTAssertFalse(argv.contains { $0.contains("StrictHostKeyChecking") }, "host keys stay checked")
        let dashes = try XCTUnwrap(argv.firstIndex(of: "--"))
        XCTAssertEqual(argv[dashes + 1], "lab.example")
        XCTAssertEqual(argv[dashes + 2], #"'/bin/sh' '-c' 'echo "$1"' 'pitex' 'it'\''s here'"#)
        XCTAssertEqual(Array(argv[(dashes - 8)..<dashes]), ["-l", "me", "-p", "2200", "-i", "/k/id", "-o", "IdentitiesOnly=yes"])
        XCTAssertThrowsError(try SSHClient(connection: SSHConnection(name: "x", destination: "-x"))
            .arguments(script: "true", arguments: [], tty: false))
    }

    // MARK: - hashing, parsing, planning

    func testControlDirectoryIsShortPrivateAndOwned() throws {
        let fileManager = FileManager.default
        // Room for "/<40 hex>.<16 chars>" under macOS's 104-byte socket limit.
        XCTAssertLessThan(try XCTUnwrap(SSHClient.defaultControlDirectory).path.utf8.count + 58, 104)
        let directory = URL(fileURLWithPath: "/tmp/pitex-cd-\(UUID().uuidString.prefix(8))")
        let link = URL(fileURLWithPath: "/tmp/pitex-cl-\(UUID().uuidString.prefix(8))")
        defer { try? fileManager.removeItem(at: directory); try? fileManager.removeItem(at: link) }
        XCTAssertEqual(SSHClient.usableControlDirectory(directory), directory, "created private on first use")
        try fileManager.setAttributes([.posixPermissions: 0o755], ofItemAtPath: directory.path)
        XCTAssertNil(SSHClient.usableControlDirectory(directory), "group/other access disables multiplexing")
        try fileManager.setAttributes([.posixPermissions: 0o700], ofItemAtPath: directory.path)
        try fileManager.createSymbolicLink(at: link, withDestinationURL: directory)
        XCTAssertNil(SSHClient.usableControlDirectory(link), "a symlink is not trusted")
        XCTAssertNil(SSHClient.usableControlDirectory(URL(fileURLWithPath: "/tmp/" + String(repeating: "x", count: 60))),
                     "too long for a socket path")
    }

    func testSHA256MatchesKnownVectors() {
        XCTAssertEqual(SHA256.hex(Data()), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        XCTAssertEqual(SHA256.hex(Data("abc".utf8)), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        XCTAssertEqual(SHA256.hex(Data("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq".utf8)),
                       "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")
        XCTAssertEqual(SHA256.hex(Data(repeating: 0x61, count: 1_000_000)),
                       "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0")
    }

    func testHashAndListingParsersDropUnsafeEntries() {
        let hash = String(repeating: "a", count: 64)
        let output = """
        \(hash)  ./main.tex
        \(hash) *./fig/plot.pdf
        \\\(hash)  ./weird\\nname
        \(hash)  -
        \(hash)  ./../escape.tex
        """
        XCTAssertEqual(RemoteScripts.parseHashes(output), ["main.tex": hash, "fig/plot.pdf": hash])
        let listing = RemoteDirectoryListing.parse("/home/me\nD/papers\nF/notes.md\nD/Archive\n")
        XCTAssertEqual(listing, RemoteDirectoryListing(path: "/home/me", folders: ["Archive", "papers"], files: ["notes.md"]))
        XCTAssertNil(RemoteDirectoryListing.parse("not a path\n"))
        XCTAssertTrue(RemoteSyncRules.isSafeRelativePath("a/b.tex"))
        for path in ["", "/etc/passwd", "../x", "a/../b", "a//b", "a/./b", "a/\u{1}b"] {
            XCTAssertFalse(RemoteSyncRules.isSafeRelativePath(path), path)
        }
        XCTAssertTrue(RemoteSyncRules.isExcluded(name: ".main.tex.texspark-1234.tmp"))
        XCTAssertTrue(RemoteSyncRules.isExcluded(name: ".git"))
        XCTAssertFalse(RemoteSyncRules.isExcluded(name: "main.tex"))
    }

    func testPullPlanNeverOverwritesLocalEdits() {
        let plan = SyncPlanner.pull(
            remote: ["same": "1", "remoteEdit": "2b", "bothEdit": "3b", "converged": "4b", "new": "5", "newClash": "6r"],
            manifest: ["same": "1", "remoteEdit": "2", "bothEdit": "3", "converged": "4", "gone": "7", "goneEdited": "8"],
            local: ["same": "1", "remoteEdit": "2", "bothEdit": "3l", "converged": "4b", "newClash": "6l",
                    "gone": "7", "goneEdited": "8l"])
        XCTAssertEqual(plan.download, ["new", "remoteEdit"])
        XCTAssertEqual(plan.adopt, ["converged": "4b"])
        XCTAssertEqual(plan.delete, ["gone"])
        XCTAssertEqual(plan.conflicts, ["bothEdit", "goneEdited", "newClash"])
    }

    func testRemoteScriptsAreValidSingleLineSh() throws {
        for script in [RemoteScripts.hashTree, RemoteScripts.probe, RemoteScripts.commitUpload,
                       RemoteScripts.tarOut, RemoteScripts.listDirectory, RemoteScripts.loginExec] {
            XCTAssertFalse(script.contains("\n"), "csh login shells need single-line scripts")
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/bin/sh")
            process.arguments = ["-n", "-c", script]
            try process.run()
            process.waitUntilExit()
            XCTAssertEqual(process.terminationStatus, 0, script)
        }
    }

    /// Runs a remote script with the local /bin/sh, as the device would.
    private func runScript(_ script: String, in root: URL, input: Data) throws -> String {
        let process = Process()
        // PITEX_TEST_SH tries another shell (macOS's /bin/sh is bash).
        process.executableURL = URL(fileURLWithPath: ProcessInfo.processInfo.environment["PITEX_TEST_SH"] ?? "/bin/sh")
        process.arguments = ["-c", script, "pitex", root.path]
        let stdin = Pipe(), stdout = Pipe()
        process.standardInput = stdin
        process.standardOutput = stdout
        try process.run()
        stdin.fileHandleForWriting.write(input)
        try stdin.fileHandleForWriting.close()
        let output = stdout.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        XCTAssertEqual(process.terminationStatus, 0)
        return String(decoding: output, as: UTF8.self)
    }

    func testCommitUploadAndProbeScriptsDecidePerFile() throws {
        let fileManager = FileManager.default
        let base = fileManager.temporaryDirectory.appendingPathComponent("pitex-script-\(UUID().uuidString)")
        defer { try? fileManager.removeItem(at: base) }
        let root = base.appendingPathComponent("project"), outside = base.appendingPathComponent("outside")
        let package = base.appendingPathComponent("package")
        for directory in [root, outside, package.appendingPathComponent("data/fresh"),
                          package.appendingPathComponent("data/link"), root.appendingPathComponent("dir.tex")] {
            try fileManager.createDirectory(at: directory, withIntermediateDirectories: true)
        }
        try fileManager.createSymbolicLink(at: root.appendingPathComponent("link"), withDestinationURL: outside)
        func put(_ text: String, _ path: String, in folder: URL) throws {
            try text.write(to: folder.appendingPathComponent(path), atomically: false, encoding: .utf8)
        }
        func hash(_ text: String) -> String { SHA256.hex(Data(text.utf8)) }
        try put("old", "keep.tex", in: root)
        try put("theirs", "moved.tex", in: root)
        try put("new", "same.tex", in: root)
        try put("exists", "taken.tex", in: root)
        // A lock left by a process that no longer exists is taken over.
        let finished = Process()
        finished.executableURL = URL(fileURLWithPath: "/bin/sh")
        finished.arguments = ["-c", "exit 0"]
        try finished.run()
        finished.waitUntilExit()
        try fileManager.createDirectory(at: root.appendingPathComponent(".pitex-upload.lock"), withIntermediateDirectories: true)
        try put("\(finished.processIdentifier)\n", ".pitex-upload.lock/pid", in: root)

        let uploads: [(path: String, base: String)] = [
            ("keep.tex", hash("old")), ("moved.tex", hash("old")), ("same.tex", hash("old")),
            ("fresh/one.tex", "-"), ("taken.tex", "-"), ("link/in.tex", "-"), ("dir.tex", "-"),
        ]
        var expect = ""
        for upload in uploads {
            try put("new", "data/" + upload.path, in: package)
            expect += "\(upload.base) \(hash("new")) \(upload.path)\n"
        }
        try put(expect, "expect", in: package)
        let archive = base.appendingPathComponent("upload.tar")
        let tar = Process()
        tar.executableURL = URL(fileURLWithPath: "/usr/bin/tar")
        tar.arguments = ["-cf", archive.path, "-C", package.path, "expect", "data"]
        try tar.run()
        tar.waitUntilExit()

        let output = try runScript(RemoteScripts.commitUpload, in: root, input: Data(contentsOf: archive))
        XCTAssertEqual(Set(output.split(separator: "\n").map(String.init)), [
            "U/keep.tex", "C/moved.tex", "S/same.tex", "U/fresh/one.tex", "C/taken.tex", "E/link/in.tex", "C/dir.tex",
        ])
        func read(_ path: String) -> String? { try? String(contentsOf: root.appendingPathComponent(path), encoding: .utf8) }
        XCTAssertEqual(read("keep.tex"), "new")
        XCTAssertEqual(read("moved.tex"), "theirs", "a changed remote file is never replaced")
        XCTAssertEqual(read("fresh/one.tex"), "new")
        XCTAssertEqual(read("taken.tex"), "exists")
        XCTAssertEqual(try fileManager.contentsOfDirectory(atPath: outside.path), [], "nothing written through the symlink")
        let leftovers = (fileManager.enumerator(atPath: root.path)?.allObjects as? [String] ?? [])
            .filter { ($0 as NSString).lastPathComponent.hasPrefix(".pitex-upload") }
        XCTAssertEqual(leftovers, [], "staging, backups and the lock are cleaned up")

        let probed = RemoteScripts.parseProbe(try runScript(
            RemoteScripts.probe, in: root,
            input: Data("keep.tex\ngone.tex\nnodir/x.tex\nlink/in.tex\ndir.tex\n".utf8)))
        XCTAssertEqual(probed, ["keep.tex": .file(hash("new")), "gone.tex": .absent, "nodir/x.tex": .absent,
                                "link/in.tex": .unavailable, "dir.tex": .unavailable])
    }

    func testProbeParserSeparatesPresentAbsentAndUnavailable() {
        let hash = String(repeating: "b", count: 64)
        let states = RemoteScripts.parseProbe("H\(hash)/a b.tex\nA/gone.tex\nE/locked/x.tex\nHzz/bad\n")
        XCTAssertEqual(states, ["a b.tex": .file(hash), "gone.tex": .absent, "locked/x.tex": .unavailable])
    }

    func testMirrorIsFoundAgainFromAnyPathInside() throws {
        let store = FileManager.default.temporaryDirectory.appendingPathComponent("pitex-store-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: store) }
        let project = RemoteProject(connection: SSHConnection(name: "Mac mini", destination: "mini"), remoteRoot: "/Users/me/thesis")
        let mirror = try RemoteMirror.prepare(for: project, store: store)
        XCTAssertEqual(mirror.root.lastPathComponent, "thesis")
        XCTAssertEqual(RemoteMirror.containing(mirror.root.appendingPathComponent("ch/one.tex"), store: store)?.project, project)
        XCTAssertEqual(try RemoteMirror.prepare(for: project, store: store).directory, mirror.directory, "same folder, same mirror")
        XCTAssertNil(RemoteMirror.containing(mirror.directory.appendingPathComponent("manifest.json"), store: store))
        XCTAssertNil(RemoteMirror.containing(URL(fileURLWithPath: "/tmp/elsewhere.tex"), store: store))
    }

    // MARK: - Live, against a real sshd (skipped unless configured)

    /// Configure with PITEX_TEST_SSH_DESTINATION (+ _PORT, _KEY,
    /// _KNOWN_HOSTS). The "remote" is expected to share this filesystem
    /// (localhost), so the test edits remote files directly.
    private func liveClient() throws -> SSHClient {
        let environment = ProcessInfo.processInfo.environment
        guard let destination = environment["PITEX_TEST_SSH_DESTINATION"] else {
            throw XCTSkip("PITEX_TEST_SSH_DESTINATION not set")
        }
        var extra: [String] = []
        if let key = environment["PITEX_TEST_SSH_KEY"] { extra += ["-i", key, "-o", "IdentitiesOnly=yes"] }
        if let known = environment["PITEX_TEST_SSH_KNOWN_HOSTS"] { extra += ["-o", "UserKnownHostsFile=\(known)"] }
        let port = environment["PITEX_TEST_SSH_PORT"].flatMap(Int.init)
        return SSHClient(connection: SSHConnection(name: "test", destination: destination, port: port),
                         extraArguments: extra)
    }

    func testLiveSyncRoundTrip() async throws {
        let client = try liveClient()
        let fileManager = FileManager.default
        let remote = fileManager.temporaryDirectory.appendingPathComponent("pitex-remote-\(UUID().uuidString)")
        let store = fileManager.temporaryDirectory.appendingPathComponent("pitex-store-\(UUID().uuidString)")
        defer { try? fileManager.removeItem(at: remote); try? fileManager.removeItem(at: store) }
        try fileManager.createDirectory(at: remote.appendingPathComponent("chapters/sub dir"), withIntermediateDirectories: true)
        try fileManager.createDirectory(at: remote.appendingPathComponent(".git"), withIntermediateDirectories: true)
        try "\\documentclass{article}\n\\input{chapters/one}\n".write(to: remote.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try "한글 $x$\n".write(to: remote.appendingPathComponent("chapters/one.tex"), atomically: true, encoding: .utf8)
        try "it's \"quoted\"\n".write(to: remote.appendingPathComponent("chapters/sub dir/it's.md"), atomically: true, encoding: .utf8)
        try "secret".write(to: remote.appendingPathComponent(".git/config"), atomically: true, encoding: .utf8)

        let listing = try await client.listDirectory(remote.path)
        XCTAssertEqual(listing.folders, ["chapters"])
        XCTAssertEqual(listing.files, ["main.tex"])
        let chapters = try await client.listDirectory(remote.appendingPathComponent("chapters").path)
        XCTAssertEqual(chapters.folders, ["sub dir"])

        let mirror = try RemoteMirror.prepare(for: RemoteProject(connection: client.connection, remoteRoot: remote.path), store: store)
        let sync = RemoteSync(mirror: mirror, client: client)
        var report = try await sync.pull()
        XCTAssertEqual(Set(report.downloaded), ["main.tex", "chapters/one.tex", "chapters/sub dir/it's.md"])
        XCTAssertFalse(fileManager.fileExists(atPath: mirror.root.appendingPathComponent(".git/config").path))
        XCTAssertEqual(try String(contentsOf: mirror.root.appendingPathComponent("chapters/one.tex"), encoding: .utf8), "한글 $x$\n")

        // Local edit + new local file → uploaded.
        try "한글 $y$\n".write(to: mirror.root.appendingPathComponent("chapters/one.tex"), atomically: true, encoding: .utf8)
        try "new\n".write(to: mirror.root.appendingPathComponent("chapters/two.tex"), atomically: true, encoding: .utf8)
        report = try await sync.push()
        XCTAssertEqual(report.uploaded, ["chapters/one.tex", "chapters/two.tex"])
        XCTAssertEqual(try String(contentsOf: remote.appendingPathComponent("chapters/one.tex"), encoding: .utf8), "한글 $y$\n")
        let idle = try await sync.push()
        XCTAssertEqual(idle.uploaded, [], "nothing left to push")
        XCTAssertFalse(try fileManager.contentsOfDirectory(atPath: remote.path).contains { $0.hasPrefix(".pitex-upload") },
                       "the device-side staging directory is cleaned up")

        // Remote edit → pulled; remote delete → deleted locally.
        try "remote edit\n".write(to: remote.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try fileManager.removeItem(at: remote.appendingPathComponent("chapters/two.tex"))
        report = try await sync.pull()
        XCTAssertEqual(report.downloaded, ["main.tex"])
        XCTAssertEqual(report.deleted, ["chapters/two.tex"])
        XCTAssertFalse(fileManager.fileExists(atPath: mirror.root.appendingPathComponent("chapters/two.tex").path))

        // Edited on both sides → conflict both ways, nothing overwritten.
        try "remote side\n".write(to: remote.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try "local side\n".write(to: mirror.root.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let pulled = try await sync.pull()
        let pushed = try await sync.push()
        XCTAssertEqual(pulled.conflicts, ["main.tex"])
        XCTAssertEqual(pushed.conflicts, ["main.tex"])
        XCTAssertEqual(try String(contentsOf: mirror.root.appendingPathComponent("main.tex"), encoding: .utf8), "local side\n")
        XCTAssertEqual(try String(contentsOf: remote.appendingPathComponent("main.tex"), encoding: .utf8), "remote side\n")

        // Resolving: keep mine uploads it; take remote replaces the local copy.
        try await sync.resolve("main.tex", keepLocal: true)
        XCTAssertEqual(try String(contentsOf: remote.appendingPathComponent("main.tex"), encoding: .utf8), "local side\n")
        try "remote again\n".write(to: remote.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try "local again\n".write(to: mirror.root.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let clash = try await sync.pull()
        XCTAssertEqual(clash.conflicts, ["main.tex"])
        try await sync.resolve("main.tex", keepLocal: false)
        XCTAssertEqual(try String(contentsOf: mirror.root.appendingPathComponent("main.tex"), encoding: .utf8), "remote again\n")

        // A fresh RemoteSync reloads the manifest from disk.
        let reopened = RemoteSync(mirror: mirror, client: client)
        let again = try await reopened.pull()
        XCTAssertEqual(again.downloaded, [])

        // Both sides arriving at the same text settles the conflict.
        try "remote 3\n".write(to: remote.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        try "local 3\n".write(to: mirror.root.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let third = try await sync.pull()
        XCTAssertEqual(third.conflicts, ["main.tex"])
        try "remote 3\n".write(to: mirror.root.appendingPathComponent("main.tex"), atomically: true, encoding: .utf8)
        let settled = try await sync.pull()
        XCTAssertEqual(settled.conflicts, [])

        // A remote folder swapped for a symlink leading through an
        // unsearchable directory is not proof that its files are gone.
        let vault = fileManager.temporaryDirectory.appendingPathComponent("pitex-vault-\(UUID().uuidString)")
        defer { try? fileManager.setAttributes([.posixPermissions: 0o755], ofItemAtPath: vault.path); try? fileManager.removeItem(at: vault) }
        try fileManager.createDirectory(at: vault, withIntermediateDirectories: true)
        try fileManager.moveItem(at: remote.appendingPathComponent("chapters/sub dir"), to: vault.appendingPathComponent("inner"))
        try fileManager.createSymbolicLink(at: remote.appendingPathComponent("chapters/sub dir"), withDestinationURL: vault.appendingPathComponent("inner"))
        try fileManager.setAttributes([.posixPermissions: 0o000], ofItemAtPath: vault.path)
        let throughLink = try await sync.pull()
        try fileManager.setAttributes([.posixPermissions: 0o755], ofItemAtPath: vault.path)
        XCTAssertEqual(throughLink.deleted, [])
        XCTAssertTrue(fileManager.fileExists(atPath: mirror.root.appendingPathComponent("chapters/sub dir/it's.md").path))
        try fileManager.removeItem(at: remote.appendingPathComponent("chapters/sub dir"))
        try fileManager.moveItem(at: vault.appendingPathComponent("inner"), to: remote.appendingPathComponent("chapters/sub dir"))

        // An upload never writes through a symlinked remote folder.
        let elsewhere = fileManager.temporaryDirectory.appendingPathComponent("pitex-elsewhere-\(UUID().uuidString)")
        defer { try? fileManager.removeItem(at: elsewhere) }
        try fileManager.createDirectory(at: elsewhere, withIntermediateDirectories: true)
        try fileManager.createSymbolicLink(at: remote.appendingPathComponent("out"), withDestinationURL: elsewhere)
        try fileManager.createDirectory(at: mirror.root.appendingPathComponent("out"), withIntermediateDirectories: true)
        try "escape\n".write(to: mirror.root.appendingPathComponent("out/new.tex"), atomically: true, encoding: .utf8)
        do {
            try await sync.push()
            XCTFail("placing through a remote symlink must fail")
        } catch {}
        XCTAssertFalse(fileManager.fileExists(atPath: elsewhere.appendingPathComponent("new.tex").path))
        try fileManager.removeItem(at: mirror.root.appendingPathComponent("out"))
        try fileManager.removeItem(at: remote.appendingPathComponent("out"))

        // A file the listing cannot see (unsearchable directory) is not
        // taken for deleted, and "take remote" refuses to guess.
        let locked = remote.appendingPathComponent("chapters/sub dir")
        try fileManager.setAttributes([.posixPermissions: 0o000], ofItemAtPath: locked.path)
        let hidden = try await sync.pull()
        try fileManager.setAttributes([.posixPermissions: 0o755], ofItemAtPath: locked.path)
        XCTAssertEqual(hidden.deleted, [])
        XCTAssertTrue(fileManager.fileExists(atPath: mirror.root.appendingPathComponent("chapters/sub dir/it's.md").path))

        // A symlinked directory in the mirror never redirects a download.
        let outside = fileManager.temporaryDirectory.appendingPathComponent("pitex-outside-\(UUID().uuidString)")
        defer { try? fileManager.removeItem(at: outside) }
        try fileManager.createDirectory(at: outside, withIntermediateDirectories: true)
        try fileManager.createSymbolicLink(at: mirror.root.appendingPathComponent("figs"), withDestinationURL: outside)
        try fileManager.createDirectory(at: remote.appendingPathComponent("figs"), withIntermediateDirectories: true)
        try "plot".write(to: remote.appendingPathComponent("figs/plot.tex"), atomically: true, encoding: .utf8)
        do {
            try await sync.pull()
            XCTFail("placing through a symlinked directory must fail")
        } catch {}
        XCTAssertFalse(fileManager.fileExists(atPath: outside.appendingPathComponent("plot.tex").path))

        // Pushes wait at the app's write gate only around local commits.
        try fileManager.removeItem(at: mirror.root.appendingPathComponent("figs"))
        let gate = RecordingGate()
        let gated = RemoteSync(mirror: mirror, client: client, gate: gate)
        let gatedPull = try await gated.pull()
        XCTAssertEqual(gatedPull.downloaded, ["figs/plot.tex"])
        let events = await gate.events
        XCTAssertEqual(events, ["begin", "end"])
    }

    func testLiveRemoteBuildStreamsOutputAndFetchesArtifacts() async throws {
        let client = try liveClient()
        let fileManager = FileManager.default
        let remote = fileManager.temporaryDirectory.appendingPathComponent("pitex-remote-\(UUID().uuidString)")
        let store = fileManager.temporaryDirectory.appendingPathComponent("pitex-store-\(UUID().uuidString)")
        defer { try? fileManager.removeItem(at: remote); try? fileManager.removeItem(at: store) }
        try fileManager.createDirectory(at: remote.appendingPathComponent("src"), withIntermediateDirectories: true)
        try "doc".write(to: remote.appendingPathComponent("src/main.tex"), atomically: true, encoding: .utf8)
        let mirror = try RemoteMirror.prepare(for: RemoteProject(connection: client.connection, remoteRoot: remote.path), store: store)
        let sync = RemoteSync(mirror: mirror, client: client)
        try await sync.pull()
        let executor = RemoteBuildExecutor(sync: sync)
        await executor.setOutputs(["src/main.pdf", "src/main.log", "src/absent.aux"])
        // A stand-in "engine": prints, writes the outputs, exits 3.
        let script = #"printf 'pass %s\n' "$PWD"; echo warn >&2; printf '%%PDF-1.4' > main.pdf; echo log > main.log; exit 3"#
        let plan = try DirectCommandPlan(executable: "/bin/sh", arguments: ["-c", script], workingDirectory: .sourceDirectory)
        let request = BuildProcessRequest(
            buildID: try BuildID(rawValue: "b1"), stageIndex: 0, command: .direct(plan),
            projectRoot: mirror.root, sourceDirectory: mirror.root.appendingPathComponent("src"))
        let collected = Collected()
        let result = try await executor.execute(request) { output in await collected.append(output) }
        XCTAssertEqual(result.exitCode, 3)
        let text = await collected.text
        XCTAssertTrue(text.contains("pass \(remote.appendingPathComponent("src").resolvingSymlinksInPath().path)"), text)
        XCTAssertTrue(text.contains("warn"), text)
        XCTAssertEqual(try String(contentsOf: mirror.root.appendingPathComponent("src/main.pdf"), encoding: .utf8), "%PDF-1.4")
        let afterBuild = try await sync.push()
        XCTAssertEqual(afterBuild.uploaded, [], "fetched outputs are in the manifest, not pending uploads")
    }

    private actor Collected {
        var text = ""
        func append(_ output: BuildProcessOutput) { text += String(decoding: output.bytes, as: UTF8.self) }
    }
}
