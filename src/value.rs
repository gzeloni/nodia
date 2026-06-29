// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Runtime value model shared by the checker, interpreter, and standard library.

use crate::ast::Stmt;
use crate::error::{ErrorSpan, NodiaError};
use crate::regex::RuntimeRegex;
use crate::temporal::{DateTimeValue, DateValue, DurationValue};
use crate::textcodec;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

/// Shared reference to a loaded module namespace.
pub type ModuleRef = Rc<RefCell<Module>>;
/// Shared reference to a mutable runtime binding cell.
pub type BindingRef = Rc<RefCell<SharedBinding>>;

/// Stream handles exposed to runtime built-ins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamId {
    /// Standard input stream.
    Stdin,
    /// Standard output stream.
    Stdout,
    /// Standard error stream.
    Stderr,
    /// Runtime-managed file stream.
    File(usize),
}

/// Loaded module state cached by the runtime.
#[derive(Debug, Clone)]
pub struct Module {
    /// Canonical file path of the module.
    pub path: PathBuf,
    /// Names declared at module top level.
    pub declared: Vec<String>,
    /// Exported binding values.
    pub exports: BTreeMap<String, Value>,
    /// Exported binding mutability by name.
    pub mutability: BTreeMap<String, bool>,
    /// Whether the module body has already been executed.
    pub loaded: bool,
}

/// Shared mutable binding cell.
#[derive(Clone)]
pub struct SharedBinding {
    /// Current bound value.
    pub value: Value,
    /// Whether assignments are allowed.
    pub mutable: bool,
}

/// Recoverable pipeline result value.
#[derive(Debug, Clone, PartialEq)]
pub enum ResultValue {
    Ok(Box<Value>),
    Err(RecoverableErrorValue),
}

/// Canonical recoverable error payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoverableErrorValue {
    pub code: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub context: Vec<String>,
    pub span: Option<ErrorSpan>,
}

/// Runtime value representation.
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Result(ResultValue),
    Date(DateValue),
    DateTime(DateTimeValue),
    Duration(DurationValue),
    Regex(RuntimeRegex),
    Stream(StreamId),
    UseBinding(ModuleRef, String),
    BuiltinFunction(String),
    Function(Function),
}

/// User-defined function value with captured bindings.
#[derive(Clone)]
pub struct Function {
    /// Parameter names in declaration order.
    pub params: Vec<String>,
    /// Function body statements.
    pub body: Vec<Stmt>,
    /// Closed-over bindings from outer scopes.
    pub captures: BTreeMap<String, BindingRef>,
}

impl Value {
    /// Evaluates the value using Nodia truthiness rules.
    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(value) => *value,
            Value::Int(value) => *value != 0,
            Value::Float(value) => *value != 0.0,
            Value::String(value) => !value.is_empty(),
            Value::Bytes(value) => !value.is_empty(),
            Value::List(value) => !value.is_empty(),
            Value::Map(value) => !value.is_empty(),
            Value::Result(value) => value.is_ok(),
            Value::Date(_) => true,
            Value::DateTime(_) => true,
            Value::Duration(_) => true,
            Value::Regex(_) => true,
            Value::Stream(_) => true,
            Value::UseBinding(_, _) => true,
            Value::BuiltinFunction(_) => true,
            Value::Function(_) => true,
        }
    }

    /// Returns the user-visible runtime type name.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Bytes(_) => "bytes",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Result(_) => "result",
            Value::Date(_) => "date",
            Value::DateTime(_) => "datetime",
            Value::Duration(_) => "duration",
            Value::Regex(_) => "regex",
            Value::Stream(_) => "stream",
            Value::UseBinding(_, _) => "use",
            Value::BuiltinFunction(_) => "function",
            Value::Function(_) => "function",
        }
    }

    fn write_display(&self, f: &mut fmt::Formatter<'_>, nested: bool) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(value) => write!(f, "{value}"),
            Value::Int(value) => write!(f, "{value}"),
            Value::Float(value) => {
                if value.fract() == 0.0 {
                    write!(f, "{value:.1}")
                } else {
                    write!(f, "{value}")
                }
            }
            Value::String(value) => {
                if nested {
                    write_string_literal(f, value)
                } else {
                    write!(f, "{value}")
                }
            }
            Value::Bytes(value) => write!(f, "{}", textcodec::quote_bytes_literal(value)),
            Value::List(values) => {
                write!(f, "[")?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    value.write_display(f, true)?;
                }
                write!(f, "]")
            }
            Value::Map(values) => {
                write!(f, "{{")?;
                for (index, (key, value)) in values.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write_map_key(f, key)?;
                    write!(f, ": ")?;
                    value.write_display(f, true)?;
                }
                write!(f, "}}")
            }
            Value::Result(value) => value.write_display(f, nested),
            Value::Date(value) => write!(f, "{}", value.isoformat()),
            Value::DateTime(value) => write!(f, "{}", value.isoformat()),
            Value::Duration(value) => write!(f, "{}", value.isoformat()),
            Value::Regex(regex) => write!(f, "{}", regex.rendered()),
            Value::Stream(stream) => write!(f, "{stream}"),
            Value::UseBinding(_, name) => write!(f, "<use {name}>"),
            Value::BuiltinFunction(name) => write!(f, "<builtin {name}>"),
            Value::Function(_) => write!(f, "<func>"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Result(a), Value::Result(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::DateTime(a), Value::DateTime(b)) => a == b,
            (Value::Duration(a), Value::Duration(b)) => a == b,
            (Value::Regex(a), Value::Regex(b)) => a == b,
            (Value::Stream(a), Value::Stream(b)) => a == b,
            (Value::BuiltinFunction(a), Value::BuiltinFunction(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::UseBinding(a_module, a_name), Value::UseBinding(b_module, b_name)) => {
                Rc::ptr_eq(a_module, b_module) && a_name == b_name
            }
            _ => false,
        }
    }
}

impl ResultValue {
    pub fn ok(value: Value) -> Self {
        Self::Ok(Box::new(value))
    }

    pub fn err(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Err(RecoverableErrorValue::new(code, message))
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok(_))
    }

    pub fn is_err(&self) -> bool {
        matches!(self, Self::Err(_))
    }

    pub fn value(&self) -> Option<&Value> {
        match self {
            Self::Ok(value) => Some(value),
            Self::Err(_) => None,
        }
    }

    pub fn error(&self) -> Option<&RecoverableErrorValue> {
        match self {
            Self::Ok(_) => None,
            Self::Err(error) => Some(error),
        }
    }

    fn write_display(&self, f: &mut fmt::Formatter<'_>, nested: bool) -> fmt::Result {
        match self {
            Self::Ok(value) => {
                write!(f, "ok(")?;
                value.write_display(f, nested)?;
                write!(f, ")")
            }
            Self::Err(error) => {
                write!(f, "err(")?;
                error.write_display(f)?;
                write!(f, ")")
            }
        }
    }
}

impl RecoverableErrorValue {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            file: None,
            line: None,
            column: None,
            context: Vec::new(),
            span: None,
        }
    }

    pub fn from_error(error: NodiaError) -> Self {
        Self {
            code: error.code,
            message: error.message,
            file: error.file,
            line: if error.line > 0 {
                Some(error.line)
            } else {
                None
            },
            column: if error.column > 0 {
                Some(error.column)
            } else {
                None
            },
            context: error.context,
            span: error.span,
        }
    }

    pub fn to_error(&self) -> NodiaError {
        NodiaError {
            code: self.code.clone(),
            message: self.message.clone(),
            line: self.line.unwrap_or(0),
            column: self.column.unwrap_or(0),
            file: self.file.clone(),
            context: self.context.clone(),
            span: self.span.clone(),
            exit_status: None,
            output: None,
        }
    }

    pub fn to_map(&self) -> BTreeMap<String, Value> {
        let mut fields = BTreeMap::new();
        fields.insert("code".to_string(), Value::String(self.code.clone()));
        fields.insert("message".to_string(), Value::String(self.message.clone()));
        fields.insert(
            "file".to_string(),
            self.file
                .as_ref()
                .map(|value| Value::String(value.clone()))
                .unwrap_or(Value::Null),
        );
        fields.insert(
            "line".to_string(),
            self.line
                .map(|value| Value::Int(value as i64))
                .unwrap_or(Value::Null),
        );
        fields.insert(
            "column".to_string(),
            self.column
                .map(|value| Value::Int(value as i64))
                .unwrap_or(Value::Null),
        );
        fields.insert(
            "context".to_string(),
            Value::List(
                self.context
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect::<Vec<_>>(),
            ),
        );
        fields.insert(
            "span".to_string(),
            self.span
                .as_ref()
                .map(|span| {
                    let mut span_fields = BTreeMap::new();
                    span_fields.insert("line".to_string(), Value::Int(span.line as i64));
                    span_fields.insert("column".to_string(), Value::Int(span.column as i64));
                    Value::Map(span_fields)
                })
                .unwrap_or(Value::Null),
        );
        fields
    }

    fn write_display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        write_map_key(f, "code")?;
        write!(f, ": ")?;
        Value::String(self.code.clone()).write_display(f, true)?;
        write!(f, ", ")?;
        write_map_key(f, "message")?;
        write!(f, ": ")?;
        Value::String(self.message.clone()).write_display(f, true)?;
        write!(f, ", ")?;
        write_map_key(f, "file")?;
        write!(f, ": ")?;
        self.file
            .as_ref()
            .map(|value| Value::String(value.clone()))
            .unwrap_or(Value::Null)
            .write_display(f, true)?;
        write!(f, ", ")?;
        write_map_key(f, "line")?;
        write!(f, ": ")?;
        self.line
            .map(|value| Value::Int(value as i64))
            .unwrap_or(Value::Null)
            .write_display(f, true)?;
        write!(f, ", ")?;
        write_map_key(f, "column")?;
        write!(f, ": ")?;
        self.column
            .map(|value| Value::Int(value as i64))
            .unwrap_or(Value::Null)
            .write_display(f, true)?;
        if !self.context.is_empty() {
            write!(f, ", ")?;
            write_map_key(f, "context")?;
            write!(f, ": ")?;
            Value::List(
                self.context
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect::<Vec<_>>(),
            )
            .write_display(f, true)?;
        }
        if let Some(span) = &self.span {
            let mut span_fields = BTreeMap::new();
            span_fields.insert("line".to_string(), Value::Int(span.line as i64));
            span_fields.insert("column".to_string(), Value::Int(span.column as i64));
            write!(f, ", ")?;
            write_map_key(f, "span")?;
            write!(f, ": ")?;
            Value::Map(span_fields).write_display(f, true)?;
        }
        write!(f, "}}")
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        self.params == other.params && self.body == other.body
    }
}

impl fmt::Debug for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let captures = self.captures.keys().cloned().collect::<Vec<_>>();
        f.debug_struct("Function")
            .field("params", &self.params)
            .field("body", &self.body)
            .field("captures", &captures)
            .finish()
    }
}

impl fmt::Debug for SharedBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedBinding")
            .field("mutable", &self.mutable)
            .field("value_type", &self.value.type_name())
            .finish()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_display(f, false)
    }
}

impl fmt::Display for StreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StreamId::Stdin => write!(f, "<stream stdin>"),
            StreamId::Stdout => write!(f, "<stream stdout>"),
            StreamId::Stderr => write!(f, "<stream stderr>"),
            StreamId::File(id) => write!(f, "<stream {id}>"),
        }
    }
}

fn write_map_key(f: &mut fmt::Formatter<'_>, key: &str) -> fmt::Result {
    if is_plain_key(key) {
        write!(f, "{key}")
    } else {
        write_string_literal(f, key)
    }
}

fn is_plain_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !matches!(first, 'a'..='z' | 'A'..='Z' | '_') {
        return false;
    }
    chars.all(|ch| matches!(ch, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
}

fn write_string_literal(f: &mut fmt::Formatter<'_>, value: &str) -> fmt::Result {
    write!(f, "\"")?;
    for ch in value.chars() {
        match ch {
            '"' => write!(f, "\\\"")?,
            '\\' => write!(f, "\\\\")?,
            '\n' => write!(f, "\\n")?,
            '\r' => write!(f, "\\r")?,
            '\t' => write!(f, "\\t")?,
            ch if ch.is_control() => write!(f, "\\u{:04x}", ch as u32)?,
            _ => write!(f, "{ch}")?,
        }
    }
    write!(f, "\"")
}
