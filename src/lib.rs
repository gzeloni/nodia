pub mod ast;
pub mod error;
pub mod formatter;
pub mod lexer;
pub mod parser;
pub mod project;
pub mod runtime;
pub mod stdlib;
pub mod token;
pub mod value;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub use ast::Program;
pub use error::{OrichError, OrichResult};
pub use token::Token;
pub use value::Value;

pub fn lex_source(source: &str) -> OrichResult<Vec<Token>> {
    lexer::Lexer::new(source).tokenize()
}

pub fn parse_source(source: &str) -> OrichResult<Program> {
    let tokens = lex_source(source)?;
    parser::Parser::new(tokens).parse_program()
}

pub fn check_source(source: &str) -> OrichResult<()> {
    parse_source(source).map(|_| ())
}

pub fn format_source(source: &str) -> OrichResult<String> {
    let program = parse_source(source)?;
    Ok(formatter::format_program(&program))
}

pub fn run_source(source: &str, input: BTreeMap<String, Value>) -> OrichResult<String> {
    let program = parse_source(source)?;
    let mut runtime = runtime::Runtime::new(input);
    runtime.run(&program)
}

pub fn run_file(path: &Path, input: BTreeMap<String, Value>) -> OrichResult<String> {
    let source = fs::read_to_string(path)
        .map_err(|err| OrichError::io(format!("cannot read '{}': {err}", path.display())))?;
    let program = parse_source(&source).map_err(|err| err.with_file(path.display().to_string()))?;
    let base_dir = path.parent().map(Path::to_path_buf);
    let mut runtime = runtime::Runtime::with_base_dir(input, base_dir);
    runtime
        .run(&program)
        .map_err(|err| err.with_file(path.display().to_string()))
}
