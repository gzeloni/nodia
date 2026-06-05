// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Expression-level semantic checks.

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
            Expr::Identifier(name) => {
                if self.lookup(name).is_some() || self.builtin_symbol(name).is_some() {
                    Ok(())
                } else {
                    Err(self.error_name("E4100", format!("undefined variable '{name}'"), name))
                }
            }
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
                self.check_expr(index)
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

        if let Some(name) = direct_identifier(callee) {
            if let Some(symbol) = self.lookup(name).cloned().or_else(|| self.builtin_symbol(name))
            {
                if let SymbolKind::Function { arities } = &symbol.kind {
                    self.check_arity(name, args.len(), arities)?;
                }
                return Ok(());
            }
            return Err(self.error_name("E4100", format!("undefined variable '{name}'"), name));
        }

        if let Expr::Get { object, field } = callee {
            match self.field_status(object, field) {
                FieldStatus::Found(symbol) => {
                    if let SymbolKind::Function { arities } = symbol.kind {
                        self.check_arity(field, args.len(), &arities)?;
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
        let chars = raw.chars().collect::<Vec<_>>();
        let mut index = 0;
        while index < chars.len() {
            if chars[index] == '{' {
                if chars.get(index + 1) == Some(&'{') {
                    index += 2;
                    continue;
                }
                let start = index + 1;
                let mut end = start;
                while end < chars.len() && chars[end] != '}' {
                    end += 1;
                }
                if end == chars.len() {
                    return Err(semantic("E4106", "unterminated interpolation", None));
                }
                let expr_text = chars[start..end].iter().collect::<String>();
                let tokens = Lexer::new(&expr_text).tokenize()?;
                let expr = Parser::new(tokens).parse_expression_only()?;
                self.check_expr(&expr)?;
                index = end + 1;
            } else if chars[index] == '}' && chars.get(index + 1) == Some(&'}') {
                index += 2;
            } else {
                index += 1;
            }
        }
        Ok(())
    }
}
