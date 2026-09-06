---
name: release-packaging
description: Build, validate, package, sign, or publish InterviewScribe for Windows, Debian-based Linux, or Android.
---

# Release packaging

Read `docs/ROADMAP.md` before changing release automation.

Expected deliverables:

- Windows portable executable when technically supported.
- Windows NSIS installer as `.exe`.
- Debian package as `.deb`.
- Android package as `.apk`.

Do not label an artifact supported until it installs and starts on the corresponding target. Keep signing material in CI secrets, never in the repository. Pin build toolchains, publish checksums, retain licenses for bundled models and native libraries, and validate upgrades and uninstallation as well as first installation.

Treat model files separately from the small application installer unless an explicit offline bundle is requested.

