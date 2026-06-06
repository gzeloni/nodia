// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Internal helpers shared across regex validation and execution.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ReplacementChunk {
    Literal(String),
    Dollar,
    CaptureIndex { raw: String, index: usize },
    CaptureName(String),
}

pub(super) fn replacement_name_is_valid(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if first == '_' || first.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

pub(super) fn char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].chars().count()
}

pub(super) fn parse_replacement_chunks(replacement: &str) -> Result<Vec<ReplacementChunk>, String> {
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
            return Err(
                "regex replacement cannot end with '$'; use '$$' for a literal dollar".to_string(),
            );
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
            return Err(
                "regex replacement placeholders must use $(0), $(1), $(name), or '$$'".to_string(),
            );
        }

        let start = index + 2;
        let mut end = start;
        while end < chars.len() && chars[end] != ')' {
            end += 1;
        }
        if end == chars.len() {
            return Err("unterminated regex replacement placeholder".to_string());
        }

        let token = chars[start..end].iter().collect::<String>();
        if token.is_empty() {
            return Err("regex replacement placeholder cannot be empty".to_string());
        }

        if token.chars().all(|ch| ch.is_ascii_digit()) {
            let capture = token
                .parse::<usize>()
                .map_err(|_| "invalid regex capture index".to_string())?;
            chunks.push(ReplacementChunk::CaptureIndex {
                raw: token,
                index: capture,
            });
        } else if replacement_name_is_valid(&token) {
            chunks.push(ReplacementChunk::CaptureName(token));
        } else {
            return Err(format!("invalid regex replacement placeholder '{token}'"));
        }

        index = end + 1;
    }

    if !literal.is_empty() {
        chunks.push(ReplacementChunk::Literal(literal));
    }

    Ok(chunks)
}

pub(super) fn regex_engine_error(err: fancy_regex::Error) -> NodiaError {
    NodiaError::runtime(format!("regex engine error: {err}"))
}

pub(super) fn regex_error(message: impl Into<String>) -> NodiaError {
    NodiaError::semantic(message).with_code("E4200")
}
