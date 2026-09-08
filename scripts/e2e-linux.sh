#!/usr/bin/env bash
# Real Tauri E2E harness (docs/TEST_IMPLEMENTATION_PLAN.md section 2): builds
# nothing itself - install the .deb first (pnpm tauri build --bundles deb,
# then dpkg -i it) so the *installed* binary exists. Deliberately does not
# drive the raw target/release/ executable: bundled resources (the Whisper
# model) only resolve correctly from the installed bundle layout in a
# release build, so an uninstalled binary fails in ways a real user's
# install never does - see wdio.conf.mjs for how this was found. Isolates
# the run to a throwaway XDG profile so it never touches real interviews,
# then starts and stops `tauri-driver` regardless of the test outcome.
set -euo pipefail
cd "$(dirname "$0")/.."

BINARY="${INTERVIEWSCRIBE_E2E_BINARY:-/usr/bin/interviewscribe}"
if [ ! -x "$BINARY" ]; then
  echo "Binaire introuvable: $BINARY. Installer d'abord le paquet (pnpm tauri build --bundles deb puis dpkg -i)." >&2
  exit 1
fi

PROFILE_DIR=$(mktemp -d)
DRIVER_PID=""
cleanup() {
  [ -n "$DRIVER_PID" ] && kill "$DRIVER_PID" 2>/dev/null || true
  rm -rf "$PROFILE_DIR"
}
trap cleanup EXIT

export XDG_DATA_HOME="$PROFILE_DIR/data"
export XDG_CONFIG_HOME="$PROFILE_DIR/config"
export XDG_CACHE_HOME="$PROFILE_DIR/cache"
mkdir -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"

tauri-driver --port 4444 &
DRIVER_PID=$!

for _ in $(seq 1 50); do
  if (exec 3<>"/dev/tcp/127.0.0.1/4444") 2>/dev/null; then
    exec 3<&- 3>&-
    break
  fi
  sleep 0.2
done

pnpm exec wdio run e2e/wdio.conf.mjs
