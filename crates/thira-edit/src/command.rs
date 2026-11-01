use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thira_model::{
    Clip, ClipId, Format, MediaId, MediaRef, Project, Timeline, TimelineId, Track, TrackId,
    TrackKind,
};

use crate::{Change, EditError};

/// A primitive edit. Each one has an exact inverse, returned when it is applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    SetProjectName {
        name: String,
    },
    SetDefaultFormat {
        format: Format,
    },

    AddMedia {
        media: MediaRef,
    },
    RemoveMedia {
        id: MediaId,
    },
    /// Replaces the media entry with the same ID.
    UpdateMedia {
        media: MediaRef,
    },

    AddTimeline {
        index: usize,
        timeline: Timeline,
    },
    RemoveTimeline {
        id: TimelineId,
    },
    UpdateTimeline {
        id: TimelineId,
        name: String,
        format: Format,
    },

    /// Inserts into the video or audio track list, chosen by `track.kind`.
    AddTrack {
        timeline: TimelineId,
        index: usize,
        track: Track,
    },
    RemoveTrack {
        id: TrackId,
    },
    RenameTrack {
        id: TrackId,
        name: String,
    },

    /// Inserts the clip in start-time order.
    InsertClip {
        track: TrackId,
        clip: Clip,
    },
    RemoveClip {
        track: TrackId,
        id: ClipId,
    },
    /// Replaces the clip with the same ID on `track`, keeping start-time order.
    UpdateClip {
        track: TrackId,
        clip: Clip,
    },
}

/// Commands applied together: either all succeed or none do.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Transaction {
    /// Shown in the undo history, such as "Ripple delete".
    pub label: String,
    pub commands: Vec<Command>,
}

impl Transaction {
    pub fn new(label: impl Into<String>, commands: Vec<Command>) -> Self {
        Self {
            label: label.into(),
            commands,
        }
    }

    /// A transaction holding one command.
    pub fn single(label: impl Into<String>, command: Command) -> Self {
        Self::new(label, vec![command])
    }
}

impl Command {
    /// Applies the command and returns its inverse. Records what changed in `changes`.
    ///
    /// On error the project may be partly modified; [`CommandBus`](crate::CommandBus)
    /// restores it from a snapshot.
    pub(crate) fn apply(
        self,
        project: &mut Project,
        changes: &mut BTreeSet<Change>,
    ) -> Result<Command, EditError> {
        match self {
            Self::SetProjectName { name } => {
                changes.insert(Change::Project);
                let old = std::mem::replace(&mut project.name, name);
                Ok(Self::SetProjectName { name: old })
            }
            Self::SetDefaultFormat { format } => {
                changes.insert(Change::Project);
                let old = std::mem::replace(&mut project.default_format, format);
                Ok(Self::SetDefaultFormat { format: old })
            }

            Self::AddMedia { media } => {
                let id = media.id;
                if project.media.contains_key(&id) {
                    return Err(EditError::exists("media", id));
                }
                Arc::make_mut(&mut project.media).insert(id, media);
                changes.insert(Change::Media(id));
                Ok(Self::RemoveMedia { id })
            }
            Self::RemoveMedia { id } => {
                if !project.media.contains_key(&id) {
                    return Err(EditError::not_found("media", id));
                }
                let media = Arc::make_mut(&mut project.media)
                    .remove(&id)
                    .ok_or_else(|| EditError::not_found("media", id))?;
                changes.insert(Change::Media(id));
                Ok(Self::AddMedia { media })
            }
            Self::UpdateMedia { media } => {
                let id = media.id;
                if !project.media.contains_key(&id) {
                    return Err(EditError::not_found("media", id));
                }
                let slot = Arc::make_mut(&mut project.media)
                    .get_mut(&id)
                    .ok_or_else(|| EditError::not_found("media", id))?;
                let old = std::mem::replace(slot, media);
                changes.insert(Change::Media(id));
                Ok(Self::UpdateMedia { media: old })
            }

            Self::AddTimeline { index, timeline } => {
                let id = timeline.id;
                if project.timelines.iter().any(|t| t.id == id) {
                    return Err(EditError::exists("timeline", id));
                }
                if index > project.timelines.len() {
                    return Err(EditError::IndexOutOfRange { index });
                }
                project.timelines.insert(index, Arc::new(timeline));
                changes.insert(Change::Timeline(id));
                Ok(Self::RemoveTimeline { id })
            }
            Self::RemoveTimeline { id } => {
                let index = timeline_index(project, id)?;
                let timeline = Arc::unwrap_or_clone(project.timelines.remove(index));
                changes.insert(Change::Timeline(id));
                Ok(Self::AddTimeline { index, timeline })
            }
            Self::UpdateTimeline { id, name, format } => {
                let index = timeline_index(project, id)?;
                let timeline = Arc::make_mut(&mut project.timelines[index]);
                let old_name = std::mem::replace(&mut timeline.name, name);
                let old_format = std::mem::replace(&mut timeline.format, format);
                changes.insert(Change::Timeline(id));
                Ok(Self::UpdateTimeline {
                    id,
                    name: old_name,
                    format: old_format,
                })
            }

            Self::AddTrack {
                timeline,
                index,
                track,
            } => {
                let id = track.id;
                if find_track(project, id).is_ok() {
                    return Err(EditError::exists("track", id));
                }
                let ti = timeline_index(project, timeline)?;
                let tracks = track_list(Arc::make_mut(&mut project.timelines[ti]), track.kind);
                if index > tracks.len() {
                    return Err(EditError::IndexOutOfRange { index });
                }
                tracks.insert(index, Arc::new(track));
                changes.extend([Change::Timeline(timeline), Change::Track(id)]);
                Ok(Self::RemoveTrack { id })
            }
            Self::RemoveTrack { id } => {
                let loc = find_track(project, id)?;
                let timeline = Arc::make_mut(&mut project.timelines[loc.timeline]);
                let timeline_id = timeline.id;
                let track = Arc::unwrap_or_clone(track_list(timeline, loc.kind).remove(loc.index));
                changes.extend([Change::Timeline(timeline_id), Change::Track(id)]);
                Ok(Self::AddTrack {
                    timeline: timeline_id,
                    index: loc.index,
                    track,
                })
            }
            Self::RenameTrack { id, name } => {
                let track = track_mut(project, id)?;
                let old = std::mem::replace(&mut track.name, name);
                changes.insert(Change::Track(id));
                Ok(Self::RenameTrack { id, name: old })
            }

            Self::InsertClip { track, clip } => {
                let id = clip.id;
                if find_clip(project, id).is_some() {
                    return Err(EditError::exists("clip", id));
                }
                insert_sorted(&mut track_mut(project, track)?.clips, clip);
                changes.extend([Change::Track(track), Change::Clip(id)]);
                Ok(Self::RemoveClip { track, id })
            }
            Self::RemoveClip { track, id } => {
                let clips = &mut track_mut(project, track)?.clips;
                let index = clips
                    .iter()
                    .position(|c| c.id == id)
                    .ok_or_else(|| EditError::not_found("clip", id))?;
                let clip = clips.remove(index);
                changes.extend([Change::Track(track), Change::Clip(id)]);
                Ok(Self::InsertClip { track, clip })
            }
            Self::UpdateClip { track, clip } => {
                let id = clip.id;
                let clips = &mut track_mut(project, track)?.clips;
                let index = clips
                    .iter()
                    .position(|c| c.id == id)
                    .ok_or_else(|| EditError::not_found("clip", id))?;
                let old = clips.remove(index);
                insert_sorted(clips, clip);
                changes.extend([Change::Track(track), Change::Clip(id)]);
                Ok(Self::UpdateClip { track, clip: old })
            }
        }
    }
}

/// Where a track sits in the project.
struct TrackLocation {
    timeline: usize,
    kind: TrackKind,
    index: usize,
}

fn timeline_index(project: &Project, id: TimelineId) -> Result<usize, EditError> {
    project
        .timelines
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| EditError::not_found("timeline", id))
}

fn track_list(timeline: &mut Timeline, kind: TrackKind) -> &mut Vec<Arc<Track>> {
    match kind {
        TrackKind::Video => &mut timeline.video_tracks,
        TrackKind::Audio => &mut timeline.audio_tracks,
    }
}

fn find_track(project: &Project, id: TrackId) -> Result<TrackLocation, EditError> {
    for (ti, timeline) in project.timelines.iter().enumerate() {
        for (kind, tracks) in [
            (TrackKind::Video, &timeline.video_tracks),
            (TrackKind::Audio, &timeline.audio_tracks),
        ] {
            if let Some(index) = tracks.iter().position(|t| t.id == id) {
                return Ok(TrackLocation {
                    timeline: ti,
                    kind,
                    index,
                });
            }
        }
    }
    Err(EditError::not_found("track", id))
}

/// Mutable access to one track, copying only the timeline and track on its path.
fn track_mut(project: &mut Project, id: TrackId) -> Result<&mut Track, EditError> {
    let loc = find_track(project, id)?;
    let timeline = Arc::make_mut(&mut project.timelines[loc.timeline]);
    Ok(Arc::make_mut(
        &mut track_list(timeline, loc.kind)[loc.index],
    ))
}

fn find_clip(project: &Project, id: ClipId) -> Option<&Clip> {
    project
        .timelines
        .iter()
        .flat_map(|t| t.tracks())
        .flat_map(|t| &t.clips)
        .find(|c| c.id == id)
}

fn insert_sorted(clips: &mut Vec<Clip>, clip: Clip) {
    let at = clips.partition_point(|c| c.start <= clip.start);
    clips.insert(at, clip);
}
