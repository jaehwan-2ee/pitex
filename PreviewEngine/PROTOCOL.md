# pitex-preview protocol

`pitex-preview` is the embedded editing-preview session helper. Pitex starts
one per workspace preview session and talks to it over stdio with one JSON
object per line (UTF-8, `\n`-terminated). It never writes into the project
tree: TeX's `.xdv/.aux/.log/.synctex` live in the helper's memory, and
preview artifacts go to the session directory given with `--out`.

## Launch

```
pitex-preview --root <abs project root> --main <main .tex, relative to root>
              --out <abs session dir, created by the client, mode 0700>
              [--engine <abs pitex-preview-xetex>] [--cache <abs dir>]
```

The helper canonicalizes `--root`/`--main` with `realpath`. `files[].path`
and `closed[]` may send non-canonical absolute paths too (e.g. under a
symlinked project root) — the helper resolves them the same way.

- Start it in its own process group (`posix_spawn` `POSIX_SPAWN_SETPGROUP`,
  Rust `CommandExt::process_group(0)`). Engine checkpoints are forked
  processes in that group; on session end or crash, `kill(-pgid, SIGKILL)`
  removes every one of them.
- `stdin`/`stdout` are pipes; `stderr` may be `/dev/null` (diagnostics go to
  `<out>/driver.log`, truncated at 8 MiB).
- `PATH` must contain the TeX distribution (`kpsewhich`), like final builds.
- `--engine` defaults to `pitex-preview-xetex` next to the helper. `--cache`
  holds the engine's own generated format (per user, e.g.
  `~/.cache/pitex/preview-engine` / `~/Library/Caches/Pitex/preview-engine`).
- Exit on stdin EOF or `{"op":"quit"}`. Exit status 2 with an `error` event
  on bad arguments, missing engine or missing TeX distribution.

- Format generation takes an exclusive lock on the cache (`format/`): two
  helpers starting concurrently serialize on the first run, loser waits.
  Normal launches don't block.

## Requests (client → helper)

```json
{"op":"update","generation":7,
 "files":[{"path":"/abs/project/chapter1.tex","text":"...unsaved buffer..."}],
 "closed":["/abs/project/intro.tex"]}
```

- `generation`: client counter, strictly increasing. Send the newest state
  as soon as it is coalesced — **never wait for a previous generation to
  finish**; the helper rolls back to the nearest checkpoint (killing a
  looping or long-running engine process) and continues.
- `files`: full text of every open, modified TeX-like buffer (`.tex .sty .cls
  .bib .bbl .def .clo ...`) of the project, unsaved content included.
- `closed`: paths previously sent in `files` that are no longer overridden
  (buffer closed, saved or reverted); the helper falls back to disk.
- Every update also re-stats files TeX read from disk, so an empty update
  after a save or figure regeneration is meaningful.

```json
{"op":"release","seq":12}
```

Returns ownership of publication `seq`; the helper deletes its directory.
Release a publication once it is neither displayed nor bound for SyncTeX,
and immediately when it is rejected as stale.

```json
{"op":"quit"}
```

## Events (helper → client)

- `{"event":"ready"}` — once, after startup checks.
- `published`:

```json
{"event":"published","seq":12,"generation":7,"complete":false,"coherent":false,
 "pages":40,"current_pages":3,"errors":0,"elapsed_ms":85,"pdf_ms":20,
 "dir":"<out>/p12","pdf":"<out>/p12/main.pdf",
 "synctex":"<out>/p12/main.synctex","log":"<out>/p12/main.log",
 "first_error":"Undefined control sequence. (line 12)","warnings":"..."}
```

  - Display rule: accept a publication when the session and its build
    source are still current, `seq` is newer than the displayed one,
    `generation` is not older than the displayed generation, and the
    publication carries a source edit newer than the last successful manual
    final build. Track that with a client-side *edit revision* (bumped only
    by real text edits, queued ones included) recorded per sent generation:
    at manual build start remember the current edit revision, on success
    make it the floor. Transport generations also advance for save-only,
    close and rescan updates, so they must never unlock display over the
    final PDF; an edit made after the build started (even while it runs)
    does, and a displayed preview of such an edit stays on screen when the
    build succeeds. A publication may lag the newest sent generation — show
    it and mark the preview as updating. Reject a publication whose
    generation was never sent; a generation evicted from the client's
    history counts as carrying no edit (revision 0), so it can never stay
    over a newer final PDF. When the publication for the newest sent
    generation is rejected by the floor, the final PDF on screen is current:
    clear the "updating" status. Floors belong to a context: reset them when
    the workspace, build target or pin changes, but keep a pending manual
    floor across helper restarts.
  - `complete:false` is a valid intermediate snapshot. When `coherent` is
    false, pages `[0, current_pages)` reflect `generation` and the rest come
    from the previous finished pass (possibly stale). Show it — long
    documents must not wait for the full pass.
  - `complete:true`: the pass finished and auxiliary files converged (or the
    rerun budget was used).
  - Artifacts are written to a temporary directory and renamed into place,
    so the files are complete when the event arrives. `synctex` is standard
    uncompressed SyncTeX with absolute `Input:` paths and is present only
    for coherent publications (all pages from this pass); for others keep
    SyncTeX disabled until a coherent one arrives.
  - SyncTeX binding for a preview additionally requires that the editor
    sources still equal the identity compiled for `generation`: a map of
    absolute path → content hash over all open TeX buffers, modified ones
    as sent and clean ones (equal to the disk text the helper reads). A
    save keeps that identity, so it does not drop the binding. Bind only on
    a match and refuse forward/inverse queries once a buffer diverges,
    until a newer matching publication (or `idle`) is bound.
  - `errors`/`first_error`: TeX errors in this pass (nonstop mode); the PDF
    is still valid.
  - The helper keeps at most **4 unreleased publications**. When the limit
    is reached it skips intermediate snapshots and defers the newest
    complete one until a `release` arrives, so a client that stops
    releasing only stops receiving publications.
- `{"event":"failed","generation":7,"code":"no_pages|crashed|stuck",
  "errors":1,"message":"...","log_tail":"..."}` — nothing publishable; keep
  the last good preview and show the message.
- `{"event":"idle","generation":7,"seq":12}` — the update changed nothing the
  engine observed; complete publication `seq` is current for `generation`
  (0 = none: no publication yet, the last pass failed, or it was partial).
  The client advances the displayed publication's generation when `seq`
  matches and may bind SyncTeX against that generation's identity.
- `{"event":"error","code":"protocol|pdf|io|usage|no_engine|no_tex",
  "message":"..."}`.

Unknown event names and fields must be ignored by clients.
