use crate::error::DagcalError;
use crate::id::ExpressionId;
use crate::label::EntryLabel;

#[derive(Debug, Clone)]
pub(super) enum EntryTarget {
    Id(ExpressionId),
    Name(String),
}

impl EntryTarget {
    pub(super) fn parse(input: &str) -> Result<Self, DagcalError> {
        match EntryLabel::parse(input)? {
            EntryLabel::Result(index) => Ok(Self::Id(ExpressionId::new(index))),
            EntryLabel::Named(name) => Ok(Self::Name(name)),
        }
    }
}
