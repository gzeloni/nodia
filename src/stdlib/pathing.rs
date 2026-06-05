// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Filesystem path standard-library functions.

use super::expect_arity;
use crate::error::{NodiaError, NodiaResult};
use crate::value::Value;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub fn basename(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 1, "basename")?;
    let path = expect_string(&args[0], "basename", "first")?;
    Ok(Value::String(path_basename(&path)))
}

pub fn dirname(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 1, "dirname")?;
    let path = expect_string(&args[0], "dirname", "first")?;
    Ok(Value::String(path_dirname(&path)))
}

pub fn exists(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 1, "exists")?;
    let path = expect_string(&args[0], "exists", "first")?;
    Ok(Value::Bool(Path::new(&path).exists()))
}

pub fn is_file(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 1, "is_file")?;
    let path = expect_string(&args[0], "is_file", "first")?;
    Ok(Value::Bool(Path::new(&path).is_file()))
}

pub fn is_dir(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 1, "is_dir")?;
    let path = expect_string(&args[0], "is_dir", "first")?;
    Ok(Value::Bool(Path::new(&path).is_dir()))
}

pub fn list_dir(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 1, "list_dir")?;
    let path = expect_string(&args[0], "list_dir", "first")?;
    let names = sorted_dir_entry_names(Path::new(&path), "list_dir")?;
    Ok(Value::List(names.into_iter().map(Value::String).collect()))
}

pub fn glob(args: &[Value]) -> NodiaResult<Value> {
    expect_arity(&args, 1, "glob")?;
    let pattern = expect_string(&args[0], "glob", "first")?;
    let matches = expand_glob(&pattern)?;
    Ok(Value::List(
        matches.into_iter().map(Value::String).collect(),
    ))
}

fn expect_string(value: &Value, name: &str, position: &str) -> NodiaResult<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        other => Err(NodiaError::runtime(format!(
            "{name}() expects string as {position} argument, got {}",
            other.type_name()
        ))),
    }
}

fn path_basename(path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }

    let trimmed = trim_trailing_separators(path);
    let normalized = Path::new(trimmed);
    if let Some(name) = normalized.file_name() {
        return name.to_string_lossy().to_string();
    }
    if normalized.has_root() {
        return std::path::MAIN_SEPARATOR.to_string();
    }
    trimmed.to_string()
}

fn path_dirname(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }

    let trimmed = trim_trailing_separators(path);
    let normalized = Path::new(trimmed);
    match normalized.parent() {
        Some(parent) if parent.as_os_str().is_empty() => ".".to_string(),
        Some(parent) => parent.to_string_lossy().to_string(),
        None if normalized.has_root() => std::path::MAIN_SEPARATOR.to_string(),
        None => ".".to_string(),
    }
}

fn trim_trailing_separators(path: &str) -> &str {
    let mut end = path.len();
    while end > 1 {
        let ch = path[..end]
            .chars()
            .next_back()
            .expect("slice is non-empty when end > 1");
        if ch == '/' || ch == '\\' {
            end -= ch.len_utf8();
        } else {
            break;
        }
    }
    &path[..end]
}

fn sorted_dir_entry_names(path: &Path, name: &str) -> NodiaResult<Vec<String>> {
    let mut entries = fs::read_dir(path)
        .map_err(|err| NodiaError::io(format!("{name}() cannot read '{}': {err}", path.display())))?
        .map(|entry| {
            entry
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .map_err(|err| {
                    NodiaError::io(format!(
                        "{name}() cannot read entry in '{}': {err}",
                        path.display()
                    ))
                })
        })
        .collect::<NodiaResult<Vec<_>>>()?;
    entries.sort();
    Ok(entries)
}

fn expand_glob(pattern: &str) -> NodiaResult<Vec<String>> {
    let output_absolute = Path::new(pattern).is_absolute();
    let root = if output_absolute {
        absolute_root(pattern)
    } else {
        env::current_dir()
            .map_err(|err| NodiaError::io(format!("glob() cannot read current dir: {err}")))?
    };
    let segments = split_pattern_segments(pattern);
    let mut matches = Vec::new();
    glob_walk(&root, &segments, 0, output_absolute, &root, &mut matches)?;
    matches.sort();
    matches.dedup();
    Ok(matches)
}

fn glob_walk(
    current: &Path,
    segments: &[String],
    index: usize,
    output_absolute: bool,
    relative_root: &Path,
    matches: &mut Vec<String>,
) -> NodiaResult<()> {
    if index == segments.len() {
        if current.exists() {
            matches.push(render_glob_path(current, output_absolute, relative_root));
        }
        return Ok(());
    }

    let segment = &segments[index];
    if segment == "**" {
        glob_walk(
            current,
            segments,
            index + 1,
            output_absolute,
            relative_root,
            matches,
        )?;
        if !current.is_dir() {
            return Ok(());
        }
        for entry in sorted_dir_entries(current, "glob")? {
            if entry
                .file_type()
                .map_err(|err| {
                    NodiaError::io(format!(
                        "glob() cannot inspect '{}': {err}",
                        entry.path().display()
                    ))
                })?
                .is_dir()
            {
                glob_walk(
                    &entry.path(),
                    segments,
                    index,
                    output_absolute,
                    relative_root,
                    matches,
                )?;
            }
        }
        return Ok(());
    }

    if !current.is_dir() {
        return Ok(());
    }

    if has_wildcards(segment) {
        for entry in sorted_dir_entries(current, "glob")? {
            let name = entry.file_name().to_string_lossy().to_string();
            if wildcard_matches(segment, &name) {
                let path = entry.path();
                if index + 1 == segments.len()
                    || entry
                        .file_type()
                        .map_err(|err| {
                            NodiaError::io(format!(
                                "glob() cannot inspect '{}': {err}",
                                path.display()
                            ))
                        })?
                        .is_dir()
                {
                    glob_walk(
                        &path,
                        segments,
                        index + 1,
                        output_absolute,
                        relative_root,
                        matches,
                    )?;
                }
            }
        }
        return Ok(());
    }

    let next = current.join(segment);
    if index + 1 == segments.len() || next.is_dir() {
        glob_walk(
            &next,
            segments,
            index + 1,
            output_absolute,
            relative_root,
            matches,
        )?;
    }
    Ok(())
}

fn sorted_dir_entries(path: &Path, name: &str) -> NodiaResult<Vec<fs::DirEntry>> {
    let mut entries = fs::read_dir(path)
        .map_err(|err| NodiaError::io(format!("{name}() cannot read '{}': {err}", path.display())))?
        .map(|entry| {
            entry.map_err(|err| {
                NodiaError::io(format!(
                    "{name}() cannot read entry in '{}': {err}",
                    path.display()
                ))
            })
        })
        .collect::<NodiaResult<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_string());
    Ok(entries)
}

fn render_glob_path(path: &Path, output_absolute: bool, relative_root: &Path) -> String {
    if output_absolute {
        return path.to_string_lossy().to_string();
    }

    match path.strip_prefix(relative_root) {
        Ok(relative) if relative.as_os_str().is_empty() => ".".to_string(),
        Ok(relative) => relative.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

fn split_pattern_segments(pattern: &str) -> Vec<String> {
    Path::new(pattern)
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            Component::CurDir => Some(".".to_string()),
            Component::ParentDir => Some("..".to_string()),
            Component::Prefix(_) | Component::RootDir => None,
        })
        .collect()
}

fn absolute_root(pattern: &str) -> PathBuf {
    let mut root = PathBuf::new();
    for component in Path::new(pattern).components() {
        match component {
            Component::Prefix(prefix) => root.push(prefix.as_os_str()),
            Component::RootDir => root.push(std::path::MAIN_SEPARATOR.to_string()),
            Component::CurDir | Component::ParentDir | Component::Normal(_) => break,
        }
    }
    root
}

fn has_wildcards(segment: &str) -> bool {
    segment.contains('*') || segment.contains('?')
}

fn wildcard_matches(pattern: &str, text: &str) -> bool {
    let pattern = pattern.chars().collect::<Vec<_>>();
    let text = text.chars().collect::<Vec<_>>();
    let mut dp = vec![vec![false; text.len() + 1]; pattern.len() + 1];
    dp[0][0] = true;

    for index in 1..=pattern.len() {
        if pattern[index - 1] == '*' {
            dp[index][0] = dp[index - 1][0];
        }
    }

    for left in 1..=pattern.len() {
        for right in 1..=text.len() {
            dp[left][right] = match pattern[left - 1] {
                '*' => dp[left - 1][right] || dp[left][right - 1],
                '?' => dp[left - 1][right - 1],
                ch => dp[left - 1][right - 1] && ch == text[right - 1],
            };
        }
    }

    dp[pattern.len()][text.len()]
}
