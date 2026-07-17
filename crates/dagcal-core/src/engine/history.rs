use std::collections::VecDeque;

/// Bounded, linear history with undo and redo navigation.
#[derive(Debug)]
pub(super) struct History<T> {
    items: VecDeque<T>,
    current: usize,
    limit: usize,
}

impl<T> History<T> {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            items: VecDeque::with_capacity(limit.saturating_add(1)),
            current: 0,
            limit,
        }
    }

    pub(super) fn can_undo(&self) -> bool {
        self.current > 0
    }

    pub(super) fn can_redo(&self) -> bool {
        self.current + 1 < self.items.len()
    }

    pub(super) fn record(&mut self, previous: T, next: T) {
        if self.limit == 0 {
            return;
        }

        if self.items.is_empty() {
            self.items.push_back(previous);
        } else {
            self.items.truncate(self.current + 1);
        }
        self.items.push_back(next);
        self.current = self.items.len() - 1;

        while self.items.len() > self.limit + 1 {
            self.items.pop_front();
            self.current -= 1;
        }
    }

    pub(super) fn take_undo(&mut self) -> Option<T>
    where
        T: Clone,
    {
        if !self.can_undo() {
            return None;
        }

        self.current -= 1;
        self.items.get(self.current).cloned()
    }

    pub(super) fn take_redo(&mut self) -> Option<T>
    where
        T: Clone,
    {
        if !self.can_redo() {
            return None;
        }

        self.current += 1;
        self.items.get(self.current).cloned()
    }

    pub(super) fn clear(&mut self) {
        self.items.clear();
        self.current = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::History;

    #[test]
    fn supports_undo_redo_for_generic_values() {
        let mut history = History::new(3);
        history.record(0, 1);
        history.record(1, 2);

        assert_eq!(history.take_undo(), Some(1));
        assert_eq!(history.take_undo(), Some(0));
        assert_eq!(history.take_undo(), None);
        assert_eq!(history.take_redo(), Some(1));
        assert_eq!(history.take_redo(), Some(2));
        assert_eq!(history.take_redo(), None);
    }

    #[test]
    fn drops_oldest_generic_values_at_the_limit() {
        let mut history = History::new(2);
        history.record("a", "b");
        history.record("b", "c");
        history.record("c", "d");

        assert_eq!(history.take_undo(), Some("c"));
        assert_eq!(history.take_undo(), Some("b"));
        assert_eq!(history.take_undo(), None);
    }

    #[test]
    fn new_records_clear_generic_redo_branch() {
        let mut history = History::new(3);
        history.record(0, 1);
        history.record(1, 2);
        assert_eq!(history.take_undo(), Some(1));

        history.record(1, 3);

        assert_eq!(history.take_redo(), None);
        assert_eq!(history.take_undo(), Some(1));
    }
}
