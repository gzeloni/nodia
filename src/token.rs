// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Lexical token definitions for the Nodia grammar.

use std::fmt;

/// Token annotated with its starting source position.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Token category and literal payload, when applicable.
    pub kind: TokenKind,
    /// One-based line number.
    pub line: usize,
    /// One-based column number.
    pub column: usize,
}

impl Token {
    /// Creates a token at the given source position.
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self { kind, line, column }
    }
}

/// Token kinds recognized by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Val,
    Var,
    Func,
    LegacyLet,
    LegacyConst,
    LegacyFn,
    Return,
    Emit,
    If,
    Else,
    For,
    In,
    While,
    Break,
    Continue,
    True,
    False,
    Null,
    And,
    Or,
    Not,
    LegacyImport,
    From,
    As,
    Pick,
    LegacyShow,
    Hide,
    Lambda,
    Match,
    Case,
    Default,
    Try,
    Catch,
    Throw,
    Defer,
    Type,
    Enum,
    Struct,
    Namespace,
    Use,
    Regex,
    Identifier(String),
    Int(i64),
    Float(f64),
    String(String),
    RawString(String),
    Bytes(Vec<u8>),
    Comment(String),
    Plus,
    Minus,
    PlusEqual,
    MinusEqual,
    Star,
    Slash,
    Percent,
    Ampersand,
    Pipe,
    Caret,
    Tilde,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LeftShift,
    RightShift,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,
    Newline,
    Eof,
}

impl TokenKind {
    /// Returns the stable display name for this token kind.
    pub fn name(&self) -> &'static str {
        match self {
            TokenKind::Val => "Val",
            TokenKind::Var => "Var",
            TokenKind::Func => "Func",
            TokenKind::LegacyLet => "LegacyLet",
            TokenKind::LegacyConst => "LegacyConst",
            TokenKind::LegacyFn => "LegacyFn",
            TokenKind::Return => "Return",
            TokenKind::Emit => "Emit",
            TokenKind::If => "If",
            TokenKind::Else => "Else",
            TokenKind::For => "For",
            TokenKind::In => "In",
            TokenKind::While => "While",
            TokenKind::Break => "Break",
            TokenKind::Continue => "Continue",
            TokenKind::True => "True",
            TokenKind::False => "False",
            TokenKind::Null => "Null",
            TokenKind::And => "And",
            TokenKind::Or => "Or",
            TokenKind::Not => "Not",
            TokenKind::LegacyImport => "LegacyImport",
            TokenKind::From => "From",
            TokenKind::As => "As",
            TokenKind::Pick => "Pick",
            TokenKind::LegacyShow => "LegacyShow",
            TokenKind::Hide => "Hide",
            TokenKind::Lambda => "Lambda",
            TokenKind::Match => "Match",
            TokenKind::Case => "Case",
            TokenKind::Default => "Default",
            TokenKind::Try => "Try",
            TokenKind::Catch => "Catch",
            TokenKind::Throw => "Throw",
            TokenKind::Defer => "Defer",
            TokenKind::Type => "Type",
            TokenKind::Enum => "Enum",
            TokenKind::Struct => "Struct",
            TokenKind::Namespace => "Namespace",
            TokenKind::Use => "Use",
            TokenKind::Regex => "Regex",
            TokenKind::Identifier(_) => "Identifier",
            TokenKind::Int(_) => "Int",
            TokenKind::Float(_) => "Float",
            TokenKind::String(_) => "String",
            TokenKind::RawString(_) => "RawString",
            TokenKind::Bytes(_) => "Bytes",
            TokenKind::Comment(_) => "Comment",
            TokenKind::Plus => "Plus",
            TokenKind::Minus => "Minus",
            TokenKind::PlusEqual => "PlusEqual",
            TokenKind::MinusEqual => "MinusEqual",
            TokenKind::Star => "Star",
            TokenKind::Slash => "Slash",
            TokenKind::Percent => "Percent",
            TokenKind::Ampersand => "Ampersand",
            TokenKind::Pipe => "Pipe",
            TokenKind::Caret => "Caret",
            TokenKind::Tilde => "Tilde",
            TokenKind::Equal => "Equal",
            TokenKind::EqualEqual => "EqualEqual",
            TokenKind::BangEqual => "BangEqual",
            TokenKind::Less => "Less",
            TokenKind::LessEqual => "LessEqual",
            TokenKind::Greater => "Greater",
            TokenKind::GreaterEqual => "GreaterEqual",
            TokenKind::LeftShift => "LeftShift",
            TokenKind::RightShift => "RightShift",
            TokenKind::LeftParen => "LeftParen",
            TokenKind::RightParen => "RightParen",
            TokenKind::LeftBrace => "LeftBrace",
            TokenKind::RightBrace => "RightBrace",
            TokenKind::LeftBracket => "LeftBracket",
            TokenKind::RightBracket => "RightBracket",
            TokenKind::Comma => "Comma",
            TokenKind::Dot => "Dot",
            TokenKind::Colon => "Colon",
            TokenKind::Semicolon => "Semicolon",
            TokenKind::Newline => "Newline",
            TokenKind::Eof => "Eof",
        }
    }

    /// Returns the literal payload when the token carries one.
    pub fn literal(&self) -> Option<String> {
        match self {
            TokenKind::Identifier(value)
            | TokenKind::String(value)
            | TokenKind::RawString(value)
            | TokenKind::Comment(value) => Some(value.clone()),
            TokenKind::Bytes(value) => Some(format!("{value:?}")),
            TokenKind::Int(value) => Some(value.to_string()),
            TokenKind::Float(value) => Some(value.to_string()),
            _ => None,
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(literal) = self.literal() {
            write!(f, "{}({literal:?})", self.name())
        } else {
            write!(f, "{}", self.name())
        }
    }
}

/// Maps a source word to its reserved-keyword token kind.
pub fn keyword_kind(text: &str) -> Option<TokenKind> {
    Some(match text {
        "val" => TokenKind::Val,
        "var" => TokenKind::Var,
        "func" => TokenKind::Func,
        "let" => TokenKind::LegacyLet,
        "const" => TokenKind::LegacyConst,
        "fn" => TokenKind::LegacyFn,
        "return" => TokenKind::Return,
        "emit" => TokenKind::Emit,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "for" => TokenKind::For,
        "in" => TokenKind::In,
        "while" => TokenKind::While,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "null" => TokenKind::Null,
        "and" => TokenKind::And,
        "or" => TokenKind::Or,
        "not" => TokenKind::Not,
        "import" => TokenKind::LegacyImport,
        "from" => TokenKind::From,
        "as" => TokenKind::As,
        "pick" => TokenKind::Pick,
        "show" => TokenKind::LegacyShow,
        "hide" => TokenKind::Hide,
        "lambda" => TokenKind::Lambda,
        "match" => TokenKind::Match,
        "case" => TokenKind::Case,
        "default" => TokenKind::Default,
        "try" => TokenKind::Try,
        "catch" => TokenKind::Catch,
        "throw" => TokenKind::Throw,
        "defer" => TokenKind::Defer,
        "type" => TokenKind::Type,
        "enum" => TokenKind::Enum,
        "struct" => TokenKind::Struct,
        "namespace" => TokenKind::Namespace,
        "use" => TokenKind::Use,
        "regex" => TokenKind::Regex,
        _ => return None,
    })
}
