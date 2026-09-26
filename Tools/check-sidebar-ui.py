#!/usr/bin/env python3
"""check-sidebar-ui.py [Build/Products/Release] (or build with Swift command-line tools).

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
if len(sys.argv) > 1:
    products = Path(sys.argv[1]).resolve()
    includes = [products]
    objects = list(products.glob('*.o'))
else:
    includes, targets = [], {}
    for package in ['TexApp', 'TexCore']:
        path = repo / 'Packages' / package
        subprocess.run(['swift', 'build', '--package-path', str(path), '--build-system', 'native'], check=True)
        products = Path(subprocess.check_output(['swift', 'build', '--package-path', str(path),
            '--build-system', 'native', '--show-bin-path'], text=True).strip())
        includes.append(products / 'Modules')
        for target in products.glob('*.build'):
            if 'Test' not in target.name and '-tool' not in target.name:
                files = list(target.glob('*.swift.o'))
                if files:
                    targets.setdefault(target.name, files)
    objects = [p for files in targets.values() for p in files]
check = r'''
import Vision

@main struct SidebarCheck {
    @MainActor static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.accessory)
        Task { @MainActor in
            do { try await run(); exit(0) }
            catch { print("FAIL", error); fflush(nil); exit(1) }
        }
        app.run()
    }

    @MainActor static func run() async throws {
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        let language = CommandLine.arguments[2]
        let workspace = WorkspaceModel()
        await workspace.open(root.appendingPathComponent("main.tex"))
        guard case .ready = workspace.phase else { fatalError("Open failed: \(workspace.phase)") }
        let deadline = ContinuousClock.now + .seconds(5)
        while workspace.todoItems.count != 1 && .now < deadline {
            try await Task.sleep(for: .milliseconds(50))
        }
        precondition(workspace.todoItems.count == 1, "TODO parsing did not finish: \(workspace.todoItems.count)")
        // Distinct dark/light theme accents must reach the custom navigator buttons.
        let tint = language == "en"
            ? NSColor(srgbRed: 0.55, green: 0.67, blue: 0.93, alpha: 1)
            : NSColor(srgbRed: 0.84, green: 0.23, blue: 0.29, alpha: 1)
        let host = NSHostingView(rootView: HStack(alignment: .top) {
            ProjectSidebarView(workspace: workspace).frame(width: 280, height: 650)
            SymbolsPaletteView(onInsert: { _ in }).frame(width: 320, height: 320)
        }.padding(10).background(Color(nsColor: .windowBackgroundColor))
            .tint(Color(nsColor: tint))
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
        let upper = descendants(host).compactMap { $0 as? NSSegmentedControl }.first { $0.segmentCount == 3 }!
        let split = descendants(host).compactMap { $0 as? NSSplitView }.first { !$0.isVertical && $0.arrangedSubviews.count == 2 }!
        precondition(workspace.projectTree.first { $0.path == "main.tex" }?.name == "main.tex")
        precondition(workspace.documentProject.tree.first?.name == "main.tex (.)", "Project must retain duplicate-name context")
        upper.selectedSegment = 1
        precondition(upper.sendAction(upper.action, to: upper.target))
        try await settle()
        precondition(workspace.sidebarSection == .labels)
        func snapshot() throws -> [(String, CGRect)] {
            let bitmap = host.bitmapImageRepForCachingDisplay(in: host.bounds)!
            host.cacheDisplay(in: host.bounds, to: bitmap)
            try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "/tmp/pitex-sidebar-layout-live-\(language).png"))
            let request = VNRecognizeTextRequest()
            request.recognitionLevel = .accurate
            request.recognitionLanguages = language == "ko" ? ["ko-KR", "en-US"] : ["en-US"]
            try VNImageRequestHandler(cgImage: bitmap.cgImage!, options: [:]).perform([request])
            let titles = ["sidebar.workspace", "sidebar.project", "sidebar.todos"].map {
                Bundle.main.localizedString(forKey: $0, value: nil, table: nil)
            }
            return try (request.results ?? []).flatMap { observation -> [(String, CGRect)] in
                guard let candidate = observation.topCandidates(1).first else { return [] }
                var rows = [(candidate.string, observation.boundingBox)]
                for title in titles {
                    if let range = candidate.string.range(of: title, options: .caseInsensitive), let box = try candidate.boundingBox(for: range) {
                        rows.append((title, box.boundingBox))
                    }
                }
                return rows
            }.filter { $0.1.minX < 0.45 }
        }
        func clickTab(_ key: String) async throws {
            let title = Bundle.main.localizedString(forKey: key, value: nil, table: nil)
            let candidates = try snapshot()
            let rows = candidates.filter { $0.0 == title }
            guard let rect = rows.max(by: { $0.1.midY < $1.1.midY })?.1 else {
                fatalError("Missing tab \(title): \(candidates)")
            }
            let local = NSPoint(x: host.bounds.width * rect.midX,
                y: host.bounds.height * (host.isFlipped ? 1 - rect.midY : rect.midY))
            let point = host.convert(local, to: nil)
            for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
                window.sendEvent(NSEvent.mouseEvent(with: type, location: point, modifierFlags: [],
                    timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                    context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!)
            }
            try await settle()
        }
        // Exercise the native split view's resize path at two divider positions.
        split.setPosition(420, ofDividerAt: 0)
        try await settle()
        let before = try snapshot().filter { $0.0.contains("zfile_") }.count
        split.setPosition(240, ofDividerAt: 0)
        try await settle()
        let after = try snapshot()
        let afterCount = after.filter { $0.0.contains("zfile_") }.count
        precondition(afterCount >= before + 5, "Resizing must reveal more Workspace rows: \(before) → \(afterCount)")
        precondition(!after.contains { $0.0.contains("(.)") }, "Workspace leaked duplicate-name folder suffix")
        let height = split.arrangedSubviews[0].frame.height
        for key in ["sidebar.project", "sidebar.todos", "sidebar.workspace"] {
            try await clickTab(key)
            precondition(abs(split.arrangedSubviews[0].frame.height - height) < 3, "Tab switch reset the divider")
            precondition(workspace.sidebarSection == .labels, "Lower tabs changed document structure selection")
            precondition(workspace.todoItems.count == 1)
        }
        let visibleRows = try snapshot().filter { $0.0.contains("zfile_") }
        precondition(visibleRows.count == afterCount, "Workspace viewport shrank after tab switching")
        print("PASS \(language): resized divider reveals \(before) → \(afterCount) rows; Workspace basenames; Project labels and split height retained")
        let expected = SymbolCategory.allCases.map {
            Bundle.main.localizedString(forKey: $0.titleKey, value: nil, table: nil)
        }
        precondition(!expected.contains { $0.hasPrefix("editor.") })
        let bitmap = host.bitmapImageRepForCachingDisplay(in: host.bounds)!
        host.cacheDisplay(in: host.bounds, to: bitmap)
        let workspaceTitle = Bundle.main.localizedString(forKey: "sidebar.workspace", value: nil, table: nil)
        let tab = try snapshot().filter { $0.0 == workspaceTitle }.max { $0.1.midY < $1.1.midY }!.1
        let pixels = bitmap.converting(to: .sRGB, renderingIntent: .default)!
        var tintedPixels = 0
        for y in Int((1 - tab.maxY) * Double(bitmap.pixelsHigh))..<Int((1 - tab.minY) * Double(bitmap.pixelsHigh)) {
            for x in Int(tab.minX * Double(bitmap.pixelsWide))..<Int(tab.maxX * Double(bitmap.pixelsWide)) {
                guard let color = pixels.colorAt(x: x, y: y) else { continue }
                if abs(color.redComponent - tint.redComponent) < 0.04 &&
                    abs(color.greenComponent - tint.greenComponent) < 0.04 &&
                    abs(color.blueComponent - tint.blueComponent) < 0.04 { tintedPixels += 1 }
            }
        }
        precondition(tintedPixels > 30, "Selected Workspace button ignored the theme tint: \(tintedPixels) pixels")
        print("PASS \(language): selected navigator button uses the inherited theme tint")
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
        print("PASS \(language): localized Symbols title and independent Workspace/Project/TODO buttons")
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
    (root / 'other').mkdir()
    (root / 'other/main.tex').write_text('Other document.\n')
    for i in range(40):
        (root / f'zfile_{i:02d}.tex').write_text('Chapter.\n')
    app_main = repo / 'Mac/Sources/AppShell/PitexApp.swift'
    stripped = root / 'PitexApp.swift'
    stripped.write_text(app_main.read_text().replace('@main\nstruct PitexApp', 'struct PitexApp'))
    workspace_view = repo / 'Mac/Sources/Features/WorkspaceView.swift'
    source = root / 'Check.swift'
    # Keep the check in the same file as the private SymbolsPaletteView.
    source.write_text(workspace_view.read_text() + '\n' + check)
    executable = bundle / 'MacOS/check'
    subprocess.run(['xcrun', 'swiftc', '-parse-as-library', '-swift-version', '6',
                    '-target', 'arm64-apple-macos15.0', *[arg for path in includes for arg in ['-I', str(path)]], str(source), str(stripped),
                    *[str(p) for p in (repo / 'Mac/Sources').rglob('*.swift') if p not in (app_main, workspace_view)],
                    *[str(p) for p in objects], '-o', str(executable)], check=True)
    env = {**os.environ, 'PI_AGENT_PATH': '/usr/bin/false', 'PI_CODING_AGENT_DIR': str(root / 'pi')}
    for language in ['en', 'ko']:
        subprocess.run([str(executable), str(root), language, '-AppleLanguages', f'({language})'],
                       env=env, check=True, timeout=60)
        shutil.copyfile(root / f'sidebar-{language}.png', Path('/tmp') / f'pitex-sidebar-{language}.png')
