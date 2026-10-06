# Application architecture

[Main README](../../README.md) · [Build instructions](../../Development/README.md)

| Platform | Application structure | Status |
|---|---|---|
| macOS | SwiftUI, AppKit, and TextKit in [Mac/](../../Mac/). Portable SwiftPM targets in [Packages/](../../Packages/). | Available for Apple Silicon. The SwiftPM targets also compile on Linux. |
| Linux | GTK4 and libadwaita in [Linux/](../../Linux/). One Rust crate for each Swift target. | Available for Ubuntu 24.04 and 22.04. [Tools/rust-target-dag.json](../../Tools/rust-target-dag.json) gives the target relationships. |
| Windows | GTK4 and libadwaita in [Windows/](../../Windows/), with the same Rust crates. | Beta installer and portable ZIP available. |

The Windows application uses the MSYS2 GTK stack.
Its `pitex-shell` crate supplies the shell.
Its `windows-platform` crate implements the Windows interfaces in `app-ports`.
The Windows beta can have errors and stability problems.

Git uses the local repository for local projects and the remote repository for
SSH projects. See the [SSH Git coordination design](../features/projects-through-ssh/GIT_WORKFLOW.md)
for mirror ownership, save ordering, and concurrent remote work.

## Development and contributions

[CI](../../.github/workflows/ci.yml) runs these checks for each pull request:

- SwiftPM tests.
- Linux Rust workspace tests for Ubuntu 24.04 and the Ubuntu 22.04 compatibility build.
- A Windows workspace build with MSYS2 UCRT64.

A pushed `v*` tag starts the release workflow.
The workflow builds the DMG, both Ubuntu packages, the Windows installer, and the
Windows portable ZIP, and attaches them to a GitHub Release with checksums.

Use [Issues](https://github.com/jaehwan-2ee/pitex/issues) for problems and feature requests.
Use pull requests for code changes.


## Embedded engine and library notices

The embedded helpers are `pitex-preview` and `pitex-preview-xetex`.
Their source is in [PreviewEngine/](../../PreviewEngine/).
It includes TeXpresso (MIT), XeTeX (SIL/MIT-style), and Tectonic (MIT) code.
The helpers link dynamically to system or included libraries.

TECkit uses LGPL-2.1-or-later.
The helpers use it without modification, and you can replace the library.
See **Corresponding source** in [THIRD-PARTY.md](../../PreviewEngine/licenses/THIRD-PARTY.md).
The helper binaries contain no TeXpresso GPL `dpx` component (xdvipdfmx) or MuPDF/SDL renderer.
Dynamically loaded system libraries keep their own licenses.

See [PROVENANCE.md](../../PreviewEngine/PROVENANCE.md) and the [license notices](../../PreviewEngine/licenses/).
The Ubuntu package installs the notices in `/usr/share/doc/pitex/preview-engine/`.
The macOS application includes them in `Contents/Resources/PreviewEngine/`.

The macOS PDF column uses the system PDFKit `PDFView`.
The macOS PDF column includes no third-party PDF viewer code.
Copyleft components keep their own notices and license obligations.
