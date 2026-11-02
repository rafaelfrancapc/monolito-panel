"""Capture real GTK/X11 surfaces; fixtures exercise missing and themed app icons."""
import os
import pathlib
import subprocess
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
os.chdir(ROOT)
helpers = {}
exec((ROOT / "scripts/verify-x11.py").read_text().split("\nwid = wait_for")[0], helpers)
run, wait_for, geometry = (helpers[k] for k in ("run", "wait_for", "geometry"))
output = ROOT / "target/visual"
output.mkdir(parents=True, exist_ok=True)
scale = int(os.environ.get("GDK_SCALE", "1"))
width, height = 1280 * scale, 800 * scale
with tempfile.TemporaryDirectory() as directory:
    env = dict(os.environ, XDG_CONFIG_HOME=directory + "/config",
               XDG_DATA_HOME=directory + "/data", GSK_RENDERER="cairo", GTK_A11Y="none")
    apps = pathlib.Path(env["XDG_DATA_HOME"]) / "applications"
    apps.mkdir(parents=True)
    for name, icon in [("Themed icon", "utilities-terminal"), ("Missing icon", "does-not-exist"),
                       ("File icon", str(ROOT / "resources/icons/start-symbolic.svg"))]:
        (apps / (name + ".desktop")).write_text(
            f"[Desktop Entry]\nType=Application\nName={name}\nExec=true\nIcon={icon}\n")
    config = pathlib.Path(env["XDG_CONFIG_HOME"]) / "monolito"
    config.mkdir(parents=True)
    (config / "panel.toml").write_text(
        'version = 1\npinned = ["Themed icon.desktop", "Missing icon.desktop", "File icon.desktop"]\n')
    wm = subprocess.Popen(["xfwm4", "--compositor=off"], env=env, stdout=subprocess.DEVNULL)
    try:
        wait_for(lambda: "window id" in run("xprop", "-root", "_NET_SUPPORTING_WM_CHECK"), "WM")
        for theme in ("dark", "light"):
            with (output / f"{theme}-{scale}x.log").open("w") as log:
                panel = subprocess.Popen(["./target/debug/monolito-panel", f"--theme={theme}"],
                                         env=env, stdout=log, stderr=log)
                try:
                    wid = wait_for(lambda: run("xdotool", "search", "--onlyvisible", "--name",
                                               "^Desktop Panel$").splitlines()[0], "Panel")
                    wait_for(lambda: geometry(wid) == (0, height - 48 * scale, width, 48 * scale), "Scaled dock")
                    time.sleep(1)
                    for name, x in (("start", width // 2 - 70 * scale), ("quick", width - 120 * scale)):
                        run("xdotool", "mousemove", str(x), str(height - 24 * scale), "click", "1")
                        popup = wait_for(helpers["visible_popups"], name)[0]
                        time.sleep(1)
                        x0, y0, w, h = geometry(popup)
                        run("import", "-window", "root", str(output / f"{theme}-{name}-{scale}x.png"))
                        print(theme, name, scale, geometry(popup), flush=True)
                        assert x0 >= 0 and y0 >= 0 and x0+w <= width and y0+h <= height-48*scale, (x0, y0, w, h)
                        run("xdotool", "key", "Escape")
                finally:
                    panel.terminate()
                    panel.wait()
    finally:
        wm.terminate()
        wm.wait()
