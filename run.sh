#!/usr/bin/env bash
# Launch parqview with Wayland-friendly graphics libs + system duckdb.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
BIN="${ROOT}/target/release/parqview"

if [[ ! -x "$BIN" ]]; then
  echo "building release binary…"
  if command -v nix >/dev/null && [[ -f "${ROOT}/flake.nix" ]]; then
    nix develop "${ROOT}" -c cargo build --release
  else
    cargo build --release
  fi
fi

# Prefer Wayland (user default). Only force X11 if PARQVIEW_X11=1.
if [[ "${PARQVIEW_X11:-}" == "1" ]]; then
  export WINIT_UNIX_BACKEND=x11
  unset WAYLAND_DISPLAY || true
else
  unset WINIT_UNIX_BACKEND || true
fi

if command -v nix >/dev/null && [[ -f "${ROOT}/flake.nix" ]]; then
  exec nix develop "${ROOT}" -c "$BIN" "$@"
fi

exec "$BIN" "$@"
