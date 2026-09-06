#!/bin/bash
# Wraps `pnpm tauri android ...` with the Android NDK CMake toolchain file exported.
# Needed because whisper-rs-sys (and any other cmake-based native dependency) cross-compiles
# via the generic `cmake` crate, which cannot locate the Android NDK on its own — it must be
# pointed at the NDK's own CMake toolchain file, or configuration fails with
# "Android: Neither the NDK or a standalone toolchain was found."
set -euo pipefail

sdk_root="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}"
if [ -z "$sdk_root" ]; then
  echo "tauri-android.sh: ANDROID_SDK_ROOT or ANDROID_HOME must be set" >&2
  exit 1
fi

ndk_home="${ANDROID_NDK_HOME:-}"
if [ -z "$ndk_home" ]; then
  ndk_home=$(find "$sdk_root/ndk" -maxdepth 1 -mindepth 1 -type d | sort -V | tail -n1)
fi
if [ -z "$ndk_home" ] || [ ! -d "$ndk_home" ]; then
  echo "tauri-android.sh: could not find an installed NDK under $sdk_root/ndk" >&2
  exit 1
fi

toolchain_file="$ndk_home/build/cmake/android.toolchain.cmake"
if [ ! -f "$toolchain_file" ]; then
  echo "tauri-android.sh: no android.toolchain.cmake under $ndk_home" >&2
  exit 1
fi

export ANDROID_NDK_HOME="$ndk_home"
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
export CMAKE_TOOLCHAIN_FILE="$script_dir/android-toolchain.cmake"

exec pnpm tauri android "$@"
