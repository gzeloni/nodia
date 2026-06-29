// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Symbol resolution for local scopes, modules, and standard-library namespaces.

use super::helpers::*;
use super::*;

impl<'a> State<'a> {
    pub(super) fn declare_use(
        &mut self,
        target: &UseTarget,
        alias: Option<&str>,
        pick: &[String],
        hide: &[String],
    ) -> NodiaResult<()> {
        match target {
            UseTarget::Path(path) => {
                let module = self.checker.load_module(path, self.base_dir.as_deref())?;

                if let Some(alias) = alias {
                    let symbols = self.selected_symbols(&module, pick, hide)?;
                    return self.declare(alias, Symbol::namespace(symbols));
                }

                for (name, symbol) in self.selected_symbols(&module, pick, hide)? {
                    self.declare(&name, symbol)?;
                }
                Ok(())
            }
            UseTarget::Stdlib(name) => {
                let symbols = self.selected_stdlib_symbols(name, pick, hide)?;
                if let Some(alias) = alias {
                    return self.declare(alias, Symbol::namespace(symbols));
                }
                if pick.is_empty() {
                    return self.declare(name, Symbol::namespace(symbols));
                }
                for (name, symbol) in symbols {
                    self.declare(&name, symbol)?;
                }
                Ok(())
            }
        }
    }

    pub(super) fn selected_symbols(
        &self,
        module: &ModuleInfo,
        pick: &[String],
        hide: &[String],
    ) -> NodiaResult<HashMap<String, Symbol>> {
        self.selected_symbol_map(&module.symbols, pick, hide)
    }

    pub(super) fn selected_stdlib_symbols(
        &self,
        name: &str,
        pick: &[String],
        hide: &[String],
    ) -> NodiaResult<HashMap<String, Symbol>> {
        let Some(items) = stdlib::module_items(name) else {
            return Err(self.error_name("E4104", format!("unknown stdlib module '{name}'"), name));
        };
        let symbols = items
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
        self.selected_symbol_map(&symbols, pick, hide)
    }

    pub(super) fn selected_symbol_map(
        &self,
        symbols: &HashMap<String, Symbol>,
        pick: &[String],
        hide: &[String],
    ) -> NodiaResult<HashMap<String, Symbol>> {
        let mut selected = HashMap::new();
        if pick.is_empty() {
            for (name, symbol) in symbols {
                if !hide.contains(name) {
                    selected.insert(name.clone(), symbol.clone());
                }
            }
            return Ok(selected);
        }

        for name in pick {
            let Some(symbol) = symbols.get(name) else {
                return Err(self.error_name(
                    "E4104",
                    format!("use does not expose '{name}'"),
                    name,
                ));
            };
            if !hide.contains(name) {
                selected.insert(name.clone(), symbol.clone());
            }
        }
        Ok(selected)
    }

    pub(super) fn declare(&mut self, name: &str, symbol: Symbol) -> NodiaResult<()> {
        let scope = self.scopes.last_mut().expect("checker always has a scope");
        if scope.contains_key(name) {
            return Err(self.error_name(
                "E4102",
                format!("'{name}' is already defined in this scope"),
                name,
            ));
        }
        scope.insert(name.to_string(), symbol);
        Ok(())
    }

    pub(super) fn update_symbol(&mut self, name: &str, symbol: Symbol) -> NodiaResult<()> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), symbol);
                return Ok(());
            }
        }
        Err(self.error_name("E4100", format!("undefined variable '{name}'"), name))
    }

    pub(super) fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    pub(super) fn field_status(&self, object: &Expr, field: &str) -> FieldStatus {
        let Some(symbol) = self.symbol_from_access(object) else {
            return FieldStatus::Unknown;
        };
        match symbol.kind {
            SymbolKind::Map(fields) | SymbolKind::Namespace(fields) => fields
                .get(field)
                .cloned()
                .map(FieldStatus::Found)
                .unwrap_or(FieldStatus::Missing),
            SymbolKind::Unknown | SymbolKind::Result | SymbolKind::Function { .. } => {
                FieldStatus::Unknown
            }
        }
    }

    pub(super) fn symbol_from_access(&self, expr: &Expr) -> Option<Symbol> {
        match expr {
            Expr::Identifier(name) => self.lookup(name).cloned(),
            Expr::Get { object, field } => match self.field_status(object, field) {
                FieldStatus::Found(symbol) => Some(symbol),
                FieldStatus::Missing | FieldStatus::Unknown => None,
            },
            _ => None,
        }
    }

    pub(super) fn symbol_for_expr(&self, expr: &Expr, mutable: bool) -> Symbol {
        let kind = match expr {
            Expr::Lambda { params, .. } => SymbolKind::Function {
                arities: vec![params.len()],
                builtin_target: None,
            },
            Expr::Call { callee, .. } => self
                .builtin_call_target(callee)
                .as_deref()
                .and_then(builtin_call_symbol)
                .unwrap_or(SymbolKind::Unknown),
            Expr::Map(pairs) => {
                let mut fields = HashMap::new();
                for (key, value) in pairs {
                    fields.insert(key.clone(), self.symbol_for_expr(value, false));
                }
                SymbolKind::Map(fields)
            }
            Expr::Identifier(_) | Expr::Get { .. } => self
                .symbol_from_access(expr)
                .map(|symbol| symbol.kind)
                .unwrap_or(SymbolKind::Unknown),
            _ => SymbolKind::Unknown,
        };
        Symbol { mutable, kind }
    }

    pub(super) fn check_arity(
        &self,
        display_name: &str,
        highlight_name: &str,
        got: usize,
        expected: &[usize],
    ) -> NodiaResult<()> {
        if expected.contains(&got) {
            return Ok(());
        }
        let expected = expected
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(" or ");
        Err(self.error_name(
            "E4107",
            format!("{display_name}() expects {expected} argument(s), got {got}"),
            highlight_name,
        ))
    }

    pub(super) fn error_name(
        &self,
        code: &'static str,
        message: impl Into<String>,
        name: &str,
    ) -> NodiaError {
        let position = self.positions.identifier(name).or_else(|| {
            keyword_from_name(name).and_then(|keyword| self.positions.keyword(keyword))
        });
        semantic(code, message, position)
    }

    pub(super) fn error_keyword(
        &self,
        code: &'static str,
        message: impl Into<String>,
        keyword: &'static str,
    ) -> NodiaError {
        semantic(code, message, self.positions.keyword(keyword))
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }
}
