#!/usr/bin/env python3
"""macOS: python3 Tools/check-remote-autosave.py <ssh-host>

Compile the actual app model and check saves against a disposable SSH project.
Requires Swift command-line tools and an existing key-authenticated SSH host.
Set SDKROOT to an installed SDK if the newest SDK needs Xcode-only macros.
"""
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parents[1]
host = sys.argv[1]
check = r'''
import AppKit
import Combine
import RemoteCore
import ProjectFeature
import SwiftUI
import Vision

@main struct Check {
    @MainActor static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        Task { @MainActor in
            do { try await run(); exit(0) }
            catch { print("FAIL: \(error)"); fflush(nil); exit(1) }
        }
        app.run()
    }

    @MainActor static func run() async throws {
        let client = SSHClient(connection: SSHConnection(name: "check", destination: CommandLine.arguments[1]))
        let created = try await client.runChecked("mktemp -d /tmp/pitex-autosave.XXXXXXXX")
        let root = created.stdoutText.trimmingCharacters(in: .whitespacesAndNewlines)
        precondition(root.hasPrefix("/tmp/pitex-autosave.") && (root as NSString).deletingLastPathComponent == "/tmp")
        let mirror = try RemoteMirror.prepare(for: RemoteProject(connection: client.connection, remoteRoot: root))
        let workspace = WorkspaceModel()
        func cleanup() async {
            await workspace.close()
            _ = try? await client.runChecked("rm -rf -- \"$1\"", arguments: [root])
            try? FileManager.default.removeItem(at: mirror.directory)
            UserDefaults.standard.removePersistentDomain(forName: "dev.pitex.remote-autosave-check")
        }
        func require(_ condition: Bool, _ message: String) throws {
            if !condition { throw NSError(domain: "RemoteAutosave", code: 1,
                                         userInfo: [NSLocalizedDescriptionKey: message]) }
        }
        func readRemote(_ file: String = "main.tex") async throws -> String {
            try await client.runChecked("cat -- \"$1/$2\"", arguments: [root, file]).stdoutText
        }
        func waitUntil(_ message: String, _ done: () -> Bool) async throws {
            let deadline = ContinuousClock.now + .seconds(20)
            while !done() {
                try require(.now < deadline, message)
                try await Task.sleep(for: .milliseconds(20))
            }
        }
        func edit(_ text: String) async throws {
            let view = workspace.environment!.editor.textView
            let range = NSRange(location: 0, length: view.string.utf16.count)
            try require(view.shouldChangeText(in: range, replacementString: text), "Editor refused edit")
            view.replaceCharacters(in: range, with: text)
            view.didChangeText()
            try await waitUntil("Editor did not publish the edit") { workspace.documentSnapshot?.text == text }
        }
        func waitForUpload() async throws {
            try await waitUntil("Save did not finish") { workspace.documentSnapshot?.saveState == .clean }
            await workspace.remotePushTask?.value
            try require(workspace.remote?.status == .synced, "Upload did not finish as Synced")
        }
        do {
            _ = try await client.runChecked("printf 'initial\\n' > \"$1/main.tex\"; git -C \"$1\" init -q", arguments: [root])
            workspace.settings.autoSave = false
            workspace.settings.aiAutocompletion = false
            await workspace.open(mirror.root)
            try require(workspace.canSave == false && workspace.environment != nil, "Remote project did not open")
            var statuses: [RemoteWorkspace.Status] = []
            let observation = workspace.$remote.sink { if let remote = $0 { statuses.append(remote.status) } }
            defer { observation.cancel() }
            workspace.bottomPanelVisible = true
            workspace.consoleSection = .git

            try await edit("manual\n")
            for _ in 0..<3 {
                workspace.refreshGit()
                try await waitUntil("Git refresh did not finish") { !workspace.gitRefreshInFlight }
            }
            try require(try await readRemote() == "initial\n", "Git polling uploaded an unsaved edit")
            try require(!statuses.contains(.syncing), "Git polling flickered the sync status")
            await workspace.save()
            try await waitForUpload()
            try require(try await readRemote() == "manual\n", "Manual save did not upload")
            print("PASS manual save and idle Git polling")

            workspace.settings.autoSave = true
            for delay in [2, 5, 10] {
                workspace.settings.autoSaveDelay = delay
                let before = try await readRemote()
                let text = "auto \(delay) 한글\n"
                try await edit(text)
                try await Task.sleep(for: .milliseconds(delay * 1000 - 500))
                try require(workspace.documentSnapshot?.saveState == .dirty, "Auto Save ran before its configured delay")
                try require(try await readRemote() == before, "Remote upload ran before Auto Save")
                try await waitForUpload()
                try require(try await readRemote() == text, "Auto Save did not upload at \(delay)s")
                print("PASS Auto Save \(delay)s: local save → remote bytes → Synced")
            }

            workspace.settings.autoSave = false
            try await edit("save all first\n")
            await workspace.createDocument()
            await workspace.remotePushTask?.value
            try await edit("save all second\n")
            let failure = await workspace.persistDirtySessions()
            try require(failure == nil, "Save All failed: \(failure ?? "")")
            await workspace.remotePushTask?.value
            try require(try await readRemote() == "save all first\n", "Save All missed the background document")
            try require(try await readRemote("untitled.tex") == "save all second\n", "Save All missed the active document")
            print("PASS Save All and new file upload")

            // A second saved revision during a transfer must reach the device.
            try await edit(String(repeating: "overlap\n", count: 100_000))
            await workspace.save()
            try await edit("last saved revision\n")
            await workspace.save()
            await workspace.remotePushTask?.value
            try require(try await readRemote("untitled.tex") == "last saved revision\n", "In-flight save was lost")
            try require(workspace.remote?.status == .synced, "In-flight save left a stale status")
            statuses.removeAll()
            workspace.remote?.status = .offline("test failure")
            workspace.refreshGit()
            try await waitUntil("Git refresh did not finish") { !workspace.gitRefreshInFlight }
            try require(workspace.remote?.status == .offline("test failure"), "Git polling erased the sync failure")
            try require(!statuses.contains(.syncing), "Idle Git polling started another sync")
            print("PASS overlapping saves and preserved failure status")

            // Four independent manuscripts with identical basenames.
            for folder in ["1_icml2026", "2_nips2026", "3_arxiv", "4_journal"] {
                _ = try await client.runChecked("mkdir -p -- \"$1/$2\"", arguments: [root, folder])
                var tex = "\\documentclass{article}\n\\begin{document}\nJournal body.\n\\bibliography{manuscript}\n\\end{document}\n"
                if folder == "4_journal" {
                    tex += "\\graphicspath{{figures/}}\n\\input{sections/intro}\n% \\includegraphics{unused}\n"
                    _ = try await client.runChecked("mkdir -p -- \"$1/4_journal/sections\" \"$1/4_journal/figures\"", arguments: [root])
                    for (file, text) in [
                        ("sections/intro.tex", "\\input{sections/deep}\n\\includegraphics[width=5cm]{chart}\n"),
                        ("sections/deep.tex", "Nested chapter.\n\\input{manuscript}\n"),
                        ("figures/chart.pdf", "figure fixture"), ("unused.pdf", "unused fixture")
                    ] {
                        _ = try await client.runChecked("cat > \"$1/4_journal/$2\"", arguments: [root, file], input: Data(text.utf8))
                    }
                }
                _ = try await client.runChecked("cat > \"$1/$2/manuscript.tex\"", arguments: [root, folder], input: Data(tex.utf8))
                _ = try await client.runChecked("cat > \"$1/$2/manuscript.bib\"", arguments: [root, folder],
                    input: Data("@article{\(folder), title={\(folder)}}\n".utf8))
                _ = try await client.runChecked("printf 'test pdf' > \"$1/$2/manuscript.pdf\"", arguments: [root, folder])
            }
            _ = try await client.runChecked("printf '# Review\\n' > \"$1/4_journal/review.md\"", arguments: [root])
            await workspace.pullRemote()
            let journal = mirror.root.appendingPathComponent("4_journal/manuscript.tex")
            await workspace.activateDocument(journal)
            try await waitUntil("Bibliography mixed independent manuscripts") {
                workspace.bibliographyItems.map(\.key) == ["4_journal"]
            }
            try require(workspace.fileDisplayName(journal) == "manuscript.tex (4_journal)", "Duplicate tab label is ambiguous")
            let tree = workspace.projectTree
            try require(tree.prefix(4).map(\.path) == ["1_icml2026", "2_nips2026", "3_arxiv", "4_journal"], "Main file moved ahead of its folder")
            let journalChildren = tree.first { $0.path == "4_journal" }?.children ?? []
            try require(journalChildren.filter { !$0.isDirectory }.map(\.path) == ["4_journal/manuscript.bib", "4_journal/manuscript.pdf", "4_journal/manuscript.tex", "4_journal/review.md", "4_journal/unused.pdf"], "Files left their real directory")
            try require(journalChildren.filter { !$0.isDirectory }.allSatisfy { $0.children == nil }, "Dependencies were nested under a file")
            let project = workspace.documentProject
            try require(project.tree.map(\.path) == ["4_journal/manuscript.tex"], "Project included unrelated manuscripts")
            let related = project.tree[0].children ?? []
            try require(related.map(\.path) == ["4_journal/sections/intro.tex", "4_journal/manuscript.bib"], "Project direct dependencies are wrong")
            try require(related[0].children?.map(\.path) == ["4_journal/sections/deep.tex", "4_journal/figures/chart.pdf"], "Project lost nested chapters or graphics")
            try require(related[0].children?[0].children == nil, "Project did not break include cycle")
            try require(project.outputs.map(\.path) == ["4_journal/manuscript.pdf"], "Project output PDF is not isolated")
            workspace.togglePinnedBuildTarget()
            try require(workspace.projectTree == tree, "Pinning a build target rearranged the tree")
            await workspace.activateDocument(mirror.root.appendingPathComponent("4_journal/review.md"))
            try require(workspace.projectTree == tree, "Markdown activation rearranged the tree")
            try require(workspace.documentProject.tree.map(\.path) == ["4_journal/review.md"] && workspace.documentProject.outputs.isEmpty, "Markdown did not get its own Project while a TeX was pinned")
            await workspace.activateDocument(journal)
            workspace.togglePinnedBuildTarget()
            await workspace.activateDocument(mirror.root.appendingPathComponent("1_icml2026/manuscript.bib"))
            try require(workspace.projectTree == tree, "Bib activation rearranged the tree")
            try require(workspace.buildSourceRelativePath() == "1_icml2026/manuscript.tex", "Bib selected the wrong main")
            try await waitUntil("Bib keys did not follow the selected manuscript") {
                workspace.bibliographyItems.map(\.key) == ["1_icml2026"]
            }
            await workspace.activateDocument(journal)
            try require(workspace.projectTree == tree, "TeX activation rearranged the tree")
            print("PASS Workspace hierarchy; scoped Project with nested TeX/Bib/figures, isolated PDF, cycles and Markdown; duplicate labels and citation scope")

            if ProcessInfo.processInfo.environment["PITEX_TEST_PI_TOOLS"] != nil {
                let agent = workspace.agent!
                try require(agent.contextProvider().projectRoot == mirror.root, "Agent cwd is not the SSH mirror")
                // The RPC fixture invokes the installed pi read/write/edit tools;
                // no model request or credentials are involved.
                agent.send(prompt: "Exercise the installed file tools on this temporary SSH project.")
                try await waitUntil("Agent edit did not reach the editor") {
                    workspace.documentSnapshot?.text.contains("Edited by Pitex Agent.") == true && !agent.isRunning
                }
                let deadline = ContinuousClock.now + .seconds(20)
                while try await !readRemote("4_journal/manuscript.tex").contains("Edited by Pitex Agent.") {
                    try require(.now < deadline, "Agent edit did not upload")
                    try await Task.sleep(for: .milliseconds(50))
                }
                try await waitUntil("Agent sync did not finish") { workspace.remote?.status == .synced }
                try require(try await readRemote("4_journal/agent-created.txt") == "Created by Pitex Agent.\n", "Agent write did not upload")
                try require(try await readRemote("1_icml2026/manuscript.tex").contains("Journal body."), "Agent edited the other manuscript")
                print("PASS installed pi read/write/edit tools → agent_end hook → SSH bytes")
            } else {
                print("SKIP installed pi tools: set PITEX_TEST_PI_TOOLS to the installed dist/core/tools directory")
            }

            workspace.settings.restoreSession = false
            workspace.bottomPanelVisible = false
            let window = NSWindow(contentRect: NSRect(x: 100, y: 100, width: 1000, height: 680),
                                  styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false
            window.contentView = NSHostingView(rootView: WorkspaceWindow(initialURL: nil, workspace: workspace))
            window.makeKeyAndOrderFront(nil)
            window.contentView?.layoutSubtreeIfNeeded()
            try await waitUntil("WindowReader did not attach the test window") { workspace.window === window }
            for status in [RemoteWorkspace.Status.syncing, .synced, .syncing, .synced] {
                workspace.remote?.status = status
                try await Task.sleep(for: .milliseconds(50))
                try await waitUntil("Window subtitle did not follow \(workspace.remote!.statusText); was \(window.subtitle)") {
                    window.subtitle == workspace.remote?.statusText
                }
            }
            if let frame = window.contentView?.superview,
               let bitmap = frame.bitmapImageRepForCachingDisplay(in: frame.bounds) {
                frame.cacheDisplay(in: frame.bounds, to: bitmap)
                let image = URL(fileURLWithPath: "/tmp/pitex-remote-subtitle.png")
                try bitmap.representation(using: .png, properties: [:])?.write(to: image)
            }
            func sidebarSnapshot(_ name: String) async throws -> [String: CGRect] {
                try await Task.sleep(for: .milliseconds(400))
                let view = window.contentView!
                view.layoutSubtreeIfNeeded()
                let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds)!
                view.cacheDisplay(in: view.bounds, to: bitmap)
                try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "/tmp/pitex-sidebar-\(name).png"))
                let request = VNRecognizeTextRequest()
                request.recognitionLevel = .accurate
                request.recognitionLanguages = ["en-US"]
                try VNImageRequestHandler(cgImage: bitmap.cgImage!, options: [:]).perform([request])
                var rows: [String: CGRect] = [:]
                for observation in request.results ?? [] where observation.boundingBox.minX < 0.28 {
                    guard let candidate = observation.topCandidates(1).first else { continue }
                    for name in ["Workspace", "Project", "TODOs", "4_journal", "figures", "intro.tex", "review.md"] {
                        if let range = candidate.string.range(of: name, options: .caseInsensitive), let box = try candidate.boundingBox(for: range),
                           box.boundingBox.minX < 0.28 {
                            if rows[name] == nil || box.boundingBox.midY > rows[name]!.midY { rows[name] = box.boundingBox }
                        }
                    }
                }
                return rows
            }
            func clickSidebar(_ name: String, rows: [String: CGRect]) throws {
                try require(rows[name] != nil, "Sidebar control is missing: \(name); found \(rows.keys)")
                let rect = rows[name]!
                let view = window.contentView!
                let local = NSPoint(x: view.bounds.width * rect.midX, y: view.bounds.height * (view.isFlipped ? 1 - rect.midY : rect.midY))
                let point = view.convert(local, to: nil)
                for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
                    window.sendEvent(NSEvent.mouseEvent(with: type, location: point, modifierFlags: [],
                        timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                        context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!)
                }
            }
            let initial = try await sidebarSnapshot("workspace-default")
            try require(initial["Workspace"] != nil && initial["Project"] != nil && initial["TODOs"] != nil, "Three sidebar tabs are clipped")
            try clickSidebar("4_journal", rows: initial)
            let expanded = try await sidebarSnapshot("workspace-expanded")
            try require(expanded["figures"] != nil, "Workspace folder did not expand")
            try clickSidebar("Project", rows: expanded)
            let projectRows = try await sidebarSnapshot("project-tex")
            try require(projectRows["intro.tex"] != nil, "Project did not show dependencies")
            await workspace.activateDocument(mirror.root.appendingPathComponent("4_journal/review.md"))
            let markdownRows = try await sidebarSnapshot("project-markdown")
            try require(markdownRows["review.md"] != nil, "Project omitted Markdown")
            try clickSidebar("Workspace", rows: markdownRows)
            let restored = try await sidebarSnapshot("workspace-restored")
            try require(restored["figures"] != nil, "Switching tabs lost Workspace folder expansion")
            try clickSidebar("TODOs", rows: restored)
            let todos = try await sidebarSnapshot("todos")
            try clickSidebar("Workspace", rows: todos)
            await workspace.activateDocument(journal)
            print("PASS Workspace default; Project/Markdown/TODOs tabs; folder expansion survives tab switches")

            workspace.consoleSection = .assistant
            workspace.bottomPanelVisible = true
            try await Task.sleep(for: .milliseconds(700))
            if let frame = window.contentView?.superview,
               let bitmap = frame.bitmapImageRepForCachingDisplay(in: frame.bounds) {
                frame.cacheDisplay(in: frame.bounds, to: bitmap)
                try bitmap.representation(using: .png, properties: [:])?.write(to: URL(fileURLWithPath: "/tmp/pitex-console-open.png"))
                let request = VNRecognizeTextRequest()
                request.recognitionLevel = .accurate
                request.recognitionLanguages = ["en-US"]
                try VNImageRequestHandler(cgImage: bitmap.cgImage!, options: [:]).perform([request])
                let text = (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }.joined(separator: " ")
                for label in ["Assistant", "Git Integration", "Issues", "Terminal", "Build Log"] {
                    try require(text.contains(label), "Console tab is clipped on first toggle: \(label); OCR: \(text)")
                }
            }
            window.orderOut(nil)
            print("PASS console tabs visible on first toggle at 1000×680")
            print("PASS native window subtitle follows Syncing → Synced")
            await cleanup()
        } catch {
            await cleanup()
            throw error
        }
    }
}
'''

products = []
for package in ['TexApp', 'TexCore']:
    path = repo / 'Packages' / package
    subprocess.run(['swift', 'build', '--package-path', str(path), '--build-system', 'native'], check=True)
    products.append(Path(subprocess.check_output(
        ['swift', 'build', '--package-path', str(path), '--build-system', 'native', '--show-bin-path'], text=True).strip()))
# TexApp already links shared TexCore targets; take each target only once.
objects = {}
for product in products:
    for directory in product.glob('*.build'):
        if 'Test' not in directory.name and '-tool' not in directory.name:
            files = list(directory.glob('*.swift.o'))
            if files:
                objects.setdefault(directory.name, files)
with tempfile.TemporaryDirectory(prefix='pitex-remote-check-') as temporary:
    directory = Path(temporary)
    source = directory / 'Check.swift'
    source.write_text(check)
    app = repo / 'Mac/Sources/AppShell/PitexApp.swift'
    stripped = directory / 'PitexApp.swift'
    stripped.write_text(app.read_text().replace('@main\nstruct PitexApp', 'struct PitexApp')
                        .replace('private struct WorkspaceWindow', 'struct WorkspaceWindow')
                        .replace('@StateObject private var workspace = WorkspaceModel()',
                                 '@ObservedObject var workspace = WorkspaceModel()'))
    contents = directory / 'Check.app/Contents'
    executable = contents / 'MacOS/check'
    executable.parent.mkdir(parents=True)
    resources = contents / 'Resources'
    resources.mkdir()
    for localization in (repo / 'Mac/Resources').glob('*.lproj'):
        shutil.copytree(localization, resources / localization.name)
    (contents / 'Info.plist').write_bytes(plistlib.dumps({
        'CFBundleIdentifier': 'dev.pitex.remote-autosave-check',
        'CFBundleExecutable': 'check', 'CFBundlePackageType': 'APPL',
    }))
    subprocess.run(['swiftc', '-parse-as-library', '-swift-version', '6',
                    '-target', 'arm64-apple-macos15.0',
                    *[arg for product in products for arg in ['-I', str(product / 'Modules')]],
                    str(source), str(stripped),
                    *[str(p) for p in (repo / 'Mac/Sources').rglob('*.swift') if p != app],
                    *[str(p) for files in objects.values() for p in files], '-o', str(executable)], check=True)
    # A deterministic RPC fixture runs real installed pi file tools in the cwd
    # selected by AgentCoordinator, then emits the normal agent_end event.
    agent = directory / 'pi-tools.mjs'
    agent.write_text(r"""#!/usr/bin/env node
import {createInterface} from 'node:readline';
import {pathToFileURL} from 'node:url';
import assert from 'node:assert/strict';
const emit = value => process.stdout.write(JSON.stringify(value) + '\n');
for await (const line of createInterface({input: process.stdin})) {
  const cmd = JSON.parse(line);
  if (cmd.type !== 'prompt') {
    emit({type:'response', command:cmd.type, id:cmd.id, success:true, data:{models:[],commands:[]}});
    continue;
  }
  emit({type:'agent_start'});
  try {
    const tools = process.env.PITEX_TEST_PI_TOOLS;
    const {createReadTool} = await import(pathToFileURL(tools + '/read.js'));
    const {createWriteTool} = await import(pathToFileURL(tools + '/write.js'));
    const {createEditTool} = await import(pathToFileURL(tools + '/edit.js'));
    const file = '4_journal/manuscript.tex';
    const read = await createReadTool(process.cwd()).execute('read-check', {path:file});
    assert(read.content.some(c => c.text?.includes('Journal body.')));
    await createWriteTool(process.cwd()).execute('write-check', {path:'4_journal/agent-created.txt',content:'Created by Pitex Agent.\n'});
    await createEditTool(process.cwd()).execute('edit-check', {path:file,edits:[{oldText:'Journal body.',newText:'Edited by Pitex Agent.'}]});
    emit({type:'agent_end',messages:[]});
  } catch (error) {
    process.stderr.write(String(error) + '\n');
    emit({type:'agent_end',messages:[{role:'assistant',stopReason:'error',errorMessage:String(error)}]});
  }
}
""")
    agent.chmod(0o755)
    subprocess.run([str(executable), host, '-AppleLanguages', '(en)'], cwd=directory, check=True, timeout=180,
                   env={**os.environ, 'PI_AGENT_PATH': str(agent), 'PI_CODING_AGENT_DIR': str(directory / 'pi')})
