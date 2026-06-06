// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Shared UTF-8 codec and byte-sequence helpers.

use crate::error::{NodiaError, NodiaResult};
use crate::value::Value;

pub(crate) fn bytes_to_value(bytes: Vec<u8>) -> Value {
    Value::List(
        bytes
            .into_iter()
            .map(|byte| Value::Int(byte as i64))
            .collect(),
    )
}

pub(crate) fn string_to_bytes_value(text: &str) -> Value {
    bytes_to_value(text.as_bytes().to_vec())
}

pub(crate) fn expect_bytes(value: &Value, name: &str, position: &str) -> NodiaResult<Vec<u8>> {
    let Value::List(values) = value else {
        return Err(NodiaError::runtime(format!(
            "{name}() expects list<int> as {position} argument, got {}",
            value.type_name()
        )));
    };

    let mut bytes = Vec::with_capacity(values.len());
    for (index, item) in values.iter().enumerate() {
        match item {
            Value::Int(byte) if (0..=255).contains(byte) => bytes.push(*byte as u8),
            Value::Int(byte) => {
                return Err(NodiaError::runtime(format!(
                    "{name}() expects byte values in range 0..255 at {position} argument index {index}, got {byte}"
                )))
            }
            other => {
                return Err(NodiaError::runtime(format!(
                    "{name}() expects int byte values at {position} argument index {index}, got {}",
                    other.type_name()
                )))
            }
        }
    }

    Ok(bytes)
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
