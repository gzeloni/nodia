// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Public library API for the Nodia language toolchain.
//!
//! The crate exposes the same high-level pipeline used by the CLI:
//! lex source text, build an AST, validate semantics, format programs,
//! and execute scripts with runtime input.

pub mod ast;
pub mod checker;
pub mod error;
pub mod formatter;
pub mod io;
pub mod lexer;
pub mod parser;
pub mod project;
pub mod regex;
pub mod runtime;
pub mod stdlib;
pub mod temporal;
pub mod token;
pub mod value;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub use ast::Program;
pub use error::{NodiaError, NodiaResult};
pub use runtime::RuntimeOptions;
pub use token::Token;
pub use value::Value;

/// Tokenizes Nodia source text into a flat stream of lexical tokens.
pub fn lex_source(source: &str) -> NodiaResult<Vec<Token>> {
    lexer::Lexer::new(source).tokenize()
}

/// Parses Nodia source text into an abstract syntax tree.
pub fn parse_source(source: &str) -> NodiaResult<Program> {
    let tokens = lex_source(source)?;
    parser::Parser::new(tokens).parse_program()
}

/// Runs semantic validation over Nodia source text without executing it.
pub fn check_source(source: &str) -> NodiaResult<()> {
    let tokens = lex_source(source)?;
    let program = parser::Parser::new(tokens.clone()).parse_program()?;
    checker::check_program_with_tokens(&program, &tokens, None)
}

/// Reads, parses, and semantically validates a source file.
pub fn check_file(path: &Path) -> NodiaResult<()> {
    checker::check_file(path)
}

/// Formats Nodia source text into the canonical project style.
pub fn format_source(source: &str) -> NodiaResult<String> {
    let program = parse_source(source)?;
    Ok(formatter::format_program(&program))
}

/// Executes source text with default runtime options.
pub fn run_source(source: &str, input: BTreeMap<String, Value>) -> NodiaResult<String> {
    run_source_with_options(source, input, RuntimeOptions::default())
}

/// Executes source text with explicit runtime options.
pub fn run_source_with_options(
    source: &str,
    input: BTreeMap<String, Value>,
    options: RuntimeOptions,
) -> NodiaResult<String> {
    let tokens = lex_source(source)?;
    let program = parser::Parser::new(tokens.clone()).parse_program()?;
    checker::check_program_with_tokens(&program, &tokens, None)?;
    let mut runtime = runtime::Runtime::with_options(input, None, options);
    runtime.run(&program)
}

/// Executes a source file with default runtime options.
pub fn run_file(path: &Path, input: BTreeMap<String, Value>) -> NodiaResult<String> {
    run_file_with_options(path, input, RuntimeOptions::default())
}

/// Executes a source file with explicit runtime options.
pub fn run_file_with_options(
    path: &Path,
    input: BTreeMap<String, Value>,
    options: RuntimeOptions,
) -> NodiaResult<String> {
    let source = fs::read_to_string(path)
        .map_err(|err| NodiaError::io(format!("cannot read '{}': {err}", path.display())))?;
    let tokens = lex_source(&source).map_err(|err| err.with_file(path.display().to_string()))?;
    let program = parser::Parser::new(tokens.clone())
        .parse_program()
        .map_err(|err| err.with_file(path.display().to_string()))?;
    checker::check_program_with_tokens(&program, &tokens, path.parent().map(Path::to_path_buf))
        .map_err(|err| err.with_file_if_missing(path.display().to_string()))?;
    let base_dir = path.parent().map(Path::to_path_buf);
    let mut runtime = runtime::Runtime::with_options(input, base_dir, options);
    runtime
        .run(&program)
        .map_err(|err| err.with_file(path.display().to_string()))
}
