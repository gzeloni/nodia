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
pub use error::{DobraError, DobraResult};
pub use runtime::RuntimeOptions;
pub use token::Token;
pub use value::Value;

pub fn lex_source(source: &str) -> DobraResult<Vec<Token>> {
    lexer::Lexer::new(source).tokenize()
}

pub fn parse_source(source: &str) -> DobraResult<Program> {
    let tokens = lex_source(source)?;
    parser::Parser::new(tokens).parse_program()
}

pub fn check_source(source: &str) -> DobraResult<()> {
    let tokens = lex_source(source)?;
    let program = parser::Parser::new(tokens.clone()).parse_program()?;
    checker::check_program_with_tokens(&program, &tokens, None)
}

pub fn check_file(path: &Path) -> DobraResult<()> {
    checker::check_file(path)
}

pub fn format_source(source: &str) -> DobraResult<String> {
    let program = parse_source(source)?;
    Ok(formatter::format_program(&program))
}

pub fn run_source(source: &str, input: BTreeMap<String, Value>) -> DobraResult<String> {
    run_source_with_options(source, input, RuntimeOptions::default())
}

pub fn run_source_with_options(
    source: &str,
    input: BTreeMap<String, Value>,
    options: RuntimeOptions,
) -> DobraResult<String> {
    let tokens = lex_source(source)?;
    let program = parser::Parser::new(tokens.clone()).parse_program()?;
    checker::check_program_with_tokens(&program, &tokens, None)?;
    let mut runtime = runtime::Runtime::with_options(input, None, options);
    runtime.run(&program)
}

pub fn run_file(path: &Path, input: BTreeMap<String, Value>) -> DobraResult<String> {
    run_file_with_options(path, input, RuntimeOptions::default())
}

pub fn run_file_with_options(
    path: &Path,
    input: BTreeMap<String, Value>,
    options: RuntimeOptions,
) -> DobraResult<String> {
    let source = fs::read_to_string(path)
        .map_err(|err| DobraError::io(format!("cannot read '{}': {err}", path.display())))?;
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
