"""Inserts a release signingConfig into a freshly generated build.gradle.kts.

Called by android-sign.sh - see that script for why this has to be a patch
re-applied every build rather than a one-time manual edit.
"""

import sys

# Fully-qualified `java.util.Properties()`/`java.io.FileInputStream(...)`
# (the form Tauri's own docs show) does not resolve inside this lambda
# receiver context - a real Gradle build caught "Unresolved reference:
# util"/"Unresolved reference: io". The generated file already imports
# `java.util.Properties` unqualified for `tauriProperties` above, so this
# just follows the same style instead, plus one more import for
# FileInputStream.
IMPORT_LINE = "import java.io.FileInputStream\n"

SIGNING_BLOCK = """    signingConfigs {
        create("release") {
            val keystoreProperties = Properties()
            val keystorePropertiesFile = file("keystore.properties")
            if (keystorePropertiesFile.exists()) {
                keystoreProperties.load(FileInputStream(keystorePropertiesFile))
                storeFile = file(keystoreProperties.getProperty("storeFile"))
                storePassword = keystoreProperties.getProperty("storePassword")
                keyAlias = keystoreProperties.getProperty("keyAlias")
                keyPassword = keystoreProperties.getProperty("keyPassword")
            }
        }
    }
    buildTypes {"""

RELEASE_ANCHOR = 'getByName("release") {'
SIGNING_CONFIG_LINE = '\n            signingConfig = signingConfigs.getByName("release")'


def main() -> None:
    path = sys.argv[1]
    with open(path, encoding="utf-8") as handle:
        content = handle.read()

    if "import java.util.Properties" not in content:
        raise SystemExit(
            "android-sign-patch.py: expected 'import java.util.Properties' in build.gradle.kts"
        )
    if IMPORT_LINE not in content:
        content = content.replace(
            "import java.util.Properties\n",
            "import java.util.Properties\n" + IMPORT_LINE,
            1,
        )

    if "    buildTypes {" not in content:
        raise SystemExit(
            "android-sign-patch.py: could not find 'buildTypes {' anchor in build.gradle.kts"
        )
    content = content.replace("    buildTypes {", SIGNING_BLOCK, 1)

    if RELEASE_ANCHOR not in content:
        raise SystemExit(
            "android-sign-patch.py: could not find the release buildType anchor in build.gradle.kts"
        )
    content = content.replace(
        RELEASE_ANCHOR, RELEASE_ANCHOR + SIGNING_CONFIG_LINE, 1
    )

    with open(path, "w", encoding="utf-8") as handle:
        handle.write(content)


if __name__ == "__main__":
    main()
