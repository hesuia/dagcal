use std::fmt;
use std::str::FromStr;

use crate::error::DagcalError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntryLabel {
    Result(usize),
    Named(String),
}

impl EntryLabel {
    /// Parses an entry label from user-facing text.
    ///
    /// Accepts result labels such as `$1` and named labels matching the parser's identifier
    /// syntax. Returns a parse error for empty, malformed, or zero-indexed result labels.
    pub fn parse(input: &str) -> Result<Self, DagcalError> {
        input.parse()
    }

    /// Creates a named label after validating identifier syntax.
    pub fn named(input: impl Into<String>) -> Result<Self, DagcalError> {
        let input = input.into();
        if is_valid_named_label(&input) {
            Ok(Self::Named(input))
        } else {
            Err(DagcalError::Parse(format!("invalid entry label `{input}`")))
        }
    }

    /// Creates a result label from a 1-based result index.
    pub fn result(index: usize) -> Self {
        assert!(index > 0, "result labels must be 1-based");
        Self::Result(index)
    }
}

impl fmt::Display for EntryLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Result(index) => write!(f, "${index}"),
            Self::Named(name) => f.write_str(name),
        }
    }
}

impl FromStr for EntryLabel {
    type Err = DagcalError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        if let Some(digits) = input.strip_prefix('$') {
            let index = digits
                .parse::<usize>()
                .map_err(|_| DagcalError::Parse(format!("invalid entry label `{input}`")))?;
            if index == 0 {
                return Err(DagcalError::Parse(format!("invalid entry label `{input}`")));
            }
            return Ok(Self::Result(index));
        }

        Self::named(input)
    }
}

pub(crate) fn is_valid_named_label(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}
