#!/usr/bin/env python3
"""macOS regression check: python3 Tools/check-pi-settings.py [path/to/pi]

Uses an isolated agent home and fake credentials; never reads or logs real auth.
"""
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import sys
import tempfile
import termios
import time

repo = Path(__file__).resolve().parent.parent
runtime = Path(sys.argv[1]) if len(sys.argv) > 1 else (
    Path.home() / "Library/Application Support/Pitex/pi-runtime/bin/pi"
)
assert runtime.is_file(), "Install the Pitex Agent runtime before running this check"

with tempfile.TemporaryDirectory(prefix="pitex-settings-", dir="/tmp") as directory:
    root = Path(directory) / "space $dollar's folder"
    agent = root / "pi"
    agent.mkdir(parents=True)
    executable = root / "pi-runtime/bin/pi"
    executable.parent.mkdir(parents=True)
    executable.symlink_to(runtime)
    env = {**os.environ, "PI_CODING_AGENT_DIR": str(agent), "TERM": "xterm-256color"}
    source = Path(directory) / "Check.swift"
    source.write_text(r'''
import Foundation

@main struct Check {
    static func main() async throws {
        let models = try PiPaths.configurationFile(customProvider: true)
        precondition(models == PiPaths.modelsFileURL)
        let initial = try Data(contentsOf: models)
        let object = try JSONSerialization.jsonObject(with: initial) as! [String: Any]
        precondition((object["providers"] as? [String: Any])?.isEmpty == true)
        let existing = "{\n  \"providers\": {}\n}\n\n"
        try existing.write(to: models, atomically: true, encoding: .utf8)
        _ = try PiPaths.configurationFile(customProvider: true)
        let preserved = try String(contentsOf: models, encoding: .utf8)
        precondition(preserved == existing, "Opening must preserve existing custom configuration")
        let settings = try PiPaths.configurationFile()
        precondition(settings == PiPaths.settingsFileURL && settings != models)
        for logout in [false, true] {
            let script = try await PiRuntimeInstaller.authenticationScript(logout: logout)
            let process = Process()
            process.executableURL = URL(fileURLWithPath: "/bin/zsh")
            process.arguments = ["-n", script.path]
            try process.run()
            process.waitUntilExit()
            precondition(process.terminationStatus == 0)
            let attributes = try FileManager.default.attributesOfItem(atPath: script.path)
            precondition((attributes[.posixPermissions] as? NSNumber)?.intValue == 0o700)
        }
        print("PASS models.json creation/preservation; settings.json unchanged; auth script syntax/permissions")
    }
}
''')
    features = repo / "Mac/Sources/Features"
    check = Path(directory) / "check"
    products = repo / "DerivedData/Build/Products/Release"
    subprocess.run(["xcrun", "swiftc", "-parse-as-library", "-swift-version", "6", "-I", str(products),
                    "-module-cache-path", str(Path(directory) / "cache"), str(source),
                    *[str(features / name) for name in ["PiAuthStore.swift", "PiAgentProcess.swift", "PiRPC.swift"]],
                    *[str(products / (name + ".o")) for name in ["BuildCore", "TexDomain"]], "-o", str(check)], check=True)
    subprocess.run([str(check)], env=env, check=True)

    auth = agent / "auth.json"
    other = {"type": "api_key", "key": "pitex-test-only-other"}
    auth.write_text(json.dumps({"openai": {"type": "api_key", "key": "pitex-test-only"}, "anthropic": other}))
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 40, 120, 0, 0))
    process = subprocess.Popen(["/bin/zsh", str(agent / "Pitex Agent Logout.command")],
                               stdin=slave, stdout=slave, stderr=slave, cwd=root,
                               env=env, start_new_session=True)
    os.close(slave)
    output = b""
    selected = False
    try:
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if select.select([master], [], [], 0.1)[0]:
                output += os.read(master, 65536)
            if not selected and b"Select provider to logout:" in output:
                os.write(master, b"openai\r")
                selected = True
            credentials = json.loads(auth.read_text())
            if selected and "openai" not in credentials:
                assert credentials.get("anthropic") == other, "Unselected provider must remain signed in"
                print("PASS generated /logout flow: selected fake provider removed, other provider preserved")
                break
        else:
            raise AssertionError("The generated logout flow did not remove the selected test credential")
    finally:
        os.write(master, b"\x03\x03")
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGTERM)
            process.wait(timeout=3)
        os.close(master)
