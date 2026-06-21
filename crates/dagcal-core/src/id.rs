use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExpressionId(usize);

impl ExpressionId {
    /// Creates an expression ID from a 1-based sequential value.
    pub fn new(value: usize) -> Self {
        assert!(value > 0, "expression IDs must be 1-based");
        Self(value)
    }

    /// Returns the 1-based numeric value for this expression ID.
    pub fn value(self) -> usize {
        self.0
    }
}

impl fmt::Display for ExpressionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "${}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct ExpressionIdGenerator {
    next: usize,
}

impl Default for ExpressionIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpressionIdGenerator {
    /// Creates a generator that starts allocating IDs at 1.
    pub fn new() -> Self {
        Self { next: 1 }
    }

    /// Allocates the next sequential expression ID.
    pub fn next(&mut self) -> ExpressionId {
        let id = ExpressionId::new(self.next);
        self.next += 1;
        id
    }

    /// Ensures future allocations are greater than or equal to `value + 1`.
    pub fn reserve_through(&mut self, value: usize) {
        self.next = self.next.max(value + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expression_id_displays_as_dollar_result_reference() {
        assert_eq!(ExpressionId::new(1).to_string(), "$1");
        assert_eq!(ExpressionId::new(42).to_string(), "$42");
    }
}
