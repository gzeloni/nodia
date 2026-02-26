use orich::{run_source, Value};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let code = match run_cli() {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("{message}");
            1
        }
    };
    std::process::exit(code);
}

fn run_cli() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() == 1 {
        print_help();
        return Ok(());
    }
    if args[1] == "--version" {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args[1] != "run" {
        print_help();
        return Err(format!("unknown command '{}'", args[1]));
    }
    run_command(&args[2..])
}

fn run_command(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut output = false;
    let mut vars = Vec::new();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "-o" | "--output" => {
                output = true;
                index += 1;
            }
            "--vars" => {
                vars.extend(args[index + 1..].iter().cloned());
                break;
            }
            value if path.is_none() => {
                path = Some(value.to_string());
                index += 1;
            }
            value => return Err(format!("unexpected argument '{value}'")),
        }
    }

    let path = path.ok_or_else(|| "missing .orich path".to_string())?;
    if !path.ends_with(".orich") {
        return Err("invalid file extension; expected .orich".to_string());
    }
    let source = fs::read_to_string(&path).map_err(|err| format!("cannot read '{path}': {err}"))?;
    let input = parse_vars(&vars)?;
    let result = run_source(&source, input).map_err(|err| format!("Error: {err}"))?;

    if output {
        let out_path = format!("{path}.out");
        fs::write(&out_path, result).map_err(|err| format!("cannot write '{out_path}': {err}"))?;
    } else {
        println!("{result}");
    }
    Ok(())
}

fn parse_vars(items: &[String]) -> Result<BTreeMap<String, Value>, String> {
    let mut input = BTreeMap::new();
    if items.is_empty() {
        return Ok(input);
    }
    if Path::new(&items[0]).is_file() {
        let content =
            fs::read_to_string(&items[0]).map_err(|err| format!("cannot read vars file: {err}"))?;
        if items[0].ends_with(".json") {
            parse_flat_json(&content, &mut input)?;
        } else if items[0].ends_with(".yaml") || items[0].ends_with(".yml") {
            parse_flat_yaml(&content, &mut input);
        } else {
            return Err("vars file must be .json, .yaml or .yml".to_string());
        }
        return Ok(input);
    }

    for item in items {
        let Some((key, value)) = item.split_once('=') else {
            return Err(format!("invalid var format: {item} (expected key=value)"));
        };
        input.insert(key.to_string(), Value::String(value.to_string()));
    }
    Ok(input)
}

fn parse_flat_json(content: &str, input: &mut BTreeMap<String, Value>) -> Result<(), String> {
    let trimmed = content.trim();
    let body = trimmed
        .strip_prefix('{')
        .and_then(|value| value.strip_suffix('}'))
        .ok_or_else(|| "vars JSON must be a flat object".to_string())?;

    for pair in split_top_level(body) {
        if pair.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = pair.split_once(':') else {
            return Err(format!("invalid JSON pair: {pair}"));
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

fn print_help() {
    println!("usage: orich run <file.orich> [-o|--output] [--vars key=value ...]");
}
