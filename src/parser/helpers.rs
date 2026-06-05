// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Shared parser helpers for token handling and diagnostics.

use super::*;

impl Parser {
    pub(super) fn assign_target_from_expr(&self, expr: Expr) -> Result<AssignTarget, &'static str> {
        match expr {
            Expr::Identifier(name) => Ok(AssignTarget::Identifier(name)),
            Expr::Get { object, field } => Ok(AssignTarget::Get {
                object: Box::new(self.assign_target_from_expr(*object)?),
                field,
            }),
            Expr::Index { object, index } => Ok(AssignTarget::Index {
                object: Box::new(self.assign_target_from_expr(*object)?),
                index: *index,
            }),
            _ => Err("invalid assignment target"),
        }
    }

    pub(super) fn expect_identifier(&mut self, message: &str) -> NodiaResult<String> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Identifier(name) => Ok(name),
            _ => Err(NodiaError::new(message, token.line, token.column)),
        }
    }

    pub(super) fn expect_name_like(&mut self, message: &str) -> NodiaResult<String> {
        let token = self.advance().clone();
        self.name_like_from_kind(&token.kind)
            .ok_or_else(|| NodiaError::new(message, token.line, token.column))
    }

    pub(super) fn name_like_from_kind(&self, kind: &TokenKind) -> Option<String> {
        match kind {
            TokenKind::Identifier(name) => Some(name.clone()),
            TokenKind::Val => Some("val".to_string()),
            TokenKind::Var => Some("var".to_string()),
            TokenKind::Func => Some("func".to_string()),
            TokenKind::LegacyLet => Some("let".to_string()),
            TokenKind::LegacyConst => Some("const".to_string()),
            TokenKind::LegacyFn => Some("fn".to_string()),
            TokenKind::Return => Some("return".to_string()),
            TokenKind::Emit => Some("emit".to_string()),
            TokenKind::If => Some("if".to_string()),
            TokenKind::Else => Some("else".to_string()),
            TokenKind::For => Some("for".to_string()),
            TokenKind::In => Some("in".to_string()),
            TokenKind::While => Some("while".to_string()),
            TokenKind::Break => Some("break".to_string()),
            TokenKind::Continue => Some("continue".to_string()),
            TokenKind::True => Some("true".to_string()),
            TokenKind::False => Some("false".to_string()),
            TokenKind::Null => Some("null".to_string()),
            TokenKind::And => Some("and".to_string()),
            TokenKind::Or => Some("or".to_string()),
            TokenKind::Not => Some("not".to_string()),
            TokenKind::LegacyImport => Some("import".to_string()),
            TokenKind::From => Some("from".to_string()),
            TokenKind::As => Some("as".to_string()),
            TokenKind::Pick => Some("pick".to_string()),
            TokenKind::LegacyShow => Some("show".to_string()),
            TokenKind::Hide => Some("hide".to_string()),
            TokenKind::Lambda => Some("lambda".to_string()),
            TokenKind::Match => Some("match".to_string()),
            TokenKind::Case => Some("case".to_string()),
            TokenKind::Default => Some("default".to_string()),
            TokenKind::Try => Some("try".to_string()),
            TokenKind::Catch => Some("catch".to_string()),
            TokenKind::Throw => Some("throw".to_string()),
            TokenKind::Defer => Some("defer".to_string()),
            TokenKind::Type => Some("type".to_string()),
            TokenKind::Enum => Some("enum".to_string()),
            TokenKind::Struct => Some("struct".to_string()),
            TokenKind::Namespace => Some("namespace".to_string()),
            TokenKind::Use => Some("use".to_string()),
            TokenKind::Regex => Some("regex".to_string()),
            _ => None,
        }
    }

    pub(super) fn expect_equal(&mut self) -> NodiaResult<()> {
        self.expect(TokenKind::Equal, "expected '='")
    }

    pub(super) fn expect(&mut self, kind: TokenKind, message: &str) -> NodiaResult<()> {
        if self.check(&kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_here(message))
        }
    }

    pub(super) fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(super) fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    pub(super) fn consume_statement_end(&mut self) {
        if self.match_kind(&TokenKind::Semicolon) {
            self.skip_newlines();
        }
    }

    pub(super) fn at_statement_end(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Newline
                | TokenKind::Semicolon
                | TokenKind::RightBrace
                | TokenKind::Eof
                | TokenKind::Comment(_)
        )
    }

    pub(super) fn skip_newlines(&mut self) {
        while self.match_kind(&TokenKind::Newline) {}
    }

    pub(super) fn skip_separators(&mut self) {
        loop {
            if self.match_kind(&TokenKind::Newline) {
                continue;
            }
            if matches!(self.peek().kind, TokenKind::Comment(_)) {
                self.advance();
                continue;
            }
            break;
        }
    }

    pub(super) fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    pub(super) fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.pos += 1;
            self.previous()
        } else {
            self.peek()
        }
    }

    pub(super) fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    pub(super) fn previous(&self) -> &Token {
        &self.tokens[self.pos - 1]
    }

    pub(super) fn error_here(&self, message: &str) -> NodiaError {
        NodiaError::new(message, self.peek().line, self.peek().column)
    }

    pub(super) fn parameter_list(&mut self, start_message: &str) -> NodiaResult<Vec<String>> {
        self.expect(TokenKind::LeftParen, start_message)?;
        self.skip_separators();
        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                params.push(self.expect_identifier("expected parameter name")?);
                self.skip_separators();
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                self.skip_separators();
                if self.check(&TokenKind::RightParen) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightParen, "expected ')' after parameters")?;
        Ok(params)
    }

    pub(super) fn lambda_expression(&mut self) -> NodiaResult<Expr> {
        let params = self.parameter_list("expected '(' after lambda")?;
        self.skip_newlines();
        let body = self.lambda_body()?;
        Ok(Expr::Lambda { params, body })
    }

    pub(super) fn lambda_body(&mut self) -> NodiaResult<Vec<Stmt>> {
        let mut body = self.block()?;
        if let Some(last) = body.last_mut() {
            if let Stmt::Expr(expr) = last.clone() {
                *last = Stmt::Return(Some(expr));
            }
        }
        Ok(body)
    }
}
