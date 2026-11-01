#!/usr/bin/env bash
# Ubuntu 24.04 LTS / Ubuntu on WSL2. Does not change the desktop session.
set -euo pipefail
if [[ ${EUID} -eq 0 ]]; then privilege=(); else privilege=(sudo); fi
apt_options=()
if [[ -n ${MONOLITO_APT_CA:-} ]]; then
  apt_options=(-o "Acquire::https::CaInfo=$MONOLITO_APT_CA")
fi
"${privilege[@]}" apt-get "${apt_options[@]}" update
"${privilege[@]}" apt-get "${apt_options[@]}" install -y --no-install-recommends \
  build-essential pkg-config curl ca-certificates libgtk-4-dev gettext \
  adwaita-icon-theme librsvg2-common fonts-inter fonts-montserrat
if ! command -v rustup >/dev/null && [[ ! -x ${CARGO_HOME:-$HOME/.cargo}/bin/rustup ]]; then
  installer=$(mktemp)
  trap 'rm -f "$installer"' EXIT
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$installer"
  sh "$installer" -y --profile minimal --default-toolchain stable --component rustfmt --component clippy
fi
source "${CARGO_HOME:-$HOME/.cargo}/env"
cd "$(dirname "$0")/.."
cargo build --locked
./scripts/translations.sh
