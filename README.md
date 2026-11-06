# Saturn

Saturn is an independent Minecraft Java launcher for Windows, written in Rust. Its visual direction uses a warm Saturn-inspired palette and a Material Design 3-inspired desktop layout.

> **Current milestone: Phase 2 — Version metadata and classifier**

## Implemented

### Phase 0 — Foundation
- Rust + egui desktop application for Windows x64.
- Saturn planetary identity and Material-inspired UI shell.
- Modular core, storage layout, and GitHub Actions workflow.

### Phase 1 — Local profiles
- Create, select, rename, and delete multiple local/offline profiles.
- Stable Java-compatible offline UUIDs using `OfflinePlayer:<username>`.
- JSON persistence with backup and rollback on write failures.
- Minecraft-style username validation.

A local profile is not Microsoft/Mojang authentication. It is intended for single-player and environments that explicitly allow offline-mode identities. Servers requiring authenticated online-mode accounts will reject it. Renaming a profile changes its offline UUID.

### Phase 2 — Version metadata
- Fetches Mojang's official `version_manifest_v2.json` from `piston-meta.mojang.com`.
- Saves the parsed manifest under `%APPDATA%\\Saturn\\metadata` and reuses it offline.
- Refresh runs on a background thread, keeping the desktop UI responsive.
- Search by version ID and browse categories separately.
- Categories: Vanilla/Release, Snapshot, April Fools, Pre-release/RC, Old Beta, Old Alpha, Experimental/Custom.
- April Fools uses known historical IDs now and supports detailed version `series_id` metadata (for example `april2024`) when available.

The manifest's official `type` field alone does not identify all these categories: pre-releases and April Fools releases can appear as `snapshot`. Saturn therefore applies an additional ID/metadata classifier. Detailed per-version metadata fetching is a later enhancement if needed for newly published special versions.

## Planned milestones

| Feature | Phase |
|---|---:|
| Download queue, retry/resume, checksums, isolated installation | 3 |
| Java runtime detection, runtime downloads, optional minimized custom JRE | 4 |
| Minecraft launch command and process lifecycle | 5 |
| Integration tests, polish, release packaging | 6 |

The future lightweight runtime will be a managed/minimized Java runtime (for example, built with `jlink` after compatibility testing), not a new JVM implementation.

## Requirements

- Windows 10/11 x64
- Rust stable toolchain
- Cargo

## Build locally

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

## GitHub Actions

`.github/workflows/build-windows.yml` runs formatting, Clippy, tests, release build, and packages `Saturn.exe` plus documentation. It runs on pushes to `main`, pull requests, and manual dispatch. Download the `Saturn-windows-x64` artifact from a successful workflow run.

## Data directory

```text
%APPDATA%\\Saturn\\
├── config/
├── accounts/
│   ├── local_profiles.json
│   └── local_profiles.json.bak
├── metadata/
│   └── version_manifest_v2.json
├── cache/
│   ├── assets/
│   ├── libraries/
│   └── runtimes/
├── instances/
│   ├── vanilla/
│   ├── snapshot/
│   ├── april-fools/
│   ├── pre-release/
│   ├── old-beta/
│   ├── old-alpha/
│   └── experimental/
└── logs/
```

Minecraft game files are not included. Later installer phases will retrieve required files from their appropriate sources. Saturn is an independent community project and is not affiliated with Mojang Studios or Microsoft.
