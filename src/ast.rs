use crate::regex::RegexPattern;
use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Comment(String),
    Use {
        path: String,
        alias: Option<String>,
        pick: Vec<String>,
        hide: Vec<String>,
    },
    Bind {
        name: String,
        value: Expr,
        mutable: bool,
    },
    Assign {
        target: AssignTarget,
        value: Expr,
    },
    Func {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Emit(Expr),
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Vec<Stmt>,
    },
    For {
        binding: ForBinding,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    Break,
    Continue,
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    Identifier(String),
    Get {
        object: Box<AssignTarget>,
        field: String,
    },
    Index {
        object: Box<AssignTarget>,
        index: Expr,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ForBinding {
    Single(String),
    Pair { key: String, value: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Value),
    Regex(RegexPattern),
    Identifier(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Get {
        object: Box<Expr>,
        field: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    List(Vec<Expr>),
    Map(Vec<(String, Expr)>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}
