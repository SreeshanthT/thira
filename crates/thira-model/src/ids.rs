use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        ///
        /// A UUIDv7, so new IDs sort roughly by creation time.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates a new unique ID.
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

id_type!(
    /// Identifies a [`Project`](crate::Project).
    ProjectId
);
id_type!(
    /// Identifies a [`MediaRef`](crate::MediaRef) in a project's media pool.
    MediaId
);
id_type!(
    /// Identifies a [`Timeline`](crate::Timeline).
    TimelineId
);
id_type!(
    /// Identifies a [`Track`](crate::Track).
    TrackId
);
id_type!(
    /// Identifies a [`Clip`](crate::Clip).
    ClipId
);
