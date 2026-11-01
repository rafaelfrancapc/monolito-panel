#!/usr/bin/env bash
# Managed cloud helper. Ordinary Ubuntu/WSL users should use setup-ubuntu.sh.
set -euo pipefail
cd "$(dirname "$0")/.."
checkout=$(pwd -P)
container=monolito-dev
docker_local() {
  env -u DOCKER_HOST -u DOCKER_CONTEXT -u DOCKER_TLS -u DOCKER_TLS_VERIFY \
    -u DOCKER_CERT_PATH docker --host=unix:///var/run/docker.sock "$@"
}
if ! docker_local container inspect "$container" >/dev/null 2>&1; then
  docker_local run -d --name "$container" \
    --mount "type=bind,src=$checkout,dst=/work" \
    --mount type=bind,src=/etc/ssl/certs/ca-certificates.crt,dst=/run/host-ca.pem,readonly \
    -e SSL_CERT_FILE=/run/host-ca.pem -e CURL_CA_BUNDLE=/run/host-ca.pem \
    -e CARGO_HTTP_CAINFO=/run/host-ca.pem -w /work ubuntu:24.04 sleep infinity
fi
mounted_checkout=$(docker_local inspect --format '{{range .Mounts}}{{if eq .Destination "/work"}}{{.Source}}{{end}}{{end}}' "$container")
if [[ $mounted_checkout != "$checkout" ]]; then
  echo "Refusing to reuse $container: /work does not refer to this checkout." >&2
  exit 1
fi
if [[ $(docker_local inspect --format '{{.State.Running}}' "$container") != true ]]; then
  docker_local start "$container"
fi
case "${1:-check}" in
  setup)
    docker_local exec "$container" sh -c '
      sed -i "s|http://archive.ubuntu.com|https://archive.ubuntu.com|g; s|http://security.ubuntu.com|https://security.ubuntu.com|g" /etc/apt/sources.list.d/ubuntu.sources'
    docker_local exec -e MONOLITO_APT_CA=/run/host-ca.pem -e DEBIAN_FRONTEND=noninteractive "$container" bash scripts/setup-ubuntu.sh
    docker_local exec -e DEBIAN_FRONTEND=noninteractive "$container" apt-get -o Acquire::https::CaInfo=/run/host-ca.pem install -y --no-install-recommends xvfb xauth xfwm4 dbus-x11 x11-utils x11-xserver-utils xdotool python3
    ;;
  check)
    docker_local exec "$container" bash -c 'source /root/.cargo/env; cargo fmt --all --check && cargo check --workspace --locked && cargo test --workspace --locked'
    ;;
  smoke)
    docker_local exec "$container" bash -c 'source /root/.cargo/env; ./scripts/smoke-x11.sh'
    ;;
  *) echo 'Usage: scripts/cloud-ubuntu.sh {setup|check|smoke}' >&2; exit 2 ;;
esac
