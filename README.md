# Rust + CEF Desktop Application

Rust + CEF desktop shell with a typed frontend bridge, embedded `app://` assets for production, and a dev workflow based on Bun/Vite.

Packaging is now split into a reusable library crate at [crates/rust-cef-packager](/Volumes/Data/Users/paul/development/src/github/rust-cef/crates/rust-cef-packager) and a thin workspace CLI at [crates/xtask](/Volumes/Data/Users/paul/development/src/github/rust-cef/crates/xtask).

## Status

This repo has completed the low-, medium-, and high-feature implementation phases from the current roadmap. Packaging is now wired through reusable workspace tooling:

- macOS `.app` + `.dmg` packaging is implemented and manually verified
- Windows MSI / NSIS packaging commands are implemented through `cargo-packager`
- Linux `.deb` / AppImage / Pacman packaging commands are implemented through `cargo-packager`
- signing / notarization hooks are configurable through environment variables and CI
- updater work is still deferred until installers are stable

See [roadmap.md](/Volumes/Data/Users/paul/development/src/github/rust-cef/roadmap.md) for the detailed matrix.

## Implemented Features

### Low

- text clipboard read/write/clear
- structured logging with `tracing`
- devtools auto-open flag
- dev-server startup and hot reload flow

### Medium

- print to PDF
- download manager
- error reporting
- HTTPS / startup URL policy
- permission denial by default
- deep linking and file association handoff
- global shortcuts
- rich notifications
- streamed file URLs over `app://`
- icon set generation

### High

- event-capable CEF IPC bridge
- binary payload helpers for frontend IPC
- bidirectional Rust to JS events
- single-instance lock with launch handoff
- image clipboard read/write

## Security Model

### Production

- frontend is expected to load from `app://localhost/...`
- production URL policy only allows `app://` and `about:blank`
- insecure dev-only browser flags are not enabled
- remote debugging is not enabled
- CEF sandbox is currently disabled by default
- set `RUST_CEF_ENABLE_SANDBOX=1` only after helper entitlements and sandbox-compatible packaging are fully validated

Relevant code:

- [app.rs](/Volumes/Data/Users/paul/development/src/github/rust-cef/src/app.rs)
- [security.rs](/Volumes/Data/Users/paul/development/src/github/rust-cef/src/security.rs)
- [lib.rs](/Volumes/Data/Users/paul/development/src/github/rust-cef/src/lib.rs)

### Development

- `cargo run -- --dev` loads the frontend from `http://localhost:5173`
- dev mode allows loopback HTTP plus a few permissive browser flags so Vite hot reload works
- `--devtools` or `RUST_CEF_OPEN_DEVTOOLS=1` opens CEF DevTools automatically

### Audit Notes

The main remaining isolation caveat is architectural rather than configuration:

- any XSS inside an `app://` page still has access to the native IPC bridge via `window.cefQuery`

That means `app://` isolation is now materially stronger than before, but frontend integrity and XSS prevention still matter.

## Prerequisites

- Rust stable
- Bun
- macOS is the primary tested target right now

## Quick Start

### Dev Mode

```bash
cd frontend
bun install
cd ..

cargo build
cargo run -p xtask -- bundle-dev-macos

cargo run -- --dev
```

Useful variants:

```bash
RUST_LOG=debug cargo run -- --dev
cargo run -- --dev --devtools
```

### Release Build

```bash
./package.sh --os mac
```

`./package.sh --help` shows the full interface:

```bash
./package.sh --os <mac|windows|linux> [--format <name>]...
```

Supported `--format` values:

- `app`
- `dmg`
- `wix`
- `nsis`
- `deb`
- `appimage`
- `pacman`

Defaults:

- `./package.sh --os mac` => `app + dmg`
- `./package.sh --os windows` => `wix`
- `./package.sh --os linux` => `deb + appimage + pacman`

Common packaging commands:

```bash
./package.sh --os mac --format app
./package.sh --os mac --format dmg
./package.sh --os windows --format wix
./package.sh --os windows --format nsis
./package.sh --os linux --format deb
./package.sh --os linux --format appimage --format pacman
```

## Frontend Bridge

Use the wrappers from [rust-api.ts](/Volumes/Data/Users/paul/development/src/github/rust-cef/frontend/src/rust-api.ts).

Example:

```ts
import { invoke, RustClipboard, RustOS, RustFileSystem } from './rust-api';

const info = await invoke<{ name: string; version: string }>('get_app_info');
await RustClipboard.writeText('hello');
const launch = await RustOS.getLaunchContext();
const stream = await RustFileSystem.createFileStreamUrl('/absolute/path/to/file.pdf');
```

The demo app in [App.tsx](/Volumes/Data/Users/paul/development/src/github/rust-cef/frontend/src/App.tsx) includes test controls for:

- shortcuts
- notifications
- streamed file URLs
- image clipboard
- app-event polling

## Production vs Dev URLs

- dev: `http://localhost:5173`
- production: `app://localhost/index.html`

The custom scheme handler is implemented in [scheme_handler.rs](/Volumes/Data/Users/paul/development/src/github/rust-cef/src/platform/scheme_handler.rs).

## Packaging

Current packaging coverage:

1. macOS `.app` + `.dmg` via `./package.sh --os mac`
2. Windows MSI via `./package.sh --os windows --format wix`
3. optional Windows NSIS installer via `./package.sh --os windows --format nsis`
4. Linux `.deb`, AppImage, and Pacman outputs via `./package.sh --os linux`
5. signing and notarization environment hooks plus CI matrix in [release-packaging.yml](/Volumes/Data/Users/paul/development/src/github/rust-cef/.github/workflows/release-packaging.yml)

`rpm` is not listed because the installed `cargo-packager` CLI on this machine does not expose an RPM format.

The current macOS packaging implementation lives in:

- [crates/rust-cef-packager](/Volumes/Data/Users/paul/development/src/github/rust-cef/crates/rust-cef-packager)
- [crates/xtask](/Volumes/Data/Users/paul/development/src/github/rust-cef/crates/xtask)

The legacy shell entrypoints [bundle_app.sh](/Volumes/Data/Users/paul/development/src/github/rust-cef/bundle_app.sh) and [package.sh](/Volumes/Data/Users/paul/development/src/github/rust-cef/package.sh) now delegate to `xtask` so the bundle logic stays in one reusable Rust implementation.

Packaging details and environment variables are documented in [packaging/README.md](/Volumes/Data/Users/paul/development/src/github/rust-cef/packaging/README.md).

## Verification

Current automated verification:

```bash
cargo test
```

Manual verification is still important for:

- packaged production launch
- sandbox-enabled production behavior if you opt into `RUST_CEF_ENABLE_SANDBOX=1`
- global shortcut firing
- notification UX
- installer/signing output on native platform runners

## License

MIT
