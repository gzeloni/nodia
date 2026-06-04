use crate::ast::{AssignTarget, BinaryOp, Expr, ForBinding, Program, Stmt, UnaryOp, UseTarget};
use crate::error::{DobraError, DobraResult};
use crate::regex::{
    RegexAnchor, RegexCharSet, RegexCharSetItem, RegexClass, RegexFlag, RegexGroupKind,
    RegexLookaroundKind, RegexNode, RegexPattern, RegexQuantifierKind, RegexQuantifierMode,
    RegexReference,
};
use crate::token::{Token, TokenKind};
use crate::value::Value;

mod expressions;
mod helpers;
mod regex;
mod statements;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> DobraResult<Program> {
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() {
            statements.push(self.statement()?);
            self.consume_statement_end();
            self.skip_newlines();
        }
        Ok(Program { statements })
    }

    pub fn parse_expression_only(&mut self) -> DobraResult<Expr> {
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
