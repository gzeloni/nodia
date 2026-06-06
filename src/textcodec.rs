// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Shared UTF-8 codec and byte-sequence helpers.

use crate::error::{NodiaError, NodiaResult};
use crate::value::Value;

pub(crate) fn bytes_to_value(bytes: Vec<u8>) -> Value {
    Value::Bytes(bytes)
}

pub(crate) fn string_to_bytes_value(text: &str) -> Value {
    bytes_to_value(text.as_bytes().to_vec())
}

pub(crate) fn expect_bytes(value: &Value, name: &str, position: &str) -> NodiaResult<Vec<u8>> {
    let Value::Bytes(bytes) = value else {
        return Err(NodiaError::runtime(format!(
            "{name}() expects bytes as {position} argument, got {}",
            value.type_name()
        )));
    };
    Ok(bytes.clone())
}

pub(crate) fn decode_utf8(bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|err| err.to_string())
}

pub(crate) fn decode_utf8_io(bytes: Vec<u8>, context: &str) -> NodiaResult<String> {
    decode_utf8(bytes).map_err(|err| NodiaError::io(format!("{context}: {err}")))
}

pub(crate) fn decode_utf8_runtime(bytes: Vec<u8>, name: &str) -> NodiaResult<String> {
    decode_utf8(bytes)
        .map_err(|err| NodiaError::runtime(format!("{name}() cannot decode bytes as UTF-8: {err}")))
}

pub(crate) fn decode_utf8_lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

pub(crate) fn normalize_lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

pub(crate) fn normalize_crlf(text: &str) -> String {
    normalize_lf(text).replace('\n', "\r\n")
}

pub(crate) fn strip_bom(text: &str) -> String {
    text.strip_prefix('\u{feff}').unwrap_or(text).to_string()
}

pub(crate) fn drop_nul(text: &str) -> String {
    text.chars().filter(|ch| *ch != '\0').collect()
}

pub(crate) fn quote_bytes_literal(bytes: &[u8]) -> String {
    let mut out = String::from("b\"");
    let mut index = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                out.push_str("\\\\");
                index += 1;
            }
            b'"' => {
                out.push_str("\\\"");
                index += 1;
            }
            b'\n' => {
                out.push_str("\\n");
                index += 1;
            }
            b'\r' => {
                out.push_str("\\r");
                index += 1;
            }
            b'\t' => {
                out.push_str("\\t");
                index += 1;
            }
            0 => {
                out.push_str("\\0");
                index += 1;
            }
            byte if matches!(byte, 0x20..=0x7e) => {
                out.push(byte as char);
                index += 1;
            }
            _ => {
                if let Some((ch, len)) = decode_one_visible_char(&bytes[index..]) {
                    out.push(ch);
                    index += len;
                } else {
                    out.push_str(&format!("\\x{:02x}", bytes[index]));
                    index += 1;
                }
            }
        }
    }
    out.push('"');
    out
}

fn decode_one_visible_char(bytes: &[u8]) -> Option<(char, usize)> {
    for len in 2..=4 {
        let slice = bytes.get(..len)?;
        let text = std::str::from_utf8(slice).ok()?;
        let mut chars = text.chars();
        let ch = chars.next()?;
        if chars.next().is_none() && !ch.is_control() && ch != '"' && ch != '\\' {
            return Some((ch, len));
        }
    }
    None
}
