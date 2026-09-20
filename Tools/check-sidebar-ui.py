#!/usr/bin/env python3
"""After Release build: check-sidebar-ui.py <Build/Products/Release>.

Render the real sidebar and Symbols picker in English and Korean, using a
temporary project and an isolated app bundle. No model requests or user files.
"""
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
check = r'''
import Vision

@main struct SidebarCheck {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        let language = CommandLine.arguments[2]
        let workspace = WorkspaceModel()
        await workspace.open(root.appendingPathComponent("main.tex"))
        guard case .ready = workspace.phase else { fatalError("Open failed: \(workspace.phase)") }
        try await Task.sleep(for: .milliseconds(300)) // Debounced structure parsing.
        precondition(workspace.todoItems.count == 1)
        let host = NSHostingView(rootView: HStack(alignment: .top) {
            ProjectSidebarView(workspace: workspace).frame(width: 280, height: 650)
            SymbolsPaletteView(onInsert: { _ in }).frame(width: 320, height: 320)
        }.padding(10).background(Color(nsColor: .windowBackgroundColor))
            .environment(\.locale, Locale(identifier: language)))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 640, height: 700),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host
        window.makeKeyAndOrderFront(nil)
        defer {
            window.orderOut(nil)
            UserDefaults.standard.removePersistentDomain(forName: Bundle.main.bundleIdentifier!)
        }
        func settle() async throws {
            host.layoutSubtreeIfNeeded()
            try await Task.sleep(for: .milliseconds(300))
            host.layoutSubtreeIfNeeded()
        }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        try await settle()
        let segments = descendants(host).compactMap { $0 as? NSSegmentedControl }
        precondition(segments.map(\.segmentCount).sorted() == [2, 3], "Expected independent three- and two-button pickers")
        let upper = segments.first { $0.segmentCount == 3 }!
        let lower = segments.first { $0.segmentCount == 2 }!
        precondition(lower.convert(lower.bounds, to: nil).maxY < upper.convert(upper.bounds, to: nil).minY)
        upper.selectedSegment = 1
        precondition(upper.sendAction(upper.action, to: upper.target))
        try await settle()
        precondition(workspace.sidebarSection == .labels)
        for selected in [1, 0, 1] {
            lower.selectedSegment = selected
            precondition(lower.sendAction(lower.action, to: lower.target))
            try await settle()
            precondition(workspace.sidebarSection == .labels, "Project/TODO switching changed document structure selection")
            precondition(workspace.todoItems.count == 1, "Switching panes changed tasks")
        }
        let expected = SymbolCategory.allCases.map {
            Bundle.main.localizedString(forKey: $0.titleKey, value: nil, table: nil)
        }
        precondition(!expected.contains { $0.hasPrefix("editor.") })
        let bitmap = host.bitmapImageRepForCachingDisplay(in: host.bounds)!
        host.cacheDisplay(in: host.bounds, to: bitmap)
        try bitmap.representation(using: .png, properties: [:])!.write(to: root.appendingPathComponent("sidebar-\(language).png"))
        // Read the rendered picker, so Text(String) regressing to a raw key
        // fails even though every translation is still present in the bundle.
        let request = VNRecognizeTextRequest()
        request.recognitionLevel = .accurate
        request.recognitionLanguages = language == "ko" ? ["ko-KR", "en-US"] : ["en-US"]
        try VNImageRequestHandler(cgImage: bitmap.cgImage!, options: [:]).perform([request])
        let text = (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }
        func compact(_ text: String) -> String { text.replacingOccurrences(of: " ", with: "").lowercased() }
        precondition(text.contains { compact($0).contains(compact(expected[0])) }, "Missing rendered category title: \(text)")
        print("PASS \(language): localized Symbols title on screen, 14 translations, independent Project/TODO buttons")
        await workspace.close()
    }
}
'''

with tempfile.TemporaryDirectory(prefix='pitex-sidebar-', dir='/tmp') as directory:
    root = Path(directory)
    bundle = root / 'SidebarCheck.app/Contents'
    (bundle / 'MacOS').mkdir(parents=True)
    resources = bundle / 'Resources'
    resources.mkdir()
    (bundle / 'Info.plist').write_bytes(plistlib.dumps({
        'CFBundleExecutable': 'check', 'CFBundleIdentifier': 'test.pitex.sidebar',
        'CFBundleDevelopmentRegion': 'en', 'CFBundlePackageType': 'APPL',
    }))
    for locale in (repo / 'Mac/Resources').glob('*.lproj'):
        shutil.copytree(locale, resources / locale.name)
    (root / 'main.tex').write_text('\\documentclass{article}\n\\begin{document}\n\\section{Test}\n% TODO: Review this section\n\\end{document}\n')
    app_main = repo / 'Mac/Sources/AppShell/PitexApp.swift'
    stripped = root / 'PitexApp.swift'
    stripped.write_text(app_main.read_text().replace('@main\nstruct PitexApp', 'struct PitexApp'))
    workspace_view = repo / 'Mac/Sources/Features/WorkspaceView.swift'
    source = root / 'Check.swift'
    # Keep the check in the same file as the private SymbolsPaletteView.
    source.write_text(workspace_view.read_text() + '\n' + check)
    executable = bundle / 'MacOS/check'
    subprocess.run(['xcrun', 'swiftc', '-parse-as-library', '-swift-version', '6',
                    '-target', 'arm64-apple-macos15.0', '-I', str(products), str(source), str(stripped),
                    *[str(p) for p in (repo / 'Mac/Sources').rglob('*.swift') if p not in (app_main, workspace_view)],
                    *[str(p) for p in products.glob('*.o')], '-o', str(executable)], check=True)
    env = {**os.environ, 'PI_AGENT_PATH': '/usr/bin/false', 'PI_CODING_AGENT_DIR': str(root / 'pi')}
    for language in ['en', 'ko']:
        subprocess.run([str(executable), str(root), language, '-AppleLanguages', f'({language})'],
                       env=env, check=True, timeout=60)
        shutil.copyfile(root / f'sidebar-{language}.png', Path('/tmp') / f'pitex-sidebar-{language}.png')
