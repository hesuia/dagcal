use crate::persistence::EngineSnapshot;

/// Snapshot history isolated from calculation and dependency state.
#[derive(Debug, Default)]
pub(super) struct History {
    undo: Vec<EngineSnapshot>,
    redo: Vec<EngineSnapshot>,
}

impl History {
    pub(super) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub(super) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub(super) fn record(&mut self, snapshot: EngineSnapshot) {
        self.undo.push(snapshot);
        self.redo.clear();
    }

    pub(super) fn take_undo(&mut self, current: EngineSnapshot) -> Option<EngineSnapshot> {
        let previous = self.undo.pop()?;
        self.redo.push(current);
        Some(previous)
    }

    pub(super) fn take_redo(&mut self, current: EngineSnapshot) -> Option<EngineSnapshot> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        Some(next)
    }

    pub(super) fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}
