# Projects through SSH

[All features](../../README.md) · [Main README](../../../README.md)

Pitex can open a project on a different Mac, Linux computer, or SSH server.
The editor runs on your computer.

1. Configure SSH key or agent authentication.
2. Check that the connection works without a password prompt.
3. Select **File → Open via SSH**.
4. Select a host from `~/.ssh/config`.
5. Browse to the project folder.

To add a host, use **Settings → SSH**.

Pitex downloads the project to a local mirror when you open it.
Text entry and the embedded editing preview use this copy.
The preview uses your computer's TeX files and fonts.
The remote device can have different TeX files and fonts.
Use the remote build result to check the final PDF.

On macOS, **Compiler compatibility preview** can compile the local mirror automatically.
It uses the TeX tools installed on the Mac.
This preview saves edited files only in the local mirror.
It does not upload changes or start remote builds.
See [Live compile](../live-compile/README.md#compiler-compatibility-preview).

**Auto Save is not available for SSH projects.**
Select **Save** or **Save All** to save your changes, upload them, and build on the remote device.
The **Build** command also saves and uploads your changes before a remote build.
Pitex downloads the PDF, SyncTeX data, and build log for the local preview.
Text entry, local preview updates, assistant work, and Git status checks do not upload changes.

The Pi assistant reads and edits the local mirror.
Its process and shell tools run on your computer and use your local environment.
Use **Save** after assistant work to send its changes to the remote device.
Git commands run on the remote device and use the files saved there.
Save your local edits before staging or committing them.
Opening Git does not create a repository.
Use the explicit initialization button to start tracking a folder.

Before you close an SSH project or quit, Pitex saves all edited open files and uploads the final changes.
If the save or upload fails, the project stays open so you can correct the problem.
If a file changes on the two devices, Pitex keeps the changes for your review.
Changes that have not been uploaded stay in the local mirror.
The window shows the device and transfer status.
When the window becomes active again, Pitex checks for remote changes.

Use SSH projects for personal work.
Make changes through one SSH connection at a time.
Concurrent changes can cause file conflicts.
Do not edit the same project from different devices at the same time.
For concurrent work, use a separate remote Git worktree and branch for Pitex.
See the [Git and SSH coordination design](GIT_WORKFLOW.md) for setup,
current limits, and the proposed save and Git transaction rules.

Files with the same name show their parent folder in the Project sidebar and editor tabs.
Pitex uses the selected file's path to find its bibliography and PDF.

The mirror excludes version-control and dependency folders, such as `.git`, `node_modules`, and `.venv`.
It also excludes files of 50 MB or more.
The assistant uses the local mirror folder.
