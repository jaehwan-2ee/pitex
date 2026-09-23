#!/usr/bin/env python3
"""After Release build: check-pi-runtime.py <Build/Products/Release>.

Reproduce Finder's minimal PATH with Homebrew, curl, npm-prefix and nvm
layouts. Install the pinned agent in temporary directories using real Bun and
Node/npm, then exercise RPC and auth-script generation without model requests
or user credentials. Requires Bun and Node/npm to validate both routes.
Pass --latest to also check manual updates and rollback after a failed update.
"""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

repo = Path(__file__).resolve().parent.parent
products = Path(sys.argv[1]).resolve()
bun = shutil.which('bun')
node = shutil.which('node')
npm = shutil.which('npm')
assert bun and node and npm, 'Install Bun and Node/npm to exercise both runtime routes'
latest = subprocess.check_output(['npm', 'view', '@earendil-works/pi-coding-agent@latest', 'version'], text=True).strip() if '--latest' in sys.argv else ''
check = r'''
import Foundation
import BuildCore

@main struct Check {
    static func main() async throws {
        let root = URL(fileURLWithPath: CommandLine.arguments[1])
        let mode = CommandLine.arguments[2]
        let realBun = URL(fileURLWithPath: CommandLine.arguments[3])
        let realNode = URL(fileURLWithPath: CommandLine.arguments[4])
        let realNpm = URL(fileURLWithPath: CommandLine.arguments[5])
        let files = FileManager.default
        func link(_ target: URL, _ path: URL) throws {
            try files.createDirectory(at: path.deletingLastPathComponent(), withIntermediateDirectories: true)
            try files.createSymbolicLink(at: path, withDestinationURL: target)
        }
        func script(_ path: URL, _ text: String) throws {
            try files.createDirectory(at: path.deletingLastPathComponent(), withIntermediateDirectories: true)
            try text.write(to: path, atomically: true, encoding: .utf8)
            try files.setAttributes([.posixPermissions: 0o755], ofItemAtPath: path.path)
        }
        let minimal = "/usr/bin:/bin:/usr/sbin:/sbin"
        let base = ["PATH": minimal, "SHELL": "/usr/bin/false", "PI_CODING_AGENT_DIR": PiPaths.agentDirectory.path]
        var selected: PiToolchain?
        if mode == "bun" {
            for layout in ["opt/homebrew/bin", "usr/local/bin", ".bun/bin", ".npm-global/bin"] {
                let home = root.appendingPathComponent(layout.replacingOccurrences(of: "/", with: "-"))
                let binary = home.appendingPathComponent(layout + "/bun")
                try link(realBun, binary)
                let system = layout.hasPrefix(".") ? [] : [binary.deletingLastPathComponent().path]
                let tools = await PiToolchain.discover(environment: base, home: home, systemDirectories: system)
                precondition(tools.bun == binary && tools.node == nil && tools.npm == nil)
                print("PASS Finder discovery: \(layout), no Node/npm"); fflush(nil)
                selected = tools
            }
            let home = root.appendingPathComponent("custom")
            let binary = home.appendingPathComponent("bun prefix/bin/bun")
            try link(realBun, binary)
            let custom = await PiToolchain.discover(environment: base.merging(["BUN_INSTALL": home.appendingPathComponent("bun prefix").path]) { _, b in b }, home: home, systemDirectories: [])
            precondition(custom.bun == binary)
            print("PASS custom BUN_INSTALL directory"); fflush(nil)

            let shellHome = root.appendingPathComponent("shell")
            try files.createDirectory(at: shellHome, withIntermediateDirectories: true)
            let zshrc = shellHome.appendingPathComponent(".zshrc")
            let quotedBin = binary.deletingLastPathComponent().path.replacingOccurrences(of: "'", with: "'\\''")
            try "export PATH='\(quotedBin)':$PATH\necho profile-noise\n".write(to: zshrc, atomically: true, encoding: .utf8)
            let shell = await PiToolchain.discover(environment: base.merging(["SHELL": "/bin/zsh", "ZDOTDIR": shellHome.path]) { _, b in b }, home: shellHome, systemDirectories: [])
            precondition(shell.bun == binary)
            print("PASS interactive .zshrc-only install with startup output"); fflush(nil)

            let prefixHome = root.appendingPathComponent("npm-prefix")
            let bin = prefixHome.appendingPathComponent("tools")
            try link(realNode, bin.appendingPathComponent("node"))
            try link(realNpm, bin.appendingPathComponent("npm"))
            let prefix = prefixHome.appendingPathComponent("global packages")
            try link(realBun, prefix.appendingPathComponent("bin/bun"))
            let config = prefixHome.appendingPathComponent("npmrc")
            try "prefix=\(prefix.path)\n".write(to: config, atomically: true, encoding: .utf8)
            let npmTools = await PiToolchain.discover(environment: base.merging(["PATH": bin.path + ":" + minimal,
                "NPM_CONFIG_USERCONFIG": config.path]) { _, b in b }, home: prefixHome, systemDirectories: [])
            precondition(npmTools.bun == prefix.appendingPathComponent("bin/bun"))
            print("PASS npm custom global prefix absent from PATH"); fflush(nil)

            let hung = root.appendingPathComponent("hung-shell")
            try script(hung, "#!/bin/sh\nexec /bin/sleep 30\n")
            let start = Date()
            let bounded = await PiToolchain.discover(environment: base.merging(["SHELL": hung.path,
                "BUN_INSTALL": home.appendingPathComponent("bun prefix").path]) { _, b in b }, home: home, systemDirectories: [])
            precondition(bounded.bun == binary && Date().timeIntervalSince(start) < 9)
            print("PASS shell startup timeout still discovers installed Bun"); fflush(nil)
        } else {
            let home = root.appendingPathComponent("nvm")
            let bin = home.appendingPathComponent(".nvm/versions/node/v25.0.0/bin")
            try link(realNode, bin.appendingPathComponent("node"))
            try link(realNpm, bin.appendingPathComponent("npm"))
            let config = home.appendingPathComponent("npmrc")
            try "prefix=\(home.appendingPathComponent("empty-prefix").path)\n".write(to: config, atomically: true, encoding: .utf8)
            selected = await PiToolchain.discover(environment: base.merging(["NPM_CONFIG_USERCONFIG": config.path]) { _, b in b }, home: home, systemDirectories: [])
            precondition(selected!.node == bin.appendingPathComponent("node") && selected!.npm == bin.appendingPathComponent("npm") && selected!.bun == nil)
            print("PASS nvm Node/npm discovery without shell initialization"); fflush(nil)
        }
        let tools = selected!
        try files.createDirectory(at: PiPaths.agentDirectory, withIntermediateDirectories: true)
        let credentials = Data("{\"fake-only\":{\"type\":\"api_key\",\"key\":\"local-test-only\"}}".utf8)
        try credentials.write(to: PiPaths.authFileURL)
        let legacyPackage = PiPaths.runtimeDirectory.appendingPathComponent("lib/node_modules/@mariozechner/pi-coding-agent")
        let legacy = legacyPackage.appendingPathComponent("dist/cli.js")
        try script(legacy, "#!/usr/bin/env node\nconsole.log('legacy');\n")
        try "{\"name\":\"@mariozechner/pi-coding-agent\",\"version\":\"0.73.0\"}".write(
            to: legacyPackage.appendingPathComponent("package.json"), atomically: true, encoding: .utf8)
        try link(legacy, PiPaths.runtimeExecutable)
        precondition(PiRuntimeInstaller.installedVersion() == "0.73.0", "Version must follow the installed launcher")
        try await PiRuntimeInstaller.install(toolchain: tools)
        let preservedCredentials = try Data(contentsOf: PiPaths.authFileURL)
        precondition(preservedCredentials == credentials)
        let preservedCLI = try String(contentsOf: legacy, encoding: .utf8)
        precondition(preservedCLI == "#!/usr/bin/env node\nconsole.log('legacy');\n")
        precondition(PiRuntimeInstaller.installedVersion() == PiRuntimeInstaller.desiredVersion)
        precondition(PiExecutableLocator.resolve(environment: tools.environment) == PiPaths.runtimeExecutable)
        print("PASS \(mode) real install and version check; credentials and old CLI preserved"); fflush(nil)
        let latest = CommandLine.arguments[6]
        if !latest.isEmpty {
            let settings = Data("{\"test-setting\":true}".utf8)
            let models = Data("{\"providers\":{}}".utf8)
            try settings.write(to: PiPaths.settingsFileURL)
            try models.write(to: PiPaths.modelsFileURL)
            try await PiRuntimeInstaller.install(latest: true, toolchain: tools)
            precondition(PiRuntimeInstaller.installedVersion() == latest)
            let bad = PiToolchain(environment: tools.environment, bun: URL(fileURLWithPath: "/usr/bin/false"))
            do {
                try await PiRuntimeInstaller.install(latest: true, toolchain: bad)
                preconditionFailure("A failed package manager must not report a successful update")
            } catch {
                precondition(PiRuntimeInstaller.installedVersion() == latest, "Failed update must restore the working runtime")
            }
            let actualCredentials = try Data(contentsOf: PiPaths.authFileURL)
            let actualSettings = try Data(contentsOf: PiPaths.settingsFileURL)
            let actualModels = try Data(contentsOf: PiPaths.modelsFileURL)
            precondition(actualCredentials == credentials && actualSettings == settings && actualModels == models)
            let remaining = try files.contentsOfDirectory(atPath: root.path)
            precondition(!remaining.contains { $0.hasPrefix("pi-runtime-backup-") })
            print("PASS \(mode) latest update to \(latest), failure rollback and auth/settings preservation"); fflush(nil)
        }
        let launch = try tools.launch(PiPaths.runtimeExecutable, arguments: ["--mode", "rpc", "--no-session"])
        precondition(launch.executable == (mode == "bun" ? tools.bun! : tools.node!))
        let process = PiAgentProcess(executableURL: launch.executable, workingDirectory: root,
                                     arguments: launch.arguments, environment: tools.environment)
        try process.start(onExit: {})
        defer { process.terminate() }
        try process.send(.getState, id: "runtime-check")
        var receivedState = false
        for await event in process.events {
            if event.string("id") == "runtime-check" {
                precondition(event.object["success"] as? Bool == true, process.stderrText)
                receivedState = true
                print("PASS \(mode) live RPC get_state, no model request"); fflush(nil)
                break
            }
        }
        precondition(receivedState, "RPC exited without a state response: \(process.stderrText)")
        for logout in [false, true] {
            let script = try await PiRuntimeInstaller.authenticationScript(logout: logout, toolchain: tools)
            let result = try await ProcessRunner().run(DirectCommandPlan(executable: "/bin/zsh", arguments: ["-n", script.path]),
                                                      projectRoot: root, timeout: .seconds(5))
            precondition(result.termination == .exited(code: 0))
            let text = try String(contentsOf: script, encoding: .utf8)
            precondition(text.contains(launch.executable.path.replacingOccurrences(of: "'", with: "'\\''")) && text.contains("export PATH="))
        }
        print("PASS \(mode) login/logout launch scripts use the discovered runtime and PATH"); fflush(nil)
    }
}
'''
with tempfile.TemporaryDirectory(prefix="pitex runtime's ", dir='/tmp') as directory:
    root = Path(directory)
    (root/'PitexAgent/skills').mkdir(parents=True)
    source = root/'Check.swift'; source.write_text(check)
    executable = root/'check'
    subprocess.run(['xcrun','swiftc','-parse-as-library','-swift-version','6','-I',str(products),str(source),
                    *[str(repo/'Mac/Sources/Features'/name) for name in ['PiAgentProcess.swift','PiAuthStore.swift','PiRPC.swift']],
                    *[str(products/(name+'.o')) for name in ['BuildCore','TexDomain']],'-o',str(executable)],check=True)
    for mode in ['bun','node']:
        work=root/mode; work.mkdir()
        env={**os.environ,'PATH':'/usr/bin:/bin:/usr/sbin:/sbin','PI_CODING_AGENT_DIR':str(work/'pi')}
        subprocess.run([str(executable),str(work),mode,bun,node,npm,latest],env=env,check=True,timeout=480)
