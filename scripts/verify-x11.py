"""Exercise real X11 properties and GTK popup geometry; no simulated shell."""
import os
import pathlib
import re
import subprocess
import sys
import time


def run(*args):
    return subprocess.check_output(args, text=True, stderr=subprocess.DEVNULL).strip()


def wait_for(check, message):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        try:
            value = check()
            if value:
                return value
        except (subprocess.CalledProcessError, ValueError, IndexError):
            pass
        time.sleep(0.1)
    raise AssertionError(message)


def prop(wid, name):
    return run('xprop', '-id', str(wid), name)


def geometry(wid):
    text = run('xwininfo', '-id', str(wid))
    return tuple(int(re.search(pattern + r'\s*:\s*(-?\d+)', text)[1]) for pattern in
                 ('Absolute upper-left X', 'Absolute upper-left Y', 'Width', 'Height'))


def visible_popups():
    tree = run('xwininfo', '-root', '-tree')
    popups = []
    for wid in re.findall(r'^\s+(0x[0-9a-f]+)', tree, re.M):
        try:
            if '_NET_WM_WINDOW_TYPE_MENU' in prop(wid, '_NET_WM_WINDOW_TYPE') and 'IsViewable' in run('xwininfo', '-id', wid) and geometry(wid)[2] > 250:
                popups.append(wid)
        except subprocess.CalledProcessError:
            pass
    return popups


def open_start(hint=None):
    positions = range(max(0, hint - 80), min(1080, hint + 80), 4) if hint else range(160, 920, 8)
    for x in positions:
        run('xdotool', 'mousemove', str(x), '776', 'click', '1')
        time.sleep(0.06)
        popups = visible_popups()
        for popup in popups:
            if geometry(popup)[2] >= 500:
                return x, popup
        if popups:
            run('xdotool', 'key', 'Escape')
    raise AssertionError('Start popup did not become visible')


def minimize_from_taskbar(client, start_x):
    del start_x
    for x in range(8, 1080, 8):
        run('xdotool', 'mousemove', str(x), '776', 'click', '1')
        time.sleep(0.06)
        if 'IsUnMapped' in run('xwininfo', '-id', client):
            return x
        if visible_popups():
            run('xdotool', 'key', 'Escape')
    raise AssertionError('Taskbar did not minimize the active client')


def open_quick_settings():
    for x in range(980, 1240, 4):
        run('xdotool', 'mousemove', str(x), '776', 'click', '1')
        time.sleep(0.06)
        popups = visible_popups()
        for popup in popups:
            width = geometry(popup)[2]
            if 400 <= width < 500:
                return popup
        if popups:
            run('xdotool', 'key', 'Escape')
    raise AssertionError('Quick settings popup did not become visible')


wid = wait_for(lambda: run('xdotool', 'search', '--onlyvisible', '--name', '^Desktop Panel$').splitlines()[0], 'Panel did not map')
watcher = run('gdbus', 'call', '--session', '--dest', 'org.kde.StatusNotifierWatcher',
              '--object-path', '/StatusNotifierWatcher', '--method',
              'org.freedesktop.DBus.Properties.Get', 'org.kde.StatusNotifierWatcher',
              'IsStatusNotifierHostRegistered')
assert 'true' in watcher, watcher
wait_for(lambda: geometry(wid) == (0, 752, 1280, 48), 'Dock geometry differs from the requested bottom strip')
assert '_NET_WM_WINDOW_TYPE_DOCK' in prop(wid, '_NET_WM_WINDOW_TYPE')
strut = prop(wid, '_NET_WM_STRUT_PARTIAL').split(' = ')[1]
values = [int(v) for v in strut.split(', ')]
assert values == ([0] * 12 if '--dev' in sys.argv else [0, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 1279]), values
assert '_NET_WM_STATE_SKIP_TASKBAR' in prop(wid, '_NET_WM_STATE')
if '--dev' not in sys.argv:
    workarea = run('xprop', '-root', '_NET_WORKAREA')
    assert '0, 0, 1280, 752' in workarea, workarea
    # Start popup, then dismiss using the keyboard.
    start_x, popup = open_start()
    x, y, width, height = geometry(popup)
    assert 0 <= x and x + width <= 1280 and y + height <= 752 and width >= 500, (x, y, width, height)
    run('xdotool', 'key', 'Escape')
    wait_for(lambda: not visible_popups(), 'Escape did not close Start')
    run('xdotool', 'key', 'Super_L')
    wait_for(lambda: any(geometry(p)[2] >= 500 for p in visible_popups()), 'Super key did not open Start')
    run('xdotool', 'key', 'Super_L')
    wait_for(lambda: not visible_popups(), 'Super key did not close Start')
    # Search an actual XDG desktop entry and launch it using the keyboard.
    start_x, _ = open_start(start_x)
    run('xdotool', 'type', '--clearmodifiers', 'Monolito Smoke Client')
    time.sleep(0.4)  # GtkSearchEntry's documented search delay.
    run('xdotool', 'key', 'Return')
    client = wait_for(lambda: run('xdotool', 'search', '--onlyvisible', '--name', '^Monolito Smoke Client$').splitlines()[0], 'XDG search/launch did not open the client')
    wait_for(lambda: not visible_popups(), 'Start did not close after launch')
    wait_for(lambda: hex(int(client)) in run('xprop', '-root', '_NET_ACTIVE_WINDOW'), 'Launched client did not become active')
    time.sleep(0.2)  # Allow the event worker to publish the active-window change.
    start_x, _ = open_start(start_x)
    run('xdotool', 'key', 'Escape')
    wait_for(lambda: not visible_popups(), 'Escape did not close Start')
    task_x = minimize_from_taskbar(client, start_x)
    time.sleep(0.6)  # Separate two user clicks beyond GTK's double-click interval.
    run('xdotool', 'mousemove', str(task_x), '776', 'click', '1')
    wait_for(lambda: 'IsViewable' in run('xwininfo', '-id', client), 'Taskbar did not restore the client')
    run('xdotool', 'windowclose', client)
    # A desktop entry installed after startup must appear without restarting.
    new_app = pathlib.Path(os.environ['XDG_DATA_HOME']) / 'applications/live-app.desktop'
    new_app.write_text('[Desktop Entry]\nType=Application\nName=Live Installed App\n'
                       'Exec=xmessage -name live-app -title "Live Installed App" live-test\n'
                       'Icon=utilities-terminal\n')
    time.sleep(1.5)
    start_x, _ = open_start(start_x)
    run('xdotool', 'type', '--clearmodifiers', 'Live Installed App')
    time.sleep(0.4)
    run('xdotool', 'key', 'Return')
    live = wait_for(lambda: run('xdotool', 'search', '--onlyvisible', '--name',
                                '^Live Installed App$').splitlines()[0],
                    'New app was not loaded automatically')
    run('xdotool', 'windowclose', live)
    # Quick settings is immediately left of the clock.
    popup = open_quick_settings()
    x, y, width, height = geometry(popup)
    assert x > 700 and x + width <= 1280 and y + height <= 752, (x, y, width, height)
    run('xdotool', 'key', 'Escape')
    wait_for(lambda: not visible_popups(), 'Escape did not close quick settings')
    # Xvfb has a fixed output mode but can resize the framebuffer while disabled.
    run('xrandr', '--output', 'screen', '--off', '--fb', '1024x768')
    wait_for(lambda: geometry(wid) == (0, 720, 1024, 48), 'Dock did not follow RandR resize')
    wait_for(lambda: prop(wid, '_NET_WM_STRUT_PARTIAL').endswith('0, 1023'), 'Strut span did not resize')
    run('xrandr', '--fb', '1280x800', '--output', 'screen', '--mode', '1280x800')
    wait_for(lambda: geometry(wid) == (0, 752, 1280, 48), 'Dock did not follow restored geometry')
print('PASS:', 'development mode, no reservation' if '--dev' in sys.argv else 'dock, strut/workarea, popups/Escape, XDG search/launch, taskbar minimize/restore, RandR resize')
