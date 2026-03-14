# Changelog

## 2.0.0 (2026-03-14)

### Initial Rust Release

This is the first release of **stackql-deploy** as a native Rust binary, replacing the Python implementation.

**Key changes from v1.x (Python):**
- Complete rewrite in Rust — single static binary, no Python runtime required
- Same CLI interface: `build`, `test`, `teardown`, `init`, `info`, `shell`, `upgrade`, `plan`
- Multi-platform binaries: Linux x86_64/ARM64, macOS Apple Silicon/Intel, Windows x86_64
- Available on [crates.io](https://crates.io/crates/stackql-deploy) via `cargo install stackql-deploy`

**The Python package (v1.x) is now archived.** See the [Python package changelog](https://github.com/stackql/stackql-deploy/blob/main/CHANGELOG.md) for the v1.x release history (last Python release: v1.9.4).
