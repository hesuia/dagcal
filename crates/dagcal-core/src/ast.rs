use std::collections::BTreeSet;

use crate::id::ExpressionId;
use crate::number::Number;

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedExpr {
    Number(Number),
    Reference(ParsedReference),
    Unary {
        op: UnaryOp,
        rhs: Box<ParsedExpr>,
    },
    Binary {
        lhs: Box<ParsedExpr>,
        op: BinaryOp,
        rhs: Box<ParsedExpr>,
    },
    Call {
        name: String,
        args: Vec<ParsedExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParsedStatement {
    Expression(ParsedExpr),
    Definition { name: String, expr: ParsedExpr },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParsedReference {
    Id(ExpressionId),
    Name(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
}

#[cfg(test)]
impl ParsedExpr {
    pub fn references(&self) -> BTreeSet<ParsedReference> {
        let mut refs = BTreeSet::new();
        self.collect_references(&mut refs);
        refs
    }

    fn collect_references(&self, refs: &mut BTreeSet<ParsedReference>) {
        match self {
            ParsedExpr::Number(_) => {}
            ParsedExpr::Reference(name) => {
                refs.insert(name.clone());
            }
            ParsedExpr::Unary { rhs, .. } => rhs.collect_references(refs),
            ParsedExpr::Binary { lhs, rhs, .. } => {
                lhs.collect_references(refs);
                rhs.collect_references(refs);
            }
            ParsedExpr::Call { args, .. } => {
                for arg in args {
                    arg.collect_references(refs);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedExpr {
    Number(Number),
    EntryReference(ExpressionId),
    Constant(String),
    Unary {
        op: UnaryOp,
        rhs: Box<ResolvedExpr>,
    },
    Binary {
        lhs: Box<ResolvedExpr>,
        op: BinaryOp,
        rhs: Box<ResolvedExpr>,
    },
    Call {
        name: String,
        args: Vec<ResolvedExpr>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExpressionAnalysis {
    pub(crate) entry_references: BTreeSet<ExpressionId>,
    pub(crate) constant_references: BTreeSet<String>,
    pub(crate) function_references: BTreeSet<String>,
}

impl ResolvedExpr {
    pub(crate) fn analyze(&self) -> ExpressionAnalysis {
        let mut analysis = ExpressionAnalysis::default();
        self.collect_analysis(&mut analysis);
        analysis
    }

    fn collect_analysis(&self, analysis: &mut ExpressionAnalysis) {
        match self {
            ResolvedExpr::Number(_) => {}
            ResolvedExpr::Constant(name) => {
                analysis.constant_references.insert(name.clone());
            }
            ResolvedExpr::EntryReference(id) => {
                analysis.entry_references.insert(*id);
            }
            ResolvedExpr::Unary { rhs, .. } => rhs.collect_analysis(analysis),
            ResolvedExpr::Binary { lhs, rhs, .. } => {
                lhs.collect_analysis(analysis);
                rhs.collect_analysis(analysis);
            }
            ResolvedExpr::Call { name, args } => {
                analysis.function_references.insert(name.clone());
                for arg in args {
                    arg.collect_analysis(analysis);
                }
            }
        }
    }
}
