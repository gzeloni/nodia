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
            TokenKind::Throw => self.throw_statement(),
            TokenKind::Emit => self.emit_statement(),
            TokenKind::Try => self.try_statement(),
            TokenKind::Match => self.match_statement(),
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
            TokenKind::Catch => Err(self.error_here("'catch' must follow a try block")),
            TokenKind::Case => Err(self.error_here("'case' must appear inside a match block")),
            TokenKind::Default => {
                Err(self.error_here("'default' must appear inside a match block"))
            }
            TokenKind::From | TokenKind::Defer => {
                Err(self.error_here("keyword is reserved for a future Nodia version"))
            }
            TokenKind::Namespace => self.namespace_statement(),
            TokenKind::Struct => self.struct_statement(),
            TokenKind::Enum => self.enum_statement(),
            TokenKind::Type => self.type_alias_statement(),
            _ => self.expr_or_assign_statement(),
        }
    }

    pub(super) fn use_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        self.skip_separators();
        let token = self.advance().clone();
        let target = match token.kind {
            TokenKind::String(path) | TokenKind::RawString(path) => UseTarget::Path(path),
            TokenKind::Identifier(name) if name == "re" => {
                return Err(NodiaError::new(
                    "'re' was removed; regex is built into the language",
                    token.line,
                    token.column,
                ))
            }
            TokenKind::Identifier(name) => UseTarget::Stdlib(name),
            TokenKind::Regex => {
                return Err(NodiaError::new(
                    "'regex' is built into the language and must not be imported",
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

    pub(super) fn throw_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        self.skip_separators();
        Ok(Stmt::Throw(self.expression()?))
    }

    pub(super) fn try_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        self.skip_separators();
        let try_branch = self.block()?;
        self.skip_separators();
        self.expect(TokenKind::Catch, "expected 'catch' after try block")?;
        self.skip_separators();
        let catch_name = self.expect_identifier("expected error binding after 'catch'")?;
        self.skip_separators();
        let catch_branch = self.block()?;
        Ok(Stmt::Try {
            try_branch,
            catch_name,
            catch_branch,
        })
    }

    pub(super) fn match_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        self.skip_separators();
        let value = self.expression()?;
        self.skip_newlines();
        self.expect(TokenKind::LeftBrace, "expected '{' after match value")?;

        let mut arms = Vec::new();
        let mut default = None;
        let mut seen_default = false;

        self.skip_newlines();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            if self.match_kind(&TokenKind::Case) {
                if seen_default {
                    return Err(self.error_here("'case' cannot appear after 'default' in match"));
                }
                self.skip_separators();
                let pattern = self.match_pattern()?;
                self.skip_newlines();
                let body = self.block()?;
                arms.push(MatchArm { pattern, body });
            } else if self.match_kind(&TokenKind::Default) {
                if seen_default {
                    return Err(self.error_here("match can only contain one 'default' arm"));
                }
                self.skip_newlines();
                default = Some(self.block()?);
                seen_default = true;
            } else {
                return Err(self.error_here("expected 'case' or 'default' in match block"));
            }

            self.consume_statement_end();
            self.skip_newlines();
        }

        self.expect(TokenKind::RightBrace, "expected '}' after match block")?;
        Ok(Stmt::Match {
            value,
            arms,
            default,
        })
    }

    pub(super) fn match_pattern(&mut self) -> NodiaResult<MatchPattern> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Identifier(name) => {
                if name == "_" {
                    Ok(MatchPattern::Wildcard)
                } else {
                    Ok(MatchPattern::Capture(name))
                }
            }
            TokenKind::Null => Ok(MatchPattern::Literal(Value::Null)),
            TokenKind::True => Ok(MatchPattern::Literal(Value::Bool(true))),
            TokenKind::False => Ok(MatchPattern::Literal(Value::Bool(false))),
            TokenKind::Int(value) => Ok(MatchPattern::Literal(Value::Int(value))),
            TokenKind::Float(value) => Ok(MatchPattern::Literal(Value::Float(value))),
            TokenKind::String(value) | TokenKind::RawString(value) => {
                Ok(MatchPattern::Literal(Value::String(value)))
            }
            TokenKind::Bytes(value) => Ok(MatchPattern::Literal(Value::Bytes(value))),
            TokenKind::LeftBracket => self.list_match_pattern(),
            TokenKind::LeftBrace => self.map_match_pattern(),
            _ => Err(NodiaError::new(
                "expected match pattern",
                token.line,
                token.column,
            )),
        }
    }

    pub(super) fn list_match_pattern(&mut self) -> NodiaResult<MatchPattern> {
        let mut items = Vec::new();
        self.skip_separators();
        if !self.check(&TokenKind::RightBracket) {
            loop {
                items.push(self.match_pattern()?);
                self.skip_separators();
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                self.skip_separators();
                if self.check(&TokenKind::RightBracket) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightBracket, "expected ']' after list pattern")?;
        Ok(MatchPattern::List(items))
    }

    pub(super) fn map_match_pattern(&mut self) -> NodiaResult<MatchPattern> {
        let mut entries = Vec::new();
        self.skip_separators();
        if !self.check(&TokenKind::RightBrace) {
            loop {
                let key_token = self.advance().clone();
                let key = match key_token.kind.clone() {
                    TokenKind::String(name) | TokenKind::RawString(name) => name,
                    kind => self.name_like_from_kind(&kind).ok_or_else(|| {
                        NodiaError::new(
                            "expected map pattern key",
                            key_token.line,
                            key_token.column,
                        )
                    })?,
                };

                self.skip_separators();
                let pattern = if self.match_kind(&TokenKind::Colon) {
                    self.skip_separators();
                    self.match_pattern()?
                } else if matches!(key_token.kind, TokenKind::Identifier(_)) {
                    MatchPattern::Capture(key.clone())
                } else {
                    return Err(NodiaError::new(
                        "expected ':' after map pattern key",
                        key_token.line,
                        key_token.column,
                    ));
                };

                entries.push((key, pattern));
                self.skip_separators();
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                self.skip_separators();
                if self.check(&TokenKind::RightBrace) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightBrace, "expected '}' after map pattern")?;
        Ok(MatchPattern::Map(entries))
    }

    pub(super) fn namespace_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        let name = self.expect_identifier("expected namespace name")?;
        self.skip_newlines();
        let body = self.block()?;
        Ok(Stmt::Namespace { name, body })
    }

    pub(super) fn struct_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        let name = self.expect_identifier("expected struct name")?;
        self.skip_newlines();
        self.expect(TokenKind::LeftBrace, "expected '{' after struct name")?;
        self.skip_newlines();
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let field_name = self.expect_identifier("expected field name")?;
            let default = if self.match_kind(&TokenKind::Colon) {
                self.skip_separators();
                Some(self.expression()?)
            } else {
                None
            };
            fields.push(StructField {
                name: field_name,
                default,
            });
            self.consume_statement_end();
            self.skip_newlines();
        }
        self.expect(TokenKind::RightBrace, "expected '}' after struct fields")?;
        Ok(Stmt::Struct { name, fields })
    }

    pub(super) fn enum_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        let name = self.expect_identifier("expected enum name")?;
        self.skip_newlines();
        self.expect(TokenKind::LeftBrace, "expected '{' after enum name")?;
        self.skip_newlines();
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            variants.push(self.expect_identifier("expected variant name")?);
            self.skip_separators();
            if !self.check(&TokenKind::RightBrace) {
                self.expect(TokenKind::Comma, "expected ',' between enum variants")?;
                self.skip_separators();
            }
        }
        self.expect(TokenKind::RightBrace, "expected '}' after enum variants")?;
        if variants.is_empty() {
            return Err(self.error_here("enum must have at least one variant"));
        }
        Ok(Stmt::Enum { name, variants })
    }

    pub(super) fn type_alias_statement(&mut self) -> NodiaResult<Stmt> {
        self.advance();
        let name = self.expect_identifier("expected type name")?;
        self.expect_equal()?;
        self.skip_separators();
        let target = self.expression()?;
        Ok(Stmt::TypeAlias { name, target })
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
        if self.match_kind(&TokenKind::PlusEqual) {
            self.skip_separators();
            let value = self.expression()?;
            let target = self
                .assign_target_from_expr(expr.clone())
                .map_err(|message| {
                    NodiaError::new(message, self.previous().line, self.previous().column)
                })?;
            let add_expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Add,
                right: Box::new(value),
            };
            return Ok(Stmt::Assign {
                target,
                value: add_expr,
            });
        }
        if self.match_kind(&TokenKind::MinusEqual) {
            self.skip_separators();
            let value = self.expression()?;
            let target = self
                .assign_target_from_expr(expr.clone())
                .map_err(|message| {
                    NodiaError::new(message, self.previous().line, self.previous().column)
                })?;
            let sub_expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Subtract,
                right: Box::new(value),
            };
            return Ok(Stmt::Assign {
                target,
                value: sub_expr,
            });
        }
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
