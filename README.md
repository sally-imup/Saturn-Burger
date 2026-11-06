# RustLauncher

A modular Minecraft Java launcher for Windows, written in Rust.

> **Current milestone: Phase 0 — Foundation**
>
> This build contains the desktop application shell, navigation, initial storage layout, version-category UI outline, and Windows GitHub Actions workflow. It does **not** install or launch Minecraft yet.

## Phase 0 features

- Rust desktop application using `eframe` / `egui`.
- Windows x64 CI workflow that runs formatting, Clippy, tests, release build, and packages an executable ZIP.
- Navigation shell: Home, Versions, Downloads, Accounts, Runtime, and Settings.
- Separate planned version groups: Vanilla / Release, Snapshot, April Fools, Pre-release / RC, Old Beta, Old Alpha, and Experimental / Custom.
- Initial application data directory layout under `%APPDATA%\RustLauncher` (with `%LOCALAPPDATA%` fallback).
- Modular source layout reserved for accounts, version metadata, downloads, installation, runtime management, and launching.

## Not implemented yet

| Feature | Planned phase |
|---|---:|
| Local/offline profiles | 1 |
| Official version manifest and category classifier | 2 |
| Download queue, retry/resume, checksums and isolated installation | 3 |
| Java runtime detection, runtime downloads and optional minimized custom JRE | 4 |
| Minecraft launch command and process lifecycle | 5 |
| Integration tests, polish and release packaging | 6 |

The Runtime page is a placeholder. No custom JVM is built in Phase 0. The later runtime plan is to manage compatible Java distributions and optionally produce minimized JRE images (for example with `jlink`), rather than writing a new JVM from scratch.

## Requirements

- Windows 10/11 x64
- Rust stable toolchain
- Cargo

## Build locally

```powershell
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Executable output:

```text
target\release\rustlauncher.exe
```

Cargo will generate `Cargo.lock` on the first build. Commit that file to the repository to pin dependency resolution for subsequent builds.

## GitHub Actions

The workflow at `.github/workflows/build-windows.yml` runs on pushes to `main`, pull requests, and manual dispatch. After a successful run, download the `RustLauncher-windows-x64` artifact from the workflow run's **Artifacts** section.

## Data directory

```text
%APPDATA%\RustLauncher\
├── config\
├── accounts\
├── metadata\
├── cache\
│   ├── assets\
│   ├── libraries\
│   └── runtimes\
├── instances\
│   ├── vanilla\
│   ├── snapshot\
│   ├── april-fools\
│   ├── pre-release\
│   ├── old-beta\
│   ├── old-alpha\
│   └── experimental\
└── logs\
```

Shared cache directories are intended for identical assets and libraries. Each installed game version will have its own instance directory so saves and configuration do not get mixed between categories or versions.

## Disclaimer

RustLauncher is an independent, community project and is not affiliated with, endorsed by, or associated with Mojang Studios or Microsoft. Minecraft game files are not included; later installer phases will retrieve required files from their appropriate sources.
