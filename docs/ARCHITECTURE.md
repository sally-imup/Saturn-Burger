# Saturn architecture — Phase 1

## Application layers

- `app.rs`: creates application state and drives the eframe UI loop.
- `ui/`: Saturn theme, Material-inspired navigation and page presentation.
- `core/account/`: local profile validation, offline UUID generation, persistence and selection.
- `core/`: version, download, installer, runtime and launch services added incrementally.
- `storage/`: centralized `%APPDATA%\\Saturn` path and directory layout.

## Local profile flow

```text
Accounts UI
    │
    ▼
LocalAccountStore
    ├── validate Minecraft-style username
    ├── generate Java-compatible offline UUID
    ├── select / rename / delete profile
    └── persist schema-versioned JSON
             │
             ▼
%APPDATA%\\Saturn\\accounts\\local_profiles.json
```

Profile updates are written to a temporary file and replace the previous file through a backup/rename sequence. A malformed or unsupported profile file is not silently overwritten.

## Offline UUID compatibility

The UUID is generated as the MD5 digest of UTF-8 bytes for `OfflinePlayer:<username>`, then the UUID version and variant bits are set to version 3 / RFC variant. This matches the legacy Java `UUID.nameUUIDFromBytes` algorithm used by Minecraft offline-mode player identities.

## Planned service boundaries

```text
UI
 ├── Accounts UI ───── core::account (Phase 1)
 ├── Version UI ─────── core::versions (Phase 2)
 ├── Downloads UI ──── core::downloads (Phase 3)
 ├── Installer UI ──── core::installer (Phase 3)
 ├── Runtime UI ────── core::runtime (Phase 4)
 └── Launch UI ─────── core::launcher (Phase 5)

Shared storage
 ├── accounts/local_profiles.json
 ├── metadata/
 ├── cache/assets/
 ├── cache/libraries/
 ├── cache/runtimes/
 └── instances/<category>/<version>/
```
