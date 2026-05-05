use crate::ast::Stmt;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

pub type ModuleRef = Rc<RefCell<Module>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StreamId {
    Stdin,
    Stdout,
    Stderr,
    File(usize),
}

#[derive(Debug, Clone)]
pub struct Module {
    pub path: PathBuf,
    pub declared: Vec<String>,
    pub exports: BTreeMap<String, Value>,
    pub mutability: BTreeMap<String, bool>,
    pub loaded: bool,
}

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Stream(StreamId),
    ImportBinding(ModuleRef, String),
    Function(Function),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub captures: BTreeMap<String, Value>,
}

impl Value {
    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(value) => *value,
            Value::Int(value) => *value != 0,
            Value::Float(value) => *value != 0.0,
            Value::String(value) => !value.is_empty(),
            Value::List(value) => !value.is_empty(),
            Value::Map(value) => !value.is_empty(),
            Value::Stream(_) => true,
            Value::ImportBinding(_, _) => true,
            Value::Function(_) => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Stream(_) => "stream",
            Value::ImportBinding(_, _) => "import",
            Value::Function(_) => "function",
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
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Stream(a), Value::Stream(b)) => a == b,
            (Value::Function(a), Value::Function(b)) => a == b,
            (Value::ImportBinding(a_module, a_name), Value::ImportBinding(b_module, b_name)) => {
                Rc::ptr_eq(a_module, b_module) && a_name == b_name
            }
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            Value::String(value) => write!(f, "{value}"),
            Value::List(values) => {
                write!(f, "[")?;
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{value}")?;
                }
                write!(f, "]")
            }
            Value::Map(values) => {
                write!(f, "{{")?;
                for (index, (key, value)) in values.iter().enumerate() {
                    if index > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{key}: {value}")?;
                }
                write!(f, "}}")
            }
            Value::Stream(stream) => write!(f, "{stream}"),
            Value::ImportBinding(_, name) => write!(f, "<import {name}>"),
            Value::Function(_) => write!(f, "<fn>"),
        }
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
