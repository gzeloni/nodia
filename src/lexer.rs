// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Lexer implementation for Nodia source text.

use crate::error::{NodiaError, NodiaResult};
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
    pub fn tokenize(mut self) -> NodiaResult<Vec<Token>> {
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
                '/' if self.peek_next() == Some('*') => {
                    let content = self.block_comment(line, column)?;
                    tokens.push(Token::new(TokenKind::Comment(content), line, column));
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
                'b' if self.peek_next() == Some('"') => tokens.push(Token::new(
                    TokenKind::Bytes(self.byte_string(line, column, '"')?),
                    line,
                    column,
                )),
                'b' if self.peek_next() == Some('\'') => tokens.push(Token::new(
                    TokenKind::Bytes(self.byte_string(line, column, '\'')?),
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
                ch if is_identifier_start(ch) => {
                    tokens.push(Token::new(self.identifier(), line, column))
                }
                '+' => {
                    self.advance();
                    if self.match_char('=') {
                        tokens.push(Token::new(TokenKind::PlusEqual, line, column));
                    } else {
                        tokens.push(Token::new(TokenKind::Plus, line, column));
                    }
                }
                '-' => {
                    self.advance();
                    if self.match_char('=') {
                        tokens.push(Token::new(TokenKind::MinusEqual, line, column));
                    } else {
                        tokens.push(Token::new(TokenKind::Minus, line, column));
                    }
                }
                '*' => self.single(&mut tokens, TokenKind::Star),
                '/' => self.single(&mut tokens, TokenKind::Slash),
                '%' => self.single(&mut tokens, TokenKind::Percent),
                '&' => {
                    self.advance();
                    if self.match_char('&') {
                        return Err(NodiaError::new(
                            "unexpected '&&'; use 'and' for logical and or '&' for bitwise and",
                            line,
                            column,
                        ));
                    }
                    tokens.push(Token::new(TokenKind::Ampersand, line, column));
                }
                '|' => {
                    self.advance();
                    if self.match_char('|') {
                        return Err(NodiaError::new(
                            "unexpected '||'; use 'or' for logical or or '|' for bitwise or",
                            line,
                            column,
                        ));
                    }
                    tokens.push(Token::new(TokenKind::Pipe, line, column));
                }
                '^' => self.single(&mut tokens, TokenKind::Caret),
                '~' => self.single(&mut tokens, TokenKind::Tilde),
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
                        return Err(NodiaError::new(
                            "unexpected '!'; use 'not' or '!='",
                            line,
                            column,
                        ));
                    }
                }
                '<' => {
                    self.advance();
                    if self.match_char('<') {
                        tokens.push(Token::new(TokenKind::LeftShift, line, column));
                    } else if self.match_char('=') {
                        tokens.push(Token::new(TokenKind::LessEqual, line, column));
                    } else {
                        tokens.push(Token::new(TokenKind::Less, line, column));
                    }
                }
                '>' => {
                    self.advance();
                    if self.match_char('>') {
                        tokens.push(Token::new(TokenKind::RightShift, line, column));
                    } else if self.match_char('=') {
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
                    return Err(NodiaError::new(
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
        while matches!(self.peek(), Some(ch) if is_identifier_continue(ch)) {
            self.advance();
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        keyword_kind(&text).unwrap_or(TokenKind::Identifier(text))
    }

    fn number(&mut self) -> NodiaResult<TokenKind> {
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
                return Err(NodiaError::new(
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
                .map_err(|_| NodiaError::new("invalid float literal", self.line, self.column))
        } else {
            text.parse::<i64>()
                .map(TokenKind::Int)
                .map_err(|_| NodiaError::new("invalid integer literal", self.line, self.column))
        }
    }

    fn string(&mut self, line: usize, column: usize, quote: char) -> NodiaResult<String> {
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
                    .ok_or_else(|| NodiaError::new("unterminated escape", line, column))?;
                out.push(match escaped {
                    'a' => '\u{0007}',
                    'b' => '\u{0008}',
                    'e' => '\u{001b}',
                    'f' => '\u{000c}',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    'v' => '\u{000b}',
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
        Err(NodiaError::new("unterminated string", line, column))
    }

    fn raw_string(&mut self, line: usize, column: usize, quote: char) -> NodiaResult<String> {
        self.advance();
        self.advance();
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            if ch == quote {
                self.advance();
                if quote == '"'
                    && looks_like_raw_json(&out)
                    && matches!(self.peek(), Some(next) if next.is_alphanumeric() || matches!(next, '"' | '\''))
                {
                    return Err(NodiaError::new(
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
        Err(NodiaError::new("unterminated raw string", line, column))
    }

    fn byte_string(&mut self, line: usize, column: usize, quote: char) -> NodiaResult<Vec<u8>> {
        self.advance();
        self.advance();
        let mut out = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == quote {
                self.advance();
                return Ok(out);
            }
            if ch == '\\' {
                self.advance();
                let escaped = self
                    .advance()
                    .ok_or_else(|| NodiaError::new("unterminated byte escape", line, column))?;
                match escaped {
                    'a' => out.push(0x07),
                    'b' => out.push(0x08),
                    'e' => out.push(0x1b),
                    'f' => out.push(0x0c),
                    'n' => out.push(b'\n'),
                    'r' => out.push(b'\r'),
                    't' => out.push(b'\t'),
                    'v' => out.push(0x0b),
                    '0' => out.push(0),
                    '"' => out.push(b'"'),
                    '\'' => out.push(b'\''),
                    '\\' => out.push(b'\\'),
                    'x' => {
                        let high = self
                            .advance()
                            .ok_or_else(|| NodiaError::new("invalid byte escape", line, column))?;
                        let low = self
                            .advance()
                            .ok_or_else(|| NodiaError::new("invalid byte escape", line, column))?;
                        let high = hex_digit(high)
                            .ok_or_else(|| NodiaError::new("invalid byte escape", line, column))?;
                        let low = hex_digit(low)
                            .ok_or_else(|| NodiaError::new("invalid byte escape", line, column))?;
                        out.push((high << 4) | low);
                    }
                    other => push_utf8_byte(&mut out, other),
                }
            } else {
                self.advance();
                push_utf8_byte(&mut out, ch);
            }
        }
        Err(NodiaError::new("unterminated bytes literal", line, column))
    }

    fn triple_string(&mut self, line: usize, column: usize) -> NodiaResult<String> {
        self.advance();
        self.advance();
        self.advance();
        let mut out = String::new();
        loop {
            if self.peek().is_none() {
                return Err(NodiaError::new("unterminated triple string", line, column));
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

    fn raw_triple_string(&mut self, line: usize, column: usize) -> NodiaResult<String> {
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

    fn block_comment(&mut self, start_line: usize, start_column: usize) -> NodiaResult<String> {
        self.advance();
        self.advance();
        let mut out = String::new();
        loop {
            match self.peek() {
                Some('*') if self.peek_next() == Some('/') => {
                    self.advance();
                    self.advance();
                    return Ok(out.trim_end().to_string());
                }
                Some(ch) => {
                    out.push(ch);
                    self.advance();
                }
                None => {
                    return Err(NodiaError::new(
                        "unterminated block comment",
                        start_line,
                        start_column,
                    ))
                }
            }
        }
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

fn looks_like_raw_json(value: &str) -> bool {
    let trimmed = value.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn push_utf8_byte(out: &mut Vec<u8>, ch: char) {
    let mut encoded = [0; 4];
    out.extend_from_slice(ch.encode_utf8(&mut encoded).as_bytes());
}

fn hex_digit(ch: char) -> Option<u8> {
    match ch {
        '0'..='9' => Some(ch as u8 - b'0'),
        'a'..='f' => Some(ch as u8 - b'a' + 10),
        'A'..='F' => Some(ch as u8 - b'A' + 10),
        _ => None,
    }
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
    fn tokenizes_block_comments() {
        let tokens = Lexer::new("/* header */\nval name = \"Ana\"")
            .tokenize()
            .unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Comment(_)));
        assert!(matches!(tokens[2].kind, TokenKind::Val));
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
    fn tokenizes_bytes_literals_and_hex_escapes() {
        let tokens = Lexer::new(r#"emit b"aé\xff\0""#).tokenize().unwrap();

        assert!(tokens.iter().any(|token| matches!(
            token.kind,
            TokenKind::Bytes(ref value) if value == &[97, 195, 169, 255, 0]
        )));
    }

    #[test]
    fn unknown_escapes_fall_back_to_literal_character() {
        let tokens = Lexer::new(r#"emit "\q-\x""#).tokenize().unwrap();

        assert!(tokens.iter().any(|token| matches!(
            token.kind,
            TokenKind::String(ref value) if value == "q-x"
        )));
    }

    #[test]
    fn tokenizes_non_ascii_identifiers() {
        let tokens = Lexer::new("val nome = \"Ana\"\nval über = 1\nemit nome + über")
            .tokenize()
            .unwrap();

        assert!(tokens
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::Identifier(name) if name == "nome")));
        assert!(tokens
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::Identifier(name) if name == "über")));
    }

    #[test]
    fn reports_helpful_error_for_json_inside_raw_double_quotes() {
        let err = Lexer::new(r#"emit r"{"a":1}"#).tokenize().unwrap_err();
        assert!(err
            .to_string()
            .contains("for inline JSON prefer r'...' or \"\"\"...\"\"\""));
    }

    #[test]
    fn tokenizes_bitwise_and_shift_operators() {
        let tokens = Lexer::new("emit ~1 & 3 | 4 ^ 2 << 1 >> 0")
            .tokenize()
            .unwrap();

        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Tilde)));
        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Ampersand)));
        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Pipe)));
        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Caret)));
        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::LeftShift)));
        assert!(tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::RightShift)));
    }

    #[test]
    fn reports_helpful_errors_for_symbolic_logical_operators() {
        let err = Lexer::new("emit a && b").tokenize().unwrap_err();
        assert!(err.to_string().contains("unexpected '&&'"));

        let err = Lexer::new("emit a || b").tokenize().unwrap_err();
        assert!(err.to_string().contains("unexpected '||'"));
    }
}
