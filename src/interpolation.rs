// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Shared parsing helpers for interpolated string contents.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Chunk<'a> {
    Text(&'a str),
    EscapedOpen,
    EscapedClose,
    Expr(&'a str),
}

pub(crate) fn parse_chunks(raw: &str) -> Result<Vec<Chunk<'_>>, String> {
    let mut chunks = Vec::new();
    let mut text_start = 0;
    let mut index = 0;

    while index < raw.len() {
        if raw[index..].starts_with("{{") {
            push_text(&mut chunks, raw, text_start, index);
            chunks.push(Chunk::EscapedOpen);
            index += 2;
            text_start = index;
            continue;
        }
        if raw[index..].starts_with("}}") {
            push_text(&mut chunks, raw, text_start, index);
            chunks.push(Chunk::EscapedClose);
            index += 2;
            text_start = index;
            continue;
        }
        if raw[index..].starts_with('{') {
            push_text(&mut chunks, raw, text_start, index);
            let expr_start = index + 1;
            let expr_end = find_interpolation_end(raw, expr_start)?;
            chunks.push(Chunk::Expr(&raw[expr_start..expr_end]));
            index = expr_end + 1;
            text_start = index;
            continue;
        }

        index = advance_char(raw, index);
    }

    push_text(&mut chunks, raw, text_start, raw.len());
    Ok(chunks)
}

fn push_text<'a>(chunks: &mut Vec<Chunk<'a>>, raw: &'a str, start: usize, end: usize) {
    if start < end {
        chunks.push(Chunk::Text(&raw[start..end]));
    }
}

fn find_interpolation_end(raw: &str, start: usize) -> Result<usize, String> {
    let mut index = start;
    let mut brace_depth = 0usize;

    while index < raw.len() {
        if raw[index..].starts_with("r\"\"\"") {
            index = scan_raw_triple_string(raw, index + 4)?;
            continue;
        }
        if raw[index..].starts_with("r\"") {
            index = scan_raw_string(raw, index + 2, '"')?;
            continue;
        }
        if raw[index..].starts_with("r'") {
            index = scan_raw_string(raw, index + 2, '\'')?;
            continue;
        }
        if raw[index..].starts_with("\"\"\"") {
            index = scan_triple_string(raw, index + 3)?;
            continue;
        }
        if raw[index..].starts_with("//") || raw[index..].starts_with('#') {
            index = scan_line_comment(raw, index);
            continue;
        }

        let ch = raw[index..]
            .chars()
            .next()
            .expect("index always points to a char boundary");
        match ch {
            '"' | '\'' => {
                index = scan_string(raw, index + ch.len_utf8(), ch)?;
            }
            '{' => {
                brace_depth += 1;
                index += 1;
            }
            '}' => {
                if brace_depth == 0 {
                    return Ok(index);
                }
                brace_depth -= 1;
                index += 1;
            }
            _ => {
                index += ch.len_utf8();
            }
        }
    }

    Err("unterminated interpolation".to_string())
}

fn scan_string(raw: &str, mut index: usize, quote: char) -> Result<usize, String> {
    while index < raw.len() {
        let ch = raw[index..]
            .chars()
            .next()
            .expect("index always points to a char boundary");
        if ch == '\\' {
            index += ch.len_utf8();
            if index >= raw.len() {
                return Err("unterminated interpolation".to_string());
            }
            index = advance_char(raw, index);
            continue;
        }
        index += ch.len_utf8();
        if ch == quote {
            return Ok(index);
        }
    }

    Err("unterminated interpolation".to_string())
}

fn scan_raw_string(raw: &str, mut index: usize, quote: char) -> Result<usize, String> {
    while index < raw.len() {
        let ch = raw[index..]
            .chars()
            .next()
            .expect("index always points to a char boundary");
        index += ch.len_utf8();
        if ch == quote {
            return Ok(index);
        }
    }

    Err("unterminated interpolation".to_string())
}

fn scan_triple_string(raw: &str, mut index: usize) -> Result<usize, String> {
    while index < raw.len() {
        if raw[index..].starts_with("\"\"\"") {
            return Ok(index + 3);
        }
        index = advance_char(raw, index);
    }

    Err("unterminated interpolation".to_string())
}

fn scan_raw_triple_string(raw: &str, mut index: usize) -> Result<usize, String> {
    while index < raw.len() {
        if raw[index..].starts_with("\"\"\"") {
            return Ok(index + 3);
        }
        index = advance_char(raw, index);
    }

    Err("unterminated interpolation".to_string())
}

fn scan_line_comment(raw: &str, mut index: usize) -> usize {
    if raw[index..].starts_with("//") {
        index += 2;
    } else {
        index += 1;
    }
    while index < raw.len() {
        let ch = raw[index..]
            .chars()
            .next()
            .expect("index always points to a char boundary");
        index += ch.len_utf8();
        if ch == '\n' {
            break;
        }
    }
    index
}

fn advance_char(raw: &str, index: usize) -> usize {
    index
        + raw[index..]
            .chars()
            .next()
            .expect("index always points to a char boundary")
            .len_utf8()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chunks_handles_balanced_nested_braces() {
        let chunks = parse_chunks(r#"{ {name: "Ana"}["name"] }"#).unwrap();

        assert_eq!(chunks, vec![Chunk::Expr(r#" {name: "Ana"}["name"] "#)]);
    }

    #[test]
    fn parse_chunks_ignores_braces_inside_string_and_regex_literals() {
        let chunks =
            parse_chunks(r#"{replace("x", "x", "}")} {regex { one_or_more digit }}"#).unwrap();

        assert_eq!(
            chunks,
            vec![
                Chunk::Expr(r#"replace("x", "x", "}")"#),
                Chunk::Text(" "),
                Chunk::Expr(r#"regex { one_or_more digit }"#),
            ]
        );
    }

    #[test]
    fn parse_chunks_preserves_escaped_braces() {
        let chunks = parse_chunks("{{value}}").unwrap();

        assert_eq!(
            chunks,
            vec![
                Chunk::EscapedOpen,
                Chunk::Text("value"),
                Chunk::EscapedClose,
            ]
        );
    }
}
