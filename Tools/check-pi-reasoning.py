#!/usr/bin/env python3
"""After building Pitex: python3 Tools/check-pi-reasoning.py <Build/Products/Release>

Exercises the real coordinator and pi RPC using local model metadata only.
No prompts, network model requests, or user credentials are used.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
runtime = Path.home() / "Library/Application Support/Pitex/pi-runtime/bin/pi"
assert runtime.is_file(), "Install the Pitex Agent runtime before running this check"

check = r'''
import AppKit
import AppPorts
import AppShell
import Foundation

private actor TestDocument: DocumentSessionPort {
    func snapshot() async -> DocumentSnapshot { .init(revision: 0, text: "") }
    func submit(_ mutation: DocumentMutation) async throws -> DocumentMutationResult {
        .rejected(current: await snapshot())
    }
}

@main struct Check {
    @MainActor static func main() async throws {
        _ = NSApplication.shared
        let environment = try await AppShell.make(documentSession: TestDocument())
        let coordinator = AgentCoordinator(environment: environment)
        coordinator.contextProvider = {
            AgentContextSnapshot(projectRoot: URL(fileURLWithPath: CommandLine.arguments[1]))
        }
        coordinator.preferredModelID = "maximum"
        coordinator.prepare()
        defer { coordinator.shutdown() }

        func settled(model: String, level: String? = nil) async throws {
            for _ in 0..<1500 {
                if coordinator.currentModel?.id == model,
                   !coordinator.isUpdatingModelSettings,
                   !coordinator.thinkingLevels.isEmpty,
                   level == nil || coordinator.thinkingLevel == level { return }
                try await Task.sleep(for: .milliseconds(20))
            }
            fatalError("Settings did not settle: \(model), \(coordinator.statusMessage ?? "no error")")
        }

        try await settled(model: "maximum")
        precondition(coordinator.thinkingLevels == ["off", "low", "medium", "high", "max"])
        coordinator.selectThinkingLevel("max")
        try await settled(model: "maximum", level: "max")
        print("PASS runtime-discovered max; unsupported minimal/xhigh excluded")

        let limited = coordinator.models.first { $0.id == "limited" }!
        coordinator.selectModel(limited)
        precondition(coordinator.isUpdatingModelSettings && coordinator.thinkingLevels.isEmpty)
        try await settled(model: "limited", level: "high")
        precondition(coordinator.thinkingLevels == ["low", "high"])
        coordinator.selectThinkingLevel("minimal")
        precondition(!coordinator.isUpdatingModelSettings && coordinator.thinkingLevel == "high")
        print("PASS model switch clamps max to high; invalid level is not sent")

        coordinator.selectModel(coordinator.models.first { $0.id == "ordinary" }!)
        try await settled(model: "ordinary", level: "off")
        precondition(coordinator.thinkingLevels == ["off"])
        print("PASS non-reasoning model only exposes off")

        coordinator.selectModel(coordinator.models.first { $0.id == "extended" }!)
        try await settled(model: "extended")
        precondition(coordinator.thinkingLevels == ["off", "minimal", "low", "medium", "high", "xhigh", "max"])
        coordinator.selectThinkingLevel("xhigh")
        try await settled(model: "extended", level: "xhigh")
        print("PASS xhigh and max are distinct runtime-supported options")

        coordinator.selectModel(PiModelDescriptor(["id": "missing", "provider": "pitex-test"])!)
        try await settled(model: "extended", level: "xhigh")
        precondition(coordinator.statusMessage?.contains("Model not found") == true)
        precondition(coordinator.thinkingLevels.contains("max"))
        print("PASS rejected model change restores actual model capabilities")
    }
}
'''

with tempfile.TemporaryDirectory(prefix="pitex-reasoning-", dir="/tmp") as directory:
    root = Path(directory)
    agent = root / "pi"
    agent.mkdir()
    (agent / "models.json").write_text(json.dumps({"providers": {"pitex-test": {
        "api": "openai-completions", "baseUrl": "http://127.0.0.1:1/v1", "apiKey": "local-test-only",
        "models": [
            {"id": "ordinary", "reasoning": False},
            {"id": "limited", "reasoning": True, "thinkingLevelMap": {
                "off": None, "minimal": None, "medium": None, "xhigh": None, "max": None}},
            {"id": "maximum", "reasoning": True, "thinkingLevelMap": {
                "minimal": None, "xhigh": None, "max": "max"}},
            {"id": "extended", "reasoning": True, "thinkingLevelMap": {"xhigh": "xhigh", "max": "max"}},
        ],
    }}}))
    (agent / "settings.json").write_text(json.dumps({"defaultProvider": "pitex-test", "defaultModel": "ordinary"}))
    source = root / "Check.swift"
    source.write_text(check)
    executable = root / "check"
    features = repo / "Mac/Sources/Features"
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6",
                    "-target", "arm64-apple-macos15.0", "-module-cache-path", str(root / "cache"),
                    "-I", str(products), str(source),
                    *[str(features / name) for name in ["PiRPC.swift", "PiAuthStore.swift", "PiAgentProcess.swift", "AgentCoordinator.swift"]],
                    *[str(obj) for obj in sorted(products.glob("*.o"))], "-o", str(executable)], check=True)
    env = {**os.environ, "PI_CODING_AGENT_DIR": str(agent), "PI_AGENT_PATH": str(runtime)}
    subprocess.run([str(executable), str(root)], cwd=root, env=env, check=True, timeout=90)
