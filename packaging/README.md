# Packaging Guide

This repo packages through the reusable library crate [crates/rust-cef-packager](/Volumes/Data/Users/paul/development/src/github/rust-cef/crates/rust-cef-packager) and the thin CLI [crates/xtask](/Volumes/Data/Users/paul/development/src/github/rust-cef/crates/xtask).

## Commands

Build frontend first:

```bash
cd frontend
bun install
bun run build
cd ..
```

Then choose the platform command:

```bash
cargo run -p xtask -- package-macos
cargo run -p xtask -- package-windows-msi
cargo run -p xtask -- package-windows-nsis
cargo run -p xtask -- package-linux
```

## Output Formats

- macOS: `.app` and `.dmg`
- Windows: `.msi` through WiX, optional NSIS `.exe`
- Linux: `.deb`, `.AppImage`, and Pacman package output

`cargo-packager` does not expose an `.rpm` format in the locally installed CLI, so Linux packaging in this repo targets the formats it does support.

## macOS Signing And Notarization

Environment overrides for `package-macos`:

```bash
export RUST_CEF_SIGNING_IDENTITY="Developer ID Application: Example Corp (TEAMID)"
export RUST_CEF_MAIN_ENTITLEMENTS="/absolute/path/to/Entitlements.plist"
export RUST_CEF_HELPER_ENTITLEMENTS="/absolute/path/to/Helper.entitlements"
export RUST_CEF_DMG_NAME="Rust CEF.dmg"
```

For `cargo-packager` notarization, provide one of these credential sets:

```bash
export APPLE_ID="name@example.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="TEAMID"
```

or:

```bash
export APPLE_API_KEY="KEYID"
export APPLE_API_ISSUER="ISSUER-ID"
export APPLE_API_KEY_PATH="/absolute/path/to/AuthKey_KEYID.p8"
```

## Windows Requirements

- run `package-windows-msi` on a Windows runner or host
- install WiX Toolset for MSI output
- use `cargo run -p xtask -- package-windows-nsis` if you want an NSIS installer instead of MSI

## Linux Requirements

- run `package-linux` on a Linux runner or host
- ensure the system packages required by `cargo-packager` are present for `.deb`, AppImage, and Pacman packaging

## CI

The release matrix workflow is defined in [release-packaging.yml](/Volumes/Data/Users/paul/development/src/github/rust-cef/.github/workflows/release-packaging.yml). It provides per-platform packaging jobs and artifact upload hooks.
