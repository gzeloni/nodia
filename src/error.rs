// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Error types shared by the lexer, parser, checker, runtime, and CLI.

use std::fmt;

/// Result type used throughout the crate.
pub type NodiaResult<T> = Result<T, NodiaError>;

/// Primary error type returned by the library and runtime layers.
#[derive(Debug, Clone, PartialEq)]
pub struct NodiaError {
    /// Stable error code for machine-readable consumers.
    pub code: String,
    /// Human-readable description of the failure.
    pub message: String,
    /// One-based line number when the error is tied to source text.
    pub line: usize,
    /// One-based column number when the error is tied to source text.
    pub column: usize,
    /// Optional file name attached during file-based operations.
    pub file: Option<String>,
    /// Optional process exit status used by runtime `exit(...)`.
    pub exit_status: Option<i32>,
    /// Optional output captured before a controlled runtime exit.
    pub output: Option<String>,
}

impl NodiaError {
    /// Creates a syntax-oriented error with source coordinates.
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            code: "E1000".to_string(),
            message: message.into(),
            line,
            column,
            file: None,
            exit_status: None,
            output: None,
        }
    }

    /// Creates a runtime error without source coordinates.
    pub fn runtime(message: impl Into<String>) -> Self {
        Self {
            code: "E2000".to_string(),
            message: message.into(),
            line: 0,
            column: 0,
            file: None,
            exit_status: None,
            output: None,
        }
    }

    /// Creates an I/O error without source coordinates.
    pub fn io(message: impl Into<String>) -> Self {
        Self {
            code: "E3000".to_string(),
            message: message.into(),
            line: 0,
            column: 0,
            file: None,
            exit_status: None,
            output: None,
        }
    }

    /// Creates a semantic error without precise coordinates.
    pub fn semantic(message: impl Into<String>) -> Self {
        Self::semantic_at(message, 0, 0)
    }

    /// Creates a semantic error with source coordinates.
    pub fn semantic_at(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            code: "E4000".to_string(),
            message: message.into(),
            line,
            column,
            file: None,
            exit_status: None,
            output: None,
        }
    }

    /// Creates a control-flow error that maps to a process exit status.
    pub fn exit(status: i32) -> Self {
        Self {
            code: "EXIT".to_string(),
            message: String::new(),
            line: 0,
            column: 0,
            file: None,
            exit_status: Some(status),
            output: None,
        }
    }

    /// Replaces the error code.
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = code.into();
        self
    }

    /// Attaches a file name to the error.
    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Attaches a file name only when one is not already present.
    pub fn with_file_if_missing(mut self, file: impl Into<String>) -> Self {
        if self.file.is_none() {
            self.file = Some(file.into());
        }
        self
    }

    /// Stores output that should be preserved on controlled exits.
    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output = Some(output.into());
        self
    }

    /// Renders the error in the same human-readable format used by the CLI.
    pub fn render(&self) -> String {
        if let Some(status) = self.exit_status {
            return format!("exit {status}");
        }
        let location = match (&self.file, self.line, self.column) {
            (Some(file), line, column) if line > 0 => format!("\n  at {file}:{line}:{column}"),
            (None, line, column) if line > 0 => format!("\n  at {line}:{column}"),
            (Some(file), _, _) => format!("\n  at {file}"),
            _ => String::new(),
        };
        format!("error[{}]: {}{}", self.code, self.message, location)
    }

    /// Serializes the error into a compact JSON object.
    pub fn to_json(&self) -> String {
        format!(
            "{{\"code\":\"{}\",\"message\":\"{}\",\"file\":{},\"line\":{},\"column\":{}}}",
            json_escape(&self.code),
            json_escape(&self.message),
            self.file
                .as_ref()
                .map(|file| format!("\"{}\"", json_escape(file)))
                .unwrap_or_else(|| "null".to_string()),
            self.line,
            self.column
        )
    }
}

impl fmt::Display for NodiaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

impl std::error::Error for NodiaError {}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out
}
