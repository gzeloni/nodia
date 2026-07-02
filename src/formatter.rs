// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Canonical source formatter for the Nodia AST.

use crate::ast::{
    AssignTarget, BinaryOp, Expr, ForBinding, FuncParam, MatchArm, MatchPattern, Program, Stmt,
    UnaryOp, UseTarget,
};
use crate::interpolation::{self, Chunk as InterpolationChunk};
use crate::regex::{
    RegexCharSet, RegexCharSetItem, RegexCondition, RegexFlag, RegexGroupKind, RegexNode,
    RegexPattern, RegexQuantifierMode, RegexReference,
};
use crate::textcodec;
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
            Stmt::Throw(expr) => {
                self.write_indent();
                let prefix = "throw ";
                self.out.push_str(prefix);
                self.out
                    .push_str(&format_expr_for_line(expr, self.indent, prefix.len()));
            }
            Stmt::Emit(expr) => {
                self.write_indent();
                let prefix = "emit ";
                self.out.push_str(prefix);
                self.out
                    .push_str(&format_expr_for_line(expr, self.indent, prefix.len()));
            }
            Stmt::Try {
                try_branch,
                catch_name,
                catch_branch,
            } => self.write_try(try_branch, catch_name, catch_branch),
            Stmt::Match {
                value,
                arms,
                default,
            } => self.write_match(value, arms, default.as_deref()),
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
            Stmt::Namespace { name, body } => {
                self.write_indent();
                self.out.push_str(&format!("namespace {name} "));
                self.write_block(body);
            }
            Stmt::Struct { name, fields } => {
                self.write_indent();
                self.out.push_str(&format!("struct {name} "));
                self.out.push_str("{\n");
                self.indent += 1;
                for field in fields {
                    self.write_indent();
                    self.out.push_str(&field.name);
                    if let Some(default) = &field.default {
                        self.out.push_str(": ");
                        self.out.push_str(&format_expr(default, self.indent));
                    }
                    self.out.push('\n');
                }
                self.indent -= 1;
                self.write_indent();
                self.out.push('}');
            }
            Stmt::Enum { name, variants } => {
                self.write_indent();
                self.out.push_str(&format!("enum {name} {{"));
                if variants.iter().all(|v| {
                    current_line_width(self.indent) + name.len() + 7 + v.len() + 2 <= LINE_WIDTH
                }) && variants.len() <= 3
                {
                    self.out.push(' ');
                    self.out.push_str(&variants.join(", "));
                    self.out.push_str(" }");
                } else {
                    self.out.push('\n');
                    self.indent += 1;
                    for variant in variants {
                        self.write_indent();
                        self.out.push_str(variant);
                        self.out.push_str(",\n");
                    }
                    self.indent -= 1;
                    self.write_indent();
                    self.out.push('}');
                }
            }
            Stmt::TypeAlias { name, target } => {
                self.write_indent();
                let prefix = format!("type {name} = ");
                self.out.push_str(&prefix);
                self.out
                    .push_str(&format_expr_for_line(target, self.indent, prefix.len()));
            }
        }
    }

    fn write_function(&mut self, name: &str, params: &[FuncParam], body: &[Stmt]) {
        self.write_indent();
        let params_str = params
            .iter()
            .map(|p| {
                if let Some(default) = &p.default {
                    format!("{} = {}", p.name, format_expr(default, self.indent))
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let inline = format!("func {name}({params_str}) ");
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
            self.out.push_str(&param.name);
            if let Some(default) = &param.default {
                self.out.push_str(" = ");
                self.out.push_str(&format_expr(default, self.indent));
            }
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

    fn write_try(&mut self, try_branch: &[Stmt], catch_name: &str, catch_branch: &[Stmt]) {
        self.write_indent();
        self.out.push_str("try ");
        self.write_block(try_branch);
        self.out.push_str(" catch ");
        self.out.push_str(catch_name);
        self.out.push(' ');
        self.write_block(catch_branch);
    }

    fn write_match(&mut self, value: &Expr, arms: &[MatchArm], default: Option<&[Stmt]>) {
        self.write_indent();
        self.out.push_str("match ");
        self.out.push_str(&format_expr(value, self.indent));
        self.out.push_str(" {");

        if arms.is_empty() && default.is_none() {
            self.out.push('}');
            return;
        }

        self.out.push('\n');
        self.indent += 1;
        for arm in arms {
            self.write_indent();
            self.out.push_str("case ");
            self.out
                .push_str(&format_match_pattern(&arm.pattern, self.indent));
            self.out.push(' ');
            self.write_block(&arm.body);
            self.out.push('\n');
        }
        if let Some(default_body) = default {
            self.write_indent();
            self.out.push_str("default ");
            self.write_block(default_body);
            self.out.push('\n');
        }
        self.indent -= 1;
        self.write_indent();
        self.out.push('}');
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
        Stmt::Func { .. }
            | Stmt::Try { .. }
            | Stmt::Match { .. }
            | Stmt::If { .. }
            | Stmt::For { .. }
            | Stmt::While { .. }
    ) || matches!(
        next,
        Stmt::Func { .. }
            | Stmt::Try { .. }
            | Stmt::Match { .. }
            | Stmt::If { .. }
            | Stmt::For { .. }
            | Stmt::While { .. }
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
            format_string_literal(value, *interpolate, indent, width)
        }
        Expr::Lambda { params, body } => format_lambda(params, body, indent),
        Expr::Regex(pattern) => format_regex(pattern, indent),
        Expr::Identifier(name) => name.clone(),
        Expr::Unary { op, expr } => {
            let op = match op {
                UnaryOp::Negate => "-",
                UnaryOp::Not => "not ",
                UnaryOp::BitNot => "~",
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
                BinaryOp::BitOr => "|",
                BinaryOp::BitXor => "^",
                BinaryOp::BitAnd => "&",
                BinaryOp::ShiftLeft => "<<",
                BinaryOp::ShiftRight => ">>",
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
            BinaryOp::BitOr => 5,
            BinaryOp::BitXor => 6,
            BinaryOp::BitAnd => 7,
            BinaryOp::ShiftLeft | BinaryOp::ShiftRight => 8,
            BinaryOp::Add | BinaryOp::Subtract => 9,
            BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => 10,
        },
        Expr::Unary { .. } => 11,
        Expr::Call { .. } | Expr::Get { .. } | Expr::Index { .. } => 12,
        _ => 13,
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
        Value::String(value) => format_string_literal(value, false, indent, width),
        Value::Bytes(value) => textcodec::quote_bytes_literal(value),
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
        Value::Date(value) => format!(
            "datetime.parse({}, datetime.as_date)",
            quote_string(&value.isoformat())
        ),
        Value::DateTime(value) => {
            format!(
                "datetime.parse({}, datetime.as_datetime)",
                quote_string(&value.isoformat())
            )
        }
        Value::Duration(value) => {
            format!(
                "datetime.parse({}, datetime.as_duration)",
                quote_string(&value.isoformat())
            )
        }
        Value::Regex(regex) => quote_string(regex.rendered()),
        Value::Stream(stream) => stream.to_string(),
        Value::Scanner(_) => "<scanner>".to_string(),
        Value::Lazy(lazy) => format!("<lazy {}>", lazy),
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

fn format_lambda(params: &[FuncParam], body: &[Stmt], indent: usize) -> String {
    let params_str = params
        .iter()
        .map(|p| {
            if let Some(default) = &p.default {
                format!("{} = {}", p.name, format_expr(default, indent))
            } else {
                p.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let mut formatter = Formatter {
        out: format!("lambda({params_str}) "),
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

fn format_match_pattern(pattern: &MatchPattern, indent: usize) -> String {
    match pattern {
        MatchPattern::Wildcard => "_".to_string(),
        MatchPattern::Capture(name) => name.clone(),
        MatchPattern::Literal(value) => format_literal(value, indent, available_width(indent, 0)),
        MatchPattern::List(items) => {
            if items.is_empty() {
                return "[]".to_string();
            }
            let inline_items = items
                .iter()
                .map(|item| format_match_pattern(item, indent))
                .collect::<Vec<_>>();
            let inline = format!("[{}]", inline_items.join(", "));
            if fits_inline(&inline, available_width(indent, 0)) {
                return inline;
            }

            let mut out = String::new();
            out.push_str("[\n");
            for item in items {
                out.push_str(&indent_string(indent + 1));
                out.push_str(&format_match_pattern(item, indent + 1));
                out.push_str(",\n");
            }
            out.push_str(&indent_string(indent));
            out.push(']');
            out
        }
        MatchPattern::Map(entries) => {
            if entries.is_empty() {
                return "{}".to_string();
            }
            let mut out = String::new();
            out.push_str("{\n");
            for (key, pattern) in entries {
                out.push_str(&indent_string(indent + 1));
                if matches!(pattern, MatchPattern::Capture(name) if name == key)
                    && is_identifier_key(key)
                {
                    out.push_str(key);
                } else {
                    let key = format_map_key(key);
                    out.push_str(&key);
                    out.push_str(": ");
                    out.push_str(&format_match_pattern(pattern, indent + 1));
                }
                out.push_str(",\n");
            }
            out.push_str(&indent_string(indent));
            out.push('}');
            out
        }
    }
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
        RegexNode::Property { name, negated } => {
            if *negated {
                format!("not_property {}", quote_string(name))
            } else {
                format!("property {}", quote_string(name))
            }
        }
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
        RegexNode::Condition(condition) => format_regex_condition_header(condition, indent),
        RegexNode::Conditional {
            condition,
            then_branch,
            else_branch,
        } => format_regex_conditional(condition, then_branch, else_branch, indent),
        RegexNode::SubroutineCall(RegexReference::Named(name)) => format!("call {name}"),
        RegexNode::SubroutineCall(RegexReference::Group(index)) => format!("call_group {index}"),
        RegexNode::BacktrackingVerb(verb) => verb.name().to_string(),
        RegexNode::Until { limit, body } => {
            let mut out = format_regex_named_block("until", limit, indent);
            if let Some(body) = body {
                out.push_str(" then ");
                out.push_str(&format_regex_sequence_block(body, indent));
            }
            out
        }
        RegexNode::UntilStop(limit) => format_regex_named_block("until_stop", limit, indent),
        RegexNode::UntilClear => "until_clear".to_string(),
        RegexNode::DefineGroup { body } => format_regex_named_block("define", body, indent),
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

fn format_regex_conditional(
    condition: &RegexCondition,
    then_branch: &[RegexNode],
    else_branch: &[RegexNode],
    indent: usize,
) -> String {
    let mut out = format_regex_condition_header(condition, indent);
    if !then_branch.is_empty() || !else_branch.is_empty() {
        out.push_str(" then ");
        out.push_str(&format_regex_sequence_block(then_branch, indent));
        if !else_branch.is_empty() {
            out.push_str(" else ");
            out.push_str(&format_regex_sequence_block(else_branch, indent));
        }
    }
    out
}

fn format_regex_condition_header(condition: &RegexCondition, indent: usize) -> String {
    match condition {
        RegexCondition::Capture(RegexReference::Named(name)) => format!("if_capture {name}"),
        RegexCondition::Capture(RegexReference::Group(index)) => format!("if_capture {index}"),
        RegexCondition::Lookaround { kind, body } => {
            format_regex_named_block(&format!("if_{}", kind.name()), body, indent)
        }
        RegexCondition::Expression(body) => format_regex_named_block("if_matches", body, indent),
    }
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
        RegexCharSetItem::Property { name, negated } => {
            if *negated {
                format!("not_property {}", quote_string(name))
            } else {
                format!("property {}", quote_string(name))
            }
        }
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

fn format_string_literal(value: &str, interpolate: bool, indent: usize, width: usize) -> String {
    if !interpolate && value.contains('\n') && !value.contains("\"\"\"") {
        return format!("\"\"\"{}\"\"\"", value);
    }

    let quoted = quote_source_string(value, interpolate);
    if fits_inline(&quoted, width) {
        return quoted;
    }

    if interpolate {
        return format_interpolated_string_literal(value, indent, width).unwrap_or(quoted);
    }

    let chunks = split_string_literal(value, width.saturating_sub(4).max(1));
    render_string_chunks(&chunks, false, indent)
}

fn format_interpolated_string_literal(value: &str, indent: usize, width: usize) -> Option<String> {
    let max_chars = width.saturating_sub(4).max(1);
    let mut pieces = Vec::new();
    let mut literal = String::new();

    for chunk in interpolation::parse_chunks(value).ok()? {
        match chunk {
            InterpolationChunk::Text(text) => literal.push_str(text),
            InterpolationChunk::EscapedOpen => literal.push_str("{{"),
            InterpolationChunk::EscapedClose => literal.push_str("}}"),
            InterpolationChunk::Expr(expr) => {
                flush_interpolated_literal(&mut pieces, &mut literal, max_chars);
                pieces.push(format!("{{{expr}}}"));
            }
        }
    }

    flush_interpolated_literal(&mut pieces, &mut literal, max_chars);

    if pieces.is_empty() {
        pieces.push(String::new());
    }

    Some(render_string_chunks(&pieces, true, indent))
}

fn flush_interpolated_literal(pieces: &mut Vec<String>, literal: &mut String, max_chars: usize) {
    if literal.is_empty() {
        return;
    }

    pieces.extend(split_string_literal(literal, max_chars));
    literal.clear();
}

fn render_string_chunks(chunks: &[String], interpolate: bool, indent: usize) -> String {
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
    if is_identifier_key(key) {
        key.to_string()
    } else {
        quote_string(key)
    }
}

fn is_identifier_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_identifier_start(first) && chars.all(is_identifier_continue)
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_identifier_continue(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn quote_string(value: &str) -> String {
    let mut out = String::new();
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\u{0007}' => out.push_str("\\a"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\u{000b}' => out.push_str("\\v"),
            '\u{001b}' => out.push_str("\\e"),
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
            '\u{0007}' => out.push_str("\\a"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\u{000b}' => out.push_str("\\v"),
            '\u{001b}' => out.push_str("\\e"),
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
    use crate::{
        check_source,
        lexer::Lexer,
        parser::Parser,
        temporal::{DateTimeValue, DateValue, DurationValue},
    };

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
        let source = "emit bullet(\"replace\", text.replace(\"text/math/files\", \"/\", \" -> \"))";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("text.replace(\"text/math/files\", \"/\", \" -> \"),"));
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
val hit=regex.find("42",regex{named val{one_or_more digit}})
emit m.from
emit hit.named.val"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("emit m.from"));
        assert!(formatted.contains("emit hit.named.val"));
    }

    #[test]
    fn canonicalizes_regex_text_items_back_into_dsl() {
        let source = r#"emit regex{r"(?i)^\d{2}$"}"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("emit regex(case_insensitive) {"));
        assert!(formatted.contains("start"));
        assert!(formatted.contains("exactly 2 digit"));
        assert!(formatted.contains("end"));
    }

    #[test]
    fn canonicalizes_regex_conditionals_back_into_dsl() {
        let source = r#"emit regex{r"(a)?b(?(1)c|d)"}"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("optional group {"));
        assert!(formatted.contains("if_capture 1 then {"));
        assert!(formatted.contains("} else {"));
    }

    #[test]
    fn formats_temporal_values_with_datetime_namespace() {
        let date = Expr::Literal(Value::Date(DateValue::parse_iso("2024-02-29").unwrap()));
        let datetime = Expr::Literal(Value::DateTime(
            DateTimeValue::parse_iso("2024-02-29T12:00:00Z").unwrap(),
        ));
        let duration = Expr::Literal(Value::Duration(DurationValue::parse_iso("PT15M").unwrap()));

        assert_eq!(
            format_expr_for_line(&date, 0, 0),
            r#"datetime.parse("2024-02-29", datetime.as_date)"#
        );
        assert_eq!(
            format_expr_for_line(&datetime, 0, 0),
            r#"datetime.parse("2024-02-29T12:00:00Z", datetime.as_datetime)"#
        );
        assert_eq!(
            format_expr_for_line(&duration, 0, 0),
            r#"datetime.parse("PT15M", datetime.as_duration)"#
        );
    }

    #[test]
    fn formats_bytes_literals_canonically() {
        let source = "emit b\"a\\xff\\0b\"";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("emit b\"a\\xff\\0b\""));
    }

    #[test]
    fn formats_try_catch_and_throw_canonically() {
        let source = r#"try{throw {code: "E8000", message: "boom"}}catch err{emit err.code}"#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("try {"));
        assert!(formatted.contains("throw {"));
        assert!(formatted.contains("code: \"E8000\","));
        assert!(formatted.contains("message: \"boom\","));
        assert!(formatted.contains("} catch err {"));
        check_source(&formatted).unwrap();
    }

    #[test]
    fn keeps_long_interpolated_strings_valid_after_formatting() {
        let source = r#"val user = {profile: {display_name: "Ana"}}
emit "prefix {{literal}} value={user.profile.display_name} suffix suffix suffix suffix suffix""#;
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("\"prefix {{literal}} value=\" +"));
        assert!(formatted.contains("\"{user.profile.display_name}\""));
        check_source(&formatted).unwrap();
    }

    #[test]
    fn formats_namespace_declarations() {
        let source = "namespace http {\nval timeout = 30\nfunc get(url) {\nreturn url\n}\n}\nemit http.timeout";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("namespace http {"));
        assert!(formatted.contains("  val timeout = 30"));
        assert!(formatted.contains("  func get(url) {"));
        assert!(formatted.contains("emit http.timeout"));
        check_source(&formatted).unwrap();
    }

    #[test]
    fn formats_struct_declarations() {
        let source = "struct Point {\nx: 0\ny: 0\n}\nemit Point.x";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("struct Point {"));
        assert!(formatted.contains("  x: 0"));
        assert!(formatted.contains("  y: 0"));
        assert!(formatted.contains("emit Point.x"));
        check_source(&formatted).unwrap();
    }

    #[test]
    fn formats_struct_without_defaults() {
        let source = "struct User {\nname\nage\n}";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("struct User {"));
        assert!(formatted.contains("  name"));
        assert!(formatted.contains("  age"));
        assert!(!formatted.contains("name:"));
        assert!(!formatted.contains("age:"));
        check_source(&formatted).unwrap();
    }

    #[test]
    fn formats_enum_inline_when_short() {
        let source = "enum Status {\nactive,\ninactive,\n}";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("enum Status { active, inactive }"));
        check_source(&formatted).unwrap();
    }

    #[test]
    fn formats_enum_multiline_when_long() {
        let source = "enum Color {\nred,\ngreen,\nblue,\nyellow,\n}";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("enum Color {"));
        assert!(formatted.contains("  red,"));
        assert!(formatted.contains("  green,"));
        assert!(formatted.contains("  blue,"));
        assert!(formatted.contains("  yellow,"));
        check_source(&formatted).unwrap();
    }

    #[test]
    fn formats_type_alias() {
        let source = "type Url = string";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert_eq!(formatted.trim(), "type Url = string");
    }

    #[test]
    fn keeps_non_ascii_map_keys_unquoted_when_they_are_identifiers() {
        let source = "val dados = {über: 1}\nemit dados.über";
        let tokens = Lexer::new(source).tokenize().unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        let formatted = format_program(&program);

        assert!(formatted.contains("über: 1"));
        assert!(!formatted.contains("\"über\": 1"));
        check_source(&formatted).unwrap();
    }
}
