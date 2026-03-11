#!/bin/bash
set -e

APP_NAME="rust-cef"
TARGET_DIR="target/debug"
FRAMEWORK_NAME="Chromium Embedded Framework.framework"

# Find the framework in the build directory
FRAMEWORK_SRC=$(find target/debug/build -name "$FRAMEWORK_NAME" -type d | head -n 1)

if [ -z "$FRAMEWORK_SRC" ]; then
    echo "Error: Could not find $FRAMEWORK_NAME in target/debug/build"
    echo "Please ensure you have run 'cargo build' first."
    exit 1
fi

echo "Found framework at: $FRAMEWORK_SRC"

# Create App Bundle Structure
BUNDLE_DIR="$TARGET_DIR/$APP_NAME.app"
CONTENTS_DIR="$BUNDLE_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
FRAMEWORKS_DIR="$CONTENTS_DIR/Frameworks"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

echo "Creating bundle at $BUNDLE_DIR..."
rm -rf "$BUNDLE_DIR"
mkdir -p "$MACOS_DIR"
mkdir -p "$FRAMEWORKS_DIR"
mkdir -p "$RESOURCES_DIR"

# Copy Executable
echo "Copying executable..."
cp "$TARGET_DIR/$APP_NAME" "$MACOS_DIR/"

# Copy Framework to Bundle
echo "Copying Framework to Bundle..."
cp -R "$FRAMEWORK_SRC" "$FRAMEWORKS_DIR/"

# Copy Resources to Bundle (Typically Resources are inside Framework on macOS, but we also need to ensure executable finds them if outside)
# Actually, for CEF on macOS, Resources are inside the Framework.
# But for 'cargo run' (non-bundled), we need them next to executable or in a known location?
# The error "icudtl.dat not found in bundle" suggests it looks in bundle resources?
# If we run raw executable, it looks next to it.
# So we copy Resources content to target/debug/.

echo "Setting up target/Frameworks for cargo run..."
mkdir -p "$TARGET_DIR/../Frameworks"
cp -R "$FRAMEWORK_SRC" "$TARGET_DIR/../Frameworks/"

echo "Copying Resources to target/debug for cargo run..."
cp -R "$FRAMEWORK_SRC/Resources/"* "$TARGET_DIR/"

echo "Copying Libraries (libGLESv2, libEGL, etc.) to target/debug for cargo run..."
cp "$FRAMEWORK_SRC/Libraries/"*.dylib "$TARGET_DIR/"

# Create Info.plist
echo "Creating Info.plist..."
cat > "$CONTENTS_DIR/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>$APP_NAME</string>
    <key>CFBundleIdentifier</key>
    <string>com.example.$APP_NAME</string>
    <key>CFBundleName</key>
    <string>$APP_NAME</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleURLTypes</key>
    <array>
        <dict>
            <key>CFBundleURLName</key>
            <string>com.rustcef.app.deeplink</string>
            <key>CFBundleURLSchemes</key>
            <array>
                <string>rustcef</string>
                <string>rust-cef</string>
            </array>
        </dict>
    </array>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Rust CEF Document</string>
            <key>LSHandlerRank</key>
            <string>Owner</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>com.rustcef.document</string>
            </array>
        </dict>
    </array>
    <key>UTExportedTypeDeclarations</key>
    <array>
        <dict>
            <key>UTTypeIdentifier</key>
            <string>com.rustcef.document</string>
            <key>UTTypeDescription</key>
            <string>Rust CEF Document</string>
            <key>UTTypeConformsTo</key>
            <array>
                <string>public.data</string>
            </array>
            <key>UTTypeTagSpecification</key>
            <dict>
                <key>public.filename-extension</key>
                <array>
                    <string>rustcef</string>
                </array>
            </dict>
        </dict>
    </array>
    <key>PrincipalClass</key>
    <string>NSApplication</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

echo "Bundle created successfully!"
echo "To run the app:"
echo "open $BUNDLE_DIR"
