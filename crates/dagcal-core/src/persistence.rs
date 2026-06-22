use serde::{Deserialize, Serialize};

/// Current on-disk/in-memory snapshot format version.
///
/// [`EngineSnapshot`] values with a different version are rejected by
/// [`Engine::restore_snapshot`](crate::Engine::restore_snapshot).
pub const ENGINE_SNAPSHOT_VERSION: u32 = 1;

/// Serializable representation of the entries in an [`Engine`](crate::Engine).
///
/// A snapshot stores only stable entry identity and source text. Computed
/// values, errors, dependency graph state, runtime constants, and user
/// functions are rebuilt or supplied by the receiving engine during restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineSnapshot {
    /// Snapshot schema version.
    pub version: u32,
    /// Persisted entries. Restore validates IDs and names before replacing
    /// engine state.
    pub entries: Vec<PersistedEntry>,
}

impl EngineSnapshot {
    /// Creates a snapshot using the current [`ENGINE_SNAPSHOT_VERSION`].
    pub fn new(entries: Vec<PersistedEntry>) -> Self {
        Self {
            version: ENGINE_SNAPSHOT_VERSION,
            entries,
        }
    }
}

/// Serializable entry record used inside [`EngineSnapshot`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedEntry {
    /// Stable 1-based expression ID. This corresponds to display reference
    /// `$id` and must be nonzero when restored.
    pub id: usize,
    /// Optional user-facing entry name.
    pub name: Option<String>,
    /// Stored expression source text for this entry.
    pub source: String,
}
