# cmake-rs passes a toolchain file but the NDK otherwise defaults to ARMv7.
# Use Cargo's target for each architecture, including multi-ABI Tauri builds.
if("$ENV{TARGET}" STREQUAL "aarch64-linux-android")
  set(ANDROID_ABI "arm64-v8a" CACHE STRING "Android ABI" FORCE)
elseif("$ENV{TARGET}" STREQUAL "armv7-linux-androideabi")
  set(ANDROID_ABI "armeabi-v7a" CACHE STRING "Android ABI" FORCE)
elseif("$ENV{TARGET}" STREQUAL "i686-linux-android")
  set(ANDROID_ABI "x86" CACHE STRING "Android ABI" FORCE)
elseif("$ENV{TARGET}" STREQUAL "x86_64-linux-android")
  set(ANDROID_ABI "x86_64" CACHE STRING "Android ABI" FORCE)
else()
  message(FATAL_ERROR "Unsupported Cargo Android target: $ENV{TARGET}")
endif()
# 26, not 24: cpal's Android backend links against AAudio unconditionally,
# and libaaudio.so only exists in the NDK sysroot from API 26 onward - a
# real link failure ("unable to find library -laaudio") caught by an actual
# build, not a guess. Must match bundle.android.minSdkVersion in
# tauri.conf.json, which gates Gradle's own compile step the same way.
set(ANDROID_PLATFORM "android-26" CACHE STRING "Android API" FORCE)
include("$ENV{ANDROID_NDK_HOME}/build/cmake/android.toolchain.cmake")
