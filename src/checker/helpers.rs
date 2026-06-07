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

    pub(super) fn function(arity: usize, mutable: bool) -> Self {
        Self::function_arities(&[arity], mutable)
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
                symbols.insert(name.clone(), Symbol::function(params.len(), false));
            }
            _ => {}
        }
    }
    symbols
}

pub(super) fn static_symbol_for_expr(expr: &Expr, mutable: bool) -> Symbol {
    let kind = match expr {
        Expr::Lambda { params, .. } => SymbolKind::Function {
            arities: vec![params.len()],
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
