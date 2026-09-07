#!/bin/bash
# Wires a release keystore into the freshly generated Android Gradle project.
#
# `src-tauri/gen/android/` is gitignored and regenerated from scratch by
# `pnpm tauri android init` every time (including in CI, every single run) -
# Tauri's documented signing approach (manually edit build.gradle.kts once)
# does not survive that. This script re-applies the patch every time instead,
# same idea as scripts/tauri-android.sh persisting the NDK toolchain fix
# outside the gitignored gen/ tree.
#
# Usage: scripts/android-sign.sh <path-to-keystore.jks>
# Required env vars: ANDROID_KEYSTORE_PASSWORD, ANDROID_KEY_ALIAS, ANDROID_KEY_PASSWORD
set -euo pipefail

keystore_path="${1:?usage: android-sign.sh <path-to-keystore.jks>}"
: "${ANDROID_KEYSTORE_PASSWORD:?ANDROID_KEYSTORE_PASSWORD must be set}"
: "${ANDROID_KEY_ALIAS:?ANDROID_KEY_ALIAS must be set}"
: "${ANDROID_KEY_PASSWORD:?ANDROID_KEY_PASSWORD must be set}"

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd -- "$script_dir/.." && pwd)
gradle_dir="$repo_root/src-tauri/gen/android"
app_dir="$gradle_dir/app"
gradle_file="$app_dir/build.gradle.kts"
# Must live next to build.gradle.kts: `file("keystore.properties")` inside a
# module-level Kotlin DSL script resolves relative to that module's own
# directory (app/), not the project root - a real Gradle failure
# ("SigningConfig \"release\" is missing required property \"storeFile\"")
# caught this when the file was written one level too high.
properties_file="$app_dir/keystore.properties"

if [ ! -f "$gradle_file" ]; then
  echo "android-sign.sh: $gradle_file not found - run 'pnpm tauri android init' first" >&2
  exit 1
fi

resolved_keystore=$(realpath "$keystore_path")
cat > "$properties_file" <<EOF
storeFile=$resolved_keystore
storePassword=$ANDROID_KEYSTORE_PASSWORD
keyAlias=$ANDROID_KEY_ALIAS
keyPassword=$ANDROID_KEY_PASSWORD
EOF
echo "android-sign.sh: wrote $properties_file"

if grep -q "signingConfigs" "$gradle_file"; then
  echo "android-sign.sh: signingConfigs already present in $gradle_file, leaving it as-is"
  exit 0
fi

python3 "$script_dir/android-sign-patch.py" "$gradle_file"
echo "android-sign.sh: signing configuration applied to $gradle_file"
