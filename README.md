# Pitex 1.0.2

Pitex is a native LaTeX application for macOS, Linux, and Windows.
It has a text editor, PDF preview, project builds, and a Pi AI assistant.

| Platform | Availability |
|---|---|
| macOS | Apple Silicon, macOS 15 or later. |
| Linux | Ubuntu 24.04 and Ubuntu 22.04 packages. |
| Windows | Beta installer and portable ZIP for Windows 10 version 1809 or later. |

[Download a release](https://github.com/jaehwan-2ee/pitex/releases) · [Install Pitex](Docs/installation/README.md) · [Documentation](Docs/README.md)

## Features

### Pi AI assistant

Use the Pi assistant to work on project files in the bottom console.
Review each proposed edit before you accept or reject it.
Pitex includes eight skills for citations, scientific writing, submission checks, and TeX tasks.

[Assistant guide and included skills](Docs/features/pi-ai-assistant/README.md)

### Inline autocompletion

Pi can suggest a LaTeX continuation at the caret.
Press **Tab** to accept it.
Press **Esc** to remove it.

[Inline autocompletion settings](Docs/features/inline-autocompletion/README.md)

### Live compile

The editing preview shows changes while you type, including unsaved changes.
You can select the embedded preview or the project compiler.
Use **Build** to create the final PDF with the project compiler.
SyncTeX connects source lines to PDF locations.

[Preview backends and SyncTeX](Docs/features/live-compile/README.md) · [Native engine compatibility](PreviewEngine/COMPATIBILITY.md)

### Equation preview

Point at an equation or put the caret in it to show a preview.
Fast preview uses the included MathJax renderer.
Exact TeX preview can compile the current equation with the project settings.

[Equation preview modes and settings](Docs/features/equation-preview/README.md)

### Markdown preview

Open a Markdown file to use the editor and a live preview.
The preview supports KaTeX math and scroll synchronization.
You can select its theme and font size.

[Markdown preview settings and platform requirements](Docs/features/markdown-preview/README.md)

### Projects through SSH

Open a remote project through SSH and edit its local mirror.
**Save**, **Save All**, and **Build** upload changes and build on the remote device.
Local editing and preview updates do not upload files.
On macOS, compiler preview can build the local mirror automatically.

[SSH connection, save behavior, and local preview](Docs/features/projects-through-ssh/README.md)

### References, citations, and tasks

Pitex suggests labels for references and keys for citations.
Use the sidebar to find labels, bibliography entries, and project tasks.
The task list reads `% TODO:` and `% DONE:` comments.

[References, citations, and task controls](Docs/features/references-citations-and-tasks/README.md)

### Symbols Table

Open the Symbols Table from the editor's symbols button.
Select a symbol to insert its LaTeX command at the caret.

[Symbols Table guide](Docs/features/symbols-table/README.md)

### Workspace sidebar

The sidebar has **Workspace**, **Project**, and **TODOs** tabs.
Use **Workspace** to browse files and **Project** to inspect document dependencies.

[Workspace sidebar layout](Docs/features/workspace-sidebar/README.md)

## Download and installation

Get the package for your platform from [Releases](https://github.com/jaehwan-2ee/pitex/releases).

For macOS, use Homebrew:

1. Add the tap:

   ```sh
   brew tap jaehwan-2ee/tap
   ```

2. Install Pitex:

   ```sh
   brew install --cask pitex
   ```

For Ubuntu, download the installer script and the matching package.
Run the command for your Ubuntu version:

| Ubuntu version | Command |
|---|---|
| 24.04 | `sh ./install-pitex-deb.sh ./Pitex-v1.0.2-ubuntu24.04-amd64.deb` |
| 22.04 | `sh ./install-pitex-deb.sh ./Pitex-v1.0.2-ubuntu22.04-amd64.deb` |

For Windows, run `Pitex-v1.0.2-windows-amd64-setup.exe`.
For a portable installation, extract the ZIP and run `pitex\bin\pitex.exe`.

[Full installation instructions](Docs/installation/README.md#download-and-installation)

## Nightly builds

Pitex Nightly is a side-by-side prerelease channel built from `main`.
It uses its own settings, caches, and update feed, and does not share state with stable Pitex.

- **macOS:** `brew install --cask pitex@nightly`, or download the `Pitex-Nightly-<version>-macos-arm64.dmg`.
- **Linux:** install `Pitex-Nightly-<version>-ubuntu24.04-amd64.deb` or the Ubuntu 22.04 package alongside stable.
- **Windows:** run `Pitex-Nightly-<version>-windows-amd64-setup.exe`, or extract the ZIP and run `pitex-nightly\bin\pitex-nightly.exe`.

Nightly updates use the rolling `nightly` prerelease and install `pitex-nightly`, `Pitex Nightly.app`, or `Pitex Nightly` in their own directories.
See the [installation guide](Docs/installation/README.md#nightly-builds) for details and warnings.

## Runtime requirements

Install a TeX distribution for document builds and the editing preview.
Pi needs Bun, or Node.js **22.19 or later** with npm.
The editor and PDF column can operate without Pi or a TeX distribution.

[Runtime and platform requirements](Docs/installation/README.md#runtime-requirements)

## Application updates

Use **Settings → Updates → Check for Updates** to find a newer release.
The update method depends on your platform and installation method.

[Update instructions](Docs/installation/README.md#application-updates)

## Development and contributions

See the [application architecture](Docs/architecture/README.md) and [build instructions](Development/README.md).
Platform source guides are available for [Linux](Linux/README.md) and [Windows](Windows/README.md).
Use [Issues](https://github.com/jaehwan-2ee/pitex/issues) for problems and feature requests.
Use pull requests for code changes.

[Keyboard shortcuts](Docs/reference/README.md) · [Engine provenance](PreviewEngine/PROVENANCE.md)

## License

Copyright (C) 2026 Pitex contributors.
Pitex source code is licensed under the [GNU Affero General Public License, version 3 or later](LICENSE) (AGPL-3.0-or-later).
You can use, modify, and distribute Pitex, including commercially.
If you distribute Pitex, or let users interact with a modified version over a network, you must offer them its source code under the same license.
Pitex comes with no warranty; see the license for the full terms.

Components keep their own licenses and notice requirements.
See the [engine license notices](PreviewEngine/licenses/THIRD-PARTY.md) and [library information](Docs/architecture/README.md#embedded-engine-and-library-notices).
