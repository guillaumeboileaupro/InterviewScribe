#!/usr/bin/env bash
# Real Tauri E2E harness (docs/TEST_IMPLEMENTATION_PLAN.md section 2): install
# the .deb first (pnpm tauri build --bundles deb --features wdio-e2e, then
# dpkg -i it) so the *installed* binary exists - bundled resources (the
# Whisper model) only resolve correctly from the installed bundle layout in
# a release build, so an uninstalled binary fails in ways a real user's
# install never does (see wdio.conf.mjs). @wdio/tauri-service's embedded
# provider manages the whole app+driver lifecycle itself now - this script's
# only remaining job is isolating app data to a throwaway profile so the
# suite never touches real interviews.
set -euo pipefail
cd "$(dirname "$0")/.."

BINARY="${INTERVIEWSCRIBE_E2E_BINARY:-/usr/bin/interviewscribe}"
if [ ! -x "$BINARY" ]; then
  echo "Binaire introuvable: $BINARY. Installer d'abord le paquet (pnpm tauri build --bundles deb --features wdio-e2e puis dpkg -i)." >&2
  exit 1
fi

PROFILE_DIR=$(mktemp -d)
trap 'rm -rf "$PROFILE_DIR"' EXIT

export XDG_DATA_HOME="$PROFILE_DIR/data"
export XDG_CONFIG_HOME="$PROFILE_DIR/config"
export XDG_CACHE_HOME="$PROFILE_DIR/cache"
mkdir -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"

pnpm exec wdio run e2e/wdio.conf.mjs "$@"
