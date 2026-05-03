use crate::ast::{BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::error::{OrichError, OrichResult};
use crate::token::{Token, TokenKind};
use crate::value::Value;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_program(&mut self) -> OrichResult<Program> {
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() {
            statements.push(self.statement()?);
            self.consume_statement_end();
            self.skip_newlines();
        }
        Ok(Program { statements })
    }

    pub fn parse_expression_only(&mut self) -> OrichResult<Expr> {
        self.skip_separators();
        let expr = self.expression()?;
        self.skip_separators();
        if !self.is_at_end() {
            return Err(self.error_here("unexpected token after expression"));
        }
        Ok(expr)
    }

    fn statement(&mut self) -> OrichResult<Stmt> {
        match &self.peek().kind {
            TokenKind::Comment(text) => {
                let text = text.clone();
                self.advance();
                Ok(Stmt::Comment(text))
            }
            TokenKind::Import => self.import_statement(),
            TokenKind::Let => self.let_statement(true),
            TokenKind::Const => self.let_statement(false),
            TokenKind::Fn => self.fn_statement(),
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
            | TokenKind::Namespace
            | TokenKind::Use => {
                Err(self.error_here("keyword is reserved for a future Orich version"))
            }
            TokenKind::Identifier(name) if self.peek_next_is_equal() => {
                let name = name.clone();
                self.advance();
                self.expect_equal()?;
                self.skip_separators();
                let value = self.expression()?;
                Ok(Stmt::Assign { name, value })
            }
            _ => Ok(Stmt::Expr(self.expression()?)),
        }
    }

    fn import_statement(&mut self) -> OrichResult<Stmt> {
        self.advance();
        self.skip_separators();
        let token = self.advance().clone();
        let path = match token.kind {
            TokenKind::String(path) => path,
            _ => {
                return Err(OrichError::new(
                    "expected string path after import",
                    token.line,
                    token.column,
                ))
            }
        };

        let mut alias = None;
        let mut show = Vec::new();
        let mut hide = Vec::new();
        loop {
            self.skip_separators();
            if self.match_kind(&TokenKind::As) {
                self.skip_separators();
                alias = Some(self.expect_identifier("expected alias after 'as'")?);
            } else if self.match_kind(&TokenKind::Show) {
                show = self.import_name_list("show")?;
            } else if self.match_kind(&TokenKind::Hide) {
                hide = self.import_name_list("hide")?;
            } else {
                break;
            }
        }

        Ok(Stmt::Import {
            path,
            alias,
            show,
            hide,
        })
    }

    fn import_name_list(&mut self, clause: &str) -> OrichResult<Vec<String>> {
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

    fn let_statement(&mut self, mutable: bool) -> OrichResult<Stmt> {
        self.advance();
        let name = self.expect_identifier("expected variable name")?;
        self.expect_equal()?;
        self.skip_separators();
        let value = self.expression()?;
        Ok(Stmt::Let {
            name,
            value,
            mutable,
        })
    }

    fn fn_statement(&mut self) -> OrichResult<Stmt> {
        self.advance();
        let name = self.expect_identifier("expected function name")?;
        self.expect(TokenKind::LeftParen, "expected '(' after function name")?;
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
        self.skip_newlines();
        let body = self.block()?;
        Ok(Stmt::Fn { name, params, body })
    }

    fn return_statement(&mut self) -> OrichResult<Stmt> {
        self.advance();
        if self.at_statement_end() || self.check(&TokenKind::RightBrace) {
            Ok(Stmt::Return(None))
        } else {
            Ok(Stmt::Return(Some(self.expression()?)))
        }
    }

    fn emit_statement(&mut self) -> OrichResult<Stmt> {
        self.advance();
        self.skip_separators();
        Ok(Stmt::Emit(self.expression()?))
    }

    fn if_statement(&mut self) -> OrichResult<Stmt> {
        self.advance();
        let condition = self.expression()?;
        self.skip_newlines();
        let then_branch = self.block()?;
        self.skip_newlines();
        let else_branch = if self.match_kind(&TokenKind::Else) {
            self.skip_newlines();
            if self.match_kind(&TokenKind::If) {
                let condition = self.expression()?;
                self.skip_newlines();
                let then_branch = self.block()?;
                vec![Stmt::If {
                    condition,
                    then_branch,
                    else_branch: Vec::new(),
                }]
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

    fn for_statement(&mut self) -> OrichResult<Stmt> {
        self.advance();
        let name = self.expect_identifier("expected loop variable")?;
        self.expect(TokenKind::In, "expected 'in' after loop variable")?;
        self.skip_separators();
        let iterable = self.expression()?;
        self.skip_newlines();
        let body = self.block()?;
        Ok(Stmt::For {
            name,
            iterable,
            body,
        })
    }

    fn while_statement(&mut self) -> OrichResult<Stmt> {
        self.advance();
        let condition = self.expression()?;
        self.skip_newlines();
        let body = self.block()?;
        Ok(Stmt::While { condition, body })
    }

    fn block(&mut self) -> OrichResult<Vec<Stmt>> {
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

    fn expression(&mut self) -> OrichResult<Expr> {
        self.or()
    }

    fn or(&mut self) -> OrichResult<Expr> {
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

    fn and(&mut self) -> OrichResult<Expr> {
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

    fn equality(&mut self) -> OrichResult<Expr> {
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

    fn comparison(&mut self) -> OrichResult<Expr> {
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

    fn term(&mut self) -> OrichResult<Expr> {
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

    fn factor(&mut self) -> OrichResult<Expr> {
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

    fn unary(&mut self) -> OrichResult<Expr> {
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

    fn call(&mut self) -> OrichResult<Expr> {
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
                let field = self.expect_identifier("expected field name after '.'")?;
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

    fn primary(&mut self) -> OrichResult<Expr> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Null => Ok(Expr::Literal(Value::Null)),
            TokenKind::True => Ok(Expr::Literal(Value::Bool(true))),
            TokenKind::False => Ok(Expr::Literal(Value::Bool(false))),
            TokenKind::Int(value) => Ok(Expr::Literal(Value::Int(value))),
            TokenKind::Float(value) => Ok(Expr::Literal(Value::Float(value))),
            TokenKind::String(value) => Ok(Expr::Literal(Value::String(value))),
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
            _ => Err(OrichError::new(
                "expected expression",
                token.line,
                token.column,
            )),
        }
    }

    fn list_literal(&mut self) -> OrichResult<Expr> {
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

    fn map_literal(&mut self) -> OrichResult<Expr> {
        let mut pairs = Vec::new();
        self.skip_separators();
        if !self.check(&TokenKind::RightBrace) {
            loop {
                let key = match self.advance().kind.clone() {
                    TokenKind::Identifier(name) => name,
                    TokenKind::String(name) => name,
                    other => {
                        return Err(OrichError::new(
                            format!("expected map key, got {other:?}"),
                            self.previous().line,
                            self.previous().column,
                        ))
                    }
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

    fn expect_identifier(&mut self, message: &str) -> OrichResult<String> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Identifier(name) => Ok(name),
            _ => Err(OrichError::new(message, token.line, token.column)),
        }
    }

    fn expect_equal(&mut self) -> OrichResult<()> {
        self.expect(TokenKind::Equal, "expected '='")
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> OrichResult<()> {
        if self.check(&kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error_here(message))
        }
    }

    fn match_kind(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn peek_next_is_equal(&self) -> bool {
        self.tokens
            .get(self.pos + 1)
            .is_some_and(|token| matches!(token.kind, TokenKind::Equal))
    }

    fn consume_statement_end(&mut self) {
        if self.match_kind(&TokenKind::Semicolon) {
            self.skip_newlines();
        }
    }

    fn at_statement_end(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Newline
                | TokenKind::Semicolon
                | TokenKind::RightBrace
                | TokenKind::Eof
                | TokenKind::Comment(_)
        )
    }

    fn skip_newlines(&mut self) {
        while self.match_kind(&TokenKind::Newline) {}
    }

    fn skip_separators(&mut self) {
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

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.pos += 1;
            self.previous()
        } else {
            self.peek()
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.pos - 1]
    }

    fn error_here(&self, message: &str) -> OrichError {
        OrichError::new(message, self.peek().line, self.peek().column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn parses_emit_and_let() {
        let tokens = Lexer::new("let name = \"Ana\"\nemit \"Hi {name}\"")
            .tokenize()
            .unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        assert_eq!(program.statements.len(), 2);
    }

    #[test]
    fn parses_multiline_maps_lists_and_calls() {
        let source = r#"
const user = {
  name: "Ana",
  tags: [
    "dev",
    "ops",
  ],
}

emit join(
  user.tags,
  ",",
)
"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        assert_eq!(program.statements.len(), 2);
    }
}
