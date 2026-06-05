// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Expression parsing with operator precedence handling.

use super::*;

impl Parser {
    pub(super) fn expression(&mut self) -> DobraResult<Expr> {
        self.or()
    }

    pub(super) fn or(&mut self) -> DobraResult<Expr> {
        let mut expr = self.and()?;
        while self.match_kind(&TokenKind::Or) {
            self.skip_separators();
            let right = self.and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    pub(super) fn and(&mut self) -> DobraResult<Expr> {
        let mut expr = self.equality()?;
        while self.match_kind(&TokenKind::And) {
            self.skip_separators();
            let right = self.equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    pub(super) fn equality(&mut self) -> DobraResult<Expr> {
        let mut expr = self.comparison()?;
        loop {
            let op = if self.match_kind(&TokenKind::EqualEqual) {
                Some(BinaryOp::Equal)
            } else if self.match_kind(&TokenKind::BangEqual) {
                Some(BinaryOp::NotEqual)
            } else {
                None
            };
            let Some(op) = op else { break };
            self.skip_separators();
            let right = self.comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    pub(super) fn comparison(&mut self) -> DobraResult<Expr> {
        let mut expr = self.term()?;
        loop {
            let op = if self.match_kind(&TokenKind::Less) {
                Some(BinaryOp::Less)
            } else if self.match_kind(&TokenKind::LessEqual) {
                Some(BinaryOp::LessEqual)
            } else if self.match_kind(&TokenKind::Greater) {
                Some(BinaryOp::Greater)
            } else if self.match_kind(&TokenKind::GreaterEqual) {
                Some(BinaryOp::GreaterEqual)
            } else {
                None
            };
            let Some(op) = op else { break };
            self.skip_separators();
            let right = self.term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    pub(super) fn term(&mut self) -> DobraResult<Expr> {
        let mut expr = self.factor()?;
        loop {
            let op = if self.match_kind(&TokenKind::Plus) {
                Some(BinaryOp::Add)
            } else if self.match_kind(&TokenKind::Minus) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };
            let Some(op) = op else { break };
            self.skip_separators();
            let right = self.factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    pub(super) fn factor(&mut self) -> DobraResult<Expr> {
        let mut expr = self.unary()?;
        loop {
            let op = if self.match_kind(&TokenKind::Star) {
                Some(BinaryOp::Multiply)
            } else if self.match_kind(&TokenKind::Slash) {
                Some(BinaryOp::Divide)
            } else if self.match_kind(&TokenKind::Percent) {
                Some(BinaryOp::Modulo)
            } else {
                None
            };
            let Some(op) = op else { break };
            self.skip_separators();
            let right = self.unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    pub(super) fn unary(&mut self) -> DobraResult<Expr> {
        if self.match_kind(&TokenKind::Minus) {
            self.skip_separators();
            return Ok(Expr::Unary {
                op: UnaryOp::Negate,
                expr: Box::new(self.unary()?),
            });
        }
        if self.match_kind(&TokenKind::Not) {
            self.skip_separators();
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(self.unary()?),
            });
        }
        self.call()
    }

    pub(super) fn call(&mut self) -> DobraResult<Expr> {
        let mut expr = self.primary()?;
        loop {
            if self.match_kind(&TokenKind::LeftParen) {
                self.skip_separators();
                let mut args = Vec::new();
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        args.push(self.expression()?);
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
                self.expect(TokenKind::RightParen, "expected ')' after arguments")?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else if self.match_kind(&TokenKind::Dot) {
                let field = self.expect_name_like("expected field name after '.'")?;
                expr = Expr::Get {
                    object: Box::new(expr),
                    field,
                };
            } else if self.match_kind(&TokenKind::LeftBracket) {
                self.skip_separators();
                let index = self.expression()?;
                self.skip_separators();
                self.expect(TokenKind::RightBracket, "expected ']' after index")?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    pub(super) fn primary(&mut self) -> DobraResult<Expr> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Null => Ok(Expr::Literal(Value::Null)),
            TokenKind::True => Ok(Expr::Literal(Value::Bool(true))),
            TokenKind::False => Ok(Expr::Literal(Value::Bool(false))),
            TokenKind::Int(value) => Ok(Expr::Literal(Value::Int(value))),
            TokenKind::Float(value) => Ok(Expr::Literal(Value::Float(value))),
            TokenKind::String(value) => Ok(Expr::String {
                value,
                interpolate: true,
            }),
            TokenKind::RawString(value) => Ok(Expr::String {
                value,
                interpolate: false,
            }),
            TokenKind::Lambda => self.lambda_expression(),
            TokenKind::Regex => self.regex_literal(),
            TokenKind::Identifier(name) => Ok(Expr::Identifier(name)),
            TokenKind::LeftParen => {
                self.skip_separators();
                let expr = self.expression()?;
                self.skip_separators();
                self.expect(TokenKind::RightParen, "expected ')' after expression")?;
                Ok(expr)
            }
            TokenKind::LeftBracket => self.list_literal(),
            TokenKind::LeftBrace => self.map_literal(),
            _ => Err(DobraError::new(
                "expected expression",
                token.line,
                token.column,
            )),
        }
    }

    pub(super) fn list_literal(&mut self) -> DobraResult<Expr> {
        let mut values = Vec::new();
        self.skip_separators();
        if !self.check(&TokenKind::RightBracket) {
            loop {
                values.push(self.expression()?);
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
        self.expect(TokenKind::RightBracket, "expected ']' after list")?;
        Ok(Expr::List(values))
    }

    pub(super) fn map_literal(&mut self) -> DobraResult<Expr> {
        let mut pairs = Vec::new();
        self.skip_separators();
        if !self.check(&TokenKind::RightBrace) {
            loop {
                let key = match self.advance().kind.clone() {
                    TokenKind::String(name) | TokenKind::RawString(name) => name,
                    kind => self.name_like_from_kind(&kind).ok_or_else(|| {
                        DobraError::new(
                            format!("expected map key, got {kind:?}"),
                            self.previous().line,
                            self.previous().column,
                        )
                    })?,
                };
                self.skip_separators();
                self.expect(TokenKind::Colon, "expected ':' after map key")?;
                self.skip_separators();
                let value = self.expression()?;
                pairs.push((key, value));
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
        self.expect(TokenKind::RightBrace, "expected '}' after map")?;
        Ok(Expr::Map(pairs))
    }
}
