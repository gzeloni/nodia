// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Recursive-descent parser for the Nodia grammar.

use crate::ast::{
    AssignTarget, BinaryOp, Expr, ForBinding, FuncParam, MatchArm, MatchPattern, Program, Stmt,
    StructField, UnaryOp, UseTarget,
};
use crate::error::{NodiaError, NodiaResult};
use crate::regex as regex_api;
use crate::regex::{
    RegexAnchor, RegexBacktrackingVerb, RegexCharSet, RegexCharSetItem, RegexClass, RegexCondition,
    RegexFlag, RegexGroupKind, RegexLookaroundKind, RegexNode, RegexPattern, RegexQuantifierKind,
    RegexQuantifierMode, RegexReference,
};
use crate::token::{Token, TokenKind};
use crate::value::Value;

mod expressions;
mod helpers;
mod regex;
mod statements;

/// Parser over an already-tokenized Nodia source stream.
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Creates a parser from a token stream.
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Parses the full input as a program.
    pub fn parse_program(&mut self) -> NodiaResult<Program> {
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() {
            statements.push(self.statement()?);
            self.consume_statement_end();
            self.skip_newlines();
        }
        Ok(Program { statements })
    }

    /// Parses a single expression and rejects trailing tokens.
    pub fn parse_expression_only(&mut self) -> NodiaResult<Expr> {
        self.skip_separators();
        let expr = self.expression()?;
        self.skip_separators();
        if !self.is_at_end() {
            return Err(self.error_here("unexpected token after expression"));
        }
        Ok(expr)
    }
}

#[cfg(test)]
mod tests;
