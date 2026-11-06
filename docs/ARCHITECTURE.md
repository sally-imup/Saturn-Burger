# Architecture — Phase 0

## Application layers

- `app.rs`: creates the application state and drives the eframe update loop.
- `ui/`: desktop navigation and page presentation only.
- `core/`: game-launcher services. Modules are placeholders until their scheduled phase.
- `storage/`: centralized application-data path and directory layout.

## Planned service boundaries

```text
UI
 ├── Accounts UI ─────── core::account
 ├── Version UI ──────── core::versions
 ├── Downloads UI ────── core::downloads
 ├── Installer UI ────── core::installer
 ├── Runtime UI ──────── core::runtime
 └── Launch UI ───────── core::launcher

Shared storage
 ├── metadata/
 ├── cache/assets/
 ├── cache/libraries/
 ├── cache/runtimes/
 └── instances/<category>/<version>/
```

## Runtime direction

The planned runtime manager will map each Minecraft version to a compatible Java major version, obtain a supported runtime, validate modules and native dependencies, and expose launch-time JVM options. An optional minimized runtime may be generated with `jlink` after compatibility testing. This project does not attempt to implement the Java Virtual Machine itself.
