// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Helper constructors and utilities for checker symbols and diagnostics.

use super::*;

impl Symbol {
    pub(super) fn unknown(mutable: bool) -> Self {
        Self {
            mutable,
            kind: SymbolKind::Unknown,
        }
    }

    pub(super) fn function_arities(arities: &[usize], mutable: bool) -> Self {
        Self::function_symbol(arities, mutable, None)
    }

    pub(super) fn builtin_function(target: &str, arities: &[usize]) -> Self {
        Self::function_symbol(arities, false, Some(target.to_string()))
    }

    fn function_symbol(arities: &[usize], mutable: bool, builtin_target: Option<String>) -> Self {
        Self {
            mutable,
            kind: SymbolKind::Function {
                arities: arities.to_vec(),
                builtin_target,
            },
        }
    }

    pub(super) fn namespace(symbols: HashMap<String, Symbol>) -> Self {
        Self {
            mutable: false,
            kind: SymbolKind::Namespace(symbols),
        }
    }

    pub(super) fn builtin_target(&self) -> Option<&str> {
        match &self.kind {
            SymbolKind::Function {
                builtin_target: Some(target),
                ..
            } => Some(target.as_str()),
            _ => None,
        }
    }
}

impl PositionIndex {
    pub(super) fn from_tokens(tokens: &[Token]) -> Self {
        let mut index = Self::default();
        for token in tokens {
            match &token.kind {
                TokenKind::Identifier(name) => index
                    .identifiers
                    .entry(name.clone())
                    .or_default()
                    .push((token.line, token.column)),
                kind => {
                    if let Some(keyword) = keyword_name(kind) {
                        index
                            .keywords
                            .entry(keyword)
                            .or_default()
                            .push((token.line, token.column));
                    }
                }
            }
        }
        index
    }

    pub(super) fn identifier(&self, name: &str) -> Option<(usize, usize)> {
        self.identifiers
            .get(name)
            .and_then(|positions| positions.first().copied())
    }

    pub(super) fn keyword(&self, keyword: &'static str) -> Option<(usize, usize)> {
        self.keywords
            .get(keyword)
            .and_then(|positions| positions.first().copied())
    }
}

pub(super) fn declared_exports(program: &Program) -> HashMap<String, Symbol> {
    let mut symbols = HashMap::new();
    for statement in &program.statements {
        match statement {
            Stmt::Bind {
                name,
                value,
                mutable,
            } => {
                symbols.insert(name.clone(), static_symbol_for_expr(value, *mutable));
            }
            Stmt::Func { name, params, .. } => {
                let required = params.iter().filter(|p| p.default.is_none()).count();
                let total = params.len();
                symbols.insert(
                    name.clone(),
                    Symbol::function_arities(&(required..=total).collect::<Vec<_>>(), false),
                );
            }
            Stmt::Namespace { name, body } => {
                symbols.insert(
                    name.clone(),
                    Symbol::namespace(static_namespace_symbols(body)),
                );
            }
            Stmt::Struct { name, fields } => {
                symbols.insert(name.clone(), static_struct_symbol(fields));
            }
            Stmt::Enum { name, variants } => {
                symbols.insert(name.clone(), static_enum_symbol(variants));
            }
            _ => {}
        }
    }
    symbols
}

pub(super) fn static_symbol_for_expr(expr: &Expr, mutable: bool) -> Symbol {
    let kind = match expr {
        Expr::Lambda { params, .. } => SymbolKind::Function {
            arities: {
                let required = params.iter().filter(|p| p.default.is_none()).count();
                let total = params.len();
                (required..=total).collect()
            },
            builtin_target: None,
        },
        Expr::Map(pairs) => {
            let mut fields = HashMap::new();
            for (key, value) in pairs {
                fields.insert(key.clone(), static_symbol_for_expr(value, false));
            }
            SymbolKind::Map(fields)
        }
        _ => SymbolKind::Unknown,
    };
    Symbol { mutable, kind }
}

pub(super) fn static_namespace_symbols(body: &[Stmt]) -> HashMap<String, Symbol> {
    let mut symbols = HashMap::new();
    for statement in body {
        match statement {
            Stmt::Bind {
                name,
                value,
                mutable,
            } => {
                symbols.insert(name.clone(), static_symbol_for_expr(value, *mutable));
            }
            Stmt::Func { name, params, .. } => {
                let required = params.iter().filter(|p| p.default.is_none()).count();
                let total = params.len();
                symbols.insert(
                    name.clone(),
                    Symbol::function_arities(&(required..=total).collect::<Vec<_>>(), false),
                );
            }
            Stmt::Namespace { name, body } => {
                symbols.insert(
                    name.clone(),
                    Symbol::namespace(static_namespace_symbols(body)),
                );
            }
            Stmt::Struct { name, fields } => {
                symbols.insert(name.clone(), static_struct_symbol(fields));
            }
            Stmt::Enum { name, variants } => {
                symbols.insert(name.clone(), static_enum_symbol(variants));
            }
            _ => {}
        }
    }
    symbols
}

pub(super) fn static_struct_symbol(fields: &[crate::ast::StructField]) -> Symbol {
    let mut field_symbols = HashMap::new();
    for field in fields {
        let symbol = field
            .default
            .as_ref()
            .map(|value| static_symbol_for_expr(value, false))
            .unwrap_or_else(|| Symbol::unknown(false));
        field_symbols.insert(field.name.clone(), symbol);
    }
    Symbol {
        mutable: false,
        kind: SymbolKind::Map(field_symbols),
    }
}

pub(super) fn static_enum_symbol(variants: &[String]) -> Symbol {
    let mut namespace = HashMap::new();
    for variant in variants {
        let mut fields = HashMap::new();
        fields.insert("kind".to_string(), Symbol::unknown(false));
        namespace.insert(
            variant.clone(),
            Symbol {
                mutable: false,
                kind: SymbolKind::Map(fields),
            },
        );
    }
    Symbol::namespace(namespace)
}

pub(super) fn builtin_call_symbol(target: &str) -> Option<SymbolKind> {
    match target {
        "scan.pos" => Some(scan_position_symbol().kind),
        "scan.match" | "scan.expect" | "scan.take_while" | "scan.take_until" | "scan.span" => {
            Some(scan_span_symbol().kind)
        }
        "scan.token" => Some(scan_token_symbol().kind),
        _ => None,
    }
}

pub(super) fn recoverable_error_symbol() -> Symbol {
    let mut fields = HashMap::new();
    fields.insert("code".to_string(), Symbol::unknown(false));
    fields.insert("message".to_string(), Symbol::unknown(false));
    fields.insert("file".to_string(), Symbol::unknown(false));
    fields.insert("line".to_string(), Symbol::unknown(false));
    fields.insert("column".to_string(), Symbol::unknown(false));
    fields.insert("context".to_string(), Symbol::unknown(false));
    let mut span_fields = HashMap::new();
    span_fields.insert("line".to_string(), Symbol::unknown(false));
    span_fields.insert("column".to_string(), Symbol::unknown(false));
    fields.insert(
        "span".to_string(),
        Symbol {
            mutable: false,
            kind: SymbolKind::Map(span_fields),
        },
    );
    Symbol {
        mutable: false,
        kind: SymbolKind::Map(fields),
    }
}

pub(super) fn regex_namespace_symbol() -> Symbol {
    let fields = stdlib::regex_surface_items()
        .iter()
        .map(|(field, target, arities)| {
            (
                (*field).to_string(),
                match arities {
                    Some(arities) => Symbol::builtin_function(target, arities),
                    None => Symbol::unknown(false),
                },
            )
        })
        .collect::<HashMap<_, _>>();
    Symbol::namespace(fields)
}

fn scan_position_symbol() -> Symbol {
    let mut fields = HashMap::new();
    fields.insert("offset".to_string(), Symbol::unknown(false));
    fields.insert("line".to_string(), Symbol::unknown(false));
    fields.insert("column".to_string(), Symbol::unknown(false));
    Symbol {
        mutable: false,
        kind: SymbolKind::Map(fields),
    }
}

fn scan_span_symbol() -> Symbol {
    let mut fields = HashMap::new();
    fields.insert("text".to_string(), Symbol::unknown(false));
    fields.insert("start".to_string(), scan_position_symbol());
    fields.insert("end".to_string(), scan_position_symbol());
    Symbol {
        mutable: false,
        kind: SymbolKind::Map(fields),
    }
}

fn scan_token_symbol() -> Symbol {
    let mut fields = HashMap::new();
    fields.insert("kind".to_string(), Symbol::unknown(false));
    fields.insert("text".to_string(), Symbol::unknown(false));
    fields.insert("span".to_string(), scan_span_symbol());
    Symbol {
        mutable: false,
        kind: SymbolKind::Map(fields),
    }
}

pub(super) fn resolve_use(path: &str, base_dir: Option<&Path>) -> NodiaResult<PathBuf> {
    let raw = Path::new(path);
    let joined = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        base_dir.unwrap_or_else(|| Path::new(".")).join(raw)
    };

    let candidates = if joined.extension().is_some() {
        vec![joined]
    } else {
        vec![
            joined.with_extension("nod"),
            joined.join("index.nod"),
            joined,
        ]
    };

    for candidate in candidates {
        if candidate.exists() {
            return candidate.canonicalize().map_err(|err| {
                NodiaError::io(format!(
                    "cannot resolve use '{}': {err}",
                    candidate.display()
                ))
            });
        }
    }

    Err(NodiaError::io(format!("cannot resolve use '{path}'")))
}

pub(super) fn direct_identifier(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Identifier(name) => Some(name),
        _ => None,
    }
}

pub(super) fn display_call_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Identifier(name) => Some(name.clone()),
        Expr::Get { object, field } => {
            let prefix = display_call_name(object)?;
            Some(format!("{prefix}.{field}"))
        }
        _ => None,
    }
}

pub(super) fn keyword_name(kind: &TokenKind) -> Option<&'static str> {
    match kind {
        TokenKind::Val => Some("val"),
        TokenKind::Var => Some("var"),
        TokenKind::Func => Some("func"),
        TokenKind::LegacyLet => Some("let"),
        TokenKind::LegacyConst => Some("const"),
        TokenKind::LegacyFn => Some("fn"),
        TokenKind::Return => Some("return"),
        TokenKind::Emit => Some("emit"),
        TokenKind::If => Some("if"),
        TokenKind::Else => Some("else"),
        TokenKind::For => Some("for"),
        TokenKind::In => Some("in"),
        TokenKind::While => Some("while"),
        TokenKind::Break => Some("break"),
        TokenKind::Continue => Some("continue"),
        TokenKind::Match => Some("match"),
        TokenKind::Case => Some("case"),
        TokenKind::Default => Some("default"),
        TokenKind::Try => Some("try"),
        TokenKind::Catch => Some("catch"),
        TokenKind::Throw => Some("throw"),
        TokenKind::LegacyImport => Some("import"),
        TokenKind::Use => Some("use"),
        TokenKind::Lambda => Some("lambda"),
        TokenKind::Regex => Some("regex"),
        TokenKind::As => Some("as"),
        TokenKind::Pick => Some("pick"),
        TokenKind::LegacyShow => Some("show"),
        TokenKind::Hide => Some("hide"),
        _ => None,
    }
}

pub(super) fn keyword_from_name(name: &str) -> Option<&'static str> {
    match name {
        "val" => Some("val"),
        "var" => Some("var"),
        "func" => Some("func"),
        "let" => Some("let"),
        "const" => Some("const"),
        "fn" => Some("fn"),
        "return" => Some("return"),
        "emit" => Some("emit"),
        "if" => Some("if"),
        "else" => Some("else"),
        "for" => Some("for"),
        "in" => Some("in"),
        "while" => Some("while"),
        "break" => Some("break"),
        "continue" => Some("continue"),
        "match" => Some("match"),
        "case" => Some("case"),
        "default" => Some("default"),
        "try" => Some("try"),
        "catch" => Some("catch"),
        "throw" => Some("throw"),
        "import" => Some("import"),
        "use" => Some("use"),
        "lambda" => Some("lambda"),
        "regex" => Some("regex"),
        "as" => Some("as"),
        "pick" => Some("pick"),
        "show" => Some("show"),
        "hide" => Some("hide"),
        _ => None,
    }
}

pub(super) fn semantic(
    code: &'static str,
    message: impl Into<String>,
    position: Option<(usize, usize)>,
) -> NodiaError {
    let (line, column) = position.unwrap_or((0, 0));
    NodiaError::semantic_at(message, line, column).with_code(code)
}

#[derive(Clone)]
pub(super) enum AssignmentSymbolStep {
    Field(String),
    UnknownIndex,
}

pub(super) fn assign_target_root_name(target: &AssignTarget) -> Option<&str> {
    match target {
        AssignTarget::Identifier(name) => Some(name),
        AssignTarget::Get { object, .. } | AssignTarget::Index { object, .. } => {
            assign_target_root_name(object)
        }
    }
}

pub(super) fn assignment_symbol_steps(target: &AssignTarget) -> Vec<AssignmentSymbolStep> {
    let mut steps = Vec::new();
    collect_target_steps(target, &mut steps);
    steps
}

pub(super) fn collect_target_steps(target: &AssignTarget, steps: &mut Vec<AssignmentSymbolStep>) {
    match target {
        AssignTarget::Identifier(_) => {}
        AssignTarget::Get { object, field } => {
            collect_target_steps(object, steps);
            steps.push(AssignmentSymbolStep::Field(field.clone()));
        }
        AssignTarget::Index { object, index } => {
            collect_target_steps(object, steps);
            if let Some(key) = static_map_key(index) {
                steps.push(AssignmentSymbolStep::Field(key));
            } else {
                steps.push(AssignmentSymbolStep::UnknownIndex);
            }
        }
    }
}

pub(super) fn static_map_key(index: &Expr) -> Option<String> {
    match index {
        Expr::Literal(value) => Some(value.to_string()),
        Expr::String { .. } => static_string_literal(index).map(str::to_string),
        _ => None,
    }
}

pub(super) fn static_string_literal(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::String { value, interpolate } if !*interpolate || !value.contains(['{', '}']) => {
            Some(value.as_str())
        }
        _ => None,
    }
}
