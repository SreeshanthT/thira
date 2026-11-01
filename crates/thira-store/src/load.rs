use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{Connection, OpenFlags, Row};
use thira_model::{
    AudioStream, Clip, ClipId, Format, FrameRate, MediaId, MediaKind, MediaRef, Project, ProjectId,
    Rational, Time, Timeline, TimelineId, Track, TrackId, TrackKind, VideoStream,
};
use uuid::Uuid;

use crate::schema::{self, APPLICATION_ID};
use crate::{StoreError, paths};

/// Loads a project file. The file is opened read-only and never modified;
/// older formats are migrated in memory.
pub fn load(path: &Path) -> Result<Project, StoreError> {
    let path = std::path::absolute(path)?;
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();

    let file = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    match schema::application_id(&file) {
        Ok(APPLICATION_ID) => {}
        Ok(_) => return Err(StoreError::NotAProject),
        Err(StoreError::Sqlite(rusqlite::Error::SqliteFailure(e, _)))
            if e.code == rusqlite::ErrorCode::NotADatabase =>
        {
            return Err(StoreError::NotAProject);
        }
        Err(err) => return Err(err),
    }

    // Copy into memory so migrations never touch the user's file.
    let mut conn = Connection::open_in_memory()?;
    // A step of -1 copies every page at once.
    match rusqlite::backup::Backup::new(&file, &mut conn)?.step(-1)? {
        rusqlite::backup::StepResult::Done => {}
        _ => return Err(StoreError::corrupt("could not read the whole file")),
    }
    drop(file);
    schema::migrate(&mut conn)?;

    let project = read_project(&conn, &dir)?;
    project.validate()?;
    Ok(project)
}

fn read_project(conn: &Connection, dir: &Path) -> Result<Project, StoreError> {
    let (id, name, default_format) = conn
        .query_row("SELECT * FROM project", [], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                raw_format(row, 2)?,
            ))
        })
        .map_err(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => StoreError::corrupt("missing project row"),
            err => err.into(),
        })?;
    let mut project = Project::new(name, format(default_format)?);
    project.id = ProjectId::from_uuid(uuid(&id)?);
    project.media = Arc::new(read_media(conn, dir)?);
    project.timelines = read_timelines(conn)?;
    Ok(project)
}

fn read_media(conn: &Connection, dir: &Path) -> Result<BTreeMap<MediaId, MediaRef>, StoreError> {
    let mut stmt = conn.prepare("SELECT * FROM media")?;
    let mut rows = stmt.query([])?;
    let mut media = BTreeMap::new();
    while let Some(row) = rows.next()? {
        let id = MediaId::from_uuid(uuid(&row.get::<_, String>(0)?)?);
        let stored = PathBuf::from(row.get::<_, String>(2)?);
        let relative: Option<String> = row.get(3)?;
        let kind = match row.get::<_, String>(4)?.as_str() {
            "video" => MediaKind::Video,
            "audio" => MediaKind::Audio,
            "image" => MediaKind::Image,
            other => return Err(StoreError::corrupt(format!("unknown media kind {other:?}"))),
        };
        let duration = optional_rational(row.get(5)?, row.get(6)?)?.map(Time::from_seconds);
        let video = match (row.get::<_, Option<u32>>(7)?, row.get::<_, Option<u32>>(8)?) {
            (Some(width), Some(height)) => Some(VideoStream {
                width,
                height,
                frame_rate: optional_rational(row.get(9)?, row.get(10)?)?
                    .map(frame_rate)
                    .transpose()?,
            }),
            _ => None,
        };
        let audio = match (
            row.get::<_, Option<u16>>(11)?,
            row.get::<_, Option<u32>>(12)?,
        ) {
            (Some(channels), Some(sample_rate)) => Some(AudioStream {
                channels,
                sample_rate,
            }),
            _ => None,
        };
        let path = locate(stored, relative.as_deref(), dir);
        media.insert(
            id,
            MediaRef {
                id,
                name: row.get(1)?,
                path,
                kind,
                duration,
                video,
                audio,
            },
        );
    }
    Ok(media)
}

/// The stored absolute path if it exists, else the project-relative one if that exists
/// (the project folder moved with its media), else the stored path for relinking.
fn locate(stored: PathBuf, relative: Option<&str>, dir: &Path) -> PathBuf {
    if stored.exists() {
        return stored;
    }
    relative
        .map(|rel| paths::resolve(dir, rel))
        .filter(|p| p.exists())
        .unwrap_or(stored)
}

fn read_timelines(conn: &Connection) -> Result<Vec<Arc<Timeline>>, StoreError> {
    let mut tracks = read_tracks(conn)?;
    let mut stmt = conn.prepare("SELECT * FROM timeline ORDER BY position")?;
    let mut rows = stmt.query([])?;
    let mut timelines = Vec::new();
    while let Some(row) = rows.next()? {
        let id = TimelineId::from_uuid(uuid(&row.get::<_, String>(0)?)?);
        let mut timeline = Timeline::new(row.get::<_, String>(2)?, format(raw_format(row, 3)?)?);
        timeline.id = id;
        for (kind, track) in tracks.remove(&id).unwrap_or_default() {
            match kind {
                TrackKind::Video => timeline.video_tracks.push(Arc::new(track)),
                TrackKind::Audio => timeline.audio_tracks.push(Arc::new(track)),
            }
        }
        timelines.push(Arc::new(timeline));
    }
    if !tracks.is_empty() {
        return Err(StoreError::corrupt("tracks refer to a missing timeline"));
    }
    Ok(timelines)
}

/// Tracks grouped by timeline, in position order, with their clips.
fn read_tracks(
    conn: &Connection,
) -> Result<HashMap<TimelineId, Vec<(TrackKind, Track)>>, StoreError> {
    let mut clips = read_clips(conn)?;
    let mut stmt =
        conn.prepare("SELECT id, timeline_id, kind, name FROM track ORDER BY position")?;
    let mut rows = stmt.query([])?;
    let mut tracks: HashMap<TimelineId, Vec<(TrackKind, Track)>> = HashMap::new();
    while let Some(row) = rows.next()? {
        let id = TrackId::from_uuid(uuid(&row.get::<_, String>(0)?)?);
        let timeline = TimelineId::from_uuid(uuid(&row.get::<_, String>(1)?)?);
        let kind = match row.get::<_, String>(2)?.as_str() {
            "video" => TrackKind::Video,
            "audio" => TrackKind::Audio,
            other => return Err(StoreError::corrupt(format!("unknown track kind {other:?}"))),
        };
        let mut track = Track::new(kind, row.get::<_, String>(3)?);
        track.id = id;
        track.clips = clips.remove(&id).unwrap_or_default();
        track.clips.sort_by_key(|c| c.start);
        tracks.entry(timeline).or_default().push((kind, track));
    }
    if !clips.is_empty() {
        return Err(StoreError::corrupt("clips refer to a missing track"));
    }
    Ok(tracks)
}

fn read_clips(conn: &Connection) -> Result<HashMap<TrackId, Vec<Clip>>, StoreError> {
    let mut stmt = conn.prepare("SELECT * FROM clip")?;
    let mut rows = stmt.query([])?;
    let mut clips: HashMap<TrackId, Vec<Clip>> = HashMap::new();
    while let Some(row) = rows.next()? {
        let track = TrackId::from_uuid(uuid(&row.get::<_, String>(1)?)?);
        let time = |n: usize| -> Result<Time, StoreError> {
            Ok(Time::from_seconds(rational(row.get(n)?, row.get(n + 1)?)?))
        };
        clips.entry(track).or_default().push(Clip {
            id: ClipId::from_uuid(uuid(&row.get::<_, String>(0)?)?),
            media: MediaId::from_uuid(uuid(&row.get::<_, String>(2)?)?),
            start: time(3)?,
            source_in: time(5)?,
            duration: time(7)?,
            enabled: row.get(9)?,
        });
    }
    Ok(clips)
}

/// Format columns as stored, starting at column `at`.
struct RawFormat {
    width: u32,
    height: u32,
    pixel_aspect: (i64, i64),
    fps: (i64, i64),
    sample_rate: u32,
}

fn raw_format(row: &Row<'_>, at: usize) -> rusqlite::Result<RawFormat> {
    Ok(RawFormat {
        width: row.get(at)?,
        height: row.get(at + 1)?,
        pixel_aspect: (row.get(at + 2)?, row.get(at + 3)?),
        fps: (row.get(at + 4)?, row.get(at + 5)?),
        sample_rate: row.get(at + 6)?,
    })
}

fn format(raw: RawFormat) -> Result<Format, StoreError> {
    Ok(Format {
        width: raw.width,
        height: raw.height,
        pixel_aspect: rational(raw.pixel_aspect.0, raw.pixel_aspect.1)?,
        frame_rate: frame_rate(rational(raw.fps.0, raw.fps.1)?)?,
        sample_rate: raw.sample_rate,
    })
}

fn rational(num: i64, den: i64) -> Result<Rational, StoreError> {
    Rational::new(num, den).map_err(|e| StoreError::corrupt(format!("{num}/{den}: {e}")))
}

fn optional_rational(num: Option<i64>, den: Option<i64>) -> Result<Option<Rational>, StoreError> {
    match (num, den) {
        (Some(n), Some(d)) => rational(n, d).map(Some),
        (None, None) => Ok(None),
        _ => Err(StoreError::corrupt("half of a rational value is missing")),
    }
}

fn frame_rate(fps: Rational) -> Result<FrameRate, StoreError> {
    FrameRate::new(fps)
        .ok_or_else(|| StoreError::corrupt(format!("frame rate {fps} is not positive")))
}

fn uuid(text: &str) -> Result<Uuid, StoreError> {
    Uuid::parse_str(text).map_err(|e| StoreError::corrupt(format!("bad ID {text:?}: {e}")))
}
