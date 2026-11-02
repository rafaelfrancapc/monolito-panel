#!/usr/bin/env bash
# Build a local Ubuntu 24.04 / Mint 22.3 package without installing it.
set -euo pipefail
cd "$(dirname "$0")/.."
source "${CARGO_HOME:-$HOME/.cargo}/env"
cargo build --release --locked
./scripts/translations.sh
version=$(sed -n '/^\[package\]/,/^\[/s/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
architecture=$(dpkg --print-architecture)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
chmod 0755 "$stage"
install -Dm755 target/release/monolito-panel "$stage/usr/bin/monolito-panel"
install -Dm755 packaging/session "$stage/usr/libexec/monolito-panel/session"
install -Dm644 packaging/monolito-panel.service "$stage/usr/lib/systemd/user/monolito-panel.service"
install -Dm644 packaging/monolito-panel.desktop "$stage/etc/xdg/autostart/monolito-panel.desktop"
for catalog in resources/locale/*/LC_MESSAGES/monolito-panel.mo; do
  language=${catalog#resources/locale/}
  install -Dm644 "$catalog" "$stage/usr/share/locale/$language"
done
install -d "$stage/DEBIAN" dist
cat > "$stage/DEBIAN/control" <<CONTROL
Package: monolito-panel
Version: $version
Section: x11
Priority: optional
Architecture: $architecture
Maintainer: Monolito Panel contributors
Depends: libc6, libgtk-4-1 (>= 4.12), libglib2.0-0t64, xfce4-panel | cinnamon
Description: Fluent style desktop panel for XFCE and Cinnamon X11
 Includes a taskbar, Start menu, quick settings, and a user session service.
CONTROL
dpkg-deb --build --root-owner-group "$stage" "dist/monolito-panel_${version}_${architecture}.deb"
