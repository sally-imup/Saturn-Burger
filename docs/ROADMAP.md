# Saturn roadmap

| Phase | Scope | Status |
|---|---|---|
| 0 | Rust desktop foundation, Saturn UI, Windows CI | Complete |
| 1 | Local/offline account profiles and persistent UUIDs | Implemented; awaiting CI confirmation |
| 2 | Mojang version manifest, cache, search, category classifier | Implemented; awaiting CI confirmation |
| 3 | Download queue, retries, resume, checksums, isolated version installation | Next |
| 4 | Java detection, runtime downloads, optional minimized custom JRE | Planned |
| 5 | Minecraft launch command, arguments, process lifecycle | Planned |
| 6 | Integration testing, polish, release packaging | Planned |

## Phase 2 acceptance criteria

- Fetch the official `version_manifest_v2.json` from Mojang's piston-meta endpoint.
- Cache the parsed manifest in `%APPDATA%\\Saturn\\metadata`.
- Load cached metadata without requiring a network connection.
- Keep release, snapshot, April Fools, pre-release/RC, old beta, old alpha, and experimental categories separate.
- Provide version ID search and category filtering.
- Perform network refresh on a background thread so the UI remains responsive.
- Classify April Fools using detailed metadata `series_id` when available, plus a curated list of known IDs for the manifest-only stage.
