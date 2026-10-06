#!/usr/bin/env python3
"""check-sidebar-ui.py [Build/Products/Release] (or build with Swift command-line tools).

Render the real sidebar and Symbols picker in English and Korean, using a
temporary project and an isolated app bundle. No model requests or user files.
The English pass additionally mounts the whole WorkspaceView in a resizable
window to prove the outer vertical split opens the sidebar at its 170pt
minimum and that real divider drags survive tab switches, hide/show, and the
mirrored layout — then checks the empty TODOs pane keeps its header row
pinned to the pane top across the inner divider resize. Fixture path and
language arrive through --env; naked argv is only -AppleLanguages.
Pass --geometry-only for the Golden Gate runner without working Vision OCR.
This runs the real-window width/retention checks and measures header Text
frames using background NSViews injected into a temporary source copy.
The default mode retains the full English/Korean rendered acceptance suite.
"""
import argparse
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile


def instrument_headers(source):
    """Add passive probes only to the two exact production header Text sites."""
    anchors = [
        ('Text("sidebar.todos")',
         'key: "sidebar.todos", active: navigatorSection == .todos'),
        ('Text(isWorkspace ? "sidebar.workspace" : "sidebar.project")',
         'key: isWorkspace ? "sidebar.workspace" : "sidebar.project", '
         'active: navigatorSection == (isWorkspace ? .workspace : .project)'),
    ]
    for text, arguments in anchors:
        anchor = (text + '\n                    .font(.caption.weight(.semibold))'
                  + '\n                    .foregroundStyle(.secondary)')
        if source.count(anchor) != 1:
            raise RuntimeError(f'Expected exactly one production header anchor: {text}')
        source = source.replace(anchor, anchor + '\n                    .background('
                                + f'SidebarHeaderProbe({arguments}))', 1)
    return source


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('products', nargs='?')
parser.add_argument('--geometry-only', action='store_true')
args = parser.parse_args()
repo = Path(__file__).resolve().parent.parent
if args.products:
    products = Path(args.products).resolve()
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
import CoreML
import Vision

// Background markers receive the foreground Text's proposed size. They have
// no drawing, intrinsic size, hit testing, or effect on the parent's layout.
final class SidebarHeaderMarker: NSView {
    var key = ""
    var active = false
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
}

struct SidebarHeaderProbe: NSViewRepresentable {
    let key: String
    let active: Bool
    func makeNSView(context: Context) -> SidebarHeaderMarker { SidebarHeaderMarker() }
    func updateNSView(_ view: SidebarHeaderMarker, context: Context) {
        view.key = key
        view.active = active
    }
    func sizeThatFits(_ proposal: ProposedViewSize, nsView: SidebarHeaderMarker,
                     context: Context) -> CGSize? {
        CGSize(width: proposal.width ?? 0, height: proposal.height ?? 0)
    }
}

@main struct SidebarCheck {
    @MainActor static func main() {
        let app = NSApplication.shared
        app.setActivationPolicy(.regular)
        Task { @MainActor in
            // Configuration travels through the environment — naked argv
            // tokens get parsed as documents/options, not data.
            let language = ProcessInfo.processInfo.environment["PITEX_SIDEBAR_LANG"] ?? "en"
            do { try await run(language); print("CHECK_COMPLETE \(language)"); fflush(nil); exit(0) }
            catch { print("FAIL", error); fflush(nil); exit(1) }
        }
        app.run()
    }

    @MainActor static func run(_ language: String) async throws {
        let geometryOnly = ProcessInfo.processInfo.environment["PITEX_SIDEBAR_GEOMETRY_ONLY"] == "1"
        let root = URL(fileURLWithPath: ProcessInfo.processInfo.environment["PITEX_SIDEBAR_ROOT"] ?? "")
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
            SymbolsPaletteView(onCopy: { _ in }).frame(width: 320, height: 320)
        }.padding(10).background(Color(nsColor: .windowBackgroundColor))
            .tint(Color(nsColor: tint))
            .environment(\.locale, Locale(identifier: language)))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 640, height: 700),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host
        window.makeKeyAndOrderFront(nil)
        // Selected segments only draw their tint in an active, key window;
        // an inactive window renders the gray bezel and would fake a .tint miss.
        NSApp.activate()
        window.makeKey()
        window.appearance = NSAppearance(named: language == "en" ? .darkAqua : .aqua)
        defer {
            window.orderOut(nil)
            UserDefaults.standard.removePersistentDomain(forName: Bundle.main.bundleIdentifier!)
        }
        func settle(_ view: NSView) async throws {
            view.layoutSubtreeIfNeeded()
            try await Task.sleep(for: .milliseconds(300))
            view.layoutSubtreeIfNeeded()
        }
        func settle() async throws { try await settle(host) }
        func descendants(_ view: NSView) -> [NSView] { [view] + view.subviews.flatMap(descendants) }
        func stage(_ message: String) { print("[stage] \(message)"); fflush(nil) }
        func capture(_ view: NSView, _ name: String) throws {
            let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds)!
            view.cacheDisplay(in: view.bounds, to: bitmap)
            try bitmap.representation(using: .png, properties: [:])!
                .write(to: URL(fileURLWithPath: "/tmp/pitex-sidebar-\(name)-\(language).png"))
        }
        stage(geometryOnly ? "native sidebar geometry mode (no OCR)" : "full rendered sidebar mode")
        /// Virtualized runners lack the scaler driver Vision's GPU path
        /// needs (CRImageReaderError) — pin every compute stage that
        /// offers a CPU device so OCR runs on the CPU the VM has.
        /// setComputeDevice is the supported API (macOS 14+); the older
        /// usesCPUOnly flag is deprecated and capped at macOS 14.
        func ocrRequest() throws -> VNRecognizeTextRequest {
            let request = VNRecognizeTextRequest()
            request.recognitionLevel = .accurate
            request.recognitionLanguages = language == "ko" ? ["ko-KR", "en-US"] : ["en-US"]
            var pinned = 0
            for (stage, devices) in try request.supportedComputeStageDevices {
                for device in devices {
                    // MLComputeDevice.cpu is an enum case — assign the
                    // instance the request itself advertises for the stage.
                    if case .cpu = device {
                        request.setComputeDevice(device, for: stage)
                        pinned += 1
                        break
                    }
                }
            }
            precondition(pinned > 0,
                         "Vision offered no CPU compute stage — OCR would hit the VM scaler path again")
            return request
        }
        let navTitles = ["sidebar.workspace", "sidebar.project", "sidebar.todos"].map {
            Bundle.main.localizedString(forKey: $0, value: nil, table: nil)
        }
        func rawTokens(of view: NSView) throws -> [String] {
            let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds)!
            view.cacheDisplay(in: view.bounds, to: bitmap)
            let request = try ocrRequest()
            try VNImageRequestHandler(cgImage: bitmap.cgImage!, options: [:]).perform([request])
            return (request.results ?? []).compactMap { $0.topCandidates(1).first?.string }
        }
        try await settle()
        // The tint check needs the real active bezel — an inactive window
        // renders selected segments gray. The `open` launch should already
        // have made us active; bound the retry so a failure fails fast.
        let activationDeadline = ContinuousClock.now + .seconds(10)
        while !(NSApp.isActive && window.isKeyWindow) && .now < activationDeadline {
            NSApp.activate()
            window.makeKeyAndOrderFront(nil)
            try await Task.sleep(for: .milliseconds(100))
        }
        precondition(NSApp.isActive && window.isKeyWindow,
            "Checker window never activated — tint check would measure the inactive bezel")
        stage("appActive=\(NSApp.isActive) keyWindow=\(window.isKeyWindow)")
        func snapshot() throws -> [(String, CGRect)] { try snapshot(of: host) }
        func snapshot(of view: NSView) throws -> [(String, CGRect)] {
            let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds)!
            view.cacheDisplay(in: view.bounds, to: bitmap)
            if view === host {
                try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: "/tmp/pitex-sidebar-layout-live-\(language).png"))
            }
            let request = try ocrRequest()
            try VNImageRequestHandler(cgImage: bitmap.cgImage!, options: [:]).perform([request])
            return try (request.results ?? []).flatMap { observation -> [(String, CGRect)] in
                guard let candidate = observation.topCandidates(1).first else { return [] }
                var rows = [(candidate.string, observation.boundingBox)]
                for title in navTitles {
                    if let range = candidate.string.range(of: title, options: .caseInsensitive), let box = try candidate.boundingBox(for: range) {
                        rows.append((title, box.boundingBox))
                    }
                }
                return rows
            }.filter { $0.1.minX < 0.45 }
        }
        if !geometryOnly {
            let upper = descendants(host).compactMap { $0 as? NSSegmentedControl }.first { $0.segmentCount == 3 }!
            let split = descendants(host).compactMap { $0 as? NSSplitView }.first { !$0.isVertical && $0.arrangedSubviews.count == 2 }!
            let lower = descendants(host).compactMap { $0 as? NSSegmentedControl }.first { control in
                control.segmentCount == 3 && (0..<3).allSatisfy { index in
                    control.label(forSegment: index) == navTitles[index]
                }
            }!
            precondition(workspace.projectTree.first { $0.path == "main.tex" }?.name == "main.tex")
            precondition(workspace.documentProject.tree.first?.name == "main.tex (.)", "Project must retain duplicate-name context")
            upper.selectedSegment = 1
            precondition(upper.sendAction(upper.action, to: upper.target))
            try await settle()
            precondition(workspace.sidebarSection == .labels)
            stage("upper picker action OK")
            // Synthetic clicks deadlock: sendEvent(.leftMouseDown) enters
            // NSSegmentedControl's tracking loop, which waits for a mouseUp the
            // caller can't post until sendEvent returns. Drive the real binding
            // like `upper` does — selectedSegment + sendAction is the same path a
            // genuine click takes through the SwiftUI Picker.
            func clickTab(_ key: String) async throws {
                let title = Bundle.main.localizedString(forKey: key, value: nil, table: nil)
                guard let index = (0..<lower.segmentCount).first(where: { lower.label(forSegment: $0) == title }) else {
                    fatalError("Missing tab \(title): \(navTitles)")
                }
                lower.selectedSegment = index
                precondition(lower.sendAction(lower.action, to: lower.target), "Tab \(title) action did not fire")
                try await settle()
                stage("tab \(title) selected")
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
            stage("tab switches done; tint check next")
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
            let text = try rawTokens(of: host)
            func compact(_ text: String) -> String { text.replacingOccurrences(of: " ", with: "").lowercased() }
            precondition(text.contains { compact($0).contains(compact(expected[0])) }, "Missing rendered category title: \(text)")
            print("PASS \(language): localized Symbols title and independent Workspace/Project/TODO buttons")
        }
        if language == "en" {
            // Real-window checks: the whole WorkspaceView (outer vertical
            // split + the sidebar pane) in a resizable window — the
            // fixed-frame standalone host can't prove the pane's real
            // initial width.
            UserDefaults.standard.set(false, forKey: "inspectorOnLeft")
            let realHost = NSHostingView(rootView: WorkspaceView(workspace: workspace)
                .environment(\.locale, Locale(identifier: language)))
            let realWindow = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1120, height: 720),
                                      styleMask: [.titled, .resizable], backing: .buffered, defer: false)
            realWindow.contentView = realHost
            realWindow.makeKeyAndOrderFront(nil)
            defer { realWindow.orderOut(nil) }
            try await settle(realHost)
            func outerSplit(in view: NSView) -> NSSplitView? {
                descendants(view).compactMap { $0 as? NSSplitView }.first { $0.isVertical }
            }
            func paneWidth(host: NSView, trailing: Bool) -> CGFloat? {
                (trailing ? outerSplit(in: host)?.arrangedSubviews.last
                          : outerSplit(in: host)?.arrangedSubviews.first)?.frame.width
            }
            // The anchor waits for the full pane set before placing, so
            // poll until the divider lands — then compare the real frame.
            func placedWidth(_ target: CGFloat, host: NSView, trailing: Bool) async throws -> CGFloat? {
                let deadline = ContinuousClock.now + .seconds(5)
                while .now < deadline {
                    if let width = paneWidth(host: host, trailing: trailing), abs(width - target) <= 1 {
                        return width
                    }
                    host.layoutSubtreeIfNeeded()
                    try await Task.sleep(for: .milliseconds(100))
                }
                return paneWidth(host: host, trailing: trailing)
            }
            // Measured before any manual setPosition — the fresh window's
            // pane must land at the 170pt minimum, not share spare space.
            let initial = try await placedWidth(170, host: realHost, trailing: false)
            precondition(initial != nil && abs(initial! - 170) <= 1,
                         "fresh window sidebar must open at the 170pt minimum, got \(initial as Any)")
            stage("measured fresh sidebar before manual resize: \(initial!)pt")
            guard let outer = outerSplit(in: realHost), outer.arrangedSubviews.count >= 2 else {
                preconditionFailure("real window produced no vertical split")
            }
            let leading = { paneWidth(host: realHost, trailing: false)! }
            let sidebarPane = outer.arrangedSubviews[0]
            let paneBitmap = sidebarPane.bitmapImageRepForCachingDisplay(in: sidebarPane.bounds)!
            sidebarPane.cacheDisplay(in: sidebarPane.bounds, to: paneBitmap)
            try paneBitmap.representation(using: .png, properties: [:])!
                .write(to: URL(fileURLWithPath: "/tmp/pitex-sidebar-minimum-\(language).png"))
            // A real divider drag must stick — an ordinary model change
            // (structure tab switch) is not a remount and keeps the width.
            outer.setPosition(230, ofDividerAt: 0)
            try await settle(realHost)
            precondition(abs(leading() - 230) <= 1.5, "divider drag to 230 did not land: \(leading())")
            workspace.sidebarSection = .bibtex
            try await settle(realHost)
            precondition(abs(leading() - 230) <= 1.5, "model change reset the divider to \(leading())")
            // Hide/show remounts the pane; the remembered real width —
            // recorded from the actual frame, not a fixed constant — must
            // come back.
            let remembered = leading()
            workspace.sidebarVisible = false
            try await settle(realHost)
            workspace.sidebarVisible = true
            try await settle(realHost)
            try await settle(realHost)
            precondition(abs(leading() - remembered) <= 1.5,
                         "hide/show must restore \(remembered), got \(leading())")
            // Mirroring moves the pane to the trailing edge — the flip is
            // a remount, so the remembered width carries over.
            UserDefaults.standard.set(true, forKey: "inspectorOnLeft")
            try await settle(realHost)
            try await settle(realHost)
            let trailing = { paneWidth(host: realHost, trailing: true)! }
            precondition(abs(trailing() - remembered) <= 1.5,
                         "mirror flip must carry \(remembered), got \(trailing())")
            try capture(realHost, "resized-mirrored")
            // Fresh mirrored minimum needs a genuinely new view state —
            // a same-type rootView swap can keep @StateObject alive, and
            // one model's editor text view must never live in two hosts.
            // inspectorOnLeft is still set from the flip above.
            let freshWorkspace = WorkspaceModel()
            await freshWorkspace.open(root.appendingPathComponent("empty/plain.tex"))
            guard case .ready = freshWorkspace.phase else {
                preconditionFailure("fresh workspace open failed: \(freshWorkspace.phase)")
            }
            let freshHost = NSHostingView(rootView: WorkspaceView(workspace: freshWorkspace)
                .environment(\.locale, Locale(identifier: language)))
            let freshWindow = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 1120, height: 720),
                                       styleMask: [.titled, .resizable], backing: .buffered, defer: false)
            freshWindow.contentView = freshHost
            freshWindow.makeKeyAndOrderFront(nil)
            defer { freshWindow.orderOut(nil) }
            try await settle(freshHost)
            let freshInitial = try await placedWidth(170, host: freshHost, trailing: true)
            precondition(freshInitial != nil && abs(freshInitial! - 170) <= 1,
                         "fresh mirrored sidebar must start at 170, got \(freshInitial as Any)")
            stage("measured fresh mirrored sidebar before manual resize: \(freshInitial!)pt")
            try capture(freshHost, "minimum-mirrored")
            await freshWorkspace.close()
            freshWindow.orderOut(nil)
            stage("real window: 170 initial, 230 survives tab change, hide/show + mirror restore")
            realWindow.orderOut(nil)

            // Empty TODOs pane: a fixture-independent project with zero
            // TODO comments (root's main.tex has one — any file in that
            // project aggregates it). The header/+ row must stay pinned at
            // the pane top like the Project header.
            let emptyWorkspace = WorkspaceModel()
            await emptyWorkspace.open(root.appendingPathComponent("empty/plain.tex"))
            guard case .ready = emptyWorkspace.phase else {
                preconditionFailure("empty project open failed: \(emptyWorkspace.phase)")
            }
            let outlineDeadline = ContinuousClock.now + .seconds(5)
            while emptyWorkspace.outlineItems.isEmpty && .now < outlineDeadline {
                try await Task.sleep(for: .milliseconds(50))
            }
            // The same refreshStructure pass publishes todoItems — a
            // parsed outline proves the scan ran, so empty is real.
            precondition(!emptyWorkspace.outlineItems.isEmpty, "empty project structure scan never ran")
            precondition(emptyWorkspace.todoItems.isEmpty, "empty project must have no TODOs")
            let emptyHost = NSHostingView(rootView: ProjectSidebarView(workspace: emptyWorkspace)
                .frame(width: 280, height: 650)
                .environment(\.locale, Locale(identifier: language)))
            window.contentView = emptyHost
            window.makeKeyAndOrderFront(nil)
            try await settle(emptyHost)
            try await settle(emptyHost)
            let nav = descendants(emptyHost).compactMap { $0 as? NSSegmentedControl }.first { control in
                control.segmentCount == 3 && (0..<3).allSatisfy { index in
                    control.label(forSegment: index) == navTitles[index]
                }
            }!
            let innerSplit = descendants(emptyHost).compactMap { $0 as? NSSplitView }
                .first { !$0.isVertical && $0.arrangedSubviews.count == 2 }!
            if geometryOnly {
                innerSplit.setPosition(330, ofDividerAt: 0)
                try await settle(emptyHost)
            }
            let initialInnerHeight = innerSplit.arrangedSubviews[0].frame.height
            func clickNav(_ key: String) async throws {
                let title = Bundle.main.localizedString(forKey: key, value: nil, table: nil)
                nav.selectedSegment = (0..<nav.segmentCount).first { nav.label(forSegment: $0) == title }!
                precondition(nav.sendAction(nav.action, to: nav.target), "nav tab \(title) did not fire")
                try await settle(emptyHost)
            }
            // The picker repeats the same titles in the row above the
            // header, and a "No TODOs" body line can OCR as the title too
            // — take the highest matching row BELOW the picker's bottom
            // edge. Vision boxes are normalized, bottom-left origin; the
            // picker frame converts into host space first (hosting views
            // are flipped, so isFlipped decides the mapping direction).
            let hostHeight = emptyHost.bounds.height
            let headerTolerance = 4 / hostHeight  // ±4pt at the host's scale
            func headerTop(_ key: String) throws -> (top: CGFloat, boxes: [CGRect]) {
                let title = Bundle.main.localizedString(forKey: key, value: nil, table: nil)
                let boxes: [CGRect]
                if geometryOnly {
                    let markers = descendants(emptyHost).compactMap { $0 as? SidebarHeaderMarker }
                        .filter { $0.key == key }
                    precondition(markers.count == 1, "expected one real header marker for \(key), got \(markers.count)")
                    let marker = markers[0]
                    precondition(marker.active && marker.window === window && !marker.isHiddenOrHasHiddenAncestor,
                                 "header marker is not on the selected, mounted page: \(key)")
                    let rect = marker.convert(marker.bounds, to: emptyHost)
                    precondition(rect.width > 0 && rect.height > 0 && emptyHost.bounds.contains(rect),
                                 "header Text must have a visible positive frame: \(key) \(rect)")
                    boxes = [CGRect(x: rect.minX / emptyHost.bounds.width,
                                    y: emptyHost.isFlipped ? 1 - rect.maxY / hostHeight : rect.minY / hostHeight,
                                    width: rect.width / emptyHost.bounds.width, height: rect.height / hostHeight)]
                    stage("rendered \(key) header Text frame: \(rect)")
                } else {
                    boxes = try snapshot(of: emptyHost).filter { $0.0 == title }.map(\.1)
                }
                // Per snapshot: the inner divider resize moves the picker
                // row along with its pane, so a stale bottom edge would
                // misclassify the header.
                let navRect = nav.convert(nav.bounds, to: emptyHost)
                let pickerBottom = emptyHost.isFlipped
                    ? 1 - navRect.maxY / hostHeight
                    : navRect.minY / hostHeight
                guard let box = boxes.filter({ $0.maxY <= pickerBottom + 0.005 })
                    .max(by: { $0.maxY < $1.maxY }) else {
                    preconditionFailure("no '\(title)' header below the picker — boxes=\(boxes) pickerBottom=\(pickerBottom)")
                }
                return (box.maxY, boxes)
            }
            try await clickNav("sidebar.project")
            let project = try headerTop("sidebar.project")
            try capture(emptyHost, "empty-project")
            try await clickNav("sidebar.todos")
            let todos = try headerTop("sidebar.todos")
            let emptyBitmap = emptyHost.bitmapImageRepForCachingDisplay(in: emptyHost.bounds)!
            emptyHost.cacheDisplay(in: emptyHost.bounds, to: emptyBitmap)
            try emptyBitmap.representation(using: .png, properties: [:])!
                .write(to: URL(fileURLWithPath: "/tmp/pitex-sidebar-empty-todo-\(language).png"))
            precondition(abs(todos.top - project.top) <= headerTolerance,
                         "empty TODOs header/add row must top-align with the Project header (±4pt): "
                         + "\(todos.top) vs \(project.top) — project=\(project.boxes) todos=\(todos.boxes)")
            innerSplit.setPosition(170, ofDividerAt: 0)
            try await settle(emptyHost)
            let resizedInnerHeight = innerSplit.arrangedSubviews[0].frame.height
            if geometryOnly {
                precondition(abs(resizedInnerHeight - 170) <= 1.5 && abs(resizedInnerHeight - initialInnerHeight) > 4,
                             "inner divider must actually resize: \(initialInnerHeight) → \(resizedInnerHeight)")
            }
            try await clickNav("sidebar.project")
            let projectResized = try headerTop("sidebar.project")
            try capture(emptyHost, "empty-project-resized")
            try await clickNav("sidebar.todos")
            let todosResized = try headerTop("sidebar.todos")
            try capture(emptyHost, "empty-todo-resized")
            precondition(abs(todosResized.top - projectResized.top) <= headerTolerance,
                         "header alignment must survive the divider resize (±4pt): "
                         + "\(todosResized.top) vs \(projectResized.top) — project=\(projectResized.boxes) todos=\(todosResized.boxes)")
            stage("empty TODOs header stays pane-top-aligned: "
                + "delta=\(abs(todos.top - project.top) * hostHeight)pt before resize; "
                + "delta=\(abs(todosResized.top - projectResized.top) * hostHeight)pt after resize")
            await emptyWorkspace.close()
        }
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
    # Independent TODO-free project — the root fixture carries one TODO,
    # so an empty-todos pane needs its own root.
    (root / 'empty').mkdir()
    (root / 'empty/plain.tex').write_text(
        '\\documentclass{article}\n\\begin{document}\n\\section{Intro}\nPlain.\n\\end{document}\n')
    for i in range(40):
        (root / f'zfile_{i:02d}.tex').write_text('Chapter.\n')
    app_main = repo / 'Mac/Sources/AppShell/PitexApp.swift'
    stripped = root / 'PitexApp.swift'
    stripped.write_text(app_main.read_text().replace('@main\nstruct PitexApp', 'struct PitexApp'))
    workspace_view = repo / 'Mac/Sources/Features/WorkspaceView.swift'
    source = root / 'Check.swift'
    # Keep the check in the same file as the private SymbolsPaletteView.
    source.write_text(workspace_view.read_text() + '\n' + check)
    sidebar_view = repo / 'Mac/Sources/Features/ProjectSidebarView.swift'
    sidebar_sources = [sidebar_view]
    if args.geometry_only:
        sidebar_copy = root / 'ProjectSidebarView.swift'
        sidebar_copy.write_text(instrument_headers(sidebar_view.read_text()))
        sidebar_sources = [sidebar_copy]
    executable = bundle / 'MacOS/check'
    subprocess.run(['xcrun', 'swiftc', '-parse-as-library', '-swift-version', '6',
                    '-target', 'arm64-apple-macos15.0', *[arg for path in includes for arg in ['-I', str(path)]], str(source), str(stripped),
                    *[str(p) for p in (repo / 'Mac/Sources').rglob('*.swift') if p not in (app_main, workspace_view, sidebar_view)],
                    *map(str, sidebar_sources),
                    *[str(p) for p in objects], '-o', str(executable)], check=True)
    # Directly exec()ing the Mach-O left the checker unable to activate on CI
    # (appActive=false → inactive gray bezel, unusable tint check), so launch
    # through LaunchServices instead. `open -W` waits for exit but does not
    # forward the child's status — the CHECK_COMPLETE marker, printed only
    # after run() returns, is authoritative.
    env = {**os.environ, 'PI_AGENT_PATH': '/usr/bin/false', 'PI_CODING_AGENT_DIR': str(root / 'pi')}
    app = root / 'SidebarCheck.app'
    for language in (['en'] if args.geometry_only else ['en', 'ko']):
        out_log = root / f'check-{language}.out.log'
        err_log = root / f'check-{language}.err.log'
        try:
            subprocess.run(['/usr/bin/open', '-n', '-W',
                            '--stdout', str(out_log), '--stderr', str(err_log),
                            '--env', 'PI_AGENT_PATH=/usr/bin/false',
                            '--env', f'PI_CODING_AGENT_DIR={root / "pi"}',
                            '--env', f'PITEX_SIDEBAR_ROOT={root}',
                            '--env', f'PITEX_SIDEBAR_LANG={language}',
                            '--env', f'PITEX_SIDEBAR_GEOMETRY_ONLY={int(args.geometry_only)}',
                            str(app), '--args',
                            '-AppleLanguages', f'({language})'],
                           env=env, check=True, timeout=60)
        finally:
            for stream_log in (out_log, err_log):
                if stream_log.exists():
                    print(stream_log.read_text(errors='replace'), end='')
        output = out_log.read_text(errors='replace') if out_log.exists() else ''
        if f'CHECK_COMPLETE {language}' not in output:
            sys.exit(f'FAIL {language}: completion marker missing from checker log')
        if not args.geometry_only:
            shutil.copyfile(root / f'sidebar-{language}.png', Path('/tmp') / f'pitex-sidebar-{language}.png')
