// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Expression-level semantic checks.

use crate::interpolation::{self, Chunk as InterpolationChunk};

use super::helpers::*;
use super::*;

impl<'a> State<'a> {
    pub(super) fn check_expr(&mut self, expr: &Expr) -> NodiaResult<()> {
        match expr {
            Expr::Literal(_) => Ok(()),
            Expr::String { value, interpolate } => {
                if *interpolate {
                    self.check_interpolations(value)?;
                }
                Ok(())
            }
            Expr::Lambda { params, body } => self.check_function(params, body),
            Expr::Regex(pattern) => regex::validate(pattern),
            Expr::Identifier(name) => self.lookup(name).map(|_| ()).ok_or_else(|| {
                self.error_name("E4100", format!("undefined variable '{name}'"), name)
            }),
            Expr::Unary { expr, .. } => self.check_expr(expr),
            Expr::Binary { left, right, .. } => {
                self.check_expr(left)?;
                self.check_expr(right)
            }
            Expr::Call { callee, args } => self.check_call(callee, args),
            Expr::Get { object, field } => {
                match self.field_status(object, field) {
                    FieldStatus::Found(_) | FieldStatus::Unknown => {}
                    FieldStatus::Missing => {
                        return Err(self.error_name(
                            "E4105",
                            format!("field '{field}' not found"),
                            field,
                        ));
                    }
                }
                self.check_expr(object)
            }
            Expr::Index { object, index } => {
                self.check_expr(object)?;
                self.check_expr(index)?;
                self.check_index_access(object, index)
            }
            Expr::List(values) => {
                for value in values {
                    self.check_expr(value)?;
                }
                Ok(())
            }
            Expr::Map(pairs) => {
                for (_, value) in pairs {
                    self.check_expr(value)?;
                }
                Ok(())
            }
        }
    }

    pub(super) fn check_call(&mut self, callee: &Expr, args: &[Expr]) -> NodiaResult<()> {
        for arg in args {
            self.check_expr(arg)?;
        }

        self.check_builtin_call_diagnostics(callee, args)?;

        if let Some(name) = direct_identifier(callee) {
            if let Some(symbol) = self.lookup(name).cloned() {
                if let SymbolKind::Function { arities, .. } = &symbol.kind {
                    self.check_arity(name, name, args.len(), arities)?;
                }
                return Ok(());
            }
            return Err(self.error_name("E4100", format!("undefined variable '{name}'"), name));
        }

        if let Expr::Get { object, field } = callee {
            match self.field_status(object, field) {
                FieldStatus::Found(symbol) => {
                    if let SymbolKind::Function { arities, .. } = symbol.kind {
                        let display_name =
                            display_call_name(callee).unwrap_or_else(|| field.clone());
                        self.check_arity(&display_name, field, args.len(), &arities)?;
                    }
                    self.check_expr(object)?;
                    return Ok(());
                }
                FieldStatus::Missing => {
                    return Err(self.error_name(
                        "E4105",
                        format!("field '{field}' not found"),
                        field,
                    ));
                }
                FieldStatus::Unknown => {}
            }
        }

        self.check_expr(callee)
    }

    pub(super) fn check_assign_target(
        &mut self,
        target: &AssignTarget,
        final_step: bool,
    ) -> NodiaResult<Option<Symbol>> {
        match target {
            AssignTarget::Identifier(name) => {
                let Some(symbol) = self.lookup(name).cloned() else {
                    return Err(self.error_name(
                        "E4100",
                        format!("undefined variable '{name}'"),
                        name,
                    ));
                };
                if !symbol.mutable
                    && (final_step || !matches!(symbol.kind, SymbolKind::Namespace(_)))
                {
                    return Err(self.error_name(
                        "E4101",
                        format!("cannot assign to val '{name}'"),
                        name,
                    ));
                }
                Ok(Some(symbol))
            }
            AssignTarget::Get { object, field } => {
                let object_symbol = self.check_assign_target(object, false)?;
                match object_symbol.map(|symbol| symbol.kind) {
                    Some(SymbolKind::Namespace(fields)) => {
                        let Some(field_symbol) = fields.get(field).cloned() else {
                            return Err(self.error_name(
                                "E4105",
                                format!("field '{field}' not found"),
                                field,
                            ));
                        };
                        if !field_symbol.mutable {
                            return Err(self.error_name(
                                "E4101",
                                format!("cannot assign to val '{field}'"),
                                field,
                            ));
                        }
                        Ok(Some(field_symbol))
                    }
                    Some(SymbolKind::Map(fields)) => {
                        if final_step {
                            Ok(fields.get(field).cloned())
                        } else {
                            let Some(field_symbol) = fields.get(field).cloned() else {
                                return Err(self.error_name(
                                    "E4105",
                                    format!("field '{field}' not found"),
                                    field,
                                ));
                            };
                            Ok(Some(field_symbol))
                        }
                    }
                    Some(SymbolKind::Unknown | SymbolKind::Function { .. }) | None => Ok(None),
                }
            }
            AssignTarget::Index { object, index } => {
                self.check_expr(index)?;
                self.check_assign_target(object, false)?;
                Ok(None)
            }
        }
    }

    pub(super) fn apply_assignment_symbol(
        &mut self,
        target: &AssignTarget,
        value: &Expr,
    ) -> NodiaResult<()> {
        let Some(root_name) = assign_target_root_name(target) else {
            return Ok(());
        };
        let Some(root_symbol) = self.lookup(root_name).cloned() else {
            return Err(self.error_name(
                "E4100",
                format!("undefined variable '{root_name}'"),
                root_name,
            ));
        };
        let value_symbol = self.symbol_for_expr(value, false);
        let steps = assignment_symbol_steps(target);
        let updated = self.update_assigned_symbol(root_symbol, &steps, value_symbol);
        self.update_symbol(root_name, updated)
    }

    pub(super) fn update_assigned_symbol(
        &self,
        symbol: Symbol,
        steps: &[AssignmentSymbolStep],
        value_symbol: Symbol,
    ) -> Symbol {
        if steps.is_empty() {
            return Symbol {
                mutable: symbol.mutable,
                kind: value_symbol.kind,
            };
        }

        match &steps[0] {
            AssignmentSymbolStep::UnknownIndex => symbol,
            AssignmentSymbolStep::Field(field) => {
                self.update_symbol_field(symbol, field, &steps[1..], value_symbol)
            }
        }
    }

    pub(super) fn update_symbol_field(
        &self,
        symbol: Symbol,
        field: &str,
        rest: &[AssignmentSymbolStep],
        value_symbol: Symbol,
    ) -> Symbol {
        let mutable = symbol.mutable;
        let kind = match symbol.kind {
            SymbolKind::Map(mut fields) => {
                let current = fields
                    .remove(field)
                    .unwrap_or_else(|| Symbol::unknown(false));
                let updated = self.update_assigned_symbol(current, rest, value_symbol);
                fields.insert(field.to_string(), updated);
                SymbolKind::Map(fields)
            }
            SymbolKind::Namespace(mut fields) => {
                if let Some(current) = fields.remove(field) {
                    let updated = self.update_assigned_symbol(current, rest, value_symbol);
                    fields.insert(field.to_string(), updated);
                }
                SymbolKind::Namespace(fields)
            }
            other => other,
        };
        Symbol { mutable, kind }
    }

    pub(super) fn check_interpolations(&mut self, raw: &str) -> NodiaResult<()> {
        for chunk in
            interpolation::parse_chunks(raw).map_err(|message| semantic("E4106", message, None))?
        {
            if let InterpolationChunk::Expr(expr_text) = chunk {
                if expr_text.trim().is_empty() {
                    return Err(semantic("E4106", "empty interpolation", None));
                }
                let tokens = Lexer::new(expr_text).tokenize().map_err(|err| {
                    semantic(
                        "E4106",
                        format!("invalid interpolation: {}", err.message),
                        None,
                    )
                })?;
                let expr = Parser::new(tokens).parse_expression_only().map_err(|err| {
                    semantic(
                        "E4106",
                        format!("invalid interpolation: {}", err.message),
                        None,
                    )
                })?;
                self.check_expr(&expr)?;
            }
        }
        Ok(())
    }

    fn check_index_access(&self, object: &Expr, index: &Expr) -> NodiaResult<()> {
        let Some(key) = static_map_key(index) else {
            return Ok(());
        };

        let symbol = self.symbol_for_expr(object, false);
        match symbol.kind {
            SymbolKind::Map(fields) | SymbolKind::Namespace(fields) => {
                if fields.contains_key(&key) {
                    Ok(())
                } else {
                    Err(semantic("E4105", format!("key '{key}' not found"), None))
                }
            }
            SymbolKind::Unknown | SymbolKind::Function { .. } => Ok(()),
        }
    }

    fn check_builtin_call_diagnostics(&self, callee: &Expr, args: &[Expr]) -> NodiaResult<()> {
        let Some(target) = self.builtin_call_target(callee) else {
            return Ok(());
        };
        match target.as_str() {
            "replace" => self.check_replace_call_diagnostics(args),
            "test" | "find" => self.check_regex_match_pattern_diagnostics(args),
            _ => Ok(()),
        }
    }

    fn builtin_call_target(&self, callee: &Expr) -> Option<String> {
        match callee {
            Expr::Identifier(name) => self
                .lookup(name)
                .and_then(Symbol::builtin_target)
                .map(str::to_string),
            Expr::Get { object, field } => match self.field_status(object, field) {
                FieldStatus::Found(symbol) => symbol.builtin_target().map(str::to_string),
                FieldStatus::Missing | FieldStatus::Unknown => None,
            },
            _ => None,
        }
    }

    fn check_replace_call_diagnostics(&self, args: &[Expr]) -> NodiaResult<()> {
        if args.len() != 3 {
            return Ok(());
        }

        let Some(replacement) = static_string_literal(&args[2]) else {
            return Ok(());
        };

        match &args[1] {
            Expr::Regex(pattern) => regex::validate_replacement(pattern, replacement),
            _ => Ok(()),
        }
    }

    fn check_regex_match_pattern_diagnostics(&self, args: &[Expr]) -> NodiaResult<()> {
        if args.len() < 2 {
            return Ok(());
        }

        let Some(pattern) = static_string_literal(&args[1]) else {
            return Ok(());
        };

        regex::validate_text(pattern)
    }
}
