// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Top-level and block statement parsing.

use super::*;

impl Parser {
    pub(super) fn statement(&mut self) -> NodiaResult<Stmt> {
        match &self.peek().kind {
            TokenKind::Comment(text) => {
                let text = text.clone();
                self.advance();
                Ok(Stmt::Comment(text))
            }
            TokenKind::Use => self.use_statement(),
            TokenKind::Var => self.bind_statement(true),
            TokenKind::Val => self.bind_statement(false),
            TokenKind::Func => self.func_statement(),
            TokenKind::LegacyLet
            | TokenKind::LegacyConst
            | TokenKind::LegacyFn
            | TokenKind::LegacyImport
            | TokenKind::LegacyShow => {
                Err(self.error_here("legacy keyword was removed in Nodia v0.6"))
            }
            TokenKind::Return => self.return_statement(),
            TokenKind::Emit => self.emit_statement(),
            TokenKind::If => self.if_statement(),
            TokenKind::For => self.for_statement(),
            TokenKind::While => self.while_statement(),
            TokenKind::Break => {
                self.advance();
                Ok(Stmt::Break)
            }
            TokenKind::Continue => {
                self.advance();
                Ok(Stmt::Continue)
            }
            TokenKind::From
            | TokenKind::Match
            | TokenKind::Case
            | TokenKind::Default
            | TokenKind::Try
            | TokenKind::Catch
            | TokenKind::Throw
            | TokenKind::Defer
            | TokenKind::Type
            | TokenKind::Enum
            | TokenKind::Struct
            | TokenKind::Namespace => {
                Err(self.error_here("keyword is reserved for a future Nodia version"))
            }
            _ => self.expr_or_assign_statement(),
        }
    }

    pub(super) fn use_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        self.skip_separators();
        let token = self.advance().clone();
        let target = match token.kind {
            TokenKind::String(path) | TokenKind::RawString(path) => UseTarget::Path(path),
            TokenKind::Identifier(name) => UseTarget::Stdlib(name),
            TokenKind::Regex => {
                return Err(NodiaError::new(
                    "use re for regex helpers; 'regex' is the DSL keyword",
                    token.line,
                    token.column,
                ))
            }
            _ => {
                return Err(NodiaError::new(
                    "expected string path or stdlib module name after use",
                    token.line,
                    token.column,
                ))
            }
        };

        let mut alias = None;
        let mut pick = Vec::new();
        let mut hide = Vec::new();
        loop {
            self.skip_separators();
            if self.match_kind(&TokenKind::As) {
                self.skip_separators();
                alias = Some(self.expect_identifier("expected alias after 'as'")?);
            } else if self.match_kind(&TokenKind::Pick) {
                pick = self.use_name_list("pick")?;
            } else if self.match_kind(&TokenKind::Hide) {
                hide = self.use_name_list("hide")?;
            } else {
                break;
            }
        }

        Ok(Stmt::Use {
            target,
            alias,
            pick,
            hide,
        })
    }

    pub(super) fn use_name_list(&mut self, clause: &str) -> NodiaResult<Vec<String>> {
        let mut names = Vec::new();
        loop {
            self.skip_separators();
            names.push(self.expect_identifier(&format!("expected name after '{clause}'"))?);
            self.skip_separators();
            if !self.match_kind(&TokenKind::Comma) {
                break;
            }
            self.skip_separators();
            if self.at_statement_end() {
                break;
            }
        }
        Ok(names)
    }

    pub(super) fn bind_statement(&mut self, mutable: bool) -> NodiaResult<Stmt> {
        self.advance();
        let name = self.expect_identifier("expected variable name")?;
        self.expect_equal()?;
        self.skip_separators();
        let value = self.expression()?;
        Ok(Stmt::Bind {
            name,
            value,
            mutable,
        })
    }

    pub(super) fn func_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        let name = self.expect_identifier("expected function name")?;
        let params = self.parameter_list("expected '(' after function name")?;
        self.skip_newlines();
        let body = self.block()?;
        Ok(Stmt::Func { name, params, body })
    }

    pub(super) fn return_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        if self.at_statement_end() || self.check(&TokenKind::RightBrace) {
            Ok(Stmt::Return(None))
        } else {
            Ok(Stmt::Return(Some(self.expression()?)))
        }
    }

    pub(super) fn emit_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        self.skip_separators();
        Ok(Stmt::Emit(self.expression()?))
    }

    pub(super) fn if_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        self.if_tail()
    }

    pub(super) fn if_tail(&mut self) -> NodiaResult<Stmt> {
        let condition = self.expression()?;
        self.skip_newlines();
        let then_branch = self.block()?;
        self.skip_newlines();
        let else_branch = if self.match_kind(&TokenKind::Else) {
            self.skip_newlines();
            if self.match_kind(&TokenKind::If) {
                vec![self.if_tail()?]
            } else {
                self.block()?
            }
        } else {
            Vec::new()
        };
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    pub(super) fn for_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        let binding = self.for_binding()?;
        self.expect(TokenKind::In, "expected 'in' after loop variable")?;
        self.skip_separators();
        let iterable = self.expression()?;
        self.skip_newlines();
        let body = self.block()?;
        Ok(Stmt::For {
            binding,
            iterable,
            body,
        })
    }

    pub(super) fn while_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        let condition = self.expression()?;
        self.skip_newlines();
        let body = self.block()?;
        Ok(Stmt::While { condition, body })
    }

    pub(super) fn block(&mut self) -> NodiaResult<Vec<Stmt>> {
        self.expect(TokenKind::LeftBrace, "expected '{' before block")?;
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.statement()?);
            self.consume_statement_end();
            self.skip_newlines();
        }
        self.expect(TokenKind::RightBrace, "expected '}' after block")?;
        Ok(statements)
    }

    pub(super) fn expr_or_assign_statement(&mut self) -> NodiaResult<Stmt> {
        let expr = self.expression()?;
        if !self.match_kind(&TokenKind::Equal) {
            return Ok(Stmt::Expr(expr));
        }
        self.skip_separators();
        let value = self.expression()?;
        let target = self.assign_target_from_expr(expr).map_err(|message| {
            NodiaError::new(message, self.previous().line, self.previous().column)
        })?;
        Ok(Stmt::Assign { target, value })
    }

    pub(super) fn for_binding(&mut self) -> NodiaResult<ForBinding> {
        if !self.match_kind(&TokenKind::LeftParen) {
            return self
                .expect_identifier("expected loop variable")
                .map(ForBinding::Single);
        }

        self.skip_separators();
        let key = self.expect_identifier("expected loop variable")?;
        self.skip_separators();
        self.expect(TokenKind::Comma, "expected ',' in loop binding")?;
        self.skip_separators();
        let value = self.expect_identifier("expected loop variable")?;
        self.skip_separators();
        self.expect(TokenKind::RightParen, "expected ')' after loop binding")?;
        Ok(ForBinding::Pair { key, value })
    }
}
