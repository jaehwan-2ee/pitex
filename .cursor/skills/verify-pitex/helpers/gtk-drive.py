#!/usr/bin/env python3
import json
import os
import shutil
import subprocess
import time
from pathlib import Path

runtime = os.environ.get("XDG_RUNTIME_DIR")
if runtime:
    os.makedirs(runtime, mode=0o700, exist_ok=True)
    os.chmod(runtime, 0o700)

MENU_Y = {"claude": 248, "claude-toolcards": 248, "claude-markdown": 248, "claude-approvals": 248, "claude-todos": 248}
# The fake Claude answers TOOLCARDS with a Read, an Edit and a failing Bash call.
TOOLCARDS = "claude-toolcards"
# MARKDOWN streams a Markdown answer (heading, list, code, math, table, links).
MARKDOWN = "claude-markdown"
# APPROVALS switches the mode picker to Plan, answers a question card, keeps
# planning once with feedback, then approves the revised plan with accept edits.
APPROVALS = "claude-approvals"
# TODOS gets three TodoWrite snapshots: the checklist above the composer and its activity rows.
TODOS = "claude-todos"
# The checklist's header row (expander) just above the composer once the divider is dragged up.
TODO_HEADER = (445, 728)
# The header's trash button (new chat).
NEW_CHAT = (1161, 227)
# The history button beside it, and the newest conversation in its list (below the search field).
HISTORY = (1203, 227)
HISTORY_FIRST_ROW = (1200, 335)
# The mode picker sits on the composer row, right of the message entry.
MODE_PICKER = (1188, 767)
# Approval card in the 1280x800 window once the divider is dragged up. Its buttons sit
# just above the composer: Submit / Approve plan, then Approve and accept edits, then Keep planning.
CARD_BUTTONS = [(476, 717), (664, 717), (837, 717)]
QUESTION_OTHER_ENTRY = (700, 569)
QUESTION_INTRO = (450, 660)
QUESTION_METHOD = (450, 688)
PLAN_FEEDBACK = (700, 677)
# Expander rows of those three cards in the 1280x800 window, top to bottom.
# The Read card's "Open file" button once all three cards are open.
OPEN_FILE_CLICK = (497, 397)
# Bottom card first, so expanding one never moves the next target.
CARD_CLICKS = [(435, 383), (435, 352), (435, 321)]
# The ```latex block's Copy button and the intro.tex:1 link once the answer is in.
MARKDOWN_COPY_CLICK = (507, 540)
MARKDOWN_FILE_LINK_CLICK = (503, 732)
STALE_STORED = {
    "stale-codex": "codex",
    "stale-grok": "grok",
}
# Selectable providers whose switch row has no measured coordinate yet. The
# preference is written before launch, and the run then clicks Approve like claude.
SEEDED = {"agy": "agy"}


def main():
    provider = os.environ["PITEX_GTK_PROVIDER"]
    evidence = Path(os.environ["PITEX_GTK_EVIDENCE"])
    project = Path(os.environ["PITEX_GTK_PROJECT"])
    binary = os.environ["PITEX_BIN"]
    evidence.mkdir(parents=True, exist_ok=True)
    stored = STALE_STORED.get(provider)
    seeded = SEEDED.get(provider)
    if stored or seeded:
        seed_stale_provider(stored or seeded)
    if os.environ.get("PITEX_GTK_THEME"):
        prefs = preferences_path()
        prefs.parent.mkdir(parents=True, exist_ok=True)
        data = json.loads(prefs.read_text()) if prefs.exists() else {}
        data["appearance.theme"] = os.environ["PITEX_GTK_THEME"]
        prefs.write_text(json.dumps(data, separators=(",", ":")))
    tex = project / "main.tex"
    tex.write_text("\\documentclass{article}\\begin{document}Title\\end{document}\n")
    if provider in (TOOLCARDS, MARKDOWN):
        (project / "intro.tex").write_text("\\section{Introduction}\n")
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
        if stored:
            result["storedAtMenu"] = stored_provider(preferences_path().read_text() if preferences_path().exists() else "")
            run(["xdotool", "key", "--window", settings, "Escape"])
            time.sleep(0.3)
        elif seeded:
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
        if provider in (TOOLCARDS, MARKDOWN, APPROVALS, TODOS):
            # Give the transcript room: drag the editor/console divider up.
            run(["xdotool", "mousemove", "840", "538", "mousedown", "1", "mousemove", "840", "400", "mousemove", "840", "160", "mouseup", "1"])
            time.sleep(0.4)
            click(560, 752)
            time.sleep(0.15)
        if provider == APPROVALS:
            return approvals(evidence, result)
        if provider == TODOS:
            return todo_list(evidence, result)
        prompt = {TOOLCARDS: "TOOLCARDS", MARKDOWN: "MARKDOWN"}.get(provider, "Fix the title")
        run(["xdotool", "type", "--delay", "12", prompt])
        time.sleep(0.15)
        run(["xdotool", "key", "Return"])
        if provider == MARKDOWN:
            return markdown(evidence, result)
        time.sleep(2.5)
        shot(evidence / "04-sent.png")
        if provider == TOOLCARDS:
            return tool_cards(evidence, result)
        if not stored:
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
        if stored:
            bridge_commands = cli_descendants(child.pid)
            result["bridgeStarted"] = len(bridge_commands) > 0
            result["bridgeCommands"] = bridge_commands
            result["storedProvider"] = stored_provider(saved)
            sent_text = screen_text(evidence / "04-sent.png")
            result["sentText"] = sent_text[:500]
            result["assistantVisible"] = assistant_visible(sent_text)
            result["searchIntercepted"] = "No Results Found" in sent_text
            result["pass"] = (
                result.get("storedAtMenu") == stored
                and result["storedProvider"] in (stored, "pi")
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


def tool_cards(evidence, result):
    collapsed = screen_text(evidence / "04-sent.png")
    for x, y in CARD_CLICKS:
        click(x, y)
        time.sleep(0.4)
    time.sleep(0.6)
    shot(evidence / "05-expanded.png")
    expanded = screen_text(evidence / "05-expanded.png")
    # "Open file" on the Read card activates intro.tex in the editor.
    click(*OPEN_FILE_CLICK)
    time.sleep(1.0)
    shot(evidence / "06-open-file.png")
    record_path = Path(os.environ["FAKE_RECORD"])
    result["promptReachedCli"] = "TOOLCARDS" in record_path.read_text()
    result["collapsedText"] = collapsed[-800:]
    result["expandedText"] = expanded[-1500:]
    result["titles"] = {
        "edited": "Edited main.tex" in collapsed,
        "ran": "Ran latexmk" in collapsed,
        "read": "Read intro.tex" in collapsed,
    }
    result["diffShown"] = "maketitle" in expanded
    # Without tesseract the shots are the evidence: look at 04 and 05.
    result["ocr"] = bool(shutil.which("tesseract"))
    checks = all(result["titles"].values()) and result["diffShown"] if result["ocr"] else True
    result["pass"] = result["promptReachedCli"] and checks
    result["steps"].append("cards")
    write(evidence, result)
    return 0 if result["pass"] else 1


def markdown(evidence, result):
    # The fake holds 4 s inside an open ```latex fence: it shows as an open code box.
    time.sleep(2.0)
    shot(evidence / "04-streaming.png")
    # Then the rest arrives; give pdflatex time to typeset the math.
    time.sleep(7.0)
    # The transcript may sit a row lower; wheel it to the top so the clicks below land.
    run(["xdotool", "mousemove", "840", "450"] + ["click", "4"] * 5)
    time.sleep(0.8)
    shot(evidence / "05-rendered.png")
    click(*MARKDOWN_COPY_CLICK)
    time.sleep(0.5)
    shot(evidence / "06-copied.png")
    # "Copied" goes back to "Copy" after 1.5 s.
    time.sleep(1.5)
    shot(evidence / "06-copy-reset.png")
    clip = subprocess.run(["xclip", "-o", "-selection", "clipboard"], capture_output=True, text=True, timeout=5)
    result["clipboard"] = clip.stdout
    result["copiedCode"] = clip.stdout.startswith("\\section{Intro}") and clip.stdout.rstrip().endswith("\\end{equation}")
    # The transcript keeps the message top in view; wheel down to the rest.
    for step in range(3):
        run(["xdotool", "mousemove", "840", "450"] + ["click", "5"] * 3)
        time.sleep(1.0)
        shot(evidence / f"07-scrolled-{step}.png")
    click(*MARKDOWN_FILE_LINK_CLICK)
    time.sleep(1.2)
    shot(evidence / "08-file-link.png")
    record_path = Path(os.environ["FAKE_RECORD"])
    result["promptReachedCli"] = "MARKDOWN" in record_path.read_text()
    result["pass"] = result["promptReachedCli"] and result["copiedCode"]
    result["steps"].append("markdown")
    write(evidence, result)
    return 0 if result["pass"] else 1


def todo_list(evidence, result):
    send_prompt("TODOS")
    # The fake sends a snapshot every 1.5 s, then answers.
    time.sleep(0.9)
    shot(evidence / "04-todos-created.png")
    time.sleep(1.5)
    shot(evidence / "05-todos-started.png")
    time.sleep(2.5)
    shot(evidence / "06-todos-collapsed.png")
    click(*TODO_HEADER)
    time.sleep(0.6)
    shot(evidence / "07-todos-expanded.png")
    # The trash button starts a new chat: the checklist goes away.
    click(*NEW_CHAT)
    time.sleep(1.0)
    shot(evidence / "08-new-chat.png")
    # Reopen the chat from history: the transcript and the last list come back.
    click(*HISTORY)
    time.sleep(1.0)
    shot(evidence / "09-history.png")
    click(*HISTORY_FIRST_ROW)
    time.sleep(2.5)
    shot(evidence / "10-resumed.png")
    # A 24-item list: the expanded checklist scrolls instead of squeezing the transcript.
    send_prompt("LONGTODOS")
    time.sleep(1.5)
    shot(evidence / "11-todos-long-running.png")
    time.sleep(4.0)
    shot(evidence / "12-todos-long-done.png")
    # Switch backend (Antigravity on, Claude Code off): the old checklist goes away.
    run(["xdotool", "key", "ctrl+comma"])
    settings = wait_window("^Settings$")
    if settings:
        click_window(settings, 448, 518)
        time.sleep(0.3)
        click_window(settings, 520, 248)
        time.sleep(0.3)
        click_window(settings, 520, 193)
        time.sleep(0.4)
        shot(evidence / "13-switch-settings.png")
        click_window(settings, 690, 22)
        time.sleep(2.0)
        shot(evidence / "14-switched.png")
        result["steps"].append("switched")
    record_path = Path(os.environ["FAKE_RECORD"])
    result["promptReachedCli"] = "TODOS" in record_path.read_text()
    result["pass"] = result["promptReachedCli"]
    result["steps"].append("todos")
    write(evidence, result)
    return 0 if result["pass"] else 1


def approvals(evidence, result):
    time.sleep(1.0)
    shot(evidence / "04-mode-picker.png")
    click(*MODE_PICKER)
    time.sleep(0.8)
    shot(evidence / "05-mode-open.png")
    # Ask before edits -> Accept edits -> Plan.
    run(["xdotool", "key", "Down", "Down", "Return"])
    time.sleep(1.0)
    shot(evidence / "06-mode-plan.png")
    send_prompt("QUESTION")
    time.sleep(2.5)
    shot(evidence / "07-question.png")
    # Typing in the first question's Other field picks Other; both parts in the multi-select.
    click(*QUESTION_OTHER_ENTRY)
    run(["xdotool", "type", "--delay", "12", "XeLaTeX"])
    click(*QUESTION_INTRO)
    click(*QUESTION_METHOD)
    time.sleep(0.4)
    shot(evidence / "08-question-filled.png")
    # Enter in the Other field submits once every question has an answer.
    click(*QUESTION_OTHER_ENTRY)
    run(["xdotool", "key", "Return"])
    time.sleep(1.5)
    shot(evidence / "09-question-answered.png")
    send_prompt("PLAN")
    time.sleep(2.5)
    shot(evidence / "10-plan.png")
    click(*PLAN_FEEDBACK)
    run(["xdotool", "type", "--delay", "12", "Also add an abstract."])
    time.sleep(0.3)
    shot(evidence / "11-plan-feedback.png")
    # Enter in the feedback field is Keep planning.
    run(["xdotool", "key", "Return"])
    time.sleep(2.0)
    shot(evidence / "12-plan-revised.png")
    click(*CARD_BUTTONS[1])
    time.sleep(2.0)
    shot(evidence / "13-plan-approved.png")
    record_path = Path(os.environ["FAKE_RECORD"])
    notes = []
    for line in record_path.read_text().splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if row.get("kind") == "note":
            notes.append(row)
    result["notes"] = notes
    find = lambda key: next((row for row in notes if key in row), {})
    modes = [row["setPermissionMode"] for row in notes if "setPermissionMode" in row]
    result["checks"] = {
        "modePlan": "plan" in modes,
        "questionAnswered": find("question").get("question") == "allow"
        and find("question").get("answers") == {
            "Which TeX engine should the build use?": "XeLaTeX",
            "Which parts need work?": "Intro, Method",
        },
        "keptPlanning": find("plan1").get("plan1") == "deny" and "Also add an abstract." in str(find("plan1").get("message")),
        "planApproved": find("plan2").get("plan2") == "allow"
        and find("plan2").get("updatedPermissions") == [{"type": "setMode", "mode": "acceptEdits", "destination": "session"}],
        "acceptEditsAfterPlan": modes[-1:] == ["acceptEdits"],
    }
    result["pass"] = all(result["checks"].values())
    result["steps"].append("approvals")
    write(evidence, result)
    return 0 if result["pass"] else 1


def send_prompt(text):
    click(560, 752)
    time.sleep(0.15)
    run(["xdotool", "type", "--delay", "12", text])
    run(["xdotool", "key", "Return"])


def seed_stale_provider(stored):
    config = Path(os.environ["XDG_CONFIG_HOME"]) / "pitex"
    config.mkdir(parents=True, exist_ok=True)
    (config / "preferences.json").write_text(
        json.dumps({"ai.agentProvider": stored}, separators=(",", ":"))
    )


def preferences_path():
    config_home = os.environ.get("XDG_CONFIG_HOME")
    if config_home:
        return Path(config_home) / "pitex" / "preferences.json"
    return Path(os.environ["HOME"]) / ".config" / "pitex" / "preferences.json"


def screen_text(path):
    if not path.exists() or not shutil.which("tesseract"):
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
    if provider == "agy":
        return '"permission":"allow"' in record
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
