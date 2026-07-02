// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Low-level textual scanner state used by the runtime `scan` module.

use crate::error::{NodiaError, NodiaResult};
use crate::regex::RuntimeRegex;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub struct ScannerValue {
    state: Rc<RefCell<ScannerState>>,
}

#[derive(Clone)]
struct ScannerState {
    text: String,
    byte_offset: usize,
    scalar_offset: usize,
    line: usize,
    column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerPosition {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannerSpan {
    pub text: String,
    pub start: ScannerPosition,
    pub end: ScannerPosition,
}

#[derive(Clone, Copy)]
pub enum ScannerPattern<'a> {
    Literal(&'a str),
    Regex(&'a RuntimeRegex),
}

impl ScannerValue {
    pub fn new(text: String) -> Self {
        Self {
            state: Rc::new(RefCell::new(ScannerState {
                text,
                byte_offset: 0,
                scalar_offset: 0,
                line: 1,
                column: 1,
            })),
        }
    }

    pub fn position(&self) -> ScannerPosition {
        let state = self.state.borrow();
        ScannerPosition {
            offset: state.scalar_offset,
            line: state.line,
            column: state.column,
        }
    }

    pub fn at_end(&self) -> bool {
        let state = self.state.borrow();
        state.byte_offset >= state.text.len()
    }

    pub fn lookahead(&self, count: usize) -> Option<String> {
        let state = self.state.borrow();
        if state.byte_offset >= state.text.len() {
            return None;
        }
        let mut out = String::new();
        for ch in state.text[state.byte_offset..].chars().take(count.max(1)) {
            out.push(ch);
        }
        Some(out)
    }

    pub fn advance(&self, count: usize) -> String {
        if count == 0 {
            return String::new();
        }

        let mut state = self.state.borrow_mut();
        let mut out = String::new();
        for _ in 0..count {
            let Some(ch) = state.text[state.byte_offset..].chars().next() else {
                break;
            };
            let len = ch.len_utf8();
            out.push(ch);
            state.byte_offset += len;
            state.scalar_offset += 1;
            if ch == '\n' {
                state.line += 1;
                state.column = 1;
            } else {
                state.column += 1;
            }
        }
        out
    }

    pub fn span_from(&self, start: &ScannerPosition) -> NodiaResult<ScannerSpan> {
        let end = self.position();
        let state = self.state.borrow();
        if start.offset > end.offset {
            return Err(NodiaError::runtime(
                "scan.span() start offset cannot be after current cursor",
            ));
        }
        let Some(start_byte) = byte_offset_at(&state.text, start.offset) else {
            return Err(NodiaError::runtime(
                "scan.span() start offset is out of range",
            ));
        };
        let text = state.text[start_byte..state.byte_offset].to_string();
        Ok(ScannerSpan {
            text,
            start: start.clone(),
            end,
        })
    }

    pub fn take_match(&self, pattern: ScannerPattern<'_>) -> NodiaResult<Option<ScannerSpan>> {
        let start = self.position();
        let Some(width) = self.prefix_width(pattern)? else {
            return Ok(None);
        };
        self.advance(width);
        self.span_from(&start).map(Some)
    }

    pub fn take_while(&self, pattern: ScannerPattern<'_>) -> NodiaResult<ScannerSpan> {
        let start = self.position();
        loop {
            let Some(width) = self.prefix_width(pattern)? else {
                break;
            };
            self.advance(width);
        }
        self.span_from(&start)
    }

    pub fn take_until(&self, pattern: ScannerPattern<'_>) -> NodiaResult<ScannerSpan> {
        let start = self.position();
        while !self.at_end() {
            if self.prefix_width(pattern)?.is_some() {
                break;
            }
            self.advance(1);
        }
        self.span_from(&start)
    }

    fn prefix_width(&self, pattern: ScannerPattern<'_>) -> NodiaResult<Option<usize>> {
        let state = self.state.borrow();
        let remaining = &state.text[state.byte_offset..];
        match pattern {
            ScannerPattern::Literal(literal) => {
                if literal.is_empty() {
                    return Err(NodiaError::runtime("scan pattern cannot be empty"));
                }
                if remaining.starts_with(literal) {
                    Ok(Some(literal.chars().count()))
                } else {
                    Ok(None)
                }
            }
            ScannerPattern::Regex(regex) => match regex.find(remaining)? {
                Some(found) if found.start == 0 => {
                    if found.text.is_empty() {
                        return Err(NodiaError::runtime(
                            "scan regex pattern cannot match empty text",
                        ));
                    }
                    Ok(Some(found.text.chars().count()))
                }
                _ => Ok(None),
            },
        }
    }
}

impl PartialEq for ScannerValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.state, &other.state)
    }
}

impl fmt::Debug for ScannerValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let position = self.position();
        f.debug_struct("ScannerValue")
            .field("offset", &position.offset)
            .field("line", &position.line)
            .field("column", &position.column)
            .finish()
    }
}

fn byte_offset_at(text: &str, scalar_offset: usize) -> Option<usize> {
    if scalar_offset == 0 {
        return Some(0);
    }
    let scalar_len = text.chars().count();
    if scalar_offset == scalar_len {
        return Some(text.len());
    }
    text.char_indices()
        .nth(scalar_offset)
        .map(|(index, _)| index)
}
