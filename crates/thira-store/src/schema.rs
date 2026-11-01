use rusqlite::Connection;

use crate::StoreError;

/// `PRAGMA application_id` of every Thira project: "THRA" in ASCII.
pub const APPLICATION_ID: i32 = 0x5448_5241;

/// Current schema version, stored in `PRAGMA user_version`.
pub const SCHEMA_VERSION: i64 = MIGRATIONS.len() as i64;

/// `MIGRATIONS[i]` upgrades a database from version `i` to `i + 1`.
///
/// Never edit a released migration; append a new one. Rational values are
/// stored as `*_num` / `*_den` integer pairs; IDs as UUID text.
const MIGRATIONS: &[&str] = &[
    // 1: initial schema.
    "
    CREATE TABLE project (
        id TEXT NOT NULL,
        name TEXT NOT NULL,
        width INTEGER NOT NULL,
        height INTEGER NOT NULL,
        pixel_aspect_num INTEGER NOT NULL,
        pixel_aspect_den INTEGER NOT NULL,
        fps_num INTEGER NOT NULL,
        fps_den INTEGER NOT NULL,
        sample_rate INTEGER NOT NULL
    );
    CREATE TABLE media (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        path TEXT NOT NULL,
        relative_path TEXT,
        kind TEXT NOT NULL,
        duration_num INTEGER,
        duration_den INTEGER,
        video_width INTEGER,
        video_height INTEGER,
        video_fps_num INTEGER,
        video_fps_den INTEGER,
        audio_channels INTEGER,
        audio_sample_rate INTEGER
    );
    CREATE TABLE timeline (
        id TEXT PRIMARY KEY,
        position INTEGER NOT NULL,
        name TEXT NOT NULL,
        width INTEGER NOT NULL,
        height INTEGER NOT NULL,
        pixel_aspect_num INTEGER NOT NULL,
        pixel_aspect_den INTEGER NOT NULL,
        fps_num INTEGER NOT NULL,
        fps_den INTEGER NOT NULL,
        sample_rate INTEGER NOT NULL
    );
    CREATE TABLE track (
        id TEXT PRIMARY KEY,
        timeline_id TEXT NOT NULL REFERENCES timeline(id),
        kind TEXT NOT NULL,
        position INTEGER NOT NULL,
        name TEXT NOT NULL
    );
    CREATE TABLE clip (
        id TEXT PRIMARY KEY,
        track_id TEXT NOT NULL REFERENCES track(id),
        media_id TEXT NOT NULL REFERENCES media(id),
        start_num INTEGER NOT NULL,
        start_den INTEGER NOT NULL,
        source_in_num INTEGER NOT NULL,
        source_in_den INTEGER NOT NULL,
        duration_num INTEGER NOT NULL,
        duration_den INTEGER NOT NULL,
        enabled INTEGER NOT NULL
    );
    CREATE INDEX clip_by_track ON clip(track_id, start_num, start_den);
    ",
];

/// Brings a database up to [`SCHEMA_VERSION`], starting from its current version.
/// A new, empty database starts at version 0.
pub(crate) fn migrate(conn: &mut Connection) -> Result<(), StoreError> {
    let current = user_version(conn)?;
    if current > SCHEMA_VERSION {
        return Err(StoreError::TooNew {
            found: current,
            supported: SCHEMA_VERSION,
        });
    }
    let from =
        usize::try_from(current).map_err(|_| StoreError::corrupt("negative schema version"))?;
    for (version, sql) in (1_i64..).zip(MIGRATIONS).skip(from) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", version)?;
        tx.commit()?;
    }
    Ok(())
}

pub(crate) fn user_version(conn: &Connection) -> Result<i64, StoreError> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

pub(crate) fn application_id(conn: &Connection) -> Result<i32, StoreError> {
    Ok(conn.pragma_query_value(None, "application_id", |row| row.get(0))?)
}
