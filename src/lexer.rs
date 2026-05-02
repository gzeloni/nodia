use crate::error::{OrichError, OrichResult};
use crate::token::{keyword_kind, Token, TokenKind};

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn tokenize(mut self) -> OrichResult<Vec<Token>> {
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
                '#' => self.skip_line_comment(),
                '/' if self.peek_next() == Some('/') => self.skip_line_comment(),
                '"' if self.peek_next() == Some('"') && self.peek_n(2) == Some('"') => {
                    tokens.push(Token::new(
                        TokenKind::String(self.triple_string(line, column)?),
                        line,
                        column,
                    ));
                }
                '"' => tokens.push(Token::new(
                    TokenKind::String(self.string(line, column)?),
                    line,
                    column,
                )),
                '0'..='9' => tokens.push(Token::new(self.number()?, line, column)),
                'a'..='z' | 'A'..='Z' | '_' => {
                    tokens.push(Token::new(self.identifier(), line, column))
                }
                '+' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Plus, line, column));
                }
                '-' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Minus, line, column));
                }
                '*' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Star, line, column));
                }
                '/' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Slash, line, column));
                }
                '%' => {
                    self.advance();
                    tokens.push(Token::new(TokenKind::Percent, line, column));
                }
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
                        return Err(OrichError::new(
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
                    return Err(OrichError::new(
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

    fn number(&mut self) -> OrichResult<TokenKind> {
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
        let text: String = self.chars[start..self.pos].iter().collect();
        if is_float {
            text.parse::<f64>()
                .map(TokenKind::Float)
                .map_err(|_| OrichError::new("invalid float literal", self.line, self.column))
        } else {
            text.parse::<i64>()
                .map(TokenKind::Int)
                .map_err(|_| OrichError::new("invalid integer literal", self.line, self.column))
        }
    }

    fn string(&mut self, line: usize, column: usize) -> OrichResult<String> {
        self.advance();
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.advance();
                return Ok(out);
            }
            if ch == '\\' {
                self.advance();
                let escaped = self
                    .advance()
                    .ok_or_else(|| OrichError::new("unterminated escape", line, column))?;
                out.push(match escaped {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    other => other,
                });
            } else {
                out.push(ch);
                self.advance();
            }
        }
        Err(OrichError::new("unterminated string", line, column))
    }

    fn triple_string(&mut self, line: usize, column: usize) -> OrichResult<String> {
        self.advance();
        self.advance();
        self.advance();
        let mut out = String::new();
        loop {
            if self.peek().is_none() {
                return Err(OrichError::new("unterminated triple string", line, column));
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

    fn skip_line_comment(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_keywords_and_strings() {
        let tokens = Lexer::new("let name = \"Ana\"\nemit \"Hi {name}\"")
            .tokenize()
            .unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Let));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Emit)));
    }
}
