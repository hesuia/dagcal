use crate::error::{DagcalError, ParseError, ParseErrorKind};
use crate::id::ExpressionId;
use crate::parser::is_valid_name;

/// Public entry target accepted by convenience engine APIs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntryTarget {
    /// Stable 1-based expression ID.
    Id(ExpressionId),
    /// User-defined entry name.
    Name(String),
}

impl EntryTarget {
    /// Parses `$n` or a valid entry name.
    pub fn parse(input: &str) -> Result<Self, DagcalError> {
        if let Some(digits) = input.strip_prefix('$') {
            let index = digits
                .parse::<usize>()
                .map_err(|_| invalid_entry_label(input))?;
            if index == 0 {
                return Err(invalid_entry_label(input));
            }
            Ok(Self::Id(ExpressionId::new(index)))
        } else if is_valid_name(input) {
            Ok(Self::Name(input.to_string()))
        } else {
            Err(invalid_entry_label(input))
        }
    }
}

impl From<ExpressionId> for EntryTarget {
    fn from(id: ExpressionId) -> Self {
        Self::Id(id)
    }
}

impl TryFrom<&str> for EntryTarget {
    type Error = DagcalError;

    fn try_from(input: &str) -> Result<Self, Self::Error> {
        Self::parse(input)
    }
}

impl TryFrom<String> for EntryTarget {
    type Error = DagcalError;

    fn try_from(input: String) -> Result<Self, Self::Error> {
        Self::parse(&input)
    }
}

/// Conversion trait for convenience engine APIs that accept either IDs or
/// user-facing target strings.
pub trait IntoEntryTarget {
    /// Converts this value into an [`EntryTarget`].
    fn into_entry_target(self) -> Result<EntryTarget, DagcalError>;
}

impl IntoEntryTarget for EntryTarget {
    fn into_entry_target(self) -> Result<EntryTarget, DagcalError> {
        Ok(self)
    }
}

impl IntoEntryTarget for ExpressionId {
    fn into_entry_target(self) -> Result<EntryTarget, DagcalError> {
        Ok(EntryTarget::Id(self))
    }
}

impl IntoEntryTarget for &str {
    fn into_entry_target(self) -> Result<EntryTarget, DagcalError> {
        EntryTarget::parse(self)
    }
}

impl IntoEntryTarget for String {
    fn into_entry_target(self) -> Result<EntryTarget, DagcalError> {
        EntryTarget::parse(&self)
    }
}

impl IntoEntryTarget for &String {
    fn into_entry_target(self) -> Result<EntryTarget, DagcalError> {
        EntryTarget::parse(self)
    }
}

fn invalid_entry_label(input: &str) -> DagcalError {
    DagcalError::Parse(ParseError::at_input(
        ParseErrorKind::InvalidEntryTarget,
        input,
        format!("invalid entry label `{input}`"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_expression_id_targets() {
        assert_eq!(
            EntryTarget::parse("$42").unwrap(),
            EntryTarget::Id(ExpressionId::new(42))
        );
    }

    #[test]
    fn parses_name_targets() {
        assert_eq!(
            EntryTarget::parse("subtotal").unwrap(),
            EntryTarget::Name("subtotal".to_string())
        );
    }

    #[test]
    fn rejects_invalid_targets_as_parse_errors() {
        let err = EntryTarget::parse("$0").unwrap_err();

        match err {
            DagcalError::Parse(err) => {
                assert_eq!(err.kind, ParseErrorKind::InvalidEntryTarget);
                assert_eq!(err.span.unwrap().end.byte, 2);
            }
            other => panic!("expected parse error, got {other:?}"),
        }
    }
}
