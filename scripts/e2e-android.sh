#!/usr/bin/env bash
set -euo pipefail

apk="${1:?usage: e2e-android.sh <apk>}"
package="com.guillaumeboileau.interviewscribe"
component="$package/.MainActivity"
artifacts="${INTERVIEWSCRIBE_ANDROID_ARTIFACTS:-android-test-artifacts}"

[ -f "$apk" ] || { echo "APK not found: $apk" >&2; exit 1; }
mkdir -p "$artifacts"

collect_diagnostics() {
  adb logcat -d > "$artifacts/logcat.txt" 2>&1 || true
  adb exec-out screencap -p > "$artifacts/screenshot.png" 2>/dev/null || true
  adb shell dumpsys activity activities > "$artifacts/activities.txt" 2>&1 || true
}
trap collect_diagnostics EXIT

device_abi=$(adb shell getprop ro.product.cpu.abi | tr -d '\r')
[ "$device_abi" = "arm64-v8a" ] || {
  echo "expected an arm64-v8a emulator, got: $device_abi" >&2
  exit 1
}

if command -v unzip >/dev/null 2>&1; then
  unzip -Z1 "$apk" | grep -q '^lib/arm64-v8a/' || {
    echo 'APK does not contain arm64-v8a native libraries' >&2
    exit 1
  }
fi

adb install --replace "$apk"
adb shell pm path "$package" | grep -q '^package:'
adb logcat -c
launch_output=$(adb shell am start -W -n "$component")
echo "$launch_output"
grep -q 'Status: ok' <<< "$launch_output"

for _ in $(seq 1 30); do
  if adb shell pidof "$package" | grep -q '[0-9]'; then
    break
  fi
  sleep 1
done

adb shell pidof "$package" | grep -q '[0-9]' || {
  echo 'application process is not running after launch' >&2
  exit 1
}

sleep 5
adb logcat -b crash -d > "$artifacts/crash-log.txt"
if grep -q 'com\.guillaumeboileau\.interviewscribe' "$artifacts/crash-log.txt"; then
  echo 'fatal Android error detected after launch' >&2
  exit 1
fi

adb shell am force-stop "$package"
adb uninstall "$package"
trap - EXIT
collect_diagnostics
echo 'arm64-v8a APK installation, launch and uninstall verified'
