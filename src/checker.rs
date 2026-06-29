// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Semantic validation for parsed Nodia programs.

use crate::ast::{AssignTarget, Expr, ForBinding, Program, Stmt, UseTarget};
use crate::error::{NodiaError, NodiaResult};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::regex;
use crate::stdlib;
use crate::token::{Token, TokenKind};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

mod core;
mod expressions;
mod helpers;
mod statements;
mod symbols;

#[derive(Debug, Clone)]
struct Symbol {
    mutable: bool,
    kind: SymbolKind,
}

#[derive(Debug, Clone)]
enum SymbolKind {
    Unknown,
    Result,
    Function {
        arities: Vec<usize>,
        builtin_target: Option<String>,
    },
    Map(HashMap<String, Symbol>),
    Namespace(HashMap<String, Symbol>),
}

#[derive(Debug, Clone)]
struct ModuleInfo {
    symbols: HashMap<String, Symbol>,
}

#[derive(Debug, Clone, Default)]
struct PositionIndex {
    identifiers: HashMap<String, Vec<(usize, usize)>>,
    keywords: HashMap<&'static str, Vec<(usize, usize)>>,
}

type Scope = HashMap<String, Symbol>;

/// Validates a parsed program without file-system context.
pub fn check_program(program: &Program) -> NodiaResult<()> {
    Checker::new().check_program(program, None, PositionIndex::default())
}

/// Validates a parsed program using its original tokens for better diagnostics.
pub fn check_program_with_tokens(
    program: &Program,
    tokens: &[Token],
    base_dir: Option<PathBuf>,
) -> NodiaResult<()> {
    Checker::new().check_program(program, base_dir, PositionIndex::from_tokens(tokens))
}

/// Validates a parsed program relative to a file-system path.
pub fn check_program_at_path(program: &Program, path: &Path) -> NodiaResult<()> {
    Checker::new().check_program(
        program,
        path.parent().map(Path::to_path_buf),
        PositionIndex::default(),
    )
}

/// Reads, parses, and validates a source file.
pub fn check_file(path: &Path) -> NodiaResult<()> {
    let source = fs::read_to_string(path)
        .map_err(|err| NodiaError::io(format!("cannot read '{}': {err}", path.display())))?;
    let tokens = Lexer::new(&source)
        .tokenize()
        .map_err(|err| err.with_file(path.display().to_string()))?;
    let positions = PositionIndex::from_tokens(&tokens);
    let program = Parser::new(tokens)
        .parse_program()
        .map_err(|err| err.with_file(path.display().to_string()))?;
    Checker::new()
        .check_program(&program, path.parent().map(Path::to_path_buf), positions)
        .map_err(|err| err.with_file_if_missing(path.display().to_string()))
}

struct Checker {
    modules: HashMap<PathBuf, ModuleInfo>,
    loading: HashSet<PathBuf>,
}

struct State<'a> {
    checker: &'a mut Checker,
    scopes: Vec<Scope>,
    base_dir: Option<PathBuf>,
    positions: PositionIndex,
    loop_depth: usize,
    function_depth: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScopeMode {
    Top,
    Nested,
}

enum FieldStatus {
    Found(Symbol),
    Missing,
    Unknown,
}

#[cfg(test)]
mod tests;
