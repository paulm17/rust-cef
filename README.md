# Rust + CEF Desktop Application

Rust + CEF desktop shell with a typed frontend bridge, embedded `app://` assets for production, and a dev workflow based on Bun/Vite.

## Status

This repo has completed the low-, medium-, and high-feature implementation phases from the current roadmap. The remaining work is the packaging/release phase:

- Windows MSI
- macOS DMG
- Linux packages
- signing / notarization
- updater work after packaging is stable

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
- CEF sandbox is enabled in production builds

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
chmod +x bundle_app.sh
./bundle_app.sh

cargo run -- --dev
```

Useful variants:

```bash
RUST_LOG=debug cargo run -- --dev
cargo run -- --dev --devtools
```

### Release Build

```bash
cd frontend
bun run build
cd ..

cargo build --release
./bundle_app.sh
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

## Packaging Phase

The next phase is release engineering rather than feature work:

1. Windows MSI output
2. macOS `.app` + DMG output
3. Linux `.deb` / `.rpm` / AppImage output
4. signing and notarization pipeline
5. updater integration after installers are stable

## Verification

Current automated verification:

```bash
cargo test
```

Manual verification is still important for:

- packaged production launch
- sandboxed production behavior
- global shortcut firing
- notification UX
- installer/signing output

## License

MIT
