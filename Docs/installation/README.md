# Install and update Pitex

[Main README](../../README.md)

## Download and installation

The [Releases](https://github.com/jaehwan-2ee/pitex/releases) page supplies the application packages.

### macOS

The recommended installation method uses Homebrew.

1. Add the tap:

   ```sh
   brew tap jaehwan-2ee/tap
   ```

2. Install the application:

   ```sh
   brew install --cask pitex
   ```

Homebrew installs Pitex in `/Applications`.
The command `brew upgrade --cask pitex` installs updates.
The application updater finds a Caskroom installation and uses Homebrew automatically.

For a DMG installation:

1. Download `Pitex-*-macos-arm64.dmg`.
2. Open the DMG.
3. Drag `Pitex.app` to `/Applications`.
4. On the first start, right-click the application.
5. Select **Open**.

The application uses ad-hoc signing and has no distribution signature.
Gatekeeper can make this first-start procedure necessary.

### Linux

Download `install-pitex-deb.sh` and the package for your Ubuntu version from the release page.
Use the installer from the folder that contains the downloads:

```sh
sh ./install-pitex-deb.sh ./Pitex-v1.0.1-ubuntu24.04-amd64.deb   # Ubuntu 24.04+
sh ./install-pitex-deb.sh ./Pitex-v1.0.1-ubuntu22.04-amd64.deb   # Ubuntu 22.04
```

APT also installs the necessary GTK libraries.
Both Ubuntu packages have an embedded terminal for interactive commands, including Pi sign-in.

The installer works with packages in private folders, such as `~/Downloads`.
It makes a temporary package copy that APT can read and deletes this copy after installation.
It keeps the original file and folder permissions.
It does not disable the APT sandbox.
Enter your password when `sudo` requests it.
Direct APT installation is also possible if the `_apt` user can read the package and its parent folders.

### Windows

Download `Pitex-v1.0.1-windows-amd64-setup.exe` for the per-user installer.
It installs into `%LOCALAPPDATA%\Programs\Pitex`.
It adds shortcuts.
It supports application updates without administrator access.

For a portable installation, extract `Pitex-v1.0.1-windows-amd64.zip` and run
`pitex\bin\pitex.exe`. Both packages include the embedded ConPTY terminal,
Markdown and equation previews, and the Windows editing-preview helper.

Windows support is beta. Use the [issue tracker](https://github.com/jaehwan-2ee/pitex/issues) to report problems.
See [Windows build instructions](../../Windows/README.md) for source builds.

## Runtime requirements

These requirements apply to the released application.
See the [build instructions](../../Development/README.md) for development dependencies.

| Requirement | Function |
|---|---|
| TeX Live, BasicTeX on macOS, or MiKTeX on Windows | Necessary for document builds. The editor and PDF preview can operate without a TeX distribution. |
| TeX Live or MacTeX, with XeTeX formats and fonts | Supplies TeX files for the embedded editing preview. |
| Bun, or Node.js **22.19 or later** with npm | Installs the Pi agent runtime in the application's support folder on the first start. |
| Internet access | Necessary for agent installation, update checks, and downloads. |

Without Bun or Node.js 22.19 or later with npm, the assistant is unavailable.
The other application functions can operate.

| Platform | Other requirements |
|---|---|
| macOS | Apple Silicon and macOS 15 or later. Homebrew is necessary only for a tap installation. |
| Linux | Ubuntu 24.04 or 22.04, with the matching package. APT supplies the GTK runtime dependencies. Updates need polkit (`pkexec`) or `sudo`. |
| Windows | Windows 10 version 1809 or later. The updater uses the included `curl` and `tar` tools. |

## Application updates

Pitex checks GitHub Releases for a release tag with a higher version.
It downloads the asset for the current platform and installs the update.

1. Select **Settings → Updates → Check for Updates**.
2. If an update is available, select **Install Update**.

The update procedure depends on the installation method:

| Installation | Update procedure |
|---|---|
| macOS, Homebrew | Updates the tap with `brew update`. Runs `brew upgrade --cask pitex`. Checks the installed application version before restart. |
| macOS, DMG | Prepares the new bundle before replacement of `/Applications/Pitex.app`. Then restarts Pitex. Without administrator access, opens the DMG for manual installation. |
| Linux | Runs `pkexec apt install`. If necessary, opens a terminal for `sudo apt install`. Checks the package version before a restart request. |
| Windows | Runs the per-user `*-setup.exe` installer after Pitex closes, then restarts the app. Portable ZIP installations migrate to the installer location. |

A stale Homebrew cask or upgrade with errors causes an error message.
You can try the update again.
Complete a Linux installation in its external terminal before restart.

The Windows updater uses the `*-setup.exe` asset.
It waits for Pitex to close, runs the installer with `/S`, and starts `pitex.exe` again.
A portable ZIP installation moves to `%LOCALAPPDATA%\Programs\Pitex`.
The helpers wait for process exit and check installation errors before restart.

**Automatically download and install updates** starts the same update procedure at each application start.
All platforms store this preference as `pitex.pref.update.autoInstall`.

Updates need network access to `api.github.com` and the release CDN.
They also need `curl`, which macOS, Ubuntu, and Windows 10 or later include.
Linux package installation needs polkit or `sudo`.
