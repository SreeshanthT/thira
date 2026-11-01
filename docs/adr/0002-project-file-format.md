# 0002. Project file format: a single SQLite file

| | |
| --- | --- |
| Status | Accepted |
| Date | 2026-11-01 |
| Issue | #1 |

## Context

PRD open question 1 asked whether a project is a single file or a folder. The PRD already fixes SQLite with versioned migrations as the storage engine (section 10) and requires:

- no corruption on forced exit, with atomic project writes (section 8, Reliability);
- autosave and crash recovery to within 30 seconds (PRJ-2, G2);
- older projects opening in newer versions (PRJ-5);
- relinking media after a folder moves (PRJ-4).

Users also email, back up and move projects, so a project should be easy to treat as one thing.

## Decision

### One file per project

A project is a single SQLite database with the extension `.thira`, for example `My Film.thira`. It contains only the project model: media references, timelines, tracks, clips and settings. Nothing in it is derived data.

- `PRAGMA application_id = 0x54485241` ("THRA") marks the file as a Thira project, so Thira can reject other SQLite files and tools like `file` can identify it.
- `PRAGMA user_version` holds the schema version. Opening an older file runs migrations in memory and on the next save; a file newer than the running build is refused with a clear message rather than opened.
- The schema uses normal tables (media, timelines, tracks, clips) rather than a single JSON blob, so the format can be documented and queried (PRJ-5). The schema itself is defined in #9.

### Saving is atomic

The editor keeps the project in memory ([`thira-edit`](../../crates/thira-edit)) and does not hold the file open for writing between saves. To save:

1. Write the complete project to a new database, `My Film.thira.tmp`, in the same folder.
2. Flush it to disk.
3. Atomically replace `My Film.thira` with it (`rename`, which on Windows is `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING`).

A crash at any point leaves either the old file or the new one, never a mix. The saved file uses SQLite's default rollback journal, not WAL, so no `-wal` or `-shm` sidecar files are left next to it. A project of a few thousand clips writes in well under a second, so rewriting the whole file each save is acceptable; incremental saves can be added later without changing the format.

While a project is open, Thira holds an advisory lock on the `.thira` file, so a second Thira instance opens it read-only instead of overwriting it.

### Media paths

Each media reference stores both the absolute path and the path relative to the `.thira` file. On open, if the absolute path is missing, Thira tries the relative path. Moving a project folder together with its media therefore needs no relinking; other moves use the relink flow (PRJ-4).

### Autosave and recovery

- Autosave writes the same format to the app data folder: `<data dir>/Thira/autosave/<project-id>/`, at most every 30 seconds while there are unsaved changes, keeping the last few copies.
- Untitled projects autosave the same way.
- A clean close deletes that project's autosaves. If autosaves exist and are newer than the project file when it is opened, Thira offers to recover them (PRJ-2).

### Caches

Thumbnails, waveforms and proxies are derived data and never go in the project file. They live in the OS cache folder, `<cache dir>/Thira/`, keyed by a fingerprint of the source media, so projects that use the same file share them. The cache is size-capped and can be deleted at any time; everything in it is regenerated on demand.

| | Windows | macOS | Linux |
| --- | --- | --- | --- |
| Data dir | `%APPDATA%` | `~/Library/Application Support` | `$XDG_DATA_HOME` (`~/.local/share`) |
| Cache dir | `%LOCALAPPDATA%` | `~/Library/Caches` | `$XDG_CACHE_HOME` (`~/.cache`) |

## Alternatives considered

- **Project folder** (`My Film.thira/` holding the database, autosaves and caches). Caches would travel with the project, but the folder is heavier to share, easy to damage by editing files inside it, and appears as a single item only on macOS. Rejected.
- **Zipped single file.** SQLite cannot be modified inside a zip, so every save would unpack and rewrite the archive. Slower, and atomicity would depend on our zip handling instead of SQLite. Rejected.
- **Editing the `.thira` file in place with WAL.** Saves would be incremental, but the file would be inconsistent with what the user sees until they save, sidecar files would appear next to it, and copying the project while Thira runs could miss data. Rejected for now.

## Consequences

- One file to share or back up; caches never bloat it.
- Saves are always whole and atomic, which keeps crash safety simple to test (#9).
- Moving a project to another machine loses its caches; they regenerate in the background (MED-4).
- The store crate needs platform code for atomic replace on Windows and for file locking.
