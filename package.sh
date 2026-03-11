#!/bin/bash
set -e

echo "📦 Starting Packaging Process..."

# 1. Build Frontend
echo "🎨 Building Frontend..."
cd frontend
if ! command -v bun &> /dev/null; then
    echo "❌ git bun could not be found. Please install bun."
    exit 1
fi

echo "   Running bun install..."
bun install
echo "   Running bun run build..."
bun run build

# Check if dist exists
if [ ! -d "dist" ]; then
    echo "❌ Frontend build failed: dist directory not found."
    exit 1
fi
echo "✅ Frontend built successfully."
cd ..

# 2. Build Rust Application
echo "🦀 Building Rust Application (Release)..."
cargo build --release

# 3. Stage CEF Framework
echo "🏗️  Staging CEF Framework..."
FRAMEWORK_NAME="Chromium Embedded Framework.framework"
# Find the framework in the build directory (it's deep inside target/release/build/cef-sys-...
FRAMEWORK_SRC=$(find target/release/build -name "$FRAMEWORK_NAME" -type d | head -n 1)

if [ -z "$FRAMEWORK_SRC" ]; then
    echo "❌ Error: Could not find $FRAMEWORK_NAME in target/release/build"
    echo "   Ensure cargo build --release ran successfully."
    exit 1
fi

echo "   Found framework at: $FRAMEWORK_SRC"

TARGET_FRAMEWORKS_DIR="target/release/Frameworks"
mkdir -p "$TARGET_FRAMEWORKS_DIR"

# Clean destination to ensure fresh copy
rm -rf "$TARGET_FRAMEWORKS_DIR/$FRAMEWORK_NAME"

echo "   Copying framework to $TARGET_FRAMEWORKS_DIR..."
cp -R "$FRAMEWORK_SRC" "$TARGET_FRAMEWORKS_DIR/"

# 4. Package Application
echo "🎁 Packaging Application..."
# Only build .app to avoid DMG creation errors (AppleScript) and ensure bundle remains
cargo packager --release --formats app

# 5. Create Helper Apps
echo "🔧 Creating Helper Apps..."
APP_NAME="Rust CEF"
# cargo packager --formats app outputs to target/release directly (apparently)
BUNDLE_DIR="target/release/${APP_NAME}.app"
FRAMEWORKS_DIR="${BUNDLE_DIR}/Contents/Frameworks"
MAIN_EXEC_NAME="rust-cef"
PLIST_PATH="${BUNDLE_DIR}/Contents/Info.plist"

echo "🧩 Configuring bundle URL schemes and document types..."
/usr/libexec/PlistBuddy -c "Delete :CFBundleURLTypes" "${PLIST_PATH}" >/dev/null 2>&1 || true
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes array" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0 dict" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLName string com.rustcef.app.deeplink" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes array" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes:0 string rustcef" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :CFBundleURLTypes:0:CFBundleURLSchemes:1 string rust-cef" "${PLIST_PATH}"

/usr/libexec/PlistBuddy -c "Delete :CFBundleDocumentTypes" "${PLIST_PATH}" >/dev/null 2>&1 || true
/usr/libexec/PlistBuddy -c "Add :CFBundleDocumentTypes array" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :CFBundleDocumentTypes:0 dict" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :CFBundleDocumentTypes:0:CFBundleTypeName string Rust CEF Document" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :CFBundleDocumentTypes:0:LSHandlerRank string Owner" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :CFBundleDocumentTypes:0:LSItemContentTypes array" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :CFBundleDocumentTypes:0:LSItemContentTypes:0 string com.rustcef.document" "${PLIST_PATH}"

/usr/libexec/PlistBuddy -c "Delete :UTExportedTypeDeclarations" "${PLIST_PATH}" >/dev/null 2>&1 || true
/usr/libexec/PlistBuddy -c "Add :UTExportedTypeDeclarations array" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :UTExportedTypeDeclarations:0 dict" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :UTExportedTypeDeclarations:0:UTTypeIdentifier string com.rustcef.document" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :UTExportedTypeDeclarations:0:UTTypeDescription string Rust CEF Document" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :UTExportedTypeDeclarations:0:UTTypeConformsTo array" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :UTExportedTypeDeclarations:0:UTTypeConformsTo:0 string public.data" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :UTExportedTypeDeclarations:0:UTTypeTagSpecification dict" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :UTExportedTypeDeclarations:0:UTTypeTagSpecification:public.filename-extension array" "${PLIST_PATH}"
/usr/libexec/PlistBuddy -c "Add :UTExportedTypeDeclarations:0:UTTypeTagSpecification:public.filename-extension:0 string rustcef" "${PLIST_PATH}"

create_helper() {
    local SUFFIX="$1"
    local HELPER_NAME="${APP_NAME} Helper${SUFFIX}"
    local HELPER_DIR="${FRAMEWORKS_DIR}/${HELPER_NAME}.app"
    
    echo "   Creating ${HELPER_NAME}..."
    mkdir -p "${HELPER_DIR}/Contents/MacOS"
    
    # Create copy of main executable (Symlinks are rejected by codesign)
    # Codesign error: "the main executable or Info.plist must be a regular file (no symlinks, etc.)"
    cp "${BUNDLE_DIR}/Contents/MacOS/${MAIN_EXEC_NAME}" \
       "${HELPER_DIR}/Contents/MacOS/${HELPER_NAME}"
    
    # Sanitize Bundle ID (replace spaces with dots)
    # "(GPU)" -> ".gpu"
    CLEAN_SUFFIX=$(echo "$SUFFIX" | tr -d '()' | tr '[:upper:]' '[:lower:]' | tr -d ' ')
    if [ -n "$CLEAN_SUFFIX" ]; then
        BUNDLE_ID="com.rustcef.helper.${CLEAN_SUFFIX}"
    else
        BUNDLE_ID="com.rustcef.helper"
    fi

    # Create Info.plist
    cat > "${HELPER_DIR}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>${HELPER_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleName</key>
    <string>${HELPER_NAME}</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>LSUIElement</key>
    <true/>
</dict>
</plist>
EOF
}

# Create all 4 Helper variants
create_helper ""
create_helper " (GPU)"
create_helper " (Plugin)"
create_helper " (Renderer)"

# 6. Code Sign Everything
echo "🔐 Signing Application..."

# Sign Frameworks first
echo "   Signing Frameworks..."
codesign --force --sign - "${FRAMEWORKS_DIR}/Chromium Embedded Framework.framework"

# Sign each Helper App
echo "   Signing Helper Apps..."
# Sign each Helper App
echo "   Signing Helper Apps..."
find "${FRAMEWORKS_DIR}" -name "Rust CEF Helper*.app" -type d -maxdepth 1 | while read -r helper_app; do
    echo "   Signing $helper_app..."
    codesign --force --sign - --entitlements Helper.entitlements "$helper_app"
done

# Sign Main App
echo "   Signing Main App..."
codesign --force --sign - --entitlements Entitlements.plist "${BUNDLE_DIR}"

# Preserve the artifact
echo "✅ Packaging Complete!"
echo "   App: ${BUNDLE_DIR}"

# 7. Create DMG
echo "💿 Creating DMG..."
DMG_NAME="target/release/${APP_NAME}.dmg"
rm -f "$DMG_NAME"
hdiutil create -volname "${APP_NAME}" -srcfolder "${BUNDLE_DIR}" -ov -format UDZO "$DMG_NAME"

echo "✅ DMG Created!"
echo "   DMG: $DMG_NAME"
