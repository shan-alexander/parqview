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
# Note: winit 0.29+ ignores WINIT_UNIX_BACKEND; backend is selected from
# WAYLAND_DISPLAY / WAYLAND_SOCKET vs DISPLAY.
if [[ "${PARQVIEW_X11:-}" == "1" ]]; then
  unset WAYLAND_DISPLAY WAYLAND_SOCKET || true
fi

if command -v nix >/dev/null && [[ -f "${ROOT}/flake.nix" ]]; then
  exec nix develop "${ROOT}" -c "$BIN" "$@"
fi

exec "$BIN" "$@"
