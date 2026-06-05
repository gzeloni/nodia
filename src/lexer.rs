// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Lexer implementation for Nodia source text.

use crate::error::{DobraError, DobraResult};
use crate::token::{keyword_kind, Token, TokenKind};

/// Stateful tokenizer that walks source text and emits [`Token`] values.
pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    /// Creates a lexer for the provided source text.
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    /// Consumes the source text and returns the full token stream.
    pub fn tokenize(mut self) -> DobraResult<Vec<Token>> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            let line = self.line;
            let column = self.column;
            match ch {
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                '\n' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Newline, line, column));
                }
                '#' => tokens.push(Token::new(
                    TokenKind::Comment(self.line_comment(1)),
                    line,
                    column,
                )),
                '/' if self.peek_next() == Some('/') => {
                    tokens.push(Token::new(
                        TokenKind::Comment(self.line_comment(2)),
                        line,
                        column,
                    ));
                }
                'r' if self.peek_next() == Some('"')
                    && self.peek_n(2) == Some('"')
                    && self.peek_n(3) == Some('"') =>
                {
                    tokens.push(Token::new(
                        TokenKind::RawString(self.raw_triple_string(line, column)?),
                        line,
                        column,
                    ));
                }
                'r' if self.peek_next() == Some('"') => tokens.push(Token::new(
                    TokenKind::RawString(self.raw_string(line, column, '"')?),
                    line,
                    column,
                )),
                'r' if self.peek_next() == Some('\'') => tokens.push(Token::new(
                    TokenKind::RawString(self.raw_string(line, column, '\'')?),
                    line,
                    column,
                )),
                '"' if self.peek_next() == Some('"') && self.peek_n(2) == Some('"') => {
                    tokens.push(Token::new(
                        TokenKind::RawString(self.triple_string(line, column)?),
                        line,
                        column,
                    ));
                }
                '"' => tokens.push(Token::new(
                    TokenKind::String(self.string(line, column, '"')?),
                    line,
                    column,
                )),
                '\'' => tokens.push(Token::new(
                    TokenKind::String(self.string(line, column, '\'')?),
                    line,
                    column,
                )),
                '0'..='9' => tokens.push(Token::new(self.number()?, line, column)),
                'a'..='z' | 'A'..='Z' | '_' => {
                    tokens.push(Token::new(self.identifier(), line, column))
                }
                '+' => self.single(&mut tokens, TokenKind::Plus),
                '-' => self.single(&mut tokens, TokenKind::Minus),
                '*' => self.single(&mut tokens, TokenKind::Star),
                '/' => self.single(&mut tokens, TokenKind::Slash),
                '%' => self.single(&mut tokens, TokenKind::Percent),
                '=' => {
                    self.advance();
                    if self.match_char('=') {
                        tokens.push(Token::new(TokenKind::EqualEqual, line, column));
                    } else {
                        tokens.push(Token::new(TokenKind::Equal, line, column));
                    }
                }
                '!' => {
                    self.advance();
                    if self.match_char('=') {
                        tokens.push(Token::new(TokenKind::BangEqual, line, column));
                    } else {
                        return Err(DobraError::new(
                            "unexpected '!'; use 'not' or '!='",
                            line,
                            column,
                        ));
                    }
                }
                '<' => {
                    self.advance();
                    if self.match_char('=') {
                        tokens.push(Token::new(TokenKind::LessEqual, line, column));
                    } else {
                        tokens.push(Token::new(TokenKind::Less, line, column));
                    }
                }
                '>' => {
                    self.advance();
                    if self.match_char('=') {
                        tokens.push(Token::new(TokenKind::GreaterEqual, line, column));
                    } else {
                        tokens.push(Token::new(TokenKind::Greater, line, column));
                    }
                }
                '(' => self.single(&mut tokens, TokenKind::LeftParen),
                ')' => self.single(&mut tokens, TokenKind::RightParen),
                '{' => self.single(&mut tokens, TokenKind::LeftBrace),
                '}' => self.single(&mut tokens, TokenKind::RightBrace),
                '[' => self.single(&mut tokens, TokenKind::LeftBracket),
                ']' => self.single(&mut tokens, TokenKind::RightBracket),
                ',' => self.single(&mut tokens, TokenKind::Comma),
                '.' => self.single(&mut tokens, TokenKind::Dot),
                ':' => self.single(&mut tokens, TokenKind::Colon),
                ';' => self.single(&mut tokens, TokenKind::Semicolon),
                _ => {
                    return Err(DobraError::new(
                        format!("unexpected character '{ch}'"),
                        line,
                        column,
                    ))
                }
            }
        }
        tokens.push(Token::new(TokenKind::Eof, self.line, self.column));
        Ok(tokens)
    }

    fn single(&mut self, tokens: &mut Vec<Token>, kind: TokenKind) {
        let line = self.line;
        let column = self.column;
        self.advance();
        tokens.push(Token::new(kind, line, column));
    }

    fn identifier(&mut self) -> TokenKind {
        let start = self.pos;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_alphanumeric() || ch == '_') {
            self.advance();
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        keyword_kind(&text).unwrap_or(TokenKind::Identifier(text))
    }

    fn number(&mut self) -> DobraResult<TokenKind> {
        let start = self.pos;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            self.advance();
        }
        let mut is_float = false;
        if self.peek() == Some('.') && matches!(self.peek_next(), Some(ch) if ch.is_ascii_digit()) {
            is_float = true;
            self.advance();
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                self.advance();
            }
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            is_float = true;
            self.advance();
            if matches!(self.peek(), Some('+' | '-')) {
                self.advance();
            }
            if !matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                return Err(DobraError::new(
                    "invalid float literal",
                    self.line,
                    self.column,
                ));
            }
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
                self.advance();
            }
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        if is_float {
            text.parse::<f64>()
                .map(TokenKind::Float)
                .map_err(|_| DobraError::new("invalid float literal", self.line, self.column))
        } else {
            text.parse::<i64>()
                .map(TokenKind::Int)
                .map_err(|_| DobraError::new("invalid integer literal", self.line, self.column))
        }
    }

    fn string(&mut self, line: usize, column: usize, quote: char) -> DobraResult<String> {
        self.advance();
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            if ch == quote {
                self.advance();
                return Ok(out);
            }
            if ch == '\\' {
                self.advance();
                let escaped = self
                    .advance()
                    .ok_or_else(|| DobraError::new("unterminated escape", line, column))?;
                out.push(match escaped {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\'' => '\'',
                    '\\' => '\\',
                    other => other,
                });
            } else {
                out.push(ch);
                self.advance();
            }
        }
        Err(DobraError::new("unterminated string", line, column))
    }

    fn raw_string(&mut self, line: usize, column: usize, quote: char) -> DobraResult<String> {
        self.advance();
        self.advance();
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            if ch == quote {
                self.advance();
                if quote == '"'
                    && looks_like_misdelimited_json_raw(&out)
                    && matches!(self.peek(), Some(next) if next.is_ascii_alphanumeric() || matches!(next, '"' | '\''))
                {
                    return Err(DobraError::new(
                        "raw string likely closed early; for inline JSON prefer r'...' or \"\"\"...\"\"\"",
                        line,
                        column,
                    ));
                }
                return Ok(out);
            }
            out.push(ch);
            self.advance();
        }
        Err(DobraError::new("unterminated raw string", line, column))
    }

    fn triple_string(&mut self, line: usize, column: usize) -> DobraResult<String> {
        self.advance();
        self.advance();
        self.advance();
        let mut out = String::new();
        loop {
            if self.peek().is_none() {
                return Err(DobraError::new("unterminated triple string", line, column));
            }
            if self.peek() == Some('"')
                && self.peek_next() == Some('"')
                && self.peek_n(2) == Some('"')
            {
                self.advance();
                self.advance();
                self.advance();
                return Ok(out);
            }
            let ch = self.advance().expect("peek checked above");
            out.push(ch);
        }
    }

    fn raw_triple_string(&mut self, line: usize, column: usize) -> DobraResult<String> {
        self.advance();
        self.triple_string(line, column)
    }

    fn line_comment(&mut self, prefix_len: usize) -> String {
        for _ in 0..prefix_len {
            self.advance();
        }
        if self.peek() == Some(' ') {
            self.advance();
        }
        let start = self.pos;
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
        self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.peek_n(1)
    }

    fn peek_n(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }
}

fn looks_like_misdelimited_json_raw(value: &str) -> bool {
    let trimmed = value.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_keywords_strings_and_comments() {
        let tokens = Lexer::new("# hi\nval name = \"Ana\"\nemit \"Hi {name}\"")
            .tokenize()
            .unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Comment(_)));
        assert!(matches!(tokens[2].kind, TokenKind::Val));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Emit)));
    }

    #[test]
    fn tokenizes_raw_strings_and_scientific_numbers() {
        let tokens = Lexer::new("emit r'{x}'\nemit \"\"\"{\"a\":1}\"\"\"\nemit 1e10")
            .tokenize()
            .unwrap();

        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::RawString(ref value) if value == "{x}")));
        assert!(tokens.iter().any(|token| matches!(
            token.kind,
            TokenKind::RawString(ref value) if value == "{\"a\":1}"
        )));
        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Float(value) if value == 1e10)));
    }

    #[test]
    fn reports_helpful_error_for_json_inside_raw_double_quotes() {
        let err = Lexer::new(r#"emit r"{"a":1}"#).tokenize().unwrap_err();
        assert!(err
            .to_string()
            .contains("for inline JSON prefer r'...' or \"\"\"...\"\"\""));
    }
}
