// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Canonical source formatter for the Nodia AST.

use crate::ast::{AssignTarget, BinaryOp, Expr, ForBinding, Program, Stmt, UnaryOp, UseTarget};
use crate::regex::{
    RegexCharSet, RegexCharSetItem, RegexFlag, RegexGroupKind, RegexNode, RegexPattern,
    RegexQuantifierMode, RegexReference,
};
use crate::value::Value;

const INDENT: &str = "  ";
const LINE_WIDTH: usize = 60;

/// Formats a parsed program using the single canonical style supported by Nodia.
pub fn format_program(program: &Program) -> String {
    let mut formatter = Formatter::default();
    formatter.write_statements(&program.statements);
    if !formatter.out.ends_with('\n') {
        formatter.out.push('\n');
    }
    formatter.out
}

#[derive(Default)]
struct Formatter {
    out: String,
    indent: usize,
}

impl Formatter {
    fn write_statements(&mut self, statements: &[Stmt]) {
        for (index, stmt) in statements.iter().enumerate() {
            if index > 0 && needs_blank_line(&statements[index - 1], stmt) {
                self.out.push('\n');
            }
            self.write_stmt(stmt);
            self.out.push('\n');
        }
    }

    fn write_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Comment(text) => {
                self.write_indent();
                self.out.push('#');
                if !text.is_empty() {
                    self.out.push(' ');
                    self.out.push_str(text.trim());
                }
            }
            Stmt::Use {
                target,
                alias,
                pick,
                hide,
            } => {
                self.write_indent();
                self.out.push_str("use ");
                match target {
                    UseTarget::Path(path) => self.out.push_str(&quote_string(path)),
                    UseTarget::Stdlib(name) => self.out.push_str(name),
                }
                if let Some(alias) = alias {
                    self.out.push_str(" as ");
                    self.out.push_str(alias);
                }
                if !pick.is_empty() {
                    self.out.push_str(" pick ");
                    self.out.push_str(&pick.join(", "));
                }
                if !hide.is_empty() {
                    self.out.push_str(" hide ");
                    self.out.push_str(&hide.join(", "));
                }
            }
            Stmt::Bind {
                name,
                value,
                mutable,
            } => {
                self.write_indent();
                let prefix = format!("{}{} = ", if *mutable { "var " } else { "val " }, name);
                self.out.push_str(&prefix);
                self.out
                    .push_str(&format_expr_for_line(value, self.indent, prefix.len()));
            }
            Stmt::Assign { target, value } => {
                self.write_indent();
                let prefix = format!("{} = ", format_assign_target(target, self.indent));
                self.out.push_str(&prefix);
                self.out
                    .push_str(&format_expr_for_line(value, self.indent, prefix.len()));
            }
            Stmt::Func { name, params, body } => {
                self.write_function(name, params, body);
            }
            Stmt::Return(Some(expr)) => {
                self.write_indent();
                let prefix = "return ";
                self.out.push_str(prefix);
                self.out
                    .push_str(&format_expr_for_line(expr, self.indent, prefix.len()));
            }
            Stmt::Return(None) => {
                self.write_indent();
                self.out.push_str("return");
            }
            Stmt::Emit(expr) => {
                self.write_indent();
                let prefix = "emit ";
                self.out.push_str(prefix);
                self.out
                    .push_str(&format_expr_for_line(expr, self.indent, prefix.len()));
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.write_if(condition, then_branch, else_branch, true);
            }
            Stmt::For {
                binding,
                iterable,
                body,
            } => {
                self.write_indent();
                self.out.push_str("for ");
                self.out.push_str(&format_for_binding(binding));
                self.out.push_str(" in ");
                self.out.push_str(&format_expr(iterable, self.indent));
                self.out.push(' ');
                self.write_block(body);
            }
            Stmt::While { condition, body } => {
                self.write_indent();
                self.out.push_str("while ");
                self.out.push_str(&format_expr(condition, self.indent));
                self.out.push(' ');
                self.write_block(body);
            }
            Stmt::Break => {
                self.write_indent();
                self.out.push_str("break");
            }
            Stmt::Continue => {
                self.write_indent();
                self.out.push_str("continue");
            }
            Stmt::Expr(expr) => {
                self.write_indent();
                self.out.push_str(&format_expr(expr, self.indent));
            }
        }
    }

    fn write_function(&mut self, name: &str, params: &[String], body: &[Stmt]) {
        self.write_indent();
        let inline = format!("func {name}({}) ", params.join(", "));
        if current_line_width(self.indent) + inline.len() <= LINE_WIDTH {
            self.out.push_str(&inline);
            self.write_block(body);
            return;
        }

        self.out.push_str("func ");
        self.out.push_str(name);
        self.out.push_str("(\n");
        self.indent += 1;
        for param in params {
            self.write_indent();
            self.out.push_str(param);
            self.out.push_str(",\n");
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push_str(") ");
        self.write_block(body);
    }

    fn write_if(
        &mut self,
        condition: &Expr,
        then_branch: &[Stmt],
        else_branch: &[Stmt],
        with_indent: bool,
    ) {
        if with_indent {
            self.write_indent();
        }
        self.out.push_str("if ");
        self.out.push_str(&format_expr(condition, self.indent));
        self.out.push(' ');
        self.write_block(then_branch);
        if else_branch.is_empty() {
            return;
        }
        self.out.push_str(" else ");
        if let [Stmt::If {
            condition,
            then_branch,
            else_branch,
        }] = else_branch
        {
            self.write_if(condition, then_branch, else_branch, false);
        } else {
            self.write_block(else_branch);
        }
    }

    fn write_block(&mut self, statements: &[Stmt]) {
        self.out.push('{');
        if statements.is_empty() {
            self.out.push('}');
            return;
        }
        self.out.push('\n');
        self.indent += 1;
        self.write_statements(statements);
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str(INDENT);
        }
    }
}

fn needs_blank_line(prev: &Stmt, next: &Stmt) -> bool {
    if matches!(prev, Stmt::Comment(_)) || matches!(next, Stmt::Comment(_)) {
        return false;
    }
    matches!(
        prev,
        Stmt::Func { .. } | Stmt::If { .. } | Stmt::For { .. } | Stmt::While { .. }
    ) || matches!(
        next,
        Stmt::Func { .. } | Stmt::If { .. } | Stmt::For { .. } | Stmt::While { .. }
    )
}

fn format_expr(expr: &Expr, indent: usize) -> String {
    format_expr_for_line(expr, indent, 0)
}

fn format_assign_target(target: &AssignTarget, indent: usize) -> String {
    match target {
        AssignTarget::Identifier(name) => name.clone(),
        AssignTarget::Get { object, field } => {
            format!("{}.{}", format_assign_target(object, indent), field)
        }
        AssignTarget::Index { object, index } => {
            format!(
                "{}[{}]",
                format_assign_target(object, indent),
                format_expr(index, indent)
            )
        }
    }
}

fn format_for_binding(binding: &ForBinding) -> String {
    match binding {
        ForBinding::Single(name) => name.clone(),
        ForBinding::Pair { key, value } => format!("({key}, {value})"),
    }
}

fn format_expr_for_line(expr: &Expr, indent: usize, prefix_len: usize) -> String {
    format_expr_prec(expr, 0, indent, available_width(indent, prefix_len))
}

fn format_expr_prec(expr: &Expr, parent_prec: u8, indent: usize, width: usize) -> String {
    let prec = precedence(expr);
    let rendered = match expr {
        Expr::Literal(value) => format_literal(value, indent, width),
        Expr::String { value, interpolate } => {
            format_source_string_literal(value, *interpolate, indent, width)
        }
        Expr::Lambda { params, body } => format_lambda(params, body, indent),
        Expr::Regex(pattern) => format_regex(pattern, indent),
        Expr::Identifier(name) => name.clone(),
        Expr::Unary { op, expr } => {
            let op = match op {
                UnaryOp::Negate => "-",
                UnaryOp::Not => "not ",
            };
            format!("{}{}", op, format_expr_prec(expr, prec, indent, width))
        }
        Expr::Binary { left, op, right } => {
            let op = match op {
                BinaryOp::Add => "+",
                BinaryOp::Subtract => "-",
                BinaryOp::Multiply => "*",
                BinaryOp::Divide => "/",
                BinaryOp::Modulo => "%",
                BinaryOp::Equal => "==",
                BinaryOp::NotEqual => "!=",
                BinaryOp::Less => "<",
                BinaryOp::LessEqual => "<=",
                BinaryOp::Greater => ">",
                BinaryOp::GreaterEqual => ">=",
                BinaryOp::And => "and",
                BinaryOp::Or => "or",
            };
            format!(
                "{} {op} {}",
                format_expr_prec(left, prec, indent, width),
                format_expr_prec(right, prec + 1, indent, width)
            )
        }
        Expr::Call { callee, args } => format_call(callee, args, indent, width),
        Expr::Get { object, field } => {
            format!(
                "{}.{}",
                format_expr_prec(object, prec, indent, width),
                field
            )
        }
        Expr::Index { object, index } => {
            format!(
                "{}[{}]",
                format_expr_prec(object, prec, indent, width),
                format_expr(index, indent)
            )
        }
        Expr::List(values) => format_list(values, indent, width),
        Expr::Map(pairs) => format_map(pairs, indent),
    };
    if prec < parent_prec {
        format!("({rendered})")
    } else {
        rendered
    }
}

fn precedence(expr: &Expr) -> u8 {
    match expr {
        Expr::Binary { op, .. } => match op {
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::Equal | BinaryOp::NotEqual => 3,
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => 4,
            BinaryOp::Add | BinaryOp::Subtract => 5,
            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => 6,
        },
        Expr::Unary { .. } => 7,
        Expr::Call { .. } | Expr::Get { .. } | Expr::Index { .. } => 8,
        _ => 9,
    }
}

fn format_literal(value: &Value, indent: usize, width: usize) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Int(value) => value.to_string(),
        Value::Float(value) => {
            if value.fract() == 0.0 {
                format!("{value:.1}")
            } else {
                value.to_string()
            }
        }
        Value::String(value) => format_source_string_literal(value, false, indent, width),
        Value::List(values) => {
            let exprs = values
                .iter()
                .map(|value| Expr::Literal(value.clone()))
                .collect::<Vec<_>>();
            format_list(&exprs, indent, available_width(indent, 0))
        }
        Value::Map(values) => {
            let pairs = values
                .iter()
                .map(|(key, value)| (key.clone(), Expr::Literal(value.clone())))
                .collect::<Vec<_>>();
            format_map(&pairs, indent)
        }
        Value::Date(value) => format!("parse_date({})", quote_string(&value.isoformat())),
        Value::DateTime(value) => format!("parse_datetime({})", quote_string(&value.isoformat())),
        Value::Duration(value) => format!("parse_duration({})", quote_string(&value.isoformat())),
        Value::Regex(regex) => quote_string(regex.rendered()),
        Value::Stream(stream) => stream.to_string(),
        Value::UseBinding(_, name) => format!("<use {name}>"),
        Value::BuiltinFunction(name) => format!("<builtin {name}>"),
        Value::Function(_) => "<func>".to_string(),
    }
}

fn format_call(callee: &Expr, args: &[Expr], indent: usize, width: usize) -> String {
    let callee = format_expr_prec(callee, 8, indent, width);
    let inline_args = args
        .iter()
        .map(|arg| format_expr_for_line(arg, indent, 0))
        .collect::<Vec<_>>();
    let inline = format!("{}({})", callee, inline_args.join(", "));
    if fits_inline(&inline, width) {
        return inline;
    }
    let mut out = String::new();
    out.push_str(&callee);
    out.push_str("(\n");
    for arg in args {
        out.push_str(&indent_string(indent + 1));
        out.push_str(&format_expr_for_line(arg, indent + 1, 1));
        out.push_str(",\n");
    }
    out.push_str(&indent_string(indent));
    out.push(')');
    out
}

fn format_lambda(params: &[String], body: &[Stmt], indent: usize) -> String {
    let mut formatter = Formatter {
        out: format!("lambda({}) ", params.join(", ")),
        indent,
    };
    formatter.write_block(body);
    formatter.out
}

fn format_list(values: &[Expr], indent: usize, width: usize) -> String {
    if values.is_empty() {
        return "[]".to_string();
    }
    let inline_values = values
        .iter()
        .map(|value| format_expr_for_line(value, indent, 0))
        .collect::<Vec<_>>();
    let inline = format!("[{}]", inline_values.join(", "));
    if fits_inline(&inline, width) {
        return inline;
    }
    let mut out = String::new();
    out.push_str("[\n");
    for value in values {
        out.push_str(&indent_string(indent + 1));
        out.push_str(&format_expr_for_line(value, indent + 1, 1));
        out.push_str(",\n");
    }
    out.push_str(&indent_string(indent));
    out.push(']');
    out
}

fn format_map(pairs: &[(String, Expr)], indent: usize) -> String {
    if pairs.is_empty() {
        return "{}".to_string();
    }
    let mut out = String::new();
    out.push_str("{\n");
    for (key, value) in pairs {
        let key = format_map_key(key);
        out.push_str(&indent_string(indent + 1));
        out.push_str(&key);
        out.push_str(": ");
        out.push_str(&format_expr_for_line(value, indent + 1, key.len() + 3));
        out.push_str(",\n");
    }
    out.push_str(&indent_string(indent));
    out.push('}');
    out
}

fn format_regex(pattern: &RegexPattern, indent: usize) -> String {
    let mut out = String::from("regex");
    if !pattern.flags.is_empty() {
        out.push('(');
        for (index, flag) in pattern.flags.iter().enumerate() {
            if index > 0 {
                out.push_str(", ");
            }
            out.push_str(flag.name());
        }
        out.push(')');
    }
    if pattern.body.is_empty() {
        out.push_str(" {}");
        return out;
    }
    out.push_str(" {\n");
    out.push_str(&format_regex_items(&pattern.body, indent + 1));
    out.push('\n');
    out.push_str(&indent_string(indent));
    out.push('}');
    out
}

fn format_regex_items(items: &[RegexNode], indent: usize) -> String {
    let mut out = String::new();
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&indent_string(indent));
        out.push_str(&format_regex_node(item, indent));
    }
    out
}

fn format_regex_node(node: &RegexNode, indent: usize) -> String {
    match node {
        RegexNode::Sequence(items) => format_regex_sequence_block(items, indent),
        RegexNode::Literal(value) => quote_string(value),
        RegexNode::Raw(value) => format!("raw_regex {}", quote_string(value)),
        RegexNode::Anchor(anchor) => anchor.name().to_string(),
        RegexNode::Class(class) => class.name().to_string(),
        RegexNode::AnyChar => "any_char".to_string(),
        RegexNode::AnyCodepoint => "any_codepoint".to_string(),
        RegexNode::Quantifier { target, kind, mode } => {
            let mut prefix = kind.format_keyword();
            if *mode != RegexQuantifierMode::Greedy {
                prefix.push(' ');
                prefix.push_str(mode.name());
            }
            match &**target {
                RegexNode::Sequence(items) => {
                    let mut out = prefix;
                    if items.is_empty() {
                        out.push_str(" {}");
                        return out;
                    }
                    out.push_str(" {\n");
                    out.push_str(&format_regex_items(items, indent + 1));
                    out.push('\n');
                    out.push_str(&indent_string(indent));
                    out.push('}');
                    out
                }
                _ => format!("{prefix} {}", format_regex_node(target, indent)),
            }
        }
        RegexNode::Group { kind, body } => match kind {
            RegexGroupKind::Capture => format_regex_named_block("group", body, indent),
            RegexGroupKind::NonCapture => format_regex_named_block("non_capture", body, indent),
            RegexGroupKind::Named(name) => {
                format_regex_named_block(&format!("named {name}"), body, indent)
            }
            RegexGroupKind::Atomic => format_regex_named_block("atomic", body, indent),
        },
        RegexNode::Alternation(branches) => {
            let mut out = String::from("either");
            if branches.is_empty() {
                out.push_str(" {}");
                return out;
            }
            out.push_str(" {\n");
            for (index, branch) in branches.iter().enumerate() {
                if index > 0 {
                    out.push('\n');
                }
                out.push_str(&indent_string(indent + 1));
                out.push_str(&format_regex_named_block("branch", branch, indent + 1));
            }
            out.push('\n');
            out.push_str(&indent_string(indent));
            out.push('}');
            out
        }
        RegexNode::CharSet(set) => format_regex_char_set(set, indent),
        RegexNode::Lookaround { kind, body } => format_regex_named_block(kind.name(), body, indent),
        RegexNode::Reference(RegexReference::Named(name)) => format!("same_as {name}"),
        RegexNode::Reference(RegexReference::Group(index)) => format!("same_as_group {index}"),
        RegexNode::ScopedFlags {
            enable,
            disable,
            body,
        } => format_regex_scoped_flags(enable, disable, body, indent),
    }
}

fn format_regex_char_set(set: &RegexCharSet, indent: usize) -> String {
    let header = if set.negated {
        "not_char_set"
    } else {
        "char_set"
    };
    let mut out = String::from(header);
    if set.items.is_empty() {
        out.push_str(" {}");
        return out;
    }
    out.push_str(" {\n");
    for (index, item) in set.items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&indent_string(indent + 1));
        out.push_str(&format_regex_char_set_item(item));
    }
    out.push('\n');
    out.push_str(&indent_string(indent));
    out.push('}');
    out
}

fn format_regex_scoped_flags(
    enable: &[RegexFlag],
    disable: &[RegexFlag],
    body: &[RegexNode],
    indent: usize,
) -> String {
    if !enable.is_empty() && disable.is_empty() {
        return format_regex_named_block(
            &format!("with_flags({})", format_regex_flag_list(enable)),
            body,
            indent,
        );
    }
    if enable.is_empty() && !disable.is_empty() {
        return format_regex_named_block(
            &format!("without_flags({})", format_regex_flag_list(disable)),
            body,
            indent,
        );
    }

    let nested = RegexNode::ScopedFlags {
        enable: Vec::new(),
        disable: disable.to_vec(),
        body: body.to_vec(),
    };
    let mut out = format!("with_flags({}) {{\n", format_regex_flag_list(enable));
    out.push_str(&indent_string(indent + 1));
    out.push_str(&format_regex_node(&nested, indent + 1));
    out.push('\n');
    out.push_str(&indent_string(indent));
    out.push('}');
    out
}

fn format_regex_named_block(header: &str, items: &[RegexNode], indent: usize) -> String {
    let mut out = String::from(header);
    if items.is_empty() {
        out.push_str(" {}");
        return out;
    }
    out.push_str(" {\n");
    out.push_str(&format_regex_items(items, indent + 1));
    out.push('\n');
    out.push_str(&indent_string(indent));
    out.push('}');
    out
}

fn format_regex_sequence_block(items: &[RegexNode], indent: usize) -> String {
    let mut out = String::new();
    if items.is_empty() {
        out.push_str("{}");
        return out;
    }
    out.push_str("{\n");
    out.push_str(&format_regex_items(items, indent + 1));
    out.push('\n');
    out.push_str(&indent_string(indent));
    out.push('}');
    out
}

fn format_regex_char_set_item(item: &RegexCharSetItem) -> String {
    match item {
        RegexCharSetItem::Char(ch) => quote_string(&ch.to_string()),
        RegexCharSetItem::Class(class) => class.name().to_string(),
        RegexCharSetItem::Raw(value) => format!("raw_regex {}", quote_string(value)),
        RegexCharSetItem::Range(start, end) => {
            format!(
                "range {} to {}",
                quote_string(&start.to_string()),
                quote_string(&end.to_string())
            )
        }
    }
}

fn format_regex_flag_list(flags: &[RegexFlag]) -> String {
    flags
        .iter()
        .map(|flag| flag.name())
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_source_string_literal(
    value: &str,
    interpolate: bool,
    indent: usize,
    width: usize,
) -> String {
    if !interpolate && value.contains('\n') && !value.contains("\"\"\"") {
        return format!("\"\"\"{}\"\"\"", value);
    }

    let quoted = quote_source_string(value, interpolate);
    if fits_inline(&quoted, width) {
        return quoted;
    }

    let chunks = split_string_literal(value, width.saturating_sub(4).max(1));
    let mut out = String::new();
    for (index, chunk) in chunks.iter().enumerate() {
        if index > 0 {
            out.push('\n');
            out.push_str(&indent_string(indent + 1));
        }
        out.push_str(&quote_source_string(chunk, interpolate));
        if index + 1 < chunks.len() {
            out.push_str(" +");
        }
    }
    out
}

fn split_string_literal(value: &str, max_chars: usize) -> Vec<String> {
    if value.is_empty() {
        return vec![String::new()];
    }

    let chars = value.chars().collect::<Vec<_>>();
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let hard_end = (start + max_chars).min(chars.len());
        let mut end = hard_end;
        if hard_end < chars.len() {
            if let Some(space) = chars[start..hard_end]
                .iter()
                .rposition(|ch| ch.is_whitespace())
            {
                if space > 0 {
                    end = start + space + 1;
                }
            }
        }
        chunks.push(chars[start..end].iter().collect());
        start = end;
    }
    chunks
}

fn format_map_key(key: &str) -> String {
    if key
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_')
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        key.to_string()
    } else {
        quote_string(key)
    }
}

fn quote_string(value: &str) -> String {
    let mut out = String::new();
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn quote_source_string(value: &str, interpolate: bool) -> String {
    if interpolate {
        return quote_string(value);
    }

    let mut out = String::new();
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '{' => out.push_str("{{"),
            '}' => out.push_str("}}"),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn indent_string(level: usize) -> String {
    INDENT.repeat(level)
}

fn fits_inline(value: &str, width: usize) -> bool {
    !value.contains('\n') && value.len() <= width
}

fn available_width(indent: usize, prefix_len: usize) -> usize {
    LINE_WIDTH
        .saturating_sub(current_line_width(indent) + prefix_len)
        .max(INDENT.len() * 4)
}

fn current_line_width(indent: usize) -> usize {
    indent * INDENT.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};

    #[test]
    fn formats_basic_program() {
        let source =
            "val user={name:\"Ana\",role:\"dev\"}\nif user.name!=\"\"{emit \"hello {user.name}\"}";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);
        assert!(formatted.contains("val user = {"));
        assert!(formatted.contains("if user.name != \"\" {"));
    }

    #[test]
    fn keeps_nested_calls_inline_when_they_fit_line_width() {
        let source = "emit bullet(\"replace\", replace(\"text/math/files\", \"/\", \" -> \"))";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("replace(\"text/math/files\", \"/\", \" -> \"),"));
        assert!(formatted
            .lines()
            .filter(|line| !line.starts_with('#'))
            .all(|line| line.len() <= LINE_WIDTH));
    }

    #[test]
    fn formats_regex_blocks_canonically() {
        let source = r#"val date=regex(case_insensitive){start named year{exactly 4 digit}"-"exactly 2 digit end}"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("val date = regex(case_insensitive) {"));
        assert!(formatted.contains("  named year {"));
        assert!(formatted.contains("    exactly 4 digit"));
        assert!(formatted.contains("  exactly 2 digit"));
    }

    #[test]
    fn formats_scoped_flags_and_any_codepoint() {
        let source = r#"emit regex{with_flags(case_insensitive){literal("abc")}one_or_more any_codepoint char_set{char(".")digit}}"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("with_flags(case_insensitive) {"));
        assert!(formatted.contains("\"abc\""));
        assert!(formatted.contains("one_or_more any_codepoint"));
        assert!(formatted.contains("char_set {"));
    }

    #[test]
    fn preserves_keyword_field_access() {
        let source = r#"val m={from:"x",val:"y"}
val hit=find("42",regex{named val{one_or_more digit}})
emit m.from
emit hit.named.val"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("emit m.from"));
        assert!(formatted.contains("emit hit.named.val"));
    }
}
