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
| Preview helper arguments, the second-output abort, the PDF predictor division, and `remove_dir` | Covered |
| Linux remote save, placeholder scan, build-log carry, agent stdout cap, second build | Covered |
| Adapter line parsers, Grok and Antigravity abort, and `get_messages` ordering | Covered |
| `Tools/install-deb.sh`, one input-audit step, one release skill step | Covered |
| Windows uninstaller and `shell_target` | Covered |
| Rest of the Windows app | Not yet audited |
| TexCore pull, fetch, and resolve downloads | Covered |
| TexApp editor rejection fallback | Covered |
| Rest of TexCore persistence and TexApp | Not yet audited |
| Linux assistant refresh during SSH close, Save As, Save PDF, local close, and the uncapped build PDF and log | Covered |
| Rest of the Linux GTK UI | Not yet audited |
| Rest of preview `publish`, unsafe blocks, and image decoders | Not yet audited |
| Nightly tag move, asset cleanup, input-audit summary, SSH preview skip, and the source-bundle check | Covered |
| Other workflow jobs outside the sites below | Not yet audited |
| Retain cycles, main-thread UI calls, and force unwraps outside the sites below | Not yet audited |

The Mac, Windows, preview, Linux, adapter, TexApp, and CI reviewers finished. Their crash and data-loss notes below were re-checked against the source in this checkout. Anything outside the sites named above is not yet audited.

Numbers below follow the order a finding was confirmed. Severity is on each item.

## Findings

Severity order is crash, then data loss, then wrong behavior, then minor. Data-loss items were read in the source or reproduced outside the app. None of them was run inside Pitex.

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

### 2. Fold detach asks the layout manager for a character range past the buffer

- Severity: crash
- Confidence: high that the range is unclamped. The AppKit exception was not executed here.
- Reachable: yes, if a folded document is shortened and the editor view is dismantled before the fold recompute runs
- Fixed in this change: no

`detach` is `Mac/Sources/Features/EditorFolding.swift` lines 80 to 88. It cancels the pending recompute, then clears hidden glyphs with the old line table.

```swift
func detach() {
    recomputeTask?.cancel()
    recomputeTask = nil
    if let observer { NotificationCenter.default.removeObserver(observer) }
    observer = nil
    if let layoutManager = textView?.layoutManager {
        clearHiddenGlyphs(layoutManager: layoutManager)
    }
```

Edits do not recompute immediately. The text-change observer waits `EditorTiming.analysisDebounce` (`EditorFolding.swift` lines 69 to 74). `clearHiddenGlyphs` builds an `NSRange` from `lineStarts` and does not clamp `start` to the current string (`EditorFolding.swift` lines 325 to 330). `applyFolds` uses the same unclamped `glyphRange` call at line 306.

Switching documents dismantles the editor view. `EditorContainerView.dismantleNSView` calls `foldEngine?.detach()` at line 206. The workspace keys that view by editor identity (`WorkspaceView.swift` line 451).

Trigger. Fold a region, delete most of the document so a stored line offset is past the new string, then switch documents before the debounce fires. `glyphRange(forCharacterRange:)` is then called with a location past the storage.

Proposed fix. Clamp each start and end to `text.length` before building the range, in both `clearHiddenGlyphs` and `applyFolds`. Skip the line when the clamped length is empty. Needs a Mac test that shortens a folded buffer and detaches the view.

### 3. Local close and quit drop unsaved editor text

- Severity: data-loss
- Confidence: high on the code path. The app was not run.
- Reachable: yes, with the default autosave setting
- Fixed in this change: no

`autoSave` starts false (`SettingsView.swift` line 278). `closeDocument` writes the project only when `remote != nil` (`PitexApp.swift` lines 1282 to 1293). For a local file it waits for writes that are already queued (line 1298) and then removes the session. `DocumentSessionRegistry.close` only deletes the session from its map (`DocumentSessionRegistry.swift` lines 97 to 103). It does not write the file.

Quit takes the same local path. `applicationShouldTerminate` returns `.terminateNow` when no workspace has pending writes and none is remote (`PitexApp.swift` lines 3131 to 3132). `prepareForTermination` for a local project only waits for queued writes (lines 1330 to 1332). `applicationWillTerminate` stops the preview helper and the agent (lines 3151 to 3158) and does not save documents.

The tab close button calls `closeDocument` (`WorkspaceView.swift` line 344). File → Close calls `performCloseCommand`, which calls `close()` when a project is open (`PitexApp.swift` lines 692 to 696). `close` saves an SSH project (lines 1363 to 1371) and then `tearDownWorkspace` closes every session without a local save (lines 1461 to 1464).

`saveAll` is the path that does call `persistDirtySessions` for a local project (lines 1177 to 1178). Close and quit do not call it.

Trigger. Leave autosave off. Type into a local file. Close the tab, choose File → Close, or quit, before an explicit Save. The new text is not written and no save dialog runs.

Proposed fix. Before `registry.close` and before `.terminateNow`, flush the editor and ask Save, Don't Save, or Cancel when a local session is dirty. Reuse the SSH flush for the Save choice. This is a behavior change in the close path. It needs a Mac test of tab close, ⌘W, and quit. It was not patched here.

### 4. Toggle Line Comment edits the live buffer from the trailing snapshot

- Severity: data-loss
- Confidence: high that the command reads `documentSnapshot` and writes the text view. The keystroke race was not executed.
- Reachable: yes, when the text view and `documentSnapshot` differ
- Fixed in this change: no

`jumpTo` records the rule. The live text view is authoritative, and `documentSnapshot` trails edits made since the last flush (`PitexApp.swift` lines 1839 to 1840). `toggleLineComment` does not flush. It reads `documentSnapshot?.text` (line 792), builds `lineRange` from that string (line 796), and writes the text view (lines 814 to 815). Save As calls `flushPendingChanges` first (line 766). The comment command does not.

The editor submits text on a task that suspends at `await session.submit` (`EditorMacAdapter.swift` lines 518 to 531). The main actor can run ⌘/ during that wait. The menu item is installed at `PitexApp.swift` lines 3023 to 3024.

Trigger. Type a character and press the line-comment shortcut before the snapshot catches the text view. The command applies a range from the old string to the live buffer. Characters that exist only in the view can be overwritten. If the snapshot is longer than the view, `replaceCharacters(in:with:)` can be asked for a range past the view.

Proposed fix. Build `lineRange` from `textView.string`, and clamp it to that string before `replaceCharacters`. A flush before the edit matches Save As, and the clamp is still required when a disk reload has published a snapshot the view has not applied yet. Needs a Mac test. Not patched here.

### 5. Mac agent stdout grows without a cap

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

### 6. A settings binding uses `try!` on a throwing preferences value

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

### 7. Uninstall deletes the chosen directory and kills every `pitex.exe`

- Severity: data-loss
- Confidence: high for what the script requests. NSIS and `taskkill` were not executed here.
- Reachable: yes. The installer shows a directory page, and uninstall uses the directory stored at install time.
- Fixed in this change: no

The uninstall section is `Windows/pitex.nsi` lines 95 to 99.

```nsis
Section "Uninstall"
  nsExec::ExecToStack 'taskkill /F /IM ${BINARY}'
  Pop $0
  RMDir /r "$INSTDIR"
  RMDir /r "$SMPROGRAMS\${NAME}"
```

`BINARY` is `pitex.exe`. `taskkill /F /IM` matches that image name for the whole machine. The pop of `$0` ignores the result. `RMDir /r "$INSTDIR"` then removes every file under the install directory. The directory page is line 50, so the user can point the install at a folder that already holds other files. There is no list of files the installer copied.

The install section says not to kill unrelated instances (`pitex.nsi` lines 59 to 60). Uninstall does not follow that.

Trigger. Install into a folder that contains other files, or add a file under the install directory later, then uninstall. Those files are deleted. Uninstall also force-kills every running `pitex.exe`, including a copy that is not the install being removed. Autosave defaults off, so a killed editor drops its unsaved buffer.

Proposed fix. Delete only the files the installer copied, and remove `$INSTDIR` only when it is empty. Stop uninstall when the installed executable is still running, instead of `taskkill /IM` on every `pitex.exe`. This is not a small patch. It was not changed here.

### 8. `shell_target` turns some `\\?\` paths into relative paths

- Severity: wrong-behavior
- Confidence: high for the string rewrite. `ShellExecuteW` was not called.
- Reachable: yes for a `\\?\Volume{...}` path. A lowercase `unc` prefix is reachable only if the path was not already spelled `UNC`.
- Fixed in this change: no

`shell_target` in `Windows/crates/windows-platform/src/lib.rs` lines 348 to 366 always drops a leading `\\?\`. It rewrites the remainder only when the next units are the exact string `UNC\`.

A replica of that prefix logic, compiled and run on Linux, printed:

```
\\?\C:\paper\draft.pdf
  -> C:\paper\draft.pdf
\\?\UNC\server\share\draft.pdf
  -> \\server\share\draft.pdf
\\?\unc\server\share\draft.pdf
  -> unc\server\share\draft.pdf
\\?\Volume{12345678-1234-1234-1234-123456789abc}\paper\main.pdf
  -> Volume{12345678-1234-1234-1234-123456789abc}\paper\main.pdf
```

The in-tree tests cover the drive letter and the uppercase `UNC` case (`lib.rs` lines 551 to 568). They do not cover `Volume{...}` or a lowercase `unc` prefix.

`open_document` calls this function (`lib.rs` lines 300 to 303). The PDF open button and the agent-config button reach it from `Linux/crates/pitex-shell/src/panes.rs`.

Trigger. Open a file whose canonical path is `\\?\Volume{guid}\...`. The shell receives a relative path.

Proposed fix. Strip `\\?\` only for a drive letter and for `UNC` compared without case. Leave every other `\\?\` path unchanged. Add those two inputs to the existing tests. Not patched here, because this environment cannot run the Windows crate.

### 9. A second `.log`, `.xdv`, `.dvi`, `.pdf`, or `.synctex` open aborts the preview process

- Severity: crash
- Confidence: high that the driver calls `abort` when that slot is already filled. The TeX engine was not run, so the sample document was not executed.
- Reachable: yes if a second write with one of those extensions is recorded after the first.
- Fixed in this change: no

`PreviewEngine/src/linux/driver/engine_tex.rs` lines 1340 to 1395 abort when `st.document`, `st.synctex`, or `st.log` already has an entry and another open arrives for `xdv`, `dvi`, `pdf`, `synctex`, or `log`. The macOS twin prints the same "two log files" error at `PreviewEngine/src/macos/driver/engine_tex.rs` line 1396.

```rust
if !(*self_0).st.log.entry.is_null() {
    fprintf(stderr, b"[error] two log files!\n\0" ...);
    print_backtrace();
    abort();
}
```

Trigger. A document that opens a second file whose extension is one of those five after the engine has already opened the job log, the xdv, or the synctex file. One candidate is `\immediate\openout` of `notes.log`.

Proposed fix. Keep the first log, output, and synctex handles, and treat a later write with the same extension as an ordinary auxiliary file. Do not call `abort`. Not patched here. The helper was not rebuilt.

### 10. PDF predictor parameters divide by zero

- Severity: crash
- Confidence: high. The arithmetic was reproduced. A PDF was not fed through the helper.
- Reachable: yes when a decoded stream reaches `apply_predictor` with these parameters.
- Fixed in this change: no

`apply_predictor` in `PreviewEngine/src/linux/shared/pdfread.rs` lines 1357 to 1364 computes the row length in `i32`, casts it to `usize`, and then divides by `row + 1`. Predictor values below 10 return earlier. Predictor 15 does not.

For `Colors 1`, `BitsPerComponent 1`, and `Columns -15` the row length is `(-8 / 8)` as `usize`, which is `usize::MAX`. Adding 1 wraps to 0. `wrapping_div` then panics. A release build of that arithmetic exited 101:

```
bpp=1 row=18446744073709551615 row+1=0
thread 'main' panicked at predictor_panic.rs:9:17:
attempt to divide by zero
```

`get_image` calls the PDF reader for a file that starts with `%PDF-`. `get_int` returns the integer with no range check (`pdfread.rs` line 1026).

Proposed fix. Reject `Colors`, `BitsPerComponent`, or `Columns` below 1, and return false when `row.wrapping_add(1)` is 0, before the division. Not patched here. The helper was not rebuilt.

### 11. `remove_dir` deletes files through a directory symlink

- Severity: data-loss
- Confidence: high. The same `opendir` and `unlink` sequence deleted a file in the symlink target. The 4096-byte path truncation was not executed.
- Reachable: yes when `{out}/p{seq}` is a symlink to another directory.
- Fixed in this change: no

`remove_dir` in `PreviewEngine/src/linux/driver/main.rs` lines 1380 to 1412 opens the path with `opendir`, unlinks each child, and then calls `rmdir`. `opendir` follows a directory symlink. `release` builds `{out}/p{seq}` with `snprintf` into a 4096-byte buffer and passes it to `remove_dir` without checking the return (`main.rs` lines 1776 to 1784).

A stand-alone copy of that loop was run against a symlink:

```
p1 -> secret
unlink .../p1/notes.txt -> 0
rmdir -> -1
```

`notes.txt` inside `secret` was gone. `rmdir` of the symlink failed, so the symlink stayed.

`snprintf` into the fixed buffer is also unchecked. A canonical `--out` path near 4096 bytes can make the formatted path collapse onto `--out` itself. That length was not executed here.

Proposed fix. If `snprintf` does not fit, skip the deletion and report an I/O error. In `remove_dir`, `lstat` the path and unlink a symlink without following it. For a real directory, delete children through a descriptor opened with `O_NOFOLLOW` and `O_DIRECTORY`. Not patched here. Another change is already editing publish in this file, and the helper was not rebuilt.

### 12. Closing an SSH project while an assistant render is deferred panics

- Severity: crash
- Confidence: high that `refresh_assistant` unwraps `agent` after `ensure_agent` has refused to create one. The GTK timeout was not run.
- Reachable: yes during an SSH close while a transcript render is already scheduled.
- Fixed in this change: no

`refresh_assistant` in `Linux/crates/pitex-shell/src/app_ui.rs` lines 4242 to 4246 calls `ensure_agent` and then `self.agent.as_mut().unwrap()`. `ensure_agent` returns immediately while `closing_remote_workspace()` is true (line 2743) and does not recreate the agent. That flag is set when `pending_remote_save` or `queued_remote_save` is any remote action except Build (lines 233 to 235 and 3463 to 3466). `begin_remote_save` for a closing action calls `shutdown_agent`, which sets `agent` to `None` (lines 3480 to 3482 and 1593 to 1596). File → Close on a remote project takes that path (lines 2075 to 2078).

A streaming reply schedules one deferred render with `glib::timeout_add_local_once` (`schedule_transcript_render`, lines 5242 to 5249). That callback calls `refresh_assistant` with no check that `agent` is still present. `shutdown_agent` does not remove the source. The same unwrap is reached from the agent-config debounce, which calls `refresh_assistant` after `ensure_agent` even when the coordinator is still absent (lines 4606 to 4610).

A local close does not take this path. `closing_remote_workspace` is false, so `ensure_agent` builds a new coordinator before the unwrap.

Proposed fix. After `ensure_agent`, return when `self.agent` is `None`. Not patched here. The GTK UI was not run.

### 13. Save As and Save PDF truncate the destination before the write finishes

- Severity: data-loss
- Confidence: high that both calls use `std::fs::write`. A full disk was not simulated.
- Reachable: yes. Save As can target the open document. Save PDF can target an existing file.
- Fixed in this change: no

`save_as` writes the editor snapshot with `std::fs::write` (`Linux/crates/pitex-shell/src/model.rs` lines 2452 to 2459). That opens the destination with truncation before the bytes are written. On `Err` it stores `WorkspacePhase::Failed` and returns, so a failed write can leave a shorter file. The failure path returns before `activate_document`.

Preview → Save PDF uses the same call and ignores the error (`Linux/crates/pitex-shell/src/panes.rs` lines 1516 to 1518).

Normal Save goes through `AtomicDocumentStore::save` (`Linux/crates/document-session-core/src/lib.rs` lines 631 to 677). A failed write there removes the temp file and leaves the original.

Proposed fix. Write Save As and the PDF export through the same temp file, fsync, and rename, and surface the error. Not patched here.

### 14. Linux local close and quit drop unsaved text, and a failed file switch covers the editor

- Severity: data-loss
- Confidence: high for the close and quit paths. The window was not run.
- Reachable: yes with autosave left off, one local edit, then close the window or quit.
- Fixed in this change: no

`connect_close_request` lets a local window close proceed (`app_ui.rs` lines 5975 to 5978). An SSH window starts `begin_remote_save` instead. `application` shutdown waits for writes already in flight and does not flush a dirty buffer (`app_ui.rs` lines 5836 to 5848). `model.close` says pending edits die and removes the sessions (`model.rs` lines 2351 to 2364).

A failed switch makes this easier to hit. `ActivateFinished(Err)` sets `WorkspacePhase::Failed` and `refresh_phase` shows the error page (`app_ui.rs` lines 4665 to 4667 and 2933 to 2940). `read_exact_utf8` rejects a source over 32 MiB or that is not UTF-8 (`model.rs` lines 5409 to 5417). The error page replaces the editor. Its Open button calls `present_open` (lines 6561 to 6562). While a project is still open, `open_routed` asks for a new window (lines 1627 to 1631). It does not call `model.open` in this window, so that button by itself does not drop the session. Quitting from the error page still does.

Proposed fix. On a local dirty session, flush and ask before the window closes and before shutdown. On `ActivateFinished(Err)`, report the error and leave the editor up. Not patched here.

### 15. A build reads the whole output PDF into memory with no size cap

- Severity: crash, by allocation, if the PDF is larger than the process can hold
- Confidence: high that the reads are uncapped. The allocation failure was not run.
- Reachable: yes when `main.pdf` already exists and a build starts.
- Fixed in this change: no

Before the engine runs, an existing output is read with `std::fs::read` and kept as `preexisting_pdf` (`Linux/crates/build-core/src/lib.rs` lines 2306 to 2309). A successful build reads the output again (line 2434). `output_changed` reads it a third time (lines 3034 to 3036). A read error fails the build. There is no size check.

Opening a project refuses the same file over 50 MiB. `MAX_PREVIEW_PDF_BYTES` is that cap (`model.rs` line 752), and `built_pdf_bytes` uses `read_bytes_capped` (lines 3467 to 3473). The build path does not.

Proposed fix. Use the existing 50 MiB cap for these three reads, leave a larger PDF on disk, and publish the path. Not patched here.

### 16. The build log string grows without a cap

- Severity: crash, by allocation, if the build command writes without stopping
- Confidence: high that each chunk is appended. A runaway command was not run.
- Reachable: yes. The build command is user-editable.
- Fixed in this change: no

`apply_build_event` appends every log chunk to `build_log_text` (`model.rs` line 4893). Git status stops at 4 MiB and kills the child. The build log has no such stop. Cancel still leaves the bytes already copied in the string.

Proposed fix. Cap `build_log_text` the way git status is capped, and drop the rest. Not patched here.

### 17. Grok abort signals the shell and leaves a background writer running

- Severity: data-loss
- Confidence: high. A stand-alone `/bin/sh -c` with the same `child.kill()` wrote the file after the signal. The full bridge was not driven.
- Reachable: yes when an approved terminal command backgrounds a writer and the user stops the turn.
- Fixed in this change: no

`startTerminal` spawns `terminalSpawnSpec` with no process group (`Mac/Resources/PitexAgent/bridge/grok-adapter.ts` lines 1014 to 1020). An empty argv becomes `/bin/sh -c` plus the command (lines 360 to 369). `abort` calls `killTerminals` (lines 638 to 643). That sends `child.kill()`, which is SIGTERM to the shell, then drops the handle (lines 1129 to 1138).

User Stop sends `abort`. It does not kill the bridge process group. Mac `AgentCoordinator.stop` is lines 731 to 734. Linux `stop` is `agent.rs` lines 2189 to 2196. Linux `terminate` does kill the group, and Stop does not call it.

A stand-alone spawn of `/bin/sh -c '(sleep 0.4; printf survived > file) & sleep 30'` then `child.kill()` left `late.txt` containing `survived`. The shell's signal was SIGTERM.

Proposed fix. Put each terminal in its own process group and SIGKILL that group, then wait. Not patched here.

### 18. A Grok update after abort reopens the turn

- Severity: data-loss
- Confidence: high that `onNotification` has no cancel flag. The CLI was not run here.
- Reachable: yes if the CLI sends `session/update` after `session/cancel`.
- Fixed in this change: no

`completeTurn` sets `running` to false and emits the events from `closeTurn` (`grok-adapter.ts` lines 837 to 845). `closeTurn` ends with `agent_end` and sets `turnOpen` to false (`grok-map.ts` lines 441 to 446). `onNotification` still records the update and maps it (lines 848 to 860). There is no cancelled check. `ensureTurn` emits a new `agent_start` when `turnOpen` is false (lines 143 to 147). `recordSessionUpdate` appends an `agent_message_chunk` to the history (lines 773 to 775).

Proposed fix. Set an ignore flag at the start of `abort` and return from `onNotification` while it is set. Clear it on the next prompt. Not patched here.

### 19. An older `get_messages` response replaces the transcript of the session now on screen

- Severity: data-loss
- Confidence: high for the missing request id. The two-thread race was not run here.
- Reachable: yes when a session switch starts a second `get_messages` before the first read returns.
- Fixed in this change: no

`cli.ts` handles each stdin line without waiting for the previous command (lines 44 to 57). Codex `getMessages` sends `thread/read` for `this.threadId` and awaits it (`codex-adapter.ts` lines 498 to 504). `switchSession` overwrites `this.threadId` (lines 480 to 496). Mac `get_messages` replaces `transcript` with no request id (`AgentCoordinator.swift` lines 1356 to 1357). Model-settings responses already require a matching id (lines 1280 to 1284). Linux does the same replace (`agent.rs` lines 2689 to 2696).

Proposed fix. Ignore a `get_messages` response whose id is not the latest request, and have `getMessages` drop the reply if `this.threadId` changed during the await. Not patched here.

### 20. Linux drops an agent JSON line longer than 4 MiB and keeps reading

- Severity: data-loss
- Confidence: high that the cap breaks before the newline and then parses the prefix. A cargo run of the reader was not done.
- Reachable: yes if one stdout line is larger than 4 MiB.
- Fixed in this change: no

The stdout reader in `agent.rs` lines 927 to 959 stops the inner read when `line.len()` passes 4 MiB, without consuming the newline (lines 937 to 939). It then parses that prefix with `serde_json::from_slice` (line 950). A prefix that is not JSON is dropped. The next read starts at the following byte, so the rest of the long line and later lines are separate parses. The oversized event is lost.

Mac `PiAgentProcess` has no such cap. That is finding 5.

Proposed fix. When the cap trips before a newline, discard bytes through the next newline and do not parse the prefix. Not patched here.

### 21. Antigravity abort returns success and still applies later stdout

- Severity: data-loss
- Confidence: high that `abort` does not bump `generation`. A child that exits on SIGINT produces no further stdout.
- Reachable: yes when the child writes another line after SIGINT.
- Fixed in this change: no

`abort` sends SIGINT and responds success (`agy-adapter.ts` lines 635 to 644). It does not increment `generation` and does not clear `turnActive`. Stdout is ingested while `generation` is unchanged (lines 441 to 443). `newSession` does increment `generation` and calls `stopChild` (lines 654 to 657).

Proposed fix. In `abort`, increment `generation`, set `turnActive` to false, then call `stopChild`. Not patched here.

### 22. Grok `closeTurn` reports an unfinished tool as success when the stop reason is `end_turn`

- Severity: wrong-behavior
- Confidence: high for the flag. The CLI was not run.
- Reachable: yes when a tool is still open and the prompt result is `end_turn`.
- Fixed in this change: no

`closeTurn` sets `isError` only when `stopReason` is `cancelled` or `refusal` (`grok-map.ts` lines 419 to 425). A tool that never reached `completed` still ends with `isError: false` on a normal stop.

Proposed fix. Set `isError: true` when the tool never completed. Not patched here.

### 23. A torn Claude tool JSON starts the tool with empty arguments

- Severity: wrong-behavior
- Confidence: medium. The mapper does this. The Claude SDK was not run.
- Reachable: yes if `content_block_stop` arrives before `partial_json` is valid JSON.
- Fixed in this change: no

On `content_block_stop`, a `JSON.parse` failure sets `args` to `{}` and calls `startTool` (`claude-map.ts` lines 524 to 537). `startTool` returns before emitting when `toolStarted` is already set (lines 379 to 384), so a later full `tool_use` does not emit a new start.

Proposed fix. On parse failure, do not call `startTool`. Not patched here.

### 24. Pull can install a short tar snapshot and record it as the baseline

- Severity: data-loss
- Confidence: high that the received bytes are placed without a check against the listing hash. GNU tar 1.35 on this machine archived 200000 bytes of a file that was 4200000 bytes when tar returned, and tar exited 0.
- Reachable: yes when a remote file changes between the hash listing and `tar -cf`.
- Fixed in this change: no

`download` runs `tarOut` with `client.run`, not `runChecked` (`Packages/TexCore/Sources/RemoteCore/RemoteSync.swift` lines 807 to 813). `run` returns a non-zero status and throws only on 255 (`SSHClient.swift` lines 187 to 199). A non-empty archive is kept. The comment on line 807 says a non-zero tar exit can still leave a usable archive.

`pull` places those bytes when the local hash is still the pre-download hash, then sets `manifest[path]` to the hash of the received file (lines 559 to 568). Nothing compares that hash to `remote[path]` from the listing. `fetch` places the same way with no local-hash re-check (lines 685 to 696). `resolve` with keep-remote does the same (lines 758 to 766).

`SyncPlanner.pull` downloads again when the remote hash differs from the manifest and the local hash still equals the manifest (lines 199 to 202). If the editor then saves a different local copy, both sides differ, so pull records a conflict (lines 203 to 204). `resolve(keepLocal: true)` uploads the local bytes over the remote file (lines 722 to 730). That upload is what would replace the remote tail.

Proposed fix. If the received hash is not the listing hash, skip `place` and do not update the manifest. Not patched here. Swift is not installed.

### 25. A rejected editor submit replaces the text view and drops the in-flight keystroke

- Severity: data-loss
- Confidence: medium. The fallback is written in the adapter. The keystroke and the disk change were not interleaved here.
- Reachable: yes when a submit is in flight and the session text changes.
- Fixed in this change: no

`EditorMacAdapter` submits on a task and, on `.rejected`, rebases only when `current.text == committed.text` and `attempts < 3`. Any other rejection calls `apply(current)` (`Packages/TexApp/Sources/EditorMacAdapter/EditorMacAdapter.swift` lines 537 to 548). The comment at lines 520 to 527 says that fallback drops the in-flight keystroke. `apply` sets `desiredText` to the snapshot text and replaces the text storage (lines 570 to 577). After that, `desiredText == committed.text`, so the follow-up `submitPendingChangeIfNeeded` does not resubmit the typed characters.

`processDiskChange` adopts the file when `saveState == .clean` (`PitexApp.swift` lines 1951 to 1956). A keystroke that has not reached `DocumentSession.apply` still leaves the session clean, so a watcher event during the await can resolve the session to the disk text and make `submit` return `.rejected`.

Proposed fix. On a divergent rejection, keep the text still in the view and resubmit that diff, or surface a conflict. Not patched here.

### 26. Nightly moves the tag before the assets exist, and the next run treats that as done

- Severity: data-loss
- Confidence: high for the step order and the skip. A failed publish was not run on GitHub.
- Reachable: yes when any command after the tag move exits non-zero.
- Fixed in this change: no

Publish moves `refs/tags/nightly` to HEAD (`.github/workflows/nightly.yml` lines 145 to 154) and uploads `dist/*` in the next step (line 172). The gate exits 0 without writing `version` when the tag already points at HEAD and `force` is not set (lines 56 to 58). An empty version skips build and publish. A red upload can leave the tag and the release notes on the new commit while the assets are still the previous upload. The next scheduled run is green and does not replace those assets.

Proposed fix. Move the tag after a successful upload. Treat "tag equals HEAD" as published only when the nightly manifest commit matches and the asset names for that version are present. Not patched here.

### 27. A failed nightly asset listing still exits 0

- Severity: data-loss
- Confidence: high that a failed `gh release view` does not fail the step. Which of two same-suffix assets the updater downloads was not run.
- Reachable: yes if the asset list fails after the upload in the same step.
- Fixed in this change: no

The cleanup is an `if` assignment with stderr discarded (`nightly.yml` lines 174 to 180). `set -euo pipefail` does not abort a command that fails in an `if` test. Old assets stay. New and old assets can share a suffix such as `windows-amd64-setup.exe`. `latest_release` keeps the first asset whose name ends with that suffix (`Linux/crates/pitex-shell/src/update.rs` lines 124 to 134). After finding 26, the next nightly does not retry the cleanup once the tag matches HEAD.

Proposed fix. Fail the step when the asset list fails. Not patched here.

### 28. The input-method audit stays green when the audit reports failures

- Severity: wrong-behavior
- Confidence: high. The session exit is discarded, and the summary line is printed before that exit.
- Reachable: yes on a manual dispatch of the input-method audit.
- Fixed in this change: no

The session ends with `|| true` (`.github/workflows/input-audit.yml` line 83). The next check is `grep -q '^INPUT_AUDIT_SUMMARY '` (line 94). `Tools/audit-ibus-capslock.py` prints that prefix and then exits 1 when `failures` is non-zero (lines 193 to 194). A log that says `"failures": 3` still satisfies the grep.

Proposed fix. Fail unless the summary JSON has `"failures": 0`. Not patched here.

### 29. The macOS SSH preview step succeeds when `gh release view` fails

- Severity: wrong-behavior
- Confidence: high. Any non-zero status takes the `exit 0` branch.
- Reachable: yes on the macOS CI job.
- Fixed in this change: no

`ci.yml` lines 176 to 178 treat every failed `gh release view` as "no published release" and exit 0. Auth failures, rate limits, and 5xx take the same path. The SSH preview fixture does not run.

Proposed fix. Exit 0 only for a confirmed missing release. Any other `gh` status should fail the step. Not patched here.

### 30. `verify-release-surface` passes when the source bundle is absent

- Severity: wrong-behavior
- Confidence: high that a missing file returns before any bundle check. Whether that early return was meant to be optional was not established.
- Reachable: yes. The gates job runs this tool, and no workflow runs `Tools/create-source-bundle.mjs`.
- Fixed in this change: no

`validateBundle` returns immediately when the path is not a file (`Tools/verify-release-surface.mjs` lines 65 to 66). A missing SBOM is a hard failure. A missing `Artifacts/source-bundle.tar.gz` is not. The tar, gzip, and manifest checks never run on CI.

Proposed fix. Fail when an explicit bundle path is not a file. Failing the default path also requires the gates job to create the bundle, so that half is not a one-line change. Not patched here.

### 31. A tag publish cannot recover when the release already exists

- Severity: wrong-behavior
- Confidence: medium. The workflow has one `gh release create` and no upload or edit fallback. `gh release create` was not re-run.
- Reachable: yes on a re-run of a `v*` publish after the release or its draft already exists.
- Fixed in this change: no

`release.yml` lines 824 to 825 are the only release mutation. There is no `release view` followed by upload. A second create fails because the tag already has a release.

Proposed fix. If `gh release view` succeeds, upload with `--clobber` and edit the notes. Create only when the release is missing. Not patched here.

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

`Tools/install-deb.sh` line 24 builds the temp directory with `mktemp` before the exit trap removes it. The release skill unpack (`.github/workflows/release.yml` lines 670 to 674) runs under the Actions default `bash -e`, so a failed `node` check fails the step. The input-audit `|| true` is finding 28. The summary line is not a failure check.

The model-popup flag already publishes the catalog when it clears (`AgentCoordinator.swift` lines 117 to 123). That was not re-opened.

## Mac-only verification

Run these on a Mac before treating the proposals as fixed.

1. Fold a region, delete most of the buffer, and switch documents before the fold debounce. The app should stay up. Today that path is finding 2.
2. With autosave off, type into a local file and close the tab, press ⌘W, and quit. Each path should ask before discarding the edit. Today those paths are finding 3.
3. Type one character and immediately toggle the line comment. The new character should remain. Today that path is finding 4.
4. Feed `PiAgentProcess` a stdout pipe of more than 4 MiB with no newline. The process should stay up and the buffer should stop growing. That is finding 5.
5. Paste a NUL into the custom-shell field, if the field accepts it. The app should keep the previous build settings. That is finding 6.
6. Point embedded preview at a project directory whose name is not UTF-8, after the helper reads `args_os`. The helper should get past argument setup. Today that case is the panic in finding 1. The Mac driver `main` has the same `std::env::args()` call, so the Mac helper has the same startup panic once a non-Unicode argument reaches it.
