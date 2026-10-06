# Windows feature comparison for 1.0.0

The Ubuntu applications and the Windows beta share `pitex-shell` and the
domain/feature crates under `Linux/crates/`. Ubuntu 22.04 uses GTK 4.6 APIs;
Ubuntu 24.04 and Windows use the `modern-gtk` feature. The Windows release
enables `markdown-preview`, `equation-preview`, and `embedded-preview`.

| Feature | Ubuntu 24.04 | Ubuntu 22.04 | Windows 1.0.0 |
|---|---|---|---|
| Editor, completion, highlighting, folding, minimap | Shared implementation | Shared implementation | Shared implementation |
| Workspace/project tree, dependency discovery, TODOs, symbols | Shared implementation | Shared implementation | Shared implementation |
| Build targets, compiler profiles, final builds, cancellation | Shared scheduler | Shared scheduler | Shared scheduler; Windows process termination |
| PDF viewing, detachable preview, forward/inverse SyncTeX | Poppler/GTK | Poppler/GTK | Poppler/GTK |
| Pi assistant, models, attachments, inline suggestions | Shared implementation | Shared implementation | Shared implementation |
| Git pane and SSH projects, explicit save/upload | Shared implementation | Shared implementation | Shared implementation; Windows OpenSSH |
| Embedded terminal | VTE | PTY/terminal-core | ConPTY/terminal-core |
| Terminal mouse/focus reports and keypad modes | VTE | terminal-core | terminal-core |
| Markdown, KaTeX, scroll sync, PDF export | WebKitGTK | WebKitGTK | WebView2 |
| Fast equation preview at pointer/caret, macros, theme and settings | MathJax/WebKitGTK | MathJax/WebKitGTK | MathJax/WebView2; enabled in this release |
| Exact equation preview using project compiler context | Shared TeX renderer | Shared TeX renderer | Shared TeX renderer; enabled in this release |
| Unsaved editing preview, original source preserved | Bundled checkpointed XeTeX helper | Bundled checkpointed XeTeX helper | Bundled snapshot helper and installed XeLaTeX; enabled in this release |
| Editing-preview PDF and coherent SyncTeX | Shared publication protocol | Shared publication protocol | Shared publication protocol |
| Intermediate page publications during a compiler pass | Available | Available | Complete passes only |
| Compiler compatibility live preview | Available | Available | Available |
| Default application registration for TeX/Markdown | Desktop MIME handlers | Desktop MIME handlers | Per-user registry and Windows Default Apps |
| Open/reveal files and application window routing | Desktop open/Unix IPC | Desktop open/Unix IPC | Windows shell/loopback IPC |
| Application updates | DEB installer | DEB installer | Per-user EXE installer; restored release assets |

The Windows snapshot helper is a separate portable implementation because
the Unix XeTeX checkpoint engine depends on `fork`, Unix sockets, and
platform-specific translated ABI code. It applies the newest unsaved
overrides to an isolated copy, cancels obsolete compiler processes, and
publishes only a complete PDF. It requires an installed XeLaTeX and the
document's packages and fonts. The final project build remains authoritative.

Windows requires the Microsoft Edge WebView2 Evergreen runtime for the two
web previews. The loader DLL, renderer assets, settings schemas, icon themes,
and runtime module dependencies ship in both the installer and portable ZIP.
