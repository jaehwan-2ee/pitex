#!/usr/bin/env python3
"""Real window document-layout regression, without TeX, OCR, or Accessibility.

Usage: check-document-layout.py build/Build/Products/Release
Optional --baseline-source-ref 249755a runs the same assertions against that
revision's Mac sources first. Only a rendered-layout mismatch counts as a
baseline reproduction; compilation, fixture, and launch failures are errors.
Logs and PNGs survive in /tmp/pitex-document-layout-{current,baseline}/.
"""
import argparse
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile


CHECK = r'''
import AppKit
import PDFKit
import SwiftUI
import WebKit

// Hosted macOS runners expose a 1024pt screen. Keep this test's requested
// viewport, including portions outside that screen, so native pane minima
// and divider tracking exercise the same geometry as a wider user display.
// Only screen fitting changes; production WorkspaceView and NSSplitView run
// unchanged, and every drag verifies the actual window/host/split dimensions.
final class LayoutFixtureWindow: NSWindow {
    private(set) var requestedViewportWidth: CGFloat = 1440

    override func constrainFrameRect(_ frameRect: NSRect, to screen: NSScreen?) -> NSRect {
        frameRect
    }

    func setViewportWidth(_ width: CGFloat) {
        requestedViewportWidth = width
        setContentSize(NSSize(width: width, height: 800))
    }
}

@main struct DocumentLayoutCheck {
    @MainActor static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        Task { @MainActor in
            do { try await run(); print("CHECK_COMPLETE"); fflush(nil); exit(0) }
            catch { print("HARNESS_ERROR", error); fflush(nil); exit(1) }
        }
        app.run()
    }

    @MainActor static func run() async throws {
        let env = ProcessInfo.processInfo.environment
        let root = URL(fileURLWithPath: env["PITEX_LAYOUT_ROOT"]!)
        let artifacts = URL(fileURLWithPath: env["PITEX_LAYOUT_ARTIFACTS"]!)
        let defaults = UserDefaults.standard
        defaults.removePersistentDomain(forName: Bundle.main.bundleIdentifier!)
        defer { defaults.removePersistentDomain(forName: Bundle.main.bundleIdentifier!) }
        defaults.set(false, forKey: "inspectorOnLeft")
        let tex = root.appendingPathComponent("manuscript.tex")
        let markdown = root.appendingPathComponent("spsa-proof-review.md")
        // A real PDF loaded by restoreBuiltPreview, never a fake retainedPDF.
        let pdfData = CFDataCreateMutable(nil, 0)!
        var mediaBox = CGRect(x: 0, y: 0, width: 612, height: 792)
        guard let consumer = CGDataConsumer(data: pdfData),
              let context = CGContext(consumer: consumer, mediaBox: &mediaBox, nil) else {
            throw NSError(domain: "PDF fixture", code: 1)
        }
        context.beginPDFPage(nil)
        context.setFillColor(NSColor.white.cgColor)
        context.fill(mediaBox)
        context.setFillColor(NSColor.black.cgColor)
        context.fill(CGRect(x: 72, y: 640, width: 280, height: 12))
        context.endPDFPage()
        context.closePDF()
        try (pdfData as Data).write(to: tex.deletingPathExtension().appendingPathExtension("pdf"))

        func stage(_ text: String) { print("[stage]", text); fflush(nil) }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        func capture(_ host: NSView, _ name: String) {
            guard let bitmap = host.bitmapImageRepForCachingDisplay(in: host.bounds) else { return }
            host.cacheDisplay(in: host.bounds, to: bitmap)
            try? bitmap.representation(using: .png, properties: [:])?.write(
                to: artifacts.appendingPathComponent(name + ".png"))
        }
        func fail(_ kind: String, _ message: String, _ host: NSView) -> Never {
            capture(host, "failure")
            print(kind, message); fflush(nil); exit(1)
        }
        func require(_ condition: Bool, _ message: String, _ host: NSView) {
            if !condition { fail("HARNESS_ERROR", message, host) }
        }
        func outer(_ host: NSView) -> NSSplitView {
            guard let split = descendants(host).compactMap({ $0 as? NSSplitView })
                .first(where: { $0.isVertical }) else {
                fail("HARNESS_ERROR", "Missing mounted outer NSSplitView", host)
            }
            return split
        }
        func widths(_ host: NSView) -> [CGFloat] {
            outer(host).arrangedSubviews.map { $0.frame.width }
        }
        func requireViewport(_ host: NSView, _ label: String, minimumWidth: CGFloat = 0) {
            guard let window = host.window as? LayoutFixtureWindow else {
                fail("HARNESS_ERROR", "Missing layout fixture window", host)
            }
            let requested = window.requestedViewportWidth
            let contentWidth = window.contentRect(forFrameRect: window.frame).width
            let hostWidth = host.bounds.width
            let splitWidth = outer(host).bounds.width
            let details = "\(label): requested=\(requested) windowFrame=\(window.frame.width) "
                + "windowContent=\(contentWidth) host=\(hostWidth) split=\(splitWidth) minimum=\(minimumWidth)"
            stage("viewport \(details)")
            require([contentWidth, hostWidth, splitWidth].allSatisfy { abs($0 - requested) <= 1 },
                    "Fixture viewport was constrained: \(details)", host)
            require(splitWidth >= minimumWidth, "Fixture cannot fit requested divider sizes: \(details)", host)
        }
        func same(_ actual: [CGFloat], _ expected: [CGFloat], tolerance: CGFloat = 1) -> Bool {
            actual.count == expected.count && zip(actual, expected).allSatisfy { abs($0 - $1) <= tolerance }
        }
        func tick(_ host: NSView) async throws {
            host.layoutSubtreeIfNeeded()
            try await Task.sleep(for: .milliseconds(75))
            host.layoutSubtreeIfNeeded()
        }
        func mounted(_ view: NSView, in host: NSView) -> Bool {
            view.window === host.window && view.isDescendant(of: host)
                && !view.isHiddenOrHasHiddenAncestor && view.visibleRect.width > 0 && view.visibleRect.height > 0
        }
        // activeDocumentURL and phase alone are insufficient: both publish
        // before SwiftUI replaces the editor and PDFKit/WebKit content.
        func rendered(_ workspace: WorkspaceModel, _ host: NSView, _ url: URL) async throws {
            let expectedText = try String(contentsOf: url, encoding: .utf8)
            let deadline = ContinuousClock.now + .seconds(12)
            while .now < deadline {
                try await tick(host)
                guard workspace.activeDocumentURL == url,
                      let editor = workspace.environment?.editor.textView,
                      mounted(editor, in: host), editor.string == expectedText else { continue }
                let views = descendants(host)
                let pdfs = views.compactMap { $0 as? PDFView }.filter { mounted($0, in: host) }
                let webs = views.compactMap { $0 as? WKWebView }.filter { mounted($0, in: host) }
                if url.pathExtension == "md" {
                    guard pdfs.isEmpty, webs.count == 1, !webs[0].isLoading else { continue }
                    let ready = try? await webs[0].evaluateJavaScript(
                        "document.readyState === 'complete' && document.querySelector('h1')?.textContent === 'Layout Markdown fixture'")
                    if ready as? Bool == true { return }
                } else if webs.isEmpty, pdfs.count == 1, pdfs[0].document?.pageCount == 1,
                          pdfs[0].currentPage != nil, pdfs[0].documentView?.visibleRect.isEmpty == false {
                    return
                }
            }
            fail("HARNESS_ERROR", "New editor and rendered preview never mounted for \(url.lastPathComponent); widths=\(widths(host))", host)
        }
        // Every sample after the new content renders must match. Waiting
        // until the expected geometry appears would hide jumps/late drift.
        func expect(_ host: NSView, _ expected: [CGFloat], _ label: String) async throws {
            for sample in 0...8 {
                if sample > 0 { try await tick(host) }
                let actual = widths(host)
                if !same(actual, expected) {
                    fail("LAYOUT_MISMATCH", "\(label) sample=\(sample) expected=\(expected) actual=\(actual) tolerance=1pt", host)
                }
            }
            stage("PASS \(label): \(widths(host))")
        }
        func stable(_ host: NSView, _ label: String) async throws -> [CGFloat] {
            var last = widths(host)
            var count = 0
            for _ in 0..<40 {
                try await tick(host)
                let actual = widths(host)
                count = same(actual, last, tolerance: 0.1) ? count + 1 : 0
                last = actual
                if count == 8 { stage("stable \(label): \(actual)"); return actual }
            }
            fail("HARNESS_ERROR", "Unstable geometry at \(label): \(last)", host)
        }
        func mount(_ workspace: WorkspaceModel) -> (LayoutFixtureWindow, NSView) {
            let host = NSHostingView(rootView: WorkspaceView(workspace: workspace))
            let window = LayoutFixtureWindow(contentRect: NSRect(x: 0, y: 0, width: 1440, height: 800),
                                  styleMask: [.titled, .resizable], backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false
            window.contentView = host
            workspace.window = window
            window.makeKeyAndOrderFront(nil)
            // Reapply after ordering, which can otherwise fit a new window
            // to the runner's screen before the first native layout pass.
            window.setViewportWidth(1440)
            NSApp.activate()
            return (window, host)
        }
        // Drive NSSplitView's real tracking loop. Queue drag/up BEFORE
        // mouseDown enters it; no synchronous sendEvent waiting for a later
        // main-actor task, global event injection, or Accessibility grant.
        func drag(_ host: NSView, divider: Int, to position: CGFloat) async throws {
            let split = outer(host)
            requireViewport(host, "before native drag divider=\(divider)",
                            minimumWidth: 170 + 460 + 300 + 2 * split.dividerThickness)
            require(split.arrangedSubviews.count == 3 && (0..<2).contains(divider),
                    "Native drag requires three panes and a valid divider", host)
            let minima: [CGFloat] = defaults.bool(forKey: "inspectorOnLeft") ? [300, 460, 170] : [170, 460, 300]
            let pane = split.arrangedSubviews[divider]
            let minimum = pane.frame.minX + minima[divider]
            let maximum = split.arrangedSubviews[divider + 1].frame.maxX
                - minima[divider + 1] - split.dividerThickness
            require(position >= minimum && position <= maximum,
                    "Drag target outside native pane constraints: target=\(position) range=\(minimum)...\(maximum) widths=\(widths(host))", host)
            let start = NSPoint(x: pane.frame.maxX + split.dividerThickness / 2, y: split.bounds.midY)
            let end = NSPoint(x: position + split.dividerThickness / 2, y: start.y)
            guard let window = split.window else { fail("HARNESS_ERROR", "Drag window missing", host) }
            func event(_ type: NSEvent.EventType, _ point: NSPoint, _ number: Int) -> NSEvent {
                NSEvent.mouseEvent(with: type, location: split.convert(point, to: nil), modifierFlags: [],
                    timestamp: ProcessInfo.processInfo.systemUptime + Double(number) * 0.01,
                    windowNumber: window.windowNumber, context: nil, eventNumber: number,
                    clickCount: 1, pressure: type == .leftMouseUp ? 0 : 1)!
            }
            NSApp.postEvent(event(.leftMouseUp, end, 3), atStart: true)
            NSApp.postEvent(event(.leftMouseDragged, end, 2), atStart: true)
            split.mouseDown(with: event(.leftMouseDown, start, 1))
            host.layoutSubtreeIfNeeded()
            let landed = outer(host).arrangedSubviews[divider].frame.maxX
            require(abs(landed - position) <= 1, "Native drag did not land: divider=\(divider) target=\(position) actual=\(landed)", host)
        }
        func resize(_ host: NSView, sidebar: CGFloat, inspector: CGFloat) async throws -> [CGFloat] {
            let split = outer(host)
            require(split.arrangedSubviews.count == 3, "Expected three panes before manual resize", host)
            requireViewport(host, "before sizing sidebar=\(sidebar) inspector=\(inspector)",
                            minimumWidth: sidebar + 460 + inspector + 2 * split.dividerThickness)
            try await drag(host, divider: 0, to: sidebar)
            try await drag(host, divider: 1, to: split.bounds.width - split.dividerThickness - inspector)
            let expected = [sidebar, split.bounds.width - sidebar - inspector - 2 * split.dividerThickness, inspector]
            try await expect(host, expected, "native manual divider sizes")
            return expected
        }
        func activate(_ workspace: WorkspaceModel, _ host: NSView, _ url: URL,
                      _ expected: [CGFloat], _ label: String) async throws {
            stage("activate \(label)")
            await workspace.activateDocument(url)
            try await rendered(workspace, host, url)
            try await expect(host, expected, label)
        }

        let workspace = WorkspaceModel()
        await workspace.open(tex)
        let (window, host) = mount(workspace)
        defer { window.orderOut(nil) }
        try await rendered(workspace, host, tex)
        let initial = try await stable(host, "fresh PDF workspace")
        requireViewport(host, "fresh PDF workspace")
        require(initial.count == 3 && abs(initial[0] - 170) <= 1, "Fresh sidebar must be 170pt: \(initial)", host)
        capture(host, "fresh-pdf")
        let a = try await resize(host, sidebar: 230, inspector: 460)
        capture(host, "tex-manual")
        try await activate(workspace, host, markdown, a, "first unseen Markdown inherits all widths")
        let b = try await resize(host, sidebar: 280, inspector: 340)
        capture(host, "markdown-manual")
        for cycle in 1...3 {
            try await activate(workspace, host, tex, a, "cycle \(cycle) TeX restores A")
            try await activate(workspace, host, markdown, b, "cycle \(cycle) Markdown restores B")
        }
        try await activate(workspace, host, tex, a, "return A before pane lifecycle")
        UserDefaults.standard.set(true, forKey: "inspectorOnLeft")
        _ = try await stable(host, "mirror")
        try await expect(host, Array(a.reversed()), "mirror preserves all role widths")
        try await activate(workspace, host, markdown, Array(b.reversed()), "mirrored B")
        try await activate(workspace, host, tex, Array(a.reversed()), "mirrored A")
        UserDefaults.standard.set(false, forKey: "inspectorOnLeft")
        _ = try await stable(host, "unmirror")
        try await expect(host, a, "unmirror A")
        workspace.sidebarVisible = false
        _ = try await stable(host, "sidebar hidden")
        require(widths(host).count == 2, "Sidebar did not unmount", host)
        workspace.sidebarVisible = true
        _ = try await stable(host, "sidebar shown")
        try await expect(host, a, "sidebar hide/show restores all widths")
        workspace.detachPreview()
        _ = try await stable(host, "preview detached")
        require(widths(host).count == 2 && !descendants(host).contains { $0 is PDFView }, "Inline preview did not detach", host)
        guard let detached = workspace.detachedPreviewWindow?.contentView else {
            fail("HARNESS_ERROR", "Detached preview window missing", host)
        }
        let detachedDeadline = ContinuousClock.now + .seconds(5)
        while !descendants(detached).contains(where: { ($0 as? PDFView)?.document?.pageCount == 1 }) && .now < detachedDeadline {
            try await tick(detached)
        }
        require(descendants(detached).contains { ($0 as? PDFView)?.document?.pageCount == 1 }, "Detached PDF never rendered", host)
        workspace.attachPreview()
        try await rendered(workspace, host, tex)
        try await expect(host, a, "detach/attach restores all widths")

        // Same URLs in a second model/window must not share the first's cache.
        let independent = WorkspaceModel()
        await independent.open(tex)
        let (secondWindow, secondHost) = mount(independent)
        defer { secondWindow.orderOut(nil) }
        try await rendered(independent, secondHost, tex)
        let secondInitial = try await stable(secondHost, "independent workspace")
        requireViewport(secondHost, "independent workspace")
        require(secondInitial.count == 3 && abs(secondInitial[0] - 170) <= 1,
                "Independent workspace inherited first's sidebar: \(secondInitial)", secondHost)
        let independentA = try await resize(secondHost, sidebar: 205, inspector: 420)
        try await activate(independent, secondHost, markdown, independentA, "independent unseen B")
        try await activate(workspace, host, markdown, b, "first workspace cache remains B")
        try await activate(workspace, host, tex, a, "first workspace cache remains A")
        await independent.close()
        secondWindow.orderOut(nil)
        window.makeKeyAndOrderFront(nil)
        NSApp.activate()

        // Clamp at a narrow viewport, then drag just one side. Recording
        // that drag must retain the other side's ACTUAL width, while widening
        // restores its undragged preference (600). Exercise both orientations.
        for mirrored in [false, true] {
            defaults.set(false, forKey: "inspectorOnLeft")
            window.setViewportWidth(1600)
            _ = try await stable(host, "wide before narrow test")
            _ = try await resize(host, sidebar: 400, inspector: 600)
            defaults.set(mirrored, forKey: "inspectorOnLeft")
            window.setViewportWidth(980)
            let narrow = try await stable(host, "narrow mirrored=\(mirrored)")
            require(narrow.count == 3 && narrow[1] >= 459, "Editor minimum violated: \(narrow)", host)
            let split = outer(host)
            let inspector = mirrored ? narrow[0] : narrow[2]
            let target = mirrored ? split.bounds.width - split.dividerThickness - 180 : 180
            try await drag(host, divider: mirrored ? 1 : 0, to: target)
            let roles: [CGFloat] = [180, split.bounds.width - 180 - inspector - 2 * split.dividerThickness, inspector]
            let realized = mirrored ? Array(roles.reversed()) : roles
            try await expect(host, realized, "narrow drag retains unmoved inspector mirrored=\(mirrored)")
            workspace.sidebarSection = .bibtex
            try await expect(host, realized, "ordinary update retains narrow realization")
            await workspace.activateDocument(markdown)
            try await rendered(workspace, host, markdown)
            try await activate(workspace, host, tex, realized, "narrow tab roundtrip")
            defaults.set(!mirrored, forKey: "inspectorOnLeft")
            _ = try await stable(host, "mirror narrow realization")
            try await expect(host, Array(realized.reversed()), "mirror retains narrow realization")
            defaults.set(mirrored, forKey: "inspectorOnLeft")
            window.setViewportWidth(1600)
            _ = try await stable(host, "expanded")
            requireViewport(host, "expanded")
            let wideRoles: [CGFloat] = [180, outer(host).bounds.width - 780 - 2 * outer(host).dividerThickness, 600]
            try await expect(host, mirrored ? Array(wideRoles.reversed()) : wideRoles,
                             "expansion restores undragged inspector preference")
        }
        capture(host, "final")
        await workspace.close()
    }
}
'''


def run_check(repo, products, source_ref=None):
    mode = 'baseline' if source_ref else 'current'
    artifacts = Path('/tmp') / f'pitex-document-layout-{mode}'
    artifacts.mkdir(exist_ok=True)
    for artifact in artifacts.glob('*'):
        if artifact.is_file():
            artifact.unlink()
    with tempfile.TemporaryDirectory(prefix=f'pitex-layout-{mode}-', dir='/tmp') as directory:
        root = Path(directory)
        source_root = repo
        if source_ref:
            # Copy ALL Mac sources: the baseline must not accidentally use
            # today's model or native split implementation. Never touch checkout.
            source_root = root / 'baseline'
            names = subprocess.check_output(
                ['git', 'ls-tree', '-r', '--name-only', source_ref, '--', 'Mac/Sources'],
                cwd=repo, text=True).splitlines()
            for name in names:
                destination = source_root / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(subprocess.check_output(['git', 'show', f'{source_ref}:{name}'], cwd=repo))
        bundle = root / 'DocumentLayoutCheck.app/Contents'
        (bundle / 'MacOS').mkdir(parents=True)
        resources = bundle / 'Resources'
        resources.mkdir()
        for locale in (repo / 'Mac/Resources').glob('*.lproj'):
            shutil.copytree(locale, resources / locale.name)
        shutil.copyfile(repo / 'Mac/Resources/markdown-preview.html', resources / 'markdown-preview.html')
        (bundle / 'Info.plist').write_bytes(plistlib.dumps({
            'CFBundleExecutable': 'check', 'CFBundleIdentifier': f'test.pitex.document-layout.{mode}',
            'CFBundleDevelopmentRegion': 'en', 'CFBundlePackageType': 'APPL',
        }))
        fixture = root / 'fixture'
        fixture.mkdir()
        (fixture / 'manuscript.tex').write_text(
            '\\documentclass{article}\n\\begin{document}\n\\section{Layout TeX fixture}\nPDF preview.\n\\end{document}\n')
        (fixture / 'spsa-proof-review.md').write_text('# Layout Markdown fixture\n\nA different editor and rendered preview.\n')
        check = root / 'Check.swift'
        check.write_text(CHECK)
        app_main = source_root / 'Mac/Sources/AppShell/PitexApp.swift'
        stripped = root / 'PitexApp.swift'
        app_text = app_main.read_text()
        anchor = '@main\nstruct PitexApp'
        if app_text.count(anchor) != 1:
            raise RuntimeError('Expected exactly one app entry point')
        stripped.write_text(app_text.replace(anchor, 'struct PitexApp', 1))
        command = ['xcrun', 'swiftc', '-parse-as-library', '-swift-version', '6',
                   '-target', 'arm64-apple-macos15.0', '-I', str(products), str(check), str(stripped),
                   *map(str, sorted(p for p in (source_root / 'Mac/Sources').rglob('*.swift') if p != app_main)),
                   *map(str, sorted(products.glob('*.o'))), '-o', str(bundle / 'MacOS/check')]
        with (artifacts / 'compile.log').open('w') as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
        if result.returncode:
            print((artifacts / 'compile.log').read_text(), end='')
            raise RuntimeError(f'{mode}: compilation failed (not a layout reproduction)')
        out_log, err_log = artifacts / 'stdout.log', artifacts / 'stderr.log'
        app = bundle.parent
        try:
            subprocess.run(['/usr/bin/open', '-n', '-W', '--stdout', str(out_log), '--stderr', str(err_log),
                            '--env', 'PI_AGENT_PATH=/usr/bin/false',
                            '--env', f'PI_CODING_AGENT_DIR={root / "pi"}',
                            '--env', f'PITEX_LAYOUT_ROOT={fixture}',
                            '--env', f'PITEX_LAYOUT_ARTIFACTS={artifacts}',
                            str(app), '--args', '-AppleLanguages', '(en)'], check=True, timeout=120)
        finally:
            subprocess.run(['/usr/bin/pkill', '-f', str(app)], check=False)
            for log in (out_log, err_log):
                if log.exists():
                    print(log.read_text(errors='replace'), end='')
        output = out_log.read_text(errors='replace') if out_log.exists() else ''
        if source_ref:
            reproduced = re.search(
                r'^LAYOUT_MISMATCH (first unseen Markdown|cycle [123] (TeX|Markdown))', output, re.MULTILINE)
            if not reproduced or 'HARNESS_ERROR ' in output or 'CHECK_COMPLETE' in output:
                raise RuntimeError(f'{mode}: expected a rendered document-switch mismatch, not a setup/drag/launch failure or pass')
            print(f'BASELINE_REPRODUCED {source_ref}')
        elif 'CHECK_COMPLETE' not in output or 'LAYOUT_MISMATCH ' in output or 'HARNESS_ERROR ' in output:
            raise RuntimeError('current: native layout acceptance failed; see retained logs and failure.png')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('products', type=Path)
    parser.add_argument('--baseline-source-ref', metavar='REF', help='diagnose old Mac sources first (use 249755a)')
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    if args.baseline_source_ref:
        run_check(repo, args.products.resolve(), args.baseline_source_ref)
    run_check(repo, args.products.resolve())


if __name__ == '__main__':
    main()
