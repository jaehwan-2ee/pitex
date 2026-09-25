#!/usr/bin/env python3
"""Render the real macOS project-tree view and exercise file/folder clicks.

Uses Command Line Tools, AppKit and Vision; no Xcode build or user project.
"""
from pathlib import Path
import subprocess
import tempfile

repo = Path(__file__).resolve().parent.parent
model = (repo / 'Packages/TexApp/Sources/ProjectFeature/ProjectFeature.swift').read_text()
model = 'public struct ProjectFileNode:' + model.split('public struct ProjectFileNode:', 1)[1]
view = (repo / 'Mac/Sources/Features/ProjectSidebarView.swift').read_text()
view = 'private struct ProjectTreeRows' + view.split('private struct ProjectTreeRows', 1)[1]
check = r'''
@main struct ProjectTreeCheck {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        let tree = buildProjectFileTree(relativePaths: [
            "manuscript.tex", "manuscript.bib", "ch1.tex", "ch2.tex", "notes.md",
            "figures/a.pdf", "figures/b.pdf", "other_tex/ch3.tex", "other_tex/ch4.tex",
        ])
        var selected = ""
        let host = NSHostingView(rootView: ProjectTreeRows(nodes: tree) { node in
            Button { selected = node.path } label: {
                HStack(spacing: 7) {
                    Image(systemName: node.name.hasSuffix(".bib") ? "book" : "doc.text")
                    Text(verbatim: node.name)
                    Spacer()
                }
                .padding(.horizontal, 8)
                .padding(.vertical, 3)
                .frame(maxWidth: .infinity, alignment: .leading)
                .contentShape(Rectangle())
            }.buttonStyle(.plain)
        }.padding(8).frame(width: 340, height: 440, alignment: .topLeading)
            .background(Color(nsColor: .windowBackgroundColor)))
        let window = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 340, height: 440),
                              styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host
        window.makeKeyAndOrderFront(nil)
        defer { window.orderOut(nil) }
        let names = ["manuscript.tex", "manuscript.bib", "ch1.tex", "ch2.tex",
                     "notes.md", "figures", "a.pdf", "b.pdf", "other_tex", "ch3.tex", "ch4.tex"]
        func snapshot(_ name: String) async throws -> [String: CGRect] {
            try await Task.sleep(for: .milliseconds(300))
            host.layoutSubtreeIfNeeded()
            let bitmap = host.bitmapImageRepForCachingDisplay(in: host.bounds)!
            host.cacheDisplay(in: host.bounds, to: bitmap)
            try bitmap.representation(using: .png, properties: [:])!.write(
                to: URL(fileURLWithPath: "/tmp/pitex-project-tree-\(name).png"))
            let request = VNRecognizeTextRequest()
            request.recognitionLevel = .accurate
            request.recognitionLanguages = ["en-US"]
            try VNImageRequestHandler(cgImage: bitmap.cgImage!, options: [:]).perform([request])
            var rows: [String: CGRect] = [:]
            for observation in request.results ?? [] {
                guard let candidate = observation.topCandidates(1).first else { continue }
                for name in names {
                    if let range = candidate.string.range(of: name),
                       let box = try candidate.boundingBox(for: range) {
                        rows[name] = box.boundingBox
                    }
                }
            }
            return rows
        }
        func click(_ rect: CGRect, disclosure: Bool = false) {
            let local = NSPoint(x: disclosure ? 12 : host.bounds.width * rect.midX,
                y: host.bounds.height * (host.isFlipped ? 1 - rect.midY : rect.midY))
            let point = host.convert(local, to: nil)
            for type in [NSEvent.EventType.leftMouseDown, .leftMouseUp] {
                window.sendEvent(NSEvent.mouseEvent(with: type, location: point, modifierFlags: [],
                    timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                    context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!)
            }
        }
        let sourceNames = ["ch1.tex", "ch2.tex", "manuscript.bib", "manuscript.tex", "notes.md"]
        func checkSources(_ rows: [String: CGRect]) {
            precondition(sourceNames.allSatisfy { rows[$0] != nil }, "Hidden source rows: \(rows)")
            for (parent, child) in zip(sourceNames, sourceNames.dropFirst()) {
                precondition(rows[parent]!.midY > rows[child]!.midY, "Source order changed")
            }
            for child in sourceNames.dropFirst() {
                precondition(abs(rows[child]!.minX - rows["ch1.tex"]!.minX) * 340 < 6,
                             "Sibling files must stay at the same depth: \(rows)")
            }
        }
        let closed = try await snapshot("closed")
        checkSources(closed)
        precondition(closed["a.pdf"] == nil && closed["ch3.tex"] == nil)
        for file in ["manuscript.tex", "manuscript.bib", "notes.md"] {
            click(closed[file]!)
            precondition(selected == file, "File row did not activate")
            let after = try await snapshot(file)
            checkSources(after)
            precondition(after == closed, "Opening a file changed the tree layout")
        }
        click(closed["figures"]!, disclosure: true)
        let figures = try await snapshot("figures")
        checkSources(figures)
        precondition(figures["a.pdf"] != nil && figures["b.pdf"] != nil && figures["ch3.tex"] == nil)
        precondition((figures["a.pdf"]!.minX - figures["figures"]!.minX) * 340 >= 16,
                     "Folder files must be indented: \(figures)")
        precondition((figures["a.pdf"]!.minX - figures["ch1.tex"]!.minX) * 340 >= 20,
                     "PDF must stay inside its folder")
        click(figures["other_tex"]!, disclosure: true)
        let expanded = try await snapshot("expanded")
        checkSources(expanded)
        precondition(expanded["ch3.tex"] != nil && expanded["ch4.tex"] != nil)
        click(expanded["figures"]!, disclosure: true)
        let collapsed = try await snapshot("collapsed")
        checkSources(collapsed)
        precondition(collapsed["a.pdf"] == nil && collapsed["b.pdf"] == nil)
        precondition(collapsed["ch3.tex"] != nil, "Collapsing one folder changed another")
        click(collapsed["figures"]!)
        let byName = try await snapshot("byName")
        precondition(byName["a.pdf"] != nil, "Clicking a folder name must expand it")
        print("PASS: stable folder hierarchy across TeX/Bib/Markdown activation; PDF stays in its folder; independent folder toggles and indentation")
    }
}
'''
with tempfile.TemporaryDirectory(prefix='pitex-project-tree-', dir='/tmp') as directory:
    source = Path(directory) / 'Check.swift'
    source.write_text('import AppKit\nimport SwiftUI\nimport Vision\n' + model + '\n' + view + check)
    executable = Path(directory) / 'check'
    subprocess.run(['xcrun', 'swiftc', '-parse-as-library', '-swift-version', '6',
                    str(source), '-o', str(executable)], check=True)
    subprocess.run([str(executable)], check=True, timeout=90)
