use crate::ast::{BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::value::Value;

const INDENT: &str = "  ";
const LINE_WIDTH: usize = 60;

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
                path,
                alias,
                pick,
                hide,
            } => {
                self.write_indent();
                self.out.push_str("use ");
                self.out.push_str(&quote_string(path));
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
            Stmt::Assign { name, value } => {
                self.write_indent();
                let prefix = format!("{name} = ");
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
                name,
                iterable,
                body,
            } => {
                self.write_indent();
                self.out.push_str("for ");
                self.out.push_str(name);
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

fn format_expr_for_line(expr: &Expr, indent: usize, prefix_len: usize) -> String {
    format_expr_prec(expr, 0, indent, available_width(indent, prefix_len))
}

fn format_expr_prec(expr: &Expr, parent_prec: u8, indent: usize, width: usize) -> String {
    let prec = precedence(expr);
    let rendered = match expr {
        Expr::Literal(value) => format_literal(value, indent, width),
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
        Value::String(value) => format_string_literal(value, indent, width),
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
        Value::Stream(stream) => stream.to_string(),
        Value::UseBinding(_, name) => format!("<use {name}>"),
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


fn format_string_literal(value: &str, indent: usize, width: usize) -> String {
    if value.contains('\n') {
        return format!("\"\"\"{}\"\"\"", value);
    }

    let quoted = quote_string(value);
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
        out.push_str(&quote_string(chunk));
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
}
