# Saturn

Saturn is an independent Minecraft Java launcher for Windows, written in Rust. Its visual direction uses a warm Saturn-inspired palette and a Material Design 3-inspired desktop layout.

> **Current milestone: Phase 1 — Local profiles**

## Phase 1 features

- Saturn branding and planetary color theme.
- Material-inspired navigation rail, top app bar, rounded surface cards, and tonal controls.
- Create multiple local/offline player profiles.
- Select the active local profile and remember it across restarts.
- Rename and delete profiles.
- Generate Java-compatible offline UUIDs using the `OfflinePlayer:<username>` name-based MD5 algorithm.
- Persist profile data in `accounts/local_profiles.json`.
- Validate Minecraft-style usernames (3–16 ASCII letters, numbers, or underscores).
- Windows x64 GitHub Actions workflow: formatting, Clippy, unit tests, release build, executable ZIP artifact.

## Important local-account limitation

A local profile is an offline identity only. It does not sign in to Microsoft, Mojang, or Xbox services and does not authenticate the player. It is intended for single-player and environments that explicitly permit offline-mode identities. Servers requiring authenticated online-mode accounts will reject it.

Renaming a profile changes its offline UUID because Minecraft derives offline UUIDs from the player name. An offline server may therefore treat the renamed player as a new identity. Deleting a Saturn profile does not delete Minecraft worlds or instance files.

## Planned milestones

| Feature | Phase |
|---|---:|
| Project foundation, desktop UI, initial storage layout, Windows CI | 0 |
| Local/offline profile management | 1 — current |
| Official version manifest and category classifier | 2 |
| Download queue, retry/resume, checksums and isolated installation | 3 |
| Java runtime detection, runtime downloads and optional minimized custom JRE | 4 |
| Minecraft launch command and process lifecycle | 5 |
| Integration tests, polish and release packaging | 6 |

The future lightweight runtime will be a managed/minimized Java runtime (for example, built with `jlink` after compatibility testing), not a new Java Virtual Machine implementation.

## Requirements

- Windows 10/11 x64
- Rust stable toolchain compatible with eframe 0.37
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
target\release\saturn-launcher.exe
```

## GitHub Actions

The workflow at `.github/workflows/build-windows.yml` runs on pushes to `main`, pull requests, and manual dispatch. After a successful run, download the `Saturn-windows-x64` artifact from the workflow run's **Artifacts** section.

## Data directory

Saturn stores its data in `%APPDATA%\Saturn` (falling back to `%LOCALAPPDATA%\Saturn`):

```text
Saturn/
├── config/
├── accounts/
│   └── local_profiles.json
├── metadata/
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

Shared cache directories are intended for identical assets and libraries. Each installed game version will have its own instance directory so saves and configuration do not get mixed between categories or versions.

## Disclaimer

Saturn is an independent community project and is not affiliated with, endorsed by, or associated with Mojang Studios or Microsoft. Minecraft game files are not included; later installer phases will retrieve required files from their appropriate sources.
