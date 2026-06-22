use serde::{Deserialize, Serialize};

pub const ENGINE_SNAPSHOT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineSnapshot {
    pub version: u32,
    pub entries: Vec<PersistedEntry>,
}

impl EngineSnapshot {
    pub fn new(entries: Vec<PersistedEntry>) -> Self {
        Self {
            version: ENGINE_SNAPSHOT_VERSION,
            entries,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedEntry {
    pub id: usize,
    pub name: Option<String>,
    pub source: String,
}
