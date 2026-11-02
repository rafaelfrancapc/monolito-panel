#!/usr/bin/env bash
# Exercise native-panel replacement without touching the current desktop.
set -euo pipefail
cd "$(dirname "$0")/.."
test_dir=$(mktemp -d)
trap 'rm -rf "$test_dir"' EXIT
mkdir -p "$test_dir/bin"
export MOCK_LOG="$test_dir/actions"
export MOCK_STATE="$test_dir/cinnamon-state"
cat > "$test_dir/bin/pgrep" <<'MOCK'
#!/usr/bin/env bash
exit 0
MOCK
cat > "$test_dir/bin/xfce4-panel" <<'MOCK'
#!/usr/bin/env bash
printf 'xfce %s\n' "$1" >> "$MOCK_LOG"
MOCK
cat > "$test_dir/bin/gsettings" <<'MOCK'
#!/usr/bin/env bash
if [[ $1 == get ]]; then
  if [[ -f $MOCK_STATE ]]; then cat "$MOCK_STATE"; else printf "['1:0:bottom']\n"; fi
else
  printf '%s\n' "$4" > "$MOCK_STATE"
  printf 'cinnamon %s\n' "$4" >> "$MOCK_LOG"
fi
MOCK
chmod +x "$test_dir/bin/"*
export PATH="$test_dir/bin:$PATH" DISPLAY=:99 XDG_SESSION_TYPE=x11
export MONOLITO_PANEL_BIN=/bin/true
XDG_CURRENT_DESKTOP=XFCE bash packaging/session run
test "$(cat "$MOCK_LOG")" = $'xfce --quit\nxfce --restart'
: > "$MOCK_LOG"
XDG_CURRENT_DESKTOP=X-Cinnamon bash packaging/session run
test "$(cat "$MOCK_LOG")" = $"cinnamon []"$'\n'"cinnamon ['1:0:bottom']"
test "$(cat "$MOCK_STATE")" = "['1:0:bottom']"
printf 'PASS: XFCE and Cinnamon native panels restored\n'
