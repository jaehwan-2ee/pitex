# Git and SSH save coordination

[SSH projects](README.md) · [Architecture](../../architecture/README.md)

This document defines the proposed conflict handling for SSH projects.
The transaction protocol and worktree setup below are a development design.
They are not enabled by the current app.

## Current behavior

The remote project is the Git working tree.
The local mirror has source files, a file hash manifest, and editor changes.
It excludes `.git`. Pitex runs SSH Git commands on the remote device.
Status checks and Git commands do not upload editor changes.
Manual Save, Save All, Build, and a successful close upload saved sources.

The manifest stores the last accepted SHA-256 hash for each file.
A download compares the baseline, local file, and remote file.
An upload checks the remote file against its baseline before replacement.
Changes on both sides become a file conflict.
The app preserves local changes when a download finds a conflict.

These checks do not record a Git branch or commit with each baseline.
The upload lock orders cooperating Pitex uploads. An external `git switch`,
`pull`, editor, or build does not acquire that lock.
Uploads publish files separately. A commit between two file replacements
can therefore record an incomplete set of changes.
Per-file hash checks alone cannot make a shared Git working tree a transaction.

The implementation points are `RemoteSync` in Swift and Rust,
`saveRemoteSources` on macOS, and `GitRunner` in both app frontends.
Individual app Git commands now use the mirror's transfer queue.
The same shared `RemoteSync` engine waits for a Git request to finish before
starting a transfer, and waits for a transfer before starting Git.
A different mirror or app process has a separate queue.
The queue does not yet cover a complete multi-command Git transaction.

## Preferred setup: a separate remote worktree

Give each Pitex workspace a dedicated remote worktree and branch.
Keep an interactive SSH session in the original working tree.
This separates the source files, index, and checked-out HEAD used by each editor.
Objects and ordinary refs remain shared, as described in the
[Git worktree manual](https://git-scm.com/docs/git-worktree).

For example, run these commands on the remote host:

```sh
git -C /srv/paper worktree add -b pitex/paper-session /srv/paper-pitex HEAD
```

Then open `/srv/paper-pitex` through SSH in Pitex.
Use a unique path and branch for each session.
Do not edit or switch branches in that worktree from another process.
Commit saved changes on its branch. Review and merge that branch in the main
working tree when ready. A separate worktree prevents file replacement races
with the main worktree; it does not remove normal Git merge conflicts.

A future **Create Pitex worktree** action must show the path and branch before
creation. Connecting, opening Git, or refreshing status must never create a
repository, branch, or worktree. The action must preserve existing dirty work
and refuse existing destinations or branch names. It must not use `--force`.
Do not remove a worktree on disconnect. Remove it only through an explicit
action after checking local pending edits and remote uncommitted changes.

`git worktree lock` protects worktree metadata from pruning. It is not a mutex
for editors, uploads, or Git commands.

## Shared working tree protocol

Support the existing shared folder as a separate mode. It needs the following
coordination in addition to current file conflict checks.

### Workspace identity and baseline

Identify a workspace by SSH destination, user, port, resolved remote project
path, resolved Git worktree path, and Git common directory. Keep the project
path distinct from the repository root: a project can be a subfolder.

Record the symbolic branch, full HEAD object ID, and a remote generation token
alongside each accepted file manifest. Record an explicit unborn HEAD state.
Retain the actual accepted file bytes in a cache keyed by content hash for
three-way review. The baseline can contain uncommitted remote edits, so a HEAD
blob is not a substitute. Keep this cache outside the mirrored project.
Keep the current index fingerprint and pending merge/rebase state in each
operation snapshot. Handle linked worktrees through Git's reported paths;
`.git` can be a file. Never infer the repository from the mirror's ancestors.

Persist each operation's target identity and expected generation before it
starts. A late response must not update a different project, connection, or
new instance of a reopened workspace.

### One operation queue

Use one queue per remote worktree across windows for Save, Save All, remote
Build, sync, conflict resolution, and mutating Git commands. Hold the queue for
the whole operation, including all commands and its final refresh.
Linux and Windows can open another window in a separate app process. Use a
process-shared local lock for the mirror's manifest and staging files, reload
the manifest after taking that lock, and coordinate remote operations across
all processes through the remote worktree lock. Two different mirrors for the
same remote worktree must also use that remote lock.
A local editor save can continue while network work runs. Pin the uploaded
source snapshot and leave subsequent edits pending for the next manual save.

On the remote device, acquire one shared project operation lock for the whole
upload or mutating Git transaction. Use a unique owner token, timeout, and
verified stale-owner recovery. An interactive SSH command can participate
through a wrapper that acquires the same lock.

The lock must live outside the tracked project, with a key derived from the
resolved worktree identity. Avoid creating tracked lock files or downloading
them into the mirror. Keep Git's own `index.lock` and ref locks intact.

Plain external Git commands do not honor this lock. Check identity and HEAD
again before publishing and before accepting results. If the branch or HEAD
changed, stop the upload, download the new remote snapshot, and keep pending
local edits for review. These checks detect changes but cannot eliminate the
last race with an uncooperative writer. Use a dedicated worktree for that case.

After a lost SSH connection, treat a mutating operation as having an unknown
outcome. Closing the local SSH process does not prove that the remote command
or its hooks stopped. Reconcile the remote lock, HEAD, index, and published
files before accepting another mutation. Persist the operation token so an
app restart can complete this check.

### Rules by action

| Action | Upload local edits? | Required handling |
| --- | --- | --- |
| Status, branch list, history, diff | No | Read remote saved state. Discard results from an old workspace or generation. |
| Stage / unstage | No | Serialize with saves. Compare the file and index snapshot shown to the user. Refresh if either changed. |
| Commit | No | Commit the remote index. Refuse a changed HEAD/index snapshot or unresolved upload conflict. Show that pending local edits are not included. Download source changes made by commit hooks. |
| Push | No | Push committed remote refs. Use normal non-force behavior and surface a rejected push. |
| Save / Save All | Yes | Pin local revisions, validate HEAD and file baselines, upload, verify, then update the baseline. |
| Remote Build | Yes | Complete the save transaction, then compile the verified snapshot while source replacement is blocked. |
| Switch branch, pull, sync, create-and-switch branch, discard | No | Block while local revisions or saved mirror files are pending. Run Git, then download and adopt the new remote state. |
| Fetch | No | Refresh refs and invalidate history caches. It does not need a source mirror download. |
| Initialize repository | No | Run only from the explicit initialization button. Refresh repository identity afterward. |

Before a worktree-changing action, flush pending editor notifications without
uploading. Check dirty document revisions, mirror files against the manifest,
and unresolved file conflicts. Include assistant edits and files not open in
an editor. If any are pending, offer **Save first**, **Cancel**, or an explicit
local snapshot export. Do not auto-stash, discard, or upload them.
Recheck after acquiring the queue because a user can type during the wait.

A failed pull or switch can still change files, create merge conflicts, or
change the index. Always refresh remote Git state after the attempt. Download
changed sources without replacing pending local files. Keep Git conflicts
distinct from mirror upload conflicts: each needs its own resolution.

### External Git activity

Poll remote status while the Git pane is visible. Check HEAD before every
manual save even when the pane is hidden. On focus, check the remote snapshot.
If the remote branch changed while local edits are pending, pause upload and
show both the old and new branch/commit. The user must review the local edits
against the new remote state before accepting a new baseline.

If only remote files changed, download them into clean local files and reload
clean editors. If both copies changed, show a three-way comparison: baseline,
local, remote. Choosing the local version must first accept the current remote
hash as a new expectation; the next manual save still performs its own check.
Keep deleted and renamed files visible in that review.

Never use `reset --hard`, `clean -fd`, a forced checkout, or force push as an
automatic conflict recovery step. Do not silently commit partially uploaded
sources. If a save publishes some files and then fails, report the exact paths
and keep the transaction incomplete until verification or explicit review.

## Implementation stages and acceptance checks

1. Add the shared app operation queue and repository identity to both cores.
   Test simultaneous Save/Stage/Commit, two windows, two separate app processes,
   two mirrors of one remote worktree, and project reopen while an SSH response
   is pending. A Git command must not split an upload.
2. Add HEAD-aware manifests and the remote transaction wrapper. Migrate an old
   manifest with an unknown Git identity; retain its hashes and pending edits.
   Read the current remote snapshot, then compare and review pending changes
   before approving a new baseline. Do not bind old hashes to the current HEAD
   automatically. Test detached and unborn HEAD, subfolder projects, linked
   worktrees, a stale lock, a dropped connection, and partial upload recovery.
3. Add the dirty-mirror guard and baseline/local/remote conflict review. Test
   dirty open buffers, local preview writes, assistant changes, renames,
   deletions, staged files, merge conflicts, and branch changes during upload.
4. Add the explicit worktree setup action. Test that a checkout or pull in the
   original tree leaves Pitex's files/index/HEAD unchanged. Test normal merge
   conflicts when the session branch is later integrated.

Use a real local SSH fixture for the transaction tests, with a controllable
pause before publication. Test an external cooperating wrapper and a plain
Git command separately. Report the remaining plain-writer race accurately.
Run the same scenarios on macOS, Ubuntu 22.04, Ubuntu 24.04, and Windows.
