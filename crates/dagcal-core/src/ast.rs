use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Reference(String),
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
    pub fn references(&self) -> BTreeSet<String> {
        let mut refs = BTreeSet::new();
        self.collect_references(&mut refs);
        refs
    }

    fn collect_references(&self, refs: &mut BTreeSet<String>) {
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
