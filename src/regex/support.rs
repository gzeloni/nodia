// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Internal helpers shared across regex validation and execution.

use super::*;

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

pub(super) fn regex_engine_error(err: fancy_regex::Error) -> DobraError {
    DobraError::runtime(format!("regex engine error: {err}"))
}

pub(super) fn regex_error(message: impl Into<String>) -> DobraError {
    DobraError::semantic(message).with_code("E4200")
}
