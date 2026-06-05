// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Command-line interface for the `nodia` executable.

use nodia::project;
use nodia::{
    check_file, format_source, lex_source, parse_source, run_file_with_options,
    run_source_with_options, NodiaError, RuntimeOptions, Value,
};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

const EXIT_LANGUAGE: i32 = 1;
const EXIT_USAGE: i32 = 2;
const EXIT_IO: i32 = 3;

#[derive(Default)]
struct Options {
    json: bool,
    quiet: bool,
    verbose: bool,
    color: ColorMode,
    allow_write: bool,
    allow_env: bool,
    allow_process: bool,
}

enum ColorMode {
    Auto,
    Always,
    Never,
}

impl Default for ColorMode {
    fn default() -> Self {
        Self::Auto
    }
}

struct CliError {
    code: i32,
    message: String,
    output: Option<String>,
    exit_status: Option<i32>,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_USAGE,
            message: message.into(),
            output: None,
            exit_status: None,
        }
    }

    fn language(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_LANGUAGE,
            message: message.into(),
            output: None,
            exit_status: None,
        }
    }

    fn io(message: impl Into<String>) -> Self {
        Self {
            code: EXIT_IO,
            message: message.into(),
            output: None,
            exit_status: None,
        }
    }

    fn language_runtime(err: NodiaError) -> Self {
        Self {
            code: err.exit_status.unwrap_or(EXIT_LANGUAGE),
            message: err.render(),
            output: err.output,
            exit_status: err.exit_status,
        }
    }
}

pub fn run(args: Vec<String>) -> i32 {
    let mut options = Options::default();
    match run_inner(args, &mut options) {
        Ok(()) => 0,
        Err(err) => {
            if let Some(status) = err.exit_status.or_else(|| exit_status(&err.message)) {
                if let Some(output) = err.output.filter(|output| !output.is_empty()) {
                    if !options.quiet {
                        println!("{output}");
                    }
                }
                return status;
            }
            if options.json {
                if err.message.trim_start().starts_with('{') {
                    eprintln!("{}", err.message);
                } else {
                    eprintln!(
                        "{{\"ok\":false,\"error\":{{\"message\":\"{}\",\"exit_code\":{}}}}}",
                        json_escape(&err.message),
                        err.code
                    );
                }
            } else {
                eprintln!("{}", err.message);
            }
            err.code
        }
    }
}

fn exit_status(message: &str) -> Option<i32> {
    message
        .strip_prefix("exit ")
        .and_then(|value| value.trim().parse::<i32>().ok())
}

fn run_inner(args: Vec<String>, options: &mut Options) -> Result<(), CliError> {
    let mut args = args.into_iter().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    parse_global_flags(&mut args, options)?;
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    let command = args.remove(0);
    match command.as_str() {
        "run" => run_command(args, options),
        "check" => check_command(args, options),
        "fmt" => fmt_command(args, options),
        "eval" | "-e" => eval_command(args, options),
        "tokens" => tokens_command(args, options),
        "ast" => ast_command(args, options),
        "init" => init_command(args, options),
        "version" => version_command(args, options),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "--version" | "-V" => version_command(Vec::new(), options),
        other => Err(CliError::usage(format!("unknown command '{other}'"))),
    }
}

fn parse_global_flags(args: &mut Vec<String>, options: &mut Options) -> Result<(), CliError> {
    while !args.is_empty() {
        match args[0].as_str() {
            "--json" => {
                options.json = true;
                args.remove(0);
            }
            "--quiet" => {
                options.quiet = true;
                args.remove(0);
            }
            "--verbose" => {
                options.verbose = true;
                args.remove(0);
            }
            "--allow-write" => {
                options.allow_write = true;
                args.remove(0);
            }
            "--allow-env" => {
                options.allow_env = true;
                args.remove(0);
            }
            "--allow-process" => {
                options.allow_process = true;
                args.remove(0);
            }
            "--color" => {
                let value = args
                    .get(1)
                    .ok_or_else(|| CliError::usage("--color expects auto, always or never"))?
                    .clone();
                options.color = match value.as_str() {
                    "auto" => ColorMode::Auto,
                    "always" => ColorMode::Always,
                    "never" => ColorMode::Never,
                    _ => return Err(CliError::usage("--color expects auto, always or never")),
                };
                args.remove(0);
                args.remove(0);
            }
            "--help" | "-h" | "--version" | "-V" => break,
            value if value.starts_with('-') => break,
            _ => break,
        }
    }
    Ok(())
}

fn parse_command_flags(args: &mut Vec<String>, options: &mut Options) -> Result<(), CliError> {
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => {
                options.json = true;
                args.remove(index);
            }
            "--quiet" => {
                options.quiet = true;
                args.remove(index);
            }
            "--verbose" => {
                options.verbose = true;
                args.remove(index);
            }
            "--allow-write" => {
                options.allow_write = true;
                args.remove(index);
            }
            "--allow-env" => {
                options.allow_env = true;
                args.remove(index);
            }
            "--allow-process" => {
                options.allow_process = true;
                args.remove(index);
            }
            "--color" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| CliError::usage("--color expects auto, always or never"))?
                    .clone();
                options.color = match value.as_str() {
                    "auto" => ColorMode::Auto,
                    "always" => ColorMode::Always,
                    "never" => ColorMode::Never,
                    _ => return Err(CliError::usage("--color expects auto, always or never")),
                };
                args.remove(index);
                args.remove(index);
            }
            _ => index += 1,
        }
    }
    Ok(())
}

fn run_command(mut args: Vec<String>, options: &mut Options) -> Result<(), CliError> {
    parse_command_flags(&mut args, options)?;
    let mut path = None;
    let mut out_path = None;
    let mut vars = Vec::new();
    let mut script_args = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--" => {
                script_args.extend_from_slice(&args[index + 1..]);
                break;
            }
            "--var" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| CliError::usage("--var expects key=value"))?
                    .clone();
                vars.push(value);
                index += 2;
            }
            "--vars" => {
                index += 1;
                while index < args.len() && !args[index].starts_with("--") {
                    vars.push(args[index].clone());
                    index += 1;
                }
            }
            "--out" | "--output" | "-o" => {
                if let Some(next) = args.get(index + 1).filter(|value| !value.starts_with("--")) {
                    out_path = Some(PathBuf::from(next));
                    index += 2;
                } else {
                    out_path = Some(PathBuf::new());
                    index += 1;
                }
            }
            "--stdout" => index += 1,
            value if path.is_none() => {
                path = Some(value.to_string());
                index += 1;
            }
            value => return Err(CliError::usage(format!("unexpected argument '{value}'"))),
        }
    }

    let input = parse_vars(&vars)?;
    let output = if path.as_deref() == Some("-") {
        let source = read_stdin()?;
        run_source_with_options(&source, input, runtime_options(options, script_args))
            .map_err(CliError::language_runtime)?
    } else {
        let path = resolve_entry(path.as_deref())?;
        ensure_dob(&path)?;
        let output = run_file_with_options(&path, input, runtime_options(options, script_args))
            .map_err(CliError::language_runtime)?;
        if let Some(target) = out_path {
            let target = if target.as_os_str().is_empty() {
                PathBuf::from(format!("{}.out", path.display()))
            } else {
                target
            };
            fs::write(&target, &output).map_err(|err| {
                CliError::io(format!("cannot write '{}': {err}", target.display()))
            })?;
            return Ok(());
        }
        output
    };

    if !options.quiet {
        println!("{output}");
    }
    Ok(())
}

fn check_command(mut args: Vec<String>, options: &mut Options) -> Result<(), CliError> {
    parse_command_flags(&mut args, options)?;
    let path = resolve_entry(args.first().map(String::as_str))?;
    ensure_dob(&path)?;
    match check_file(&path) {
        Ok(()) => {
            if options.json {
                println!("{{\"ok\":true,\"errors\":[]}}");
            } else if !options.quiet {
                println!("ok {}", path.display());
            }
            Ok(())
        }
        Err(err) => {
            let err = err.with_file(path.display().to_string());
            if options.json {
                Err(CliError::language(format!(
                    "{{\"ok\":false,\"errors\":[{}]}}",
                    err.to_json()
                )))
            } else {
                Err(CliError::language(err.render()))
            }
        }
    }
}

fn fmt_command(mut args: Vec<String>, options: &mut Options) -> Result<(), CliError> {
    parse_command_flags(&mut args, options)?;
    let mut check = false;
    let mut stdout = false;
    let mut targets = Vec::new();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--check" => {
                check = true;
                index += 1;
            }
            "--stdout" => {
                stdout = true;
                index += 1;
            }
            value => {
                targets.push(PathBuf::from(value));
                index += 1;
            }
        }
    }
    if targets.is_empty() {
        targets.push(PathBuf::from("."));
    }

    let mut files = Vec::new();
    for target in targets {
        collect_dob_files(&target, &mut files)?;
    }
    files.sort();
    files.dedup();

    let mut changed = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file)
            .map_err(|err| CliError::io(format!("cannot read '{}': {err}", file.display())))?;
        let formatted = format_source(&source).map_err(|err| {
            CliError::language(err.with_file(file.display().to_string()).render())
        })?;
        if stdout {
            print!("{formatted}");
            continue;
        }
        if formatted != source {
            changed.push(file.clone());
            if !check {
                fs::write(&file, formatted).map_err(|err| {
                    CliError::io(format!("cannot write '{}': {err}", file.display()))
                })?;
            }
        }
    }

    if check && !changed.is_empty() {
        return Err(CliError::language(format!(
            "format check failed: {} file(s) need formatting",
            changed.len()
        )));
    }
    if !options.quiet && !stdout {
        if check {
            println!("ok format check");
        } else {
            println!("ok formatted");
        }
    }
    Ok(())
}

fn eval_command(mut args: Vec<String>, options: &mut Options) -> Result<(), CliError> {
    parse_command_flags(&mut args, options)?;
    if args.is_empty() {
        return Err(CliError::usage("eval expects source code"));
    }
    let (source_args, script_args) = split_script_args(args);
    if source_args.is_empty() {
        return Err(CliError::usage("eval expects source code"));
    }
    let source = source_args.join(" ");
    let output = run_source_with_options(
        &source,
        BTreeMap::new(),
        runtime_options(options, script_args),
    )
    .map_err(CliError::language_runtime)?;
    if !options.quiet {
        println!("{output}");
    }
    Ok(())
}

fn tokens_command(mut args: Vec<String>, options: &mut Options) -> Result<(), CliError> {
    parse_command_flags(&mut args, options)?;
    let path = args
        .first()
        .ok_or_else(|| CliError::usage("tokens expects a .nod path"))?;
    let source = fs::read_to_string(path)
        .map_err(|err| CliError::io(format!("cannot read '{path}': {err}")))?;
    let tokens =
        lex_source(&source).map_err(|err| CliError::language(err.with_file(path).render()))?;
    if options.json {
        println!("{}", tokens_json(&tokens));
    } else {
        for token in tokens {
            println!("{}:{} {}", token.line, token.column, token.kind);
        }
    }
    Ok(())
}

fn ast_command(mut args: Vec<String>, options: &mut Options) -> Result<(), CliError> {
    parse_command_flags(&mut args, options)?;
    let path = args
        .first()
        .ok_or_else(|| CliError::usage("ast expects a .nod path"))?;
    let source = fs::read_to_string(path)
        .map_err(|err| CliError::io(format!("cannot read '{path}': {err}")))?;
    let program =
        parse_source(&source).map_err(|err| CliError::language(err.with_file(path).render()))?;
    if options.json {
        println!(
            "{{\"ok\":true,\"ast\":\"{}\"}}",
            json_escape(&format!("{program:#?}"))
        );
    } else {
        println!("{program:#?}");
    }
    Ok(())
}

fn init_command(mut args: Vec<String>, options: &mut Options) -> Result<(), CliError> {
    parse_command_flags(&mut args, options)?;
    let dir = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    project::init_project(&dir)
        .map_err(|err| CliError::io(format!("cannot initialize '{}': {err}", dir.display())))?;
    if options.json {
        println!(
            "{{\"ok\":true,\"path\":\"{}\"}}",
            json_escape(&dir.display().to_string())
        );
    } else if !options.quiet {
        println!("created Nodia project at {}", dir.display());
    }
    Ok(())
}

fn version_command(mut args: Vec<String>, options: &mut Options) -> Result<(), CliError> {
    parse_command_flags(&mut args, options)?;
    if let Some(value) = args.first() {
        return Err(CliError::usage(format!("unexpected argument '{value}'")));
    }
    if options.json {
        println!(
            "{{\"name\":\"nodia\",\"version\":\"{}\",\"rust_std_only\":true}}",
            env!("CARGO_PKG_VERSION")
        );
    } else {
        println!("nodia {}", env!("CARGO_PKG_VERSION"));
    }
    Ok(())
}

fn runtime_options(options: &Options, args: Vec<String>) -> RuntimeOptions {
    RuntimeOptions {
        allow_write: options.allow_write,
        allow_env: options.allow_env,
        allow_process: options.allow_process,
        args,
    }
}

fn split_script_args(args: Vec<String>) -> (Vec<String>, Vec<String>) {
    if let Some(index) = args.iter().position(|value| value == "--") {
        (args[..index].to_vec(), args[index + 1..].to_vec())
    } else {
        (args, Vec::new())
    }
}

fn resolve_entry(path: Option<&str>) -> Result<PathBuf, CliError> {
    if let Some(path) = path {
        return Ok(PathBuf::from(path));
    }
    let cwd = env::current_dir()
        .map_err(|err| CliError::io(format!("cannot read current dir: {err}")))?;
    let config = project::find_project_config(&cwd)
        .ok_or_else(|| CliError::usage("missing .nod path and no nodia.toml found"))?;
    let config = project::read_project_config(&config)
        .map_err(|err| CliError::io(format!("cannot read project config: {err}")))?;
    Ok(config.entry)
}

fn collect_dob_files(target: &Path, files: &mut Vec<PathBuf>) -> Result<(), CliError> {
    if target.is_file() {
        ensure_dob(target)?;
        files.push(target.to_path_buf());
        return Ok(());
    }
    if target.is_dir() {
        for entry in fs::read_dir(target)
            .map_err(|err| CliError::io(format!("cannot read '{}': {err}", target.display())))?
        {
            let entry =
                entry.map_err(|err| CliError::io(format!("cannot read dir entry: {err}")))?;
            let path = entry.path();
            if path.file_name().is_some_and(|name| name == "target") {
                continue;
            }
            if path.is_dir() {
                collect_dob_files(&path, files)?;
            } else if path.extension().is_some_and(|ext| ext == "nod") {
                files.push(path);
            }
        }
        return Ok(());
    }
    Err(CliError::io(format!(
        "target '{}' does not exist",
        target.display()
    )))
}

fn ensure_dob(path: &Path) -> Result<(), CliError> {
    if path.extension().is_some_and(|ext| ext == "nod") {
        Ok(())
    } else {
        Err(CliError::usage(format!(
            "invalid file extension for '{}'; expected .nod",
            path.display()
        )))
    }
}

fn parse_vars(items: &[String]) -> Result<BTreeMap<String, Value>, CliError> {
    let mut input = BTreeMap::new();
    if items.is_empty() {
        return Ok(input);
    }
    if items.len() == 1 && Path::new(&items[0]).is_file() {
        let content = fs::read_to_string(&items[0])
            .map_err(|err| CliError::io(format!("cannot read vars file: {err}")))?;
        if items[0].ends_with(".json") {
            parse_flat_json(&content, &mut input)?;
        } else if items[0].ends_with(".yaml") || items[0].ends_with(".yml") {
            parse_flat_yaml(&content, &mut input);
        } else {
            return Err(CliError::usage("vars file must be .json, .yaml or .yml"));
        }
        return Ok(input);
    }

    for item in items {
        let Some((key, value)) = item.split_once('=') else {
            return Err(CliError::usage(format!(
                "invalid var format: {item} (expected key=value)"
            )));
        };
        input.insert(key.to_string(), Value::String(value.to_string()));
    }
    Ok(input)
}

fn parse_flat_json(content: &str, input: &mut BTreeMap<String, Value>) -> Result<(), CliError> {
    let trimmed = content.trim();
    let body = trimmed
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| CliError::usage("vars JSON must be a flat object"))?;

    for pair in split_top_level(body) {
        if pair.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = pair.split_once(':') else {
            return Err(CliError::usage(format!("invalid JSON pair: {pair}")));
        };
        input.insert(unquote(key.trim()), json_value(value.trim()));
    }
    Ok(())
}

fn parse_flat_yaml(content: &str, input: &mut BTreeMap<String, Value>) {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            input.insert(key.trim().to_string(), Value::String(unquote(value.trim())));
        }
    }
}

fn split_top_level(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escape = false;
    for ch in text.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        if ch == '\\' {
            current.push(ch);
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            current.push(ch);
            continue;
        }
        if ch == ',' && !in_string {
            parts.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn json_value(value: &str) -> Value {
    match value {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "null" => Value::Null,
        _ if value.starts_with('"') => Value::String(unquote(value)),
        _ => value
            .parse::<i64>()
            .map(Value::Int)
            .or_else(|_| value.parse::<f64>().map(Value::Float))
            .unwrap_or_else(|_| Value::String(value.to_string())),
    }
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1]
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\\"", "\"")
    } else {
        value.to_string()
    }
}

fn tokens_json(tokens: &[nodia::Token]) -> String {
    let mut out = String::from("{\"ok\":true,\"tokens\":[");
    for (index, token) in tokens.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let literal = token
            .kind
            .literal()
            .map(|value| format!("\"{}\"", json_escape(&value)))
            .unwrap_or_else(|| "null".to_string());
        out.push_str(&format!(
            "{{\"kind\":\"{}\",\"literal\":{},\"line\":{},\"column\":{}}}",
            token.kind.name(),
            literal,
            token.line,
            token.column
        ));
    }
    out.push_str("]}");
    out
}

fn read_stdin() -> Result<String, CliError> {
    let mut source = String::new();
    io::stdin()
        .read_to_string(&mut source)
        .map_err(|err| CliError::io(format!("cannot read stdin: {err}")))?;
    Ok(source)
}

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

fn print_help() {
    println!(
        "Nodia {}\n\nUsage:\n  nodia run [file.nod] [--var key=value] [--vars key=value ...] [--out output.txt] [--allow-write] [--allow-env] [--allow-process] [-- script-args...]\n  nodia check [file.nod] [--json]\n  nodia fmt [file.nod|dir] [--check] [--stdout]\n  nodia eval 'emit \"hello\"' [-- script-args...]\n  nodia -e 'emit \"hello\"' [-- script-args...]\n  nodia tokens file.nod [--json]\n  nodia ast file.nod [--json]\n  nodia init [dir]\n  nodia version [--json]\n\nGlobal flags:\n  --json\n  --quiet\n  --verbose\n  --color auto|always|never\n  --allow-write\n  --allow-env\n  --allow-process\n  --help\n  --version",
        env!("CARGO_PKG_VERSION")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_script_args_keeps_source_and_trailing_args_separate() {
        let (source, script_args) = split_script_args(vec![
            "emit args".to_string(),
            "--".to_string(),
            "one".to_string(),
            "two".to_string(),
        ]);

        assert_eq!(source, vec!["emit args".to_string()]);
        assert_eq!(script_args, vec!["one".to_string(), "two".to_string()]);
    }

    #[test]
    fn exit_status_parser_reads_special_exit_message() {
        assert_eq!(exit_status("exit 7"), Some(7));
        assert_eq!(exit_status("error[E2000]: boom"), None);
    }

    #[test]
    fn eval_alias_returns_custom_exit_code() {
        let code = run(vec![
            "nodia".to_string(),
            "-e".to_string(),
            "emit \"before\"\nexit(7)".to_string(),
        ]);

        assert_eq!(code, 7);
    }
}
