// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Abstract syntax tree nodes used by parsing, checking, formatting, and runtime evaluation.

use crate::regex::RegexPattern;
use crate::value::Value;

/// Parsed Nodia program made of top-level statements.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Statements in source order.
    pub statements: Vec<Stmt>,
}

/// Source of a `use` statement.
#[derive(Debug, Clone, PartialEq)]
pub enum UseTarget {
    /// Relative or absolute module path resolved from source text.
    Path(String),
    /// Built-in standard-library namespace.
    Stdlib(String),
}

/// Statement node in the Nodia AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Preserved source comment.
    Comment(String),
    /// Module import or namespace binding.
    Use {
        target: UseTarget,
        alias: Option<String>,
        pick: Vec<String>,
        hide: Vec<String>,
    },
    /// Immutable or mutable binding declaration.
    Bind {
        name: String,
        value: Expr,
        mutable: bool,
    },
    /// Assignment to an existing binding, field, or index.
    Assign { target: AssignTarget, value: Expr },
    /// Named function declaration.
    Func {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    /// Function return with an optional value.
    Return(Option<Expr>),
    /// Output emission appended to the runtime buffer.
    Emit(Expr),
    /// Conditional branch.
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Vec<Stmt>,
    },
    /// Iteration over list, map, or string values.
    For {
        binding: ForBinding,
        iterable: Expr,
        body: Vec<Stmt>,
    },
    /// Loop that re-evaluates a condition before each iteration.
    While { condition: Expr, body: Vec<Stmt> },
    /// Loop break.
    Break,
    /// Loop continue.
    Continue,
    /// Standalone expression statement.
    Expr(Expr),
}

/// Writable target accepted by assignment statements.
#[derive(Debug, Clone, PartialEq)]
pub enum AssignTarget {
    /// Simple identifier assignment.
    Identifier(String),
    /// Field assignment on a nested object target.
    Get {
        object: Box<AssignTarget>,
        field: String,
    },
    /// Indexed assignment on a nested object target.
    Index {
        object: Box<AssignTarget>,
        index: Expr,
    },
}

/// Binding shape used by `for` statements.
#[derive(Debug, Clone, PartialEq)]
pub enum ForBinding {
    /// Single loop variable.
    Single(String),
    /// Key and value loop variables when iterating maps.
    Pair { key: String, value: String },
}

/// Expression node in the Nodia AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Already-typed literal runtime value.
    Literal(Value),
    /// String literal with optional interpolation markers.
    String { value: String, interpolate: bool },
    /// Anonymous function value.
    Lambda {
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    /// Native regex DSL literal.
    Regex(RegexPattern),
    /// Identifier lookup.
    Identifier(String),
    /// Prefix unary expression.
    Unary { op: UnaryOp, expr: Box<Expr> },
    /// Binary operator expression.
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    /// Function or callable invocation.
    Call { callee: Box<Expr>, args: Vec<Expr> },
    /// Field access on a value.
    Get { object: Box<Expr>, field: String },
    /// Indexed access on a value.
    Index { object: Box<Expr>, index: Box<Expr> },
    /// List literal.
    List(Vec<Expr>),
    /// Map literal.
    Map(Vec<(String, Expr)>),
}

/// Unary operators supported by the language.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    /// Numeric negation.
    Negate,
    /// Logical negation.
    Not,
}

/// Binary operators supported by the language.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    /// Addition or concatenation.
    Add,
    /// Subtraction.
    Subtract,
    /// Multiplication.
    Multiply,
    /// Division.
    Divide,
    /// Modulo.
    Modulo,
    /// Equality comparison.
    Equal,
    /// Inequality comparison.
    NotEqual,
    /// Strict less-than comparison.
    Less,
    /// Less-than or equal comparison.
    LessEqual,
    /// Strict greater-than comparison.
    Greater,
    /// Greater-than or equal comparison.
    GreaterEqual,
    /// Logical conjunction.
    And,
    /// Logical disjunction.
    Or,
}
