use std::collections::BTreeSet;

use crate::id::ExpressionId;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Reference(Reference),
    Unary {
        op: UnaryOp,
        rhs: Box<Expr>,
    },
    Binary {
        lhs: Box<Expr>,
        op: BinaryOp,
        rhs: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expression(Expr),
    Definition { name: String, expr: Expr },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reference {
    Id(ExpressionId),
    Name(String),
}

impl Reference {
    pub fn display_name(&self) -> String {
        match self {
            Self::Id(id) => format!("${}", id.value()),
            Self::Name(name) => name.clone(),
        }
    }
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

impl Expr {
    pub fn references(&self) -> BTreeSet<Reference> {
        let mut refs = BTreeSet::new();
        self.collect_references(&mut refs);
        refs
    }

    fn collect_references(&self, refs: &mut BTreeSet<Reference>) {
        match self {
            Expr::Number(_) => {}
            Expr::Reference(name) => {
                refs.insert(name.clone());
            }
            Expr::Unary { rhs, .. } => rhs.collect_references(refs),
            Expr::Binary { lhs, rhs, .. } => {
                lhs.collect_references(refs);
                rhs.collect_references(refs);
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    arg.collect_references(refs);
                }
            }
        }
    }
}
