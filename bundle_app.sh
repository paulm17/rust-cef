#!/bin/bash
set -e

echo "Bundling macOS dev app via xtask..."
cargo run -p xtask -- bundle-dev-macos
