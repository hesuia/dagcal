use crate::error::{DagcalError, ParseError, ParseErrorKind};
use crate::id::ExpressionId;

#[derive(Debug, Clone)]
pub(super) enum EntryTarget {
    Id(ExpressionId),
    Name(String),
}

impl EntryTarget {
    pub(super) fn parse(input: &str) -> Result<Self, DagcalError> {
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

fn invalid_entry_label(input: &str) -> DagcalError {
    DagcalError::Parse(ParseError::at_input(
        ParseErrorKind::InvalidEntryTarget,
        input,
        format!("invalid entry label `{input}`"),
    ))
}

pub(super) fn is_valid_name(input: &str) -> bool {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}
