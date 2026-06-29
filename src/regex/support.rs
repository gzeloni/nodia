// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Internal helpers shared across regex validation and execution.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReplacementChunk {
    Literal(String),
    Dollar,
    CaptureIndex {
        raw: String,
        index: usize,
        line: usize,
        column: usize,
    },
    CaptureName {
        name: String,
        line: usize,
        column: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReplacementError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

pub(super) fn replacement_name_is_valid(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

pub(super) fn scalar_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].chars().count()
}

pub(super) fn parse_replacement_chunks(
    replacement: &str,
) -> Result<Vec<ReplacementChunk>, ReplacementError> {
    let chars = replacement.chars().collect::<Vec<_>>();
    let mut chunks = Vec::new();
    let mut literal = String::new();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] != '$' {
            literal.push(chars[index]);
            index += 1;
            continue;
        }

        let Some(next) = chars.get(index + 1).copied() else {
            return Err(replacement_error(
                &chars,
                index,
                "regex replacement cannot end with '$'; use '$$' for a literal dollar",
            ));
        };

        if !literal.is_empty() {
            chunks.push(ReplacementChunk::Literal(std::mem::take(&mut literal)));
        }

        if next == '$' {
            chunks.push(ReplacementChunk::Dollar);
            index += 2;
            continue;
        }

        if next != '(' {
            return Err(replacement_error(
                &chars,
                index,
                "regex replacement placeholders must use $(0), $(1), $(name), or '$$'",
            ));
        }

        let start = index + 2;
        let mut end = start;
        while end < chars.len() && chars[end] != ')' {
            end += 1;
        }
        if end == chars.len() {
            return Err(replacement_error(
                &chars,
                index,
                "unterminated regex replacement placeholder",
            ));
        }

        let token = chars[start..end].iter().collect::<String>();
        if token.is_empty() {
            return Err(replacement_error(
                &chars,
                index,
                "regex replacement placeholder cannot be empty",
            ));
        }

        let (line, column) = replacement_line_column(&chars, index);

        if token.chars().all(|ch| ch.is_ascii_digit()) {
            let capture = token
                .parse::<usize>()
                .map_err(|_| replacement_error(&chars, index, "invalid regex capture index"))?;
            chunks.push(ReplacementChunk::CaptureIndex {
                raw: token,
                index: capture,
                line,
                column,
            });
        } else if replacement_name_is_valid(&token) {
            chunks.push(ReplacementChunk::CaptureName {
                name: token,
                line,
                column,
            });
        } else {
            return Err(replacement_error(
                &chars,
                index,
                format!("invalid regex replacement placeholder '{token}'"),
            ));
        }

        index = end + 1;
    }

    if !literal.is_empty() {
        chunks.push(ReplacementChunk::Literal(literal));
    }

    Ok(chunks)
}

fn replacement_error(
    chars: &[char],
    char_index: usize,
    message: impl Into<String>,
) -> ReplacementError {
    let (line, column) = replacement_line_column(chars, char_index);
    ReplacementError {
        message: message.into(),
        line,
        column,
    }
}

fn replacement_line_column(chars: &[char], char_index: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    for ch in chars.iter().take(char_index) {
        if *ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

pub(super) fn regex_engine_error(err: fancy_regex::Error) -> NodiaError {
    NodiaError::runtime(format!("regex engine error: {err}"))
}

pub(super) fn regex_error(message: impl Into<String>) -> NodiaError {
    NodiaError::semantic(message).with_code("E4200")
}
