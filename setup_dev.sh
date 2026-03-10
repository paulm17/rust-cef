#!/bin/bash
set -e

# When target directory is deleted, run these commands to restore CEF files needed for running `cargo run`:

echo "Setting up CEF Frameworks for development..."

# 1. Create the Frameworks directory expected by the LibraryLoader
mkdir -p target/Frameworks

# 2. Find and copy the CEF Framework to target/Frameworks
# Note: The hash in 'cef-sys-*' or 'cef-dll-sys-*' may change.
# We need to find where the framework is.

# Check for cef-sys or similar in the build directory
FRAMEWORK_SRC=$(find target/debug/build -name "Chromium Embedded Framework.framework" -type d | head -n 1)

if [ -z "$FRAMEWORK_SRC" ]; then
    echo "Error: Could not find 'Chromium Embedded Framework.framework' in target/debug/build"
    echo "Please ensure you have run 'cargo build' first."
    exit 1
fi

echo "Found framework at: $FRAMEWORK_SRC"

cp -R "$FRAMEWORK_SRC" target/Frameworks/

# 3. Copy the GPU libraries and Resources to the executable directory (target/debug)
# These are needed for the GPU process to start correctly and for resources to be found.
echo "Copying helper libraries and resources to target/debug..."
cp "$FRAMEWORK_SRC/Libraries/libGLESv2.dylib" target/debug/
cp "$FRAMEWORK_SRC/Libraries/libEGL.dylib" target/debug/
cp "$FRAMEWORK_SRC/Libraries/libvk_swiftshader.dylib" target/debug/ 2>/dev/null || true
cp -R "$FRAMEWORK_SRC/Resources/"* target/debug/

echo "Setup complete. You can now run 'cargo run'."
