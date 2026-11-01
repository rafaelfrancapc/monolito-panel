#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
for catalog in po/*.po; do
  language=$(basename "$catalog" .po)
  mkdir -p "resources/locale/$language/LC_MESSAGES"
  msgfmt --check "$catalog" -o "resources/locale/$language/LC_MESSAGES/monolito-panel.mo"
done
