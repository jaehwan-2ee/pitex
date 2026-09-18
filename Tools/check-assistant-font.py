#!/usr/bin/env python3
"""After a Release build: check-assistant-font.py <Build/Products/Release>.
Checks persisted font preference and real SwiftUI sizing for every message role.
No assistant requests or user account access.
"""
from pathlib import Path
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
features = repo / 'Mac/Sources/Features'
check = r'''
@main struct Check {
    @MainActor static func main() throws {
        _ = NSApplication.shared
        let defaults = UserDefaults.standard
        let previous = defaults.object(forKey: "ai.fontSize")
        defer {
            if let previous { defaults.set(previous, forKey: "ai.fontSize") }
            else { defaults.removeObject(forKey: "ai.fontSize") }
        }
        defaults.removeObject(forKey: "ai.fontSize")
        let settings = SettingsStore()
        precondition(settings.aiFontSize == 13)
        settings.aiFontSize = 21
        precondition(SettingsStore().aiFontSize == 21, "Font size must survive a new settings instance")
        print("PASS assistant font preference defaults to 13 pt and persists")
        for role in [AgentTranscriptEntry.Role.user, .assistant, .thinking, .tool, .notice] {
            let entry = AgentTranscriptEntry(role: role, title: "Message title", text: "Conversation text with a readable font.", detail: "Tool result")
            func height(_ size: Double) -> CGFloat {
                let host = NSHostingView(rootView: AgentEntryRow(entry: entry, fontSize: size).frame(width: 300))
                return host.fittingSize.height
            }
            let small = height(10), large = height(24)
            precondition(large > small, "Font change must resize every transcript role")
            print("PASS \(role): rendered height \(small) → \(large)")
        }
    }
}
'''
with tempfile.TemporaryDirectory(prefix='pitex-assistant-font-', dir='/tmp') as directory:
    root = Path(directory)
    settings = (features / 'SettingsView.swift').read_text().split('/// Command presets')[0]
    coordinator = (features / 'AgentCoordinator.swift').read_text()
    entry = coordinator[coordinator.index('struct AgentTranscriptEntry:'):coordinator.index('/// Drives the assistant panel')]
    agent_panel = (features / 'AgentPanel.swift').read_text()
    row = agent_panel[agent_panel.index('private struct AgentEntryRow:'):]
    source = root / 'Check.swift'
    source.write_text(settings + entry + row + check)
    executable = root / 'check'
    subprocess.run(['xcrun', 'swiftc', '-parse-as-library', '-swift-version', '6', '-target', 'arm64-apple-macos15.0',
                    '-I', str(products), str(source), *[str(p) for p in products.glob('*.o')], '-o', str(executable)], check=True)
    subprocess.run([str(executable)], check=True, timeout=30)
