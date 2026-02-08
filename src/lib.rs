pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod runtime;
pub mod stdlib;
pub mod token;
pub mod value;

use std::collections::BTreeMap;

pub use error::{OrichError, OrichResult};
pub use value::Value;

pub fn run_source(source: &str, input: BTreeMap<String, Value>) -> OrichResult<String> {
    let tokens = lexer::Lexer::new(source).tokenize()?;
    let program = parser::Parser::new(tokens).parse_program()?;
    let mut runtime = runtime::Runtime::new(input);
    runtime.run(&program)
}
