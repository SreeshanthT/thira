use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, Transaction, params};
use thira_model::{Format, MediaKind, Project, Rational, Time, TrackKind};

use crate::schema::{self, APPLICATION_ID};
use crate::{StoreError, paths};

/// Saves `project` to `path` atomically (ADR 0002).
///
/// Writes the whole project to `<path>.tmp`, flushes it to disk, then renames it
/// over `path`. If the process dies at any point, `path` holds either the previous
/// save or this one, never a mix.
pub fn save(project: &Project, path: &Path) -> Result<(), StoreError> {
    let path = std::path::absolute(path)?;
    let dir = path
        .parent()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "project path has no folder",
            )
        })?
        .to_path_buf();
    let tmp = temp_path(&path);

    match fs::remove_file(&tmp) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err.into()),
        _ => {}
    }
    write_database(project, &tmp, &dir)?;

    // SQLite already syncs on commit with synchronous=FULL; this also covers the
    // file's metadata before it becomes visible under the real name.
    OpenOptions::new().write(true).open(&tmp)?.sync_all()?;
    // Atomic replace on every OS (MoveFileExW with MOVEFILE_REPLACE_EXISTING on Windows).
    fs::rename(&tmp, &path)?;
    #[cfg(unix)]
    fs::File::open(&dir)?.sync_all()?;
    Ok(())
}

/// `My Film.thira` → `My Film.thira.tmp`, in the same directory so the rename is atomic.
fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

fn write_database(project: &Project, file: &Path, dir: &Path) -> Result<(), StoreError> {
    let mut conn = Connection::open(file)?;
    conn.pragma_update(None, "application_id", APPLICATION_ID)?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    conn.pragma_update(None, "foreign_keys", true)?;
    schema::migrate(&mut conn)?;

    let tx = conn.transaction()?;
    write_project(&tx, project, dir)?;
    tx.commit()?;
    conn.close().map_err(|(_, err)| err)?;
    Ok(())
}

fn write_project(tx: &Transaction<'_>, project: &Project, dir: &Path) -> Result<(), StoreError> {
    let f = project.default_format;
    let (pa, fps) = (f.pixel_aspect, f.frame_rate.as_rational());
    tx.execute(
        "INSERT INTO project VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            project.id.to_string(),
            project.name,
            f.width,
            f.height,
            pa.numer(),
            pa.denom(),
            fps.numer(),
            fps.denom(),
            f.sample_rate
        ],
    )?;

    let mut insert = tx.prepare(
        "INSERT INTO media VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )?;
    for media in project.media.values() {
        let path = media
            .path
            .to_str()
            .ok_or_else(|| StoreError::NonUnicodePath(media.path.clone()))?;
        let video_fps = media
            .video
            .and_then(|v| v.frame_rate)
            .map(|r| r.as_rational());
        insert.execute(params![
            media.id.to_string(),
            media.name,
            path,
            paths::relative(dir, &media.path),
            media_kind(media.kind),
            media.duration.map(|d| d.seconds().numer()),
            media.duration.map(|d| d.seconds().denom()),
            media.video.map(|v| v.width),
            media.video.map(|v| v.height),
            video_fps.map(Rational::numer),
            video_fps.map(Rational::denom),
            media.audio.map(|a| a.channels),
            media.audio.map(|a| a.sample_rate),
        ])?;
    }

    let mut insert_timeline =
        tx.prepare("INSERT INTO timeline VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)")?;
    let mut insert_track = tx.prepare("INSERT INTO track VALUES (?1, ?2, ?3, ?4, ?5)")?;
    let mut insert_clip =
        tx.prepare("INSERT INTO clip VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)")?;
    for (position, timeline) in (0_i64..).zip(&project.timelines) {
        let f: Format = timeline.format;
        let (pa, fps) = (f.pixel_aspect, f.frame_rate.as_rational());
        insert_timeline.execute(params![
            timeline.id.to_string(),
            position,
            timeline.name,
            f.width,
            f.height,
            pa.numer(),
            pa.denom(),
            fps.numer(),
            fps.denom(),
            f.sample_rate
        ])?;
        for (kind, tracks) in [
            (TrackKind::Video, &timeline.video_tracks),
            (TrackKind::Audio, &timeline.audio_tracks),
        ] {
            for (position, track) in (0_i64..).zip(tracks.iter()) {
                insert_track.execute(params![
                    track.id.to_string(),
                    timeline.id.to_string(),
                    track_kind(kind),
                    position,
                    track.name
                ])?;
                for clip in &track.clips {
                    let (s, i, d) = (secs(clip.start), secs(clip.source_in), secs(clip.duration));
                    insert_clip.execute(params![
                        clip.id.to_string(),
                        track.id.to_string(),
                        clip.media.to_string(),
                        s.numer(),
                        s.denom(),
                        i.numer(),
                        i.denom(),
                        d.numer(),
                        d.denom(),
                        clip.enabled
                    ])?;
                }
            }
        }
    }
    Ok(())
}

fn secs(t: Time) -> Rational {
    t.seconds()
}

pub(crate) fn media_kind(kind: MediaKind) -> &'static str {
    match kind {
        MediaKind::Video => "video",
        MediaKind::Audio => "audio",
        MediaKind::Image => "image",
    }
}

pub(crate) fn track_kind(kind: TrackKind) -> &'static str {
    match kind {
        TrackKind::Video => "video",
        TrackKind::Audio => "audio",
    }
}
