#!/bin/bash
set -e

show_help() {
    cat <<'EOF'
Usage: ./package.sh --os <mac|windows|linux> [--format <name>]...

Examples:
  ./package.sh --os mac
  ./package.sh --os mac --format app
  ./package.sh --os mac --format dmg
  ./package.sh --os windows --format wix
  ./package.sh --os windows --format nsis
  ./package.sh --os linux --format deb
  ./package.sh --os linux --format appimage --format pacman

Supported OS values:
  mac
  windows
  linux

Supported format values:
  app
  dmg
  wix
  nsis
  deb
  appimage
  pacman

Defaults:
  --os mac      => app + dmg
  --os windows  => wix
  --os linux    => deb + appimage + pacman

Notes:
  mac packaging uses the custom CEF bundle flow through xtask
  windows packaging must run on Windows
  linux packaging must run on Linux
EOF
}

OS_NAME=""
FORMATS=()

while [ $# -gt 0 ]; do
    case "$1" in
        -h|--help)
            show_help
            exit 0
            ;;
        --os)
            if [ $# -lt 2 ]; then
                echo "Error: --os requires a value"
                exit 1
            fi
            OS_NAME="$2"
            shift 2
            ;;
        --format)
            if [ $# -lt 2 ]; then
                echo "Error: --format requires a value"
                exit 1
            fi
            FORMATS+=("$2")
            shift 2
            ;;
        *)
            echo "Error: unknown argument: $1"
            echo
            show_help
            exit 1
            ;;
    esac
done

if [ -z "$OS_NAME" ]; then
    show_help
    exit 0
fi

validate_formats() {
    local os_name="$1"
    shift
    for format in "$@"; do
        case "$os_name:$format" in
            mac:app|mac:dmg|windows:wix|windows:nsis|linux:deb|linux:appimage|linux:pacman)
                ;;
            *)
                echo "Error: format '$format' is not valid for --os $os_name"
                exit 1
                ;;
        esac
    done
}

if [ ${#FORMATS[@]} -gt 0 ]; then
    validate_formats "$OS_NAME" "${FORMATS[@]}"
fi

echo "📦 Starting Packaging Process..."

echo "🎨 Building Frontend..."
cd frontend
if ! command -v bun >/dev/null 2>&1; then
    echo "❌ bun could not be found. Please install bun."
    exit 1
fi

echo "   Running bun install..."
bun install
echo "   Running bun run build..."
bun run build

if [ ! -d "dist" ]; then
    echo "❌ Frontend build failed: dist directory not found."
    exit 1
fi
echo "✅ Frontend built successfully."
cd ..

echo "🦀 Building Rust Application (Release)..."
cargo build --release

case "$OS_NAME" in
    mac)
        echo "🎁 Packaging macOS artifacts..."
        if [ ${#FORMATS[@]} -eq 0 ]; then
            cargo run -p xtask -- package-macos
        else
            CMD=(cargo run -p xtask -- package-macos)
            for format in "${FORMATS[@]}"; do
                CMD+=(--format "$format")
            done
            "${CMD[@]}"
        fi
        ;;
    windows)
        echo "🎁 Packaging Windows artifacts..."
        if [ ${#FORMATS[@]} -eq 0 ]; then
            cargo run -p xtask -- package-windows-msi
        else
            for format in "${FORMATS[@]}"; do
                case "$format" in
                    wix)
                        cargo run -p xtask -- package-windows-msi
                        ;;
                    nsis)
                        cargo run -p xtask -- package-windows-nsis
                        ;;
                esac
            done
        fi
        ;;
    linux)
        echo "🎁 Packaging Linux artifacts..."
        if [ ${#FORMATS[@]} -eq 0 ]; then
            cargo run -p xtask -- package-linux
        else
            CMD=(cargo run -p xtask -- package-linux)
            for format in "${FORMATS[@]}"; do
                CMD+=(--format "$format")
            done
            "${CMD[@]}"
        fi
        ;;
    *)
        echo "Error: unsupported --os value '$OS_NAME'"
        echo
        show_help
        exit 1
        ;;
esac
