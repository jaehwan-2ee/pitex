#!/usr/bin/env python3
"""Offline scheduling benchmark: real Swift/Rust schedulers, virtual milliseconds.

Compiler service times are prescribed inputs, NOT measured TeX/PDF render times.
Every edit waits for the first accepted completion covering its source revision.
This penalizes starvation rather than timing only the final keystroke of a burst.
"""
from contextlib import ExitStack
from pathlib import Path
import math
import os
import selectors
import subprocess
import tempfile

REPO = Path(__file__).resolve().parent.parent
# Fixed host model: 150ms overdue/IME timer retry; 50ms cancellation acknowledgement.
# ponytail: virtual compiler/host timing isolates policy; use a separate GUI benchmark
# before claiming faster typesetting, less flicker, or preserved viewport geometry.
RETRY_MS = 150
CANCEL_MS = 50


def command(processes, *args):
    message = " ".join(map(str, args)) + "\n"
    replies = []
    for process in processes:
        process.stdin.write(message)
        process.stdin.flush()
        with selectors.DefaultSelector() as selector:
            selector.register(process.stdout, selectors.EVENT_READ)
            assert selector.select(timeout=5), f"scheduler hung: {message.strip()}"
        reply = process.stdout.readline().strip()
        assert reply, f"scheduler exited during {message.strip()}"
        replies.append(reply)
    assert replies[0] == replies[1], f"Swift/Rust divergence after {message.strip()}: {replies}"
    deadline, active, accepts, status, requests = replies[0].split("|")
    return {
        "deadline": None if deadline == "-" else int(deadline),
        "active": None if active == "-" else int(active),
        "accepts": accepts == "1",
        "status": status,
        "requests": [(kind, int(token)) for kind, token in
                     (request.split(":") for request in requests.split(",") if request)],
    }


def safety_checks(processes):
    """Consumer-visible ownership/generation guards, not exact debounce assertions."""
    call = lambda *args: command(processes, *args)
    call("reset", 700)
    call("edit", 0)
    assert not call("poll", 20_000, 1)["requests"], "built during IME composition"
    first = call("poll", 20_000, 0)["active"]
    assert first is not None
    cancelled = call("manual")
    assert cancelled["requests"] == [("cancel", first)]
    assert not cancelled["accepts"], "cancelled build still accepts streaming events"
    call("enable", 0)
    finished = call("finish", first, 20_050, 0)
    assert finished["status"] == "superseded"
    manual = finished["active"]
    assert finished["requests"] == [("manual", manual)], "toggle lost queued manual build"
    call("enable", 1)
    assert not call("edit", 20_060)["requests"], "typing interrupted manual build"
    duplicate = call("finish", first, 20_070, 0)
    assert duplicate["status"] == "stale" and duplicate["active"] == manual
    assert not duplicate["requests"]
    finished = call("finish", manual, 40_000, 0)
    assert finished["status"] == "current"
    live = finished["active"]
    assert finished["requests"] == [("live", live)], "pending edit lost behind manual build"
    call("invalidate")
    retired = call("finish", live, 40_050, 0)
    assert retired["status"] == "superseded" and retired["active"] is None
    assert retired["deadline"] is None and not retired["requests"]
    call("enable", 0)
    assert not call("edit", 40_100)["requests"]
    disabled = call("poll", 60_000, 0)
    assert disabled["deadline"] is None and not disabled["requests"]


def scenario(processes, name, events, compile_ms, delay_ms=700):
    state = command(processes, "reset", delay_ms)
    now = 0
    composing = False
    active = None
    timer = None
    cursor = 0
    edits = []
    published = 0
    latencies = []
    starts = cancels = publishes = busy_ms = wasted_ms = 0

    def apply(reply):
        nonlocal state, active, starts, cancels
        state = reply
        for kind, token in reply["requests"]:
            if kind == "cancel":
                assert active and active["token"] == token, f"{name}: cancelled wrong run"
                assert not active["cancelled"], f"{name}: duplicate cancellation"
                active["cancelled"] = True
                active["end"] = min(active["end"], now + CANCEL_MS)
                cancels += 1
            else:
                assert kind == "live", f"{name}: unexpected manual build"
                assert active is None, f"{name}: overlapping builds"
                assert not composing, f"{name}: built uncommitted IME text"
                active = dict(token=token, revision=len(edits), start=now,
                              end=now + compile_ms, cancelled=False)
                starts += 1
        assert reply["active"] == (active["token"] if active else None)

    # Equal timestamps: source/IME events, then completion, then timer. This makes
    # the edit-at-completion race deterministic and exercises stale-result guards.
    for _ in range(100_000):
        candidates = []
        if cursor < len(events):
            candidates.append(events[cursor][0])
        if active:
            candidates.append(active["end"])
        if timer is not None:
            candidates.append(timer)
        if not candidates:
            break
        now = min(candidates)
        assert now <= events[-1][0] + 30_000, f"{name}: scheduler failed to drain"
        while cursor < len(events) and events[cursor][0] == now:
            _, kind, value = events[cursor]
            cursor += 1
            if kind == "compose":
                composing = value
            else:
                assert kind == "edit"
                edits.append(now)
                apply(command(processes, "edit", now))
        if active and active["end"] == now:
            run, active = active, None
            elapsed = now - run["start"]
            busy_ms += elapsed
            result = command(processes, "finish", run["token"], now, int(composing))
            assert result["status"] != "stale", f"{name}: lost active completion"
            if result["status"] == "current":
                assert not run["cancelled"], f"{name}: cancelled result published"
                assert published < run["revision"] <= len(edits), f"{name}: revision regressed"
                latencies.extend(now - edited for edited in edits[published:run["revision"]])
                published = run["revision"]
                publishes += 1
            else:
                wasted_ms += elapsed
            apply(result)
        if timer is not None and timer <= now:
            apply(command(processes, "poll", now, int(composing)))
        deadline = state["deadline"]
        timer = None if deadline is None else (deadline if deadline > now else now + RETRY_MS)
    else:
        raise AssertionError(f"{name}: event loop did not converge")
    assert published == len(edits) and edits, f"{name}: final source revision never published"
    assert len(latencies) == len(edits) and state["active"] is None
    assert starts <= len(edits), f"{name}: self-triggering build loop"
    ordered = sorted(latencies)
    p95 = ordered[math.ceil(len(ordered) * 0.95) - 1]
    mean = sum(latencies) / len(latencies)
    print(f"CASE {name} edits={len(edits)} mean_ms={mean:.3f} p95_ms={p95} "
          f"builds={starts} cancels={cancels} publishes={publishes} "
          f"busy_ms={busy_ms} wasted_ms={wasted_ms}")
    return mean, latencies, starts, cancels, publishes, busy_ms, wasted_ms


def run(processes):
    safety_checks(processes)
    edit_events = lambda times: [(time, "edit", None) for time in times]
    continuous = edit_events(range(0, 12_000, 120))
    bursts = edit_events(start + offset for start in range(0, 18_000, 1_800)
                         for offset in (0, 90, 180, 270))
    hesitant = edit_events(range(0, 13_600, 850))
    ime = []
    for start in (0, 2400, 4800, 7200):
        ime.append((start, "compose", True))
        ime.extend(edit_events(start + offset for offset in (0, 80, 160)))
        ime.append((start + 1600, "compose", False))
    results = []
    for compile_ms in (120, 600, 1800):
        for name, events in (("continuous", continuous), ("bursts", bursts),
                             ("hesitant", hesitant), ("ime", ime)):
            results.append(scenario(processes, f"{name}-{compile_ms}", events, compile_ms))
    results.append(scenario(processes, "remote-bursts", bursts, 1600, delay_ms=1500))
    samples = sorted(value for result in results for value in result[1])
    # Equal weighting of scenarios prevents long traces drowning out IME/remote.
    metrics = {
        "scheduler_edit_latency_ms": sum(result[0] for result in results) / len(results),
        "scheduler_edit_p95_ms": samples[math.ceil(len(samples) * 0.95) - 1],
        "scheduler_edit_max_ms": max(samples),
        "build_starts": sum(result[2] for result in results),
        "build_cancels": sum(result[3] for result in results),
        "accepted_results": sum(result[4] for result in results),
        "compiler_busy_ms": sum(result[5] for result in results),
        "compiler_wasted_ms": sum(result[6] for result in results),
    }
    print(f"PASS Swift/Rust parity, ownership/IME/stale guards, {len(results)} traces, "
          f"{len(samples)} edits accounted for")
    return metrics


def main():
    # No package managers/network. Build both production implementations directly;
    # compilation time and IPC overhead are deliberately excluded from the score.
    env = dict(os.environ, LC_ALL="C", TZ="UTC")
    with tempfile.TemporaryDirectory(prefix="pitex-live-benchmark-") as directory:
        root = Path(directory)
        rust = root / "rust-driver"
        swift = root / "swift-driver"
        subprocess.run(["rustc", "--edition=2021", "-O", str(REPO / "Tools/live-compile-driver.rs"),
                        "-o", str(rust)], check=True, env=env, timeout=120)
        subprocess.run(["swiftc", "-O", "-parse-as-library",
                        str(REPO / "Packages/TexApp/Sources/BuildFeature/LiveCompileScheduler.swift"),
                        str(REPO / "Tools/live-compile-driver.swift"), "-o", str(swift)],
                       check=True, env=env, timeout=120)
        with ExitStack() as stack:
            processes = [stack.enter_context(subprocess.Popen(
                [str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                text=True, bufsize=1, env=env)) for binary in (rust, swift)]
            try:
                metrics = run(processes)
            finally:
                for process in processes:
                    process.stdin.close()
                    try:
                        process.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        process.wait()
            assert all(process.returncode == 0 for process in processes), "scheduler driver failed"
        # Emit machine-readable results only after every workload/check succeeds.
        for name, value in metrics.items():
            assert math.isfinite(value) and value >= 0
            print(f"METRIC {name}={value:.3f}")


if __name__ == "__main__":
    main()
