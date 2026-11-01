#!/usr/bin/env bash
# Private X11/DBus session only. Never touches the user's XFCE panel.
set -euo pipefail
cd "$(dirname "$0")/.."
for executable in Xvfb xfwm4 dbus-run-session xprop xwininfo xdotool python3; do
  command -v "$executable" >/dev/null || { echo "Missing: $executable" >&2; exit 1; }
done
cargo build --locked
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export MONOLITO_SMOKE_DIR="$work"
# xvfb-run selects a free display and supplies an Xauthority cookie.
xvfb-run -a -s '-screen 0 1280x800x24 -nolisten tcp' dbus-run-session -- bash -eu <<'SESSION'
export XDG_CONFIG_HOME="$MONOLITO_SMOKE_DIR/config"
export XDG_DATA_HOME="$MONOLITO_SMOKE_DIR/data"
export GSK_RENDERER=cairo
export NO_AT_BRIDGE=1
export GTK_A11Y=none
mkdir -p "$XDG_DATA_HOME/applications"
cat > "$XDG_DATA_HOME/applications/monolito-smoke.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Monolito Smoke Client
Exec=xmessage -name monolito-smoke -title "Monolito Smoke Client" monolito-test
Icon=application-x-executable
StartupWMClass=Xmessage
DESKTOP
xfwm4 --compositor=off >"$MONOLITO_SMOKE_DIR/wm.log" 2>&1 &
wm_pid=$!
panel_pid=''
cleanup() {
  result=$?
  kill "${panel_pid:-}" "$wm_pid" 2>/dev/null || true
  if (( result != 0 )); then
    for log in "$MONOLITO_SMOKE_DIR"/*.log; do cat "$log" >&2; done
  fi
  exit "$result"
}
trap cleanup EXIT
# Wait for the WM's EWMH support window.
for attempt in {1..50}; do
  if xprop -root _NET_SUPPORTING_WM_CHECK | grep -q 'window id'; then break; fi
  sleep 0.1
done
./target/debug/monolito-panel >"$MONOLITO_SMOKE_DIR/panel.log" 2>&1 &
panel_pid=$!
export MONOLITO_PANEL_PID="$panel_pid"
python3 scripts/verify-x11.py
kill "$panel_pid"
wait "$panel_pid" || true
# Repeat startup in non-reserving mode.
./target/debug/monolito-panel --dev >"$MONOLITO_SMOKE_DIR/dev.log" 2>&1 &
panel_pid=$!
python3 scripts/verify-x11.py --dev
cat "$MONOLITO_SMOKE_DIR/panel.log" "$MONOLITO_SMOKE_DIR/dev.log"
SESSION
