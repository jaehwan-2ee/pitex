# Pitex code audit, 13 November 2026

The goal is a ranked list of real crash and data-loss bugs in Pitex v1.1.1 (`a6bbed8766e7ae7c3a3e3f2b97272eabe4f13320`), plus a patch for any high-confidence crash or data-loss bug that is small enough to test here. The recent split-view crash is the pattern to hunt. AppKit called `responds(to:)` on a split view that was its own delegate, the call asked the delegate again, and menu validation from Spotlight overflowed the stack. A separate agent is changing redundant writes in `pitex-preview` `publish`. This report does not cover that change.

This pull request does not change runtime code. No crash fix was both small and tested before the cutoff. The findings below are proposals.

The tree under review is main at v1.1.1. Swift is not installed in this environment, so Mac findings were not executed in the app. The preview helper was not rebuilt. The reproduction below uses the same standard-library call the helper makes.

## Coverage

| Area | Status |
| --- | --- |
| Mac app, self-delegate and menu-validation class | Covered for the sites named below |
| Mac app, `PiAgentProcess` stdout buffering | Covered |
| Mac settings custom-shell binding | Covered |
| Preview helper argument startup, Linux spawn path | Covered |
| Linux remote save, placeholder scan, build-log carry, agent stdout cap, second build | Covered |
| Adapter line parsers named below | Covered |
| `Tools/install-deb.sh`, one input-audit step, one release skill step | Covered |
| Windows app | Not yet audited |
| TexCore persistence, SSH, and sync | Not yet audited |
| TexApp beyond the editor delegate assignment and `ShellAuthority` | Not yet audited |
| Linux GTK UI and the rest of `pitex-shell` | Not yet audited |
| Preview `publish`, unsafe blocks, and image decoders | Not yet audited |
| Rest of CI and release scripts | Not yet audited |
| Retain cycles, main-thread UI calls, and force unwraps outside the sites below | Not yet audited |

Seven area reviewers were still running at 12:28 UTC and had not returned reports. Their notes are not used here. Anything they might still find is not yet audited.

## Findings

Severity order is crash, then data loss, then wrong behavior, then minor. Nothing below is a confirmed data-loss bug.

### 1. Preview helper panics when an argument is not Unicode

- Severity: crash of `pitex-preview` (and the same startup code in the XeTeX `main`)
- Confidence: high
- Reachable: yes, when a spawned path is not Unicode
- Fixed in this change: no

`pitex-preview` starts by iterating `std::env::args()`. That iterator panics if any argument is not Unicode. Rust records the panic as an unwrap of the invalid argument. The Linux driver entry is `PreviewEngine/src/linux/driver/main.rs` lines 7152 to 7158.

```rust
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
```

The same call is in `PreviewEngine/src/macos/driver/main.rs` at line 4035, `PreviewEngine/src/linux/xetex/main/main.rs` at line 1981, and `PreviewEngine/src/macos/xetex/main/main.rs` at line 1918.

The Linux app passes the project directory through as a path, not as a Unicode string. The spawn is `Linux/crates/pitex-shell/src/embedded_preview.rs` lines 677 to 684.

```rust
let mut command = Command::new(&helper);
command
    .arg("--root")
    .arg(ctx.root)
    .arg("--main")
    .arg(ctx.main_relative)
    .arg("--out")
    .arg(&dir)
```

`UpdateContext.root` is `&Path` (`embedded_preview.rs` line 553). On Linux a directory name may contain bytes that are not UTF-8. `Command::arg` keeps those bytes. The helper then panics inside `std::env::args()` before it parses flags.

Trigger. Open a project whose root path contains a non-UTF-8 byte, with embedded preview enabled. The helper process exits at startup. The editor process is not the one that panics.

Evidence. A throwaway program spawned itself with argument bytes `FF FE` and called `std::env::args()`, which is the call in `main`. The child exited 101. Its stderr was:

```
thread 'main' panicked at /rustc/0ef1bc386059596dada8fb65b50425aab2ea637a/library/std/src/env.rs:877:51:
called `Result::unwrap()` on an `Err` value: "\xFF\xFE"
```

The log is `/opt/cursor/artifacts/args-panic.log`. The helper binary itself was not built, so this is the standard-library failure on the call the helper makes, not a run of `pitex-preview`.

Proposed fix. In each of those four `main` functions, read `std::env::args_os()` and turn the bytes into a C string without going through `String`. On Unix, `OsStrExt::as_bytes()` plus a trailing NUL matches what `CString::into_bytes_with_nul` already stores. Add a test that spawns the helper, or a small extracted argument converter, with a non-UTF-8 `--root` and expects the process to keep running past argument setup. Keep the original path bytes. A lossy conversion would send the helper to a different directory.

Refusing the spawn in the parent would stop this one call path. The helper would still panic for any other caller.

### 2. Mac agent stdout grows without a cap

- Severity: crash, by using up memory, if one stdout line is very large
- Confidence: high that the buffer is uncapped. The out-of-memory crash was not run.
- Reachable: yes, if the agent writes a long line with no newline, or one newline only at the end of a huge line
- Fixed in this change: no

`PiAgentProcess` appends every stdout chunk and removes bytes only when it sees a newline. The buffer is `Mac/Sources/Features/PiAgentProcess.swift` lines 512 to 522.

```swift
private func consumeStdout(_ chunk: Data) {
    ioQueue.async { [self] in
        stdoutBuffer.append(chunk)
        drainStdout()
    }
}

private func drainStdout() {
    while let newline = stdoutBuffer.firstIndex(of: 0x0a) {
        var line = stdoutBuffer.subdata(in: stdoutBuffer.startIndex..<newline)
        stdoutBuffer.removeSubrange(stdoutBuffer.startIndex...newline)
```

Stderr on the same type drops the front of the buffer once it passes 65,536 bytes (`PiAgentProcess.swift` lines 532 to 537). The Linux agent stops a single stdout line at 4 MiB (`Linux/crates/pitex-shell/src/agent.rs` lines 937 to 938) and caps stderr at 65,536 bytes (lines 973 to 976). The Mac stdout path has neither cap.

Trigger. An agent CLI writes a multi-megabyte JSON line, or a stream with no newline. `stdoutBuffer` keeps the bytes until a newline arrives. A large enough line can jetsam the app.

Proposed fix. If `stdoutBuffer` grows past 4 MiB with no newline, drop that partial line and wait for the next newline, matching the Linux reader. Add a Mac test that feeds a pipe more than the cap with no newline and asserts the buffer stays bounded. This environment has no Swift toolchain, so the test was not added.

### 3. A settings binding uses `try!` on a throwing preferences value

- Severity: crash of the Mac app
- Confidence: medium
- Reachable: only if the custom-shell field contains a NUL. That path was not executed.
- Fixed in this change: no

`BuildPreferences` rejects a custom shell command that contains a NUL.

The check is `Packages/TexApp/Sources/SettingsFeature/SettingsFeature.swift` lines 72 to 74.

```swift
if case let .custom(command) = shellExecution {
    guard !command.isEmpty else { throw SettingsValidationError.emptyCustomShell }
    guard !command.contains("\0") else { throw SettingsValidationError.invalidCustomShell }
}
```

The settings field builds that value with `try!`. The setter is `Mac/Sources/Features/SettingsView.swift` lines 1432 to 1441.

```swift
set: { newValue in
    store.updateBuild {
        let execution: ShellExecutionPreference = newValue.isEmpty ? .disabled : .custom(command: newValue)
        return try! BuildPreferences(
            engine: $0.build.engine,
            maximumPasses: $0.build.maximumPasses,
            stopsAfterFirstError: $0.build.stopsAfterFirstError,
            shellExecution: execution,
            customShellAcknowledged: newValue.isEmpty ? false : $0.build.customShellAcknowledged
        )
```

An empty string becomes `.disabled`, so `emptyCustomShell` is not the throw. `UInt8(clamping:)` on the numeric bindings keeps font size, tab width, and pass count inside the allowed range. The remaining throw from this setter is `invalidCustomShell`.

Trigger. The custom-shell field is set to a string that contains U+0000. The `try!` aborts the process.

Needs Mac verification. Confirm whether the SwiftUI text field can deliver a NUL. If it can, replace `try!` with a failed update that keeps the previous `BuildPreferences`.

## Checks that did not show a bug

These are the places that looked like the split-view crash, or like an obvious panic, and did not survive a walk of the call.

The split view is no longer its own delegate. `LayoutDelegate` is a separate object (`Mac/Sources/Features/WorkspaceView.swift` lines 690 to 749). That fix was not re-opened.

The other `delegate = self` assignments are not the view assigning itself:

- `FoldEngine` sets the layout manager delegate to the fold engine (`EditorFolding.swift` line 61). The fold engine is not the layout manager.
- `EquationPreviewPopover` sets `popover.delegate` (`EquationPreview.swift` line 400). The popover object and the delegate object are different.
- `EditorMacAdapter` sets `nativeView.delegate` (`EditorMacAdapter.swift` line 258). `nativeView` is a `SessionTextView`. The adapter is not that text view.

`SessionTextView.validateUserInterfaceItem` calls `super` after handling find actions (`EditorMacAdapter.swift` lines 33 to 38). That is not the split view calling itself.

`WorkspaceWindowCloseDelegate.responds(to:)` forwards to the previous window delegate (`PitexApp.swift` lines 3330 to 3336). The delegate is not the window. After a remote close, `close()` clears `remote` before `performClose`, so the second `windowShouldClose` returns at the `remote != nil` check (`PitexApp.swift` lines 3338 to 3348 and 1357 to 1373) instead of sticking on `closeRequested`.

`BuildSupport.swift` lines 832 to 840 call `try! ShellAuthority(...)` with `source: .userConfiguration`, `approvedByUser: true`, and a non-empty disclosure. `ShellAuthority.init` throws only when the source is not built-in and the user has not approved it, or when the disclosure is empty (`BuildCore.swift` lines 67 to 69). Those arguments do not throw.

`save_remote_sources` returns immediately when `remote` is `None` (`model.rs` lines 2028 to 2029). The later `unwrap` (line 2058) follows that check. `end_remote_session` is only called from `close` (line 2356), which this function does not call.

`substitute_placeholders` reads the next character only while `i < template.len()` (`model.rs` lines 4067 and 4103), so `chars().next()` is `Some`.

`apply_build_finished` calls `active_run.take().unwrap()` only after `active_run` matched the build id (`model.rs` lines 4914 to 4915).

`utf8_complete_prefix` holds at most three trailing bytes (`build-core` `lib.rs` lines 2155 to 2167). A build log does not grow the carry without bound.

`BuildOrchestrator::build` returns `BuildAlreadyRunning` when a build is already active (`build-core` `lib.rs` line 2263).

Malformed adapter lines are skipped in `cli.ts` lines 47 to 52, `jsonrpc.ts` lines 127 to 132, `grok-adapter.ts` lines 262 to 267, `agy-adapter.ts` lines 508 to 514, and `claude-map.ts` lines 529 to 535. `loadIndex` returns an empty index when the file is missing or invalid (`session-index.ts` lines 30 to 39).

`Tools/install-deb.sh` line 24 builds the temp directory with `mktemp` before the exit trap removes it. The input-audit step that ends a command with `|| true` (`.github/workflows/input-audit.yml` line 83) still requires `INPUT_AUDIT_SUMMARY` on the next lines. The release skill unpack (`.github/workflows/release.yml` lines 670 to 674) runs under the Actions default `bash -e`, so a failed `node` check fails the step.

The model-popup flag already publishes the catalog when it clears (`AgentCoordinator.swift` lines 117 to 123). That was not re-opened.

## Mac-only verification

Run these on a Mac before treating finding 2 or finding 3 as fixed.

1. Feed `PiAgentProcess` a stdout pipe of more than 4 MiB with no newline. The process should stay up and the buffer should stop growing.
2. Paste a NUL into the custom-shell field, if the field accepts it. The app should keep the previous build settings.
3. Point embedded preview at a project directory whose name is not UTF-8, after the helper reads `args_os`. The helper should get past argument setup. Today that case is the panic in finding 1. The Mac driver `main` has the same `std::env::args()` call, so the Mac helper has the same startup panic once a non-Unicode argument reaches it.
