// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Project discovery and initialization helpers for `nodia.toml`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Parsed subset of project configuration used by the CLI.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectConfig {
    /// Human-readable project name.
    pub name: String,
    /// Entry-point source file resolved relative to the config file.
    pub entry: PathBuf,
}

/// Searches upward for the nearest `nodia.toml` file.
pub fn find_project_config(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?
    } else {
        start
    };
    loop {
        let candidate = current.join("nodia.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        current = current.parent()?;
    }
}

/// Reads a project configuration file from disk.
pub fn read_project_config(path: &Path) -> io::Result<ProjectConfig> {
    let content = fs::read_to_string(path)?;
    let mut name = None;
    let mut entry = None;
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "name" => name = Some(unquote(value.trim()).to_string()),
            "entry" => entry = Some(PathBuf::from(unquote(value.trim()))),
            _ => {}
        }
    }
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    Ok(ProjectConfig {
        name: name.unwrap_or_else(|| "nodia-project".to_string()),
        entry: base.join(entry.unwrap_or_else(|| PathBuf::from("src/main.nod"))),
    })
}

/// Creates a minimal project layout in the target directory.
pub fn init_project(dir: &Path) -> io::Result<()> {
    let src = dir.join("src");
    fs::create_dir_all(&src)?;

    let config = dir.join("nodia.toml");
    if !config.exists() {
        fs::write(
            &config,
            "name = \"nodia-project\"\nentry = \"src/main.nod\"\n",
        )?;
    }

    let main = src.join("main.nod");
    if !main.exists() {
        fs::write(&main, "val name = input.name\n\nemit \"Hello, {name}\"\n")?;
    }

    Ok(())
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}
