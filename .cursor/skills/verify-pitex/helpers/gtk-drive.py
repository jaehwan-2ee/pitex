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

MENU_Y = {"codex": 248, "claude": 284, "grok": 320, "agy": 356}


def main():
    provider = os.environ["PITEX_GTK_PROVIDER"]
    evidence = Path(os.environ["PITEX_GTK_EVIDENCE"])
    project = Path(os.environ["PITEX_GTK_PROJECT"])
    binary = os.environ["PITEX_BIN"]
    evidence.mkdir(parents=True, exist_ok=True)
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
        time.sleep(1.4)
        shot(evidence / "04-sent.png")
        click(470, 716)
        time.sleep(1.0)
        shot(evidence / "05-approved.png")
        prefs = Path(os.environ["HOME"]) / ".config" / "pitex" / "preferences.json"
        saved = ""
        if prefs.exists():
            saved = prefs.read_text()
            (evidence / "preferences.json").write_text(saved)
        record = Path(os.environ["FAKE_RECORD"]).read_text() if Path(os.environ["FAKE_RECORD"]).exists() else ""
        result["providerSaved"] = f'"ai.agentProvider": "{provider}"' in saved
        result["promptReachedCli"] = "Fix the title" in record
        result["approvalReachedCli"] = 'decision\\":\\"accept' in record or '"decision":"accept"' in record
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
