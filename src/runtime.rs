// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Runtime state and execution engine for Nodia programs.

use crate::ast::{AssignTarget, BinaryOp, Expr, ForBinding, Program, Stmt, UnaryOp, UseTarget};
use crate::error::{NodiaError, NodiaResult};
use crate::io::{self as fsio, IoRegistry};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::regex;
use crate::stdlib;
use crate::value::{BindingRef, Function, Module, ModuleRef, SharedBinding, StreamId, Value};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self as stdio, BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

mod builtins;
mod core;
mod eval;
mod modules;
mod state;

type ModuleCache = Rc<RefCell<HashMap<PathBuf, ModuleRef>>>;
type IoState = Rc<RefCell<IoRegistry>>;

/// Runtime capabilities and process-like context passed into execution.
#[derive(Debug, Clone, Default)]
pub struct RuntimeOptions {
    /// Enables built-ins that write to the filesystem.
    pub allow_write: bool,
    /// Enables environment-variable access.
    pub allow_env: bool,
    /// Enables subprocess execution.
    pub allow_process: bool,
    /// Mirrors the program output channel to process stdout as it is produced.
    pub mirror_output: bool,
    /// Positional arguments exposed through the standard library.
    pub args: Vec<String>,
}

enum Flow {
    None,
    Return(Value),
    Break,
    Continue,
}

#[derive(Clone)]
enum TargetStep {
    Field(String),
    Index(Value),
}

/// Interpreter for already-parsed Nodia programs.
pub struct Runtime {
    scopes: Vec<HashMap<String, BindingRef>>,
    output: String,
    input: BTreeMap<String, Value>,
    base_dir: Option<PathBuf>,
    modules: ModuleCache,
    current_module: Option<ModuleRef>,
    io: IoState,
    options: RuntimeOptions,
}

fn declared_bindings(program: &Program) -> BTreeMap<String, bool> {
    program
        .statements
        .iter()
        .filter_map(|statement| match statement {
            Stmt::Bind { name, mutable, .. } => Some((name.clone(), *mutable)),
            Stmt::Func { name, .. } => Some((name.clone(), false)),
            _ => None,
        })
        .collect()
}

fn statement_export_name(statement: &Stmt) -> Option<&str> {
    match statement {
        Stmt::Bind { name, .. } | Stmt::Func { name, .. } => Some(name),
        Stmt::Assign { target, .. } => assignment_target_root_name(target),
        _ => None,
    }
}

fn assignment_target_root_name(target: &AssignTarget) -> Option<&str> {
    match target {
        AssignTarget::Identifier(name) => Some(name),
        AssignTarget::Get { object, .. } | AssignTarget::Index { object, .. } => {
            assignment_target_root_name(object)
        }
    }
}

fn assign_use_binding(module: ModuleRef, name: &str, value: Value) -> NodiaResult<()> {
    let mut module = module.borrow_mut();
    let mutable = module.mutability.get(name).copied().unwrap_or(false);
    if !mutable {
        return Err(NodiaError::runtime(format!(
            "cannot assign to val '{name}'"
        )));
    }
    if !module.exports.contains_key(name) {
        return Err(NodiaError::runtime(format!(
            "used binding '{name}' is not initialized yet"
        )));
    }
    module.exports.insert(name.to_string(), value);
    Ok(())
}

fn binding_ref(value: Value, mutable: bool) -> BindingRef {
    Rc::new(RefCell::new(SharedBinding { value, mutable }))
}

fn binding_scope(values: &BTreeMap<String, BindingRef>) -> HashMap<String, BindingRef> {
    values
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

fn to_number(value: &Value) -> NodiaResult<f64> {
    match value {
        Value::Int(value) => Ok(*value as f64),
        Value::Float(value) => Ok(*value),
        other => Err(NodiaError::runtime(format!(
            "expected number, got {}",
            other.type_name()
        ))),
    }
}

fn result_access_error(action: &str) -> NodiaError {
    NodiaError::runtime(format!(
        "cannot {action} result; use result.value(...) / result.then(...) for success, or result.error(...) / result.recover(...) for failures"
    ))
}

#[cfg(test)]
mod tests;
