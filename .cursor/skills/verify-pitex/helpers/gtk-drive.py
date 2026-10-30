#!/usr/bin/env python3
import json
import os
import subprocess
import time
from pathlib import Path

runtime = os.environ.get("XDG_RUNTIME_DIR")
if runtime:
    os.makedirs(runtime, mode=0o700, exist_ok=True)
    os.chmod(runtime, 0o700)

MENU_Y = {"codex": 248, "claude": 284, "grok": 320}


def main():
    provider = os.environ["PITEX_GTK_PROVIDER"]
    evidence = Path(os.environ["PITEX_GTK_EVIDENCE"])
    project = Path(os.environ["PITEX_GTK_PROJECT"])
    binary = os.environ["PITEX_BIN"]
    evidence.mkdir(parents=True, exist_ok=True)
    if provider == "stale-agy":
        seed_stale_provider()
    tex = project / "main.tex"
    tex.write_text("\\documentclass{article}\\begin{document}Title\\end{document}\n")
    log = open(evidence / "pitex.log", "w")
    child = subprocess.Popen([binary, str(tex)], stdout=log, stderr=subprocess.STDOUT)
    (evidence / "pids.json").write_text(json.dumps([child.pid]) + "\n")
    result = {"provider": provider, "pid": child.pid, "pass": False, "steps": []}
    try:
        window = wait_window("^project$")
        if not window:
            result["steps"].append("no-project-window")
            write(evidence, result)
            return 1
        result["steps"].append("project-window")
        shot(evidence / "01-project.png")
        run(["xdotool", "key", "--window", window, "ctrl+comma"])
        settings = wait_window("^Settings$")
        if not settings:
            result["steps"].append("no-settings-window")
            write(evidence, result)
            return 1
        click_window(settings, 448, 518)
        time.sleep(0.3)
        click_window(settings, 620, 128)
        time.sleep(0.3)
        shot(evidence / "02-menu.png")
        result["steps"].append("menu-open")
        if provider == "stale-agy":
            result["storedAtMenu"] = stored_provider(preferences_path().read_text() if preferences_path().exists() else "")
            run(["xdotool", "key", "--window", settings, "Escape"])
            time.sleep(0.3)
        else:
            click_window(settings, 520, MENU_Y[provider])
            time.sleep(0.4)
            shot(evidence / "02-provider.png")
        click_window(settings, 690, 22)
        time.sleep(0.4)
        click(1048, 36)
        time.sleep(0.4)
        shot(evidence / "03-assistant.png")
        click(560, 752)
        time.sleep(0.15)
        run(["xdotool", "type", "--delay", "12", "Fix the title"])
        time.sleep(0.15)
        run(["xdotool", "key", "Return"])
        time.sleep(2.5)
        shot(evidence / "04-sent.png")
        if provider != "stale-agy":
            click(470, 716)
            time.sleep(1.2)
            shot(evidence / "05-approved.png")
        prefs = preferences_path()
        saved = ""
        if prefs.exists():
            saved = prefs.read_text()
            (evidence / "preferences.json").write_text(saved)
        record_path = Path(os.environ["FAKE_RECORD"]) if os.environ.get("FAKE_RECORD") else None
        record = record_path.read_text() if record_path and record_path.exists() else ""
        result["promptReachedCli"] = "Fix the title" in record
        if provider == "stale-agy":
            bridge_commands = cli_descendants(child.pid)
            result["bridgeStarted"] = len(bridge_commands) > 0
            result["bridgeCommands"] = bridge_commands
            result["storedProvider"] = stored_provider(saved)
            sent_text = screen_text(evidence / "04-sent.png")
            result["sentText"] = sent_text[:500]
            result["assistantVisible"] = assistant_visible(sent_text)
            result["searchIntercepted"] = "No Results Found" in sent_text
            result["pass"] = (
                result.get("storedAtMenu") == "agy"
                and result["storedProvider"] in ("agy", "pi")
                and not result["promptReachedCli"]
                and not result["bridgeStarted"]
                and result["assistantVisible"]
                and not result["searchIntercepted"]
            )
        else:
            result["providerSaved"] = f'"ai.agentProvider": "{provider}"' in saved
            result["approvalReachedCli"] = approval_reached(provider, record)
            result["pass"] = result["providerSaved"] and result["promptReachedCli"] and result["approvalReachedCli"]
        result["steps"].append("sent")
        write(evidence, result)
        return 0 if result["pass"] else 1
    finally:
        child.terminate()
        try:
            child.wait(timeout=3)
        except subprocess.TimeoutExpired:
            child.kill()
        log.close()


def seed_stale_provider():
    config = Path(os.environ["XDG_CONFIG_HOME"]) / "pitex"
    config.mkdir(parents=True, exist_ok=True)
    (config / "preferences.json").write_text('{"ai.agentProvider":"agy"}')


def preferences_path():
    config_home = os.environ.get("XDG_CONFIG_HOME")
    if config_home:
        return Path(config_home) / "pitex" / "preferences.json"
    return Path(os.environ["HOME"]) / ".config" / "pitex" / "preferences.json"


def screen_text(path):
    if not path.exists():
        return ""
    proc = subprocess.run(["tesseract", str(path), "stdout"], capture_output=True, text=True)
    return proc.stdout or ""


def assistant_visible(text):
    folded = text.lower()
    return "ask about your document" in folded or "ask anything" in folded or "not installed" in folded


def stored_provider(saved):
    try:
        data = json.loads(saved)
    except json.JSONDecodeError:
        return None
    if not isinstance(data, dict):
        return None
    value = data.get("ai.agentProvider")
    return value if isinstance(value, str) else None


def read_cmdline(pid):
    try:
        raw = Path(f"/proc/{pid}/cmdline").read_bytes()
    except OSError:
        return ""
    return raw.replace(b"\0", b" ").decode("utf-8", "replace")


def child_pids(pid):
    try:
        text = Path(f"/proc/{pid}/task/{pid}/children").read_text()
    except OSError:
        return []
    found = []
    for part in text.split():
        try:
            found.append(int(part))
        except ValueError:
            continue
    return found


def cli_descendants(pid):
    found = []
    stack = child_pids(pid)
    seen = set()
    while stack:
        current = stack.pop()
        if current in seen:
            continue
        seen.add(current)
        cmdline = read_cmdline(current)
        if "cli.ts" in cmdline:
            found.append(cmdline)
        stack.extend(child_pids(current))
    return found


def write(evidence, result):
    (evidence / "gtk-result.json").write_text(json.dumps(result, indent=2) + "\n")


def run(argv):
    subprocess.run(argv, check=False)


def click(x, y):
    run(["xdotool", "mousemove", str(x), str(y), "click", "1"])


def click_window(window, x, y):
    run(["xdotool", "mousemove", "--window", window, str(x), str(y), "click", "1"])


def shot(path):
    run(["import", "-window", "root", str(path)])


def approval_reached(provider, record):
    if provider == "claude":
        return "user_temporary" in record and "allow" in record
    if provider == "grok":
        return "allow-once" in record
    return 'decision\\":\\"accept' in record or '"decision":"accept"' in record


def wait_window(pattern):
    for _ in range(25):
        found = subprocess.run(["xdotool", "search", "--name", pattern], capture_output=True, text=True)
        lines = [line for line in found.stdout.splitlines() if line.strip()]
        if lines:
            return lines[0]
        time.sleep(0.3)
    return ""


if __name__ == "__main__":
    raise SystemExit(main())
