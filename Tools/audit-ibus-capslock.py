#!/usr/bin/env python3
"""Real IBus/X11 keyboard audit; run in an isolated D-Bus + Xvfb session."""
import json
from pathlib import Path
import re
import subprocess
import time
import gi

gi.require_version('Gtk', '4.0')
gi.require_version('GtkSource', '5')
from gi.repository import Gtk, GtkSource, GLib, Gio


def pump(seconds=0.12):
    end = time.monotonic() + seconds
    context = GLib.MainContext.default()
    while time.monotonic() < end:
        while context.pending():
            context.iteration(False)
        time.sleep(0.003)


def keys(*sequence):
    child = subprocess.Popen(['xdotool', 'key', '--delay', '20', *sequence])
    while child.poll() is None:
        pump(0.01)
    assert child.returncode == 0
    pump()


Gtk.init()
app = Gtk.Application(application_id='app.pitex.InputAudit')
app.register()
window = Gtk.ApplicationWindow(application=app, title='Pitex Caps Lock toolkit audit', default_width=600, default_height=300)
actions = {}
source = (Path(__file__).resolve().parents[1] / 'Linux/crates/pitex-shell/src/app_ui.rs').read_text()
accelerators = re.findall(r'app\.set_accels_for_action\("win\.([^\"]+)", &\["([^\"]+)"\]\);', source)
assert len(accelerators) >= 18
def activated(action, parameter):
    actions[action.get_name()] += 1
for name, accelerator in accelerators:
    actions[name] = 0
    action = Gio.SimpleAction.new(name, None)
    action.connect('activate', activated)
    window.add_action(action)
    app.set_accels_for_action('win.' + name, [accelerator])
box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
view = GtkSource.View(vexpand=True)
view.get_buffer().set_enable_undo(True)
entry = Gtk.Entry()
box.append(view)
box.append(entry)
window.set_child(box)
window.present()
pump(0.4)
wid = subprocess.check_output(['xdotool', 'search', '--name', '^Pitex Caps Lock toolkit audit$'], text=True).splitlines()[0]
subprocess.run(['xdotool', 'windowfocus', '--sync', wid], check=True)
subprocess.run(['ibus', 'engine', 'hangul'], check=True)
pump(0.5)
failures = []

for widget_name, widget in [('GtkSourceView', view), ('GtkEntry', entry)]:
    widget.grab_focus()
    pump(0.3)

    def reset(text='', cursor=None, selection=None):
        widget.reset_im_context()
        if widget is view:
            buffer = view.get_buffer()
            buffer.set_text(text)
            buffer.place_cursor(buffer.get_end_iter())
            if cursor is not None:
                buffer.place_cursor(buffer.get_iter_at_offset(cursor))
            if selection is not None:
                buffer.select_range(buffer.get_iter_at_offset(selection[1]), buffer.get_iter_at_offset(selection[0]))
        else:
            entry.set_text(text)
            entry.set_position(-1 if cursor is None else cursor)
            if selection is not None:
                entry.select_region(*selection)
        pump(0.03)

    def text():
        if widget is view:
            buffer = view.get_buffer()
            return buffer.get_text(buffer.get_start_iter(), buffer.get_end_iter(), True)
        return entry.get_text()

    def record(label, expected, actual):
        passed = actual == expected
        item = dict(widget=widget_name, phase=phase, case=label, expected=expected, actual=actual, passed=passed)
        print('INPUT_AUDIT ' + json.dumps(item, ensure_ascii=False), flush=True)
        if not passed:
            failures.append(item)

    def check(label, sequence, expected, seed='', selection=None, cursor=None):
        reset(seed, selection=selection, cursor=cursor)
        keys(*sequence)
        record(label, expected, text())

    def check_cursor(label, sequence, expected, cursor=3):
        reset('123', cursor=cursor)
        keys(*sequence)
        actual = view.get_buffer().get_iter_at_mark(view.get_buffer().get_insert()).get_offset() if widget is view else entry.get_position()
        record(label, expected, actual)

    def check_selection(label, sequence, expected):
        reset('123')
        keys(*sequence)
        bounds = view.get_buffer().get_selection_bounds() if widget is view else entry.get_selection_bounds()
        actual = [it.get_offset() for it in bounds] if widget is view else list(bounds)
        record(label, expected, actual)

    # Fresh contexts start in Latin mode. Four toggles return both the engine
    # and the physical Caps Lock latch to their starting states for the next widget.
    for phase in ['latin', 'caps-hangul', 'caps-latin', 'caps-hangul-again']:
        if phase != 'latin':
            reset()
            keys('Caps_Lock')
        korean = 'hangul' in phase
        check('language-mode', ['g', 'k', 's', 'space'], '한 ' if korean else 'gks ')
        check('number-row', list('0123456789'), '0123456789')
        check('shift-number-symbols', [f'shift+{key}' for key in '1234567890'], '!@#$%^&*()')
        check('tex-punctuation', ['backslash', 'braceleft', 'braceright', 'underscore', 'dollar', 'percent', 'asciicircum', 'ampersand', 'numbersign'], '\\{}_\u0024%^&#')
        check('backspace', ['BackSpace'], '12', '123')
        check('delete', ['Delete'], '23', '123', cursor=0)
        check('left-arrow-insert', ['Left', '4'], '1243', '123')
        check('home-insert', ['Home', '4'], '4123', '123')
        check('selection-replace', ['shift+Left', '4'], '124', '123')
        check('select-all-replace', ['ctrl+a', '4'], '4', '123')
        check_cursor('left-cursor', ['Left'], 2)
        check_cursor('right-cursor', ['Right'], 1, cursor=0)
        check_cursor('home-cursor', ['Home'], 0)
        check_cursor('end-cursor', ['End'], 3, cursor=0)
        check_selection('shift-left-selection', ['shift+Left'], [2, 3])
        check_selection('ctrl-a-selection', ['ctrl+a'], [0, 3])
        for name, accelerator in accelerators:
            sequence = accelerator.replace('<Control>', 'ctrl+').replace('<Shift>', 'shift+').replace('<Alt>', 'alt+')
            before = actions[name]
            reset()
            keys(sequence)
            record('accelerator-' + name, before + 1, actions[name])
        check('selection-backspace', ['BackSpace'], '1', '123', selection=(1, 3))
        check('selection-delete', ['Delete'], '1', '123', selection=(1, 3))
        check('cut', ['ctrl+x'], '', '123', selection=(0, 3))
        reset('123', selection=(0, 3))
        keys('ctrl+c')
        reset()
        keys('ctrl+v')
        record('copy-paste', '123', text())
        if widget is view:
            # Seed an undoable native edit so a broken number key cannot
            # masquerade as an undo/redo failure (or a false undo pass).
            reset('abc')
            buffer = view.get_buffer()
            buffer.begin_user_action()
            buffer.insert_at_cursor('4')
            buffer.end_user_action()
            keys('ctrl+z')
            record('undo', 'abc', text())
            keys('ctrl+shift+z')
            record('redo', 'abc4', text())
        if korean:
            check('compose-then-digit', ['g', 'k', 's', '1'], '한1')
            check('compose-then-symbol', ['g', 'k', 's', 'shift+4'], '한$')
            # Commit through two mode toggles, not Space (which is itself
            # broken on the affected stack), and restore the original mode.
            check('compose-jamo-backspace', ['g', 'k', 'BackSpace', 'k', 's', 'Caps_Lock', 'Caps_Lock'], '한')
        check('space', ['space'], ' ', '')
        if widget is view:
            check('return', ['Return'], '\n')
            check('tab', ['Tab'], '\t')
            widget.grab_focus()
        # Num Lock is enabled only around the keypad cases.
        keys('Num_Lock')
        check('keypad-digits', [f'KP_{key}' for key in '0123456789'], '0123456789')
        check('keypad-operators', ['KP_Add', 'KP_Subtract', 'KP_Multiply', 'KP_Divide', 'KP_Decimal'], '+-*/.')
        keys('Num_Lock')
    reset()
    keys('Caps_Lock')

window.close()
print('INPUT_AUDIT_SUMMARY ' + json.dumps(dict(failures=len(failures)), ensure_ascii=False), flush=True)
raise SystemExit(bool(failures))
