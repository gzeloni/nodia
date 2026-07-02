// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Statement-level semantic checks and scope management.

use super::helpers::{
    recoverable_error_symbol, regex_namespace_symbol, semantic, static_enum_symbol,
    static_namespace_symbols, static_struct_symbol,
};
use super::*;

impl<'a> State<'a> {
    pub(super) fn new(
        checker: &'a mut Checker,
        base_dir: Option<PathBuf>,
        positions: PositionIndex,
    ) -> Self {
        let mut root = HashMap::new();
        root.insert("input".to_string(), Symbol::unknown(false));
        root.insert("regex".to_string(), regex_namespace_symbol());
        Self {
            checker,
            scopes: vec![root],
            base_dir,
            positions,
            loop_depth: 0,
            function_depth: 0,
        }
    }

    pub(super) fn predeclare_top_level(&mut self, program: &Program) -> NodiaResult<()> {
        for statement in &program.statements {
            match statement {
                Stmt::Use {
                    target,
                    alias,
                    pick,
                    hide,
                } => self.declare_use(target, alias.as_deref(), pick, hide)?,
                Stmt::Bind { name, mutable, .. } => {
                    self.declare(name, Symbol::unknown(*mutable))?
                }
                Stmt::Func { name, params, .. } => {
                    let required = params.iter().filter(|p| p.default.is_none()).count();
                    let total = params.len();
                    self.declare(
                        name,
                        Symbol::function_arities(
                            &(required..=total).map(|n| n).collect::<Vec<_>>(),
                            false,
                        ),
                    )?
                }
                Stmt::Namespace { name, body } => {
                    self.declare(name, Symbol::namespace(static_namespace_symbols(body)))?;
                }
                Stmt::Struct { name, .. } => {
                    if let Stmt::Struct { fields, .. } = statement {
                        self.declare(name, static_struct_symbol(fields))?;
                    }
                }
                Stmt::Enum { name, .. } => {
                    if let Stmt::Enum { variants, .. } = statement {
                        self.declare(name, static_enum_symbol(variants))?;
                    }
                }
                Stmt::TypeAlias { name, .. } => {
                    self.declare(name, Symbol::unknown(false))?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn check_statements(
        &mut self,
        statements: &[Stmt],
        mode: ScopeMode,
    ) -> NodiaResult<()> {
        for statement in statements {
            self.check_statement(statement, mode)?;
        }
        Ok(())
    }

    pub(super) fn check_statement(&mut self, statement: &Stmt, mode: ScopeMode) -> NodiaResult<()> {
        match statement {
            Stmt::Comment(_) => Ok(()),
            Stmt::Use {
                target,
                alias,
                pick,
                hide,
            } => {
                if mode != ScopeMode::Top {
                    self.declare_use(target, alias.as_deref(), pick, hide)?;
                }
                Ok(())
            }
            Stmt::Bind {
                name,
                value,
                mutable,
            } => {
                self.check_expr(value)?;
                let symbol = self.symbol_for_expr(value, *mutable);
                if mode == ScopeMode::Top {
                    self.update_symbol(name, symbol)?;
                } else {
                    self.declare(name, symbol)?;
                }
                Ok(())
            }
            Stmt::Assign { target, value } => {
                self.check_assign_target(target, true)?;
                self.check_expr(value)?;
                self.apply_assignment_symbol(target, value)
            }
            Stmt::Func { name, params, body } => {
                if mode != ScopeMode::Top {
                    let required = params.iter().filter(|p| p.default.is_none()).count();
                    let total = params.len();
                    self.declare(
                        name,
                        Symbol::function_arities(&(required..=total).collect::<Vec<_>>(), false),
                    )?;
                }
                self.check_function(params, body)
            }
            Stmt::Return(value) => {
                if self.function_depth == 0 {
                    return Err(self.error_keyword("E4103", "return outside function", "return"));
                }
                if let Some(value) = value {
                    self.check_expr(value)?;
                }
                Ok(())
            }
            Stmt::Throw(expr) => self.check_expr(expr),
            Stmt::Emit(expr) | Stmt::Expr(expr) => self.check_expr(expr),
            Stmt::Try {
                try_branch,
                catch_name,
                catch_branch,
            } => {
                self.check_block(try_branch)?;
                self.push_scope();
                self.declare(catch_name, recoverable_error_symbol())?;
                let result = self.check_block(catch_branch);
                self.pop_scope();
                result
            }
            Stmt::Match {
                value,
                arms,
                default,
            } => {
                self.check_expr(value)?;
                for arm in arms {
                    let mut bindings = HashSet::new();
                    self.validate_match_pattern(&arm.pattern, &mut bindings)?;
                    self.push_scope();
                    self.declare_match_pattern_bindings(&arm.pattern)?;
                    let result = self.check_block(&arm.body);
                    self.pop_scope();
                    result?;
                }
                let Some(default_body) = default else {
                    return Err(self.error_keyword(
                        "E4109",
                        "match requires a default arm",
                        "match",
                    ));
                };
                self.check_block(default_body)
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.check_expr(condition)?;
                self.check_block(then_branch)?;
                self.check_block(else_branch)
            }
            Stmt::For {
                binding,
                iterable,
                body,
            } => {
                self.check_expr(iterable)?;
                self.loop_depth += 1;
                self.push_scope();
                match binding {
                    ForBinding::Single(name) => self.declare(name, Symbol::unknown(true))?,
                    ForBinding::Pair { key, value } => {
                        self.declare(key, Symbol::unknown(true))?;
                        self.declare(value, Symbol::unknown(true))?;
                    }
                }
                let result = self.check_block(body);
                self.pop_scope();
                self.loop_depth -= 1;
                result
            }
            Stmt::While { condition, body } => {
                self.check_expr(condition)?;
                self.loop_depth += 1;
                let result = self.check_block(body);
                self.loop_depth -= 1;
                result
            }
            Stmt::Break => {
                if self.loop_depth == 0 {
                    return Err(self.error_keyword("E4103", "break outside loop", "break"));
                }
                Ok(())
            }
            Stmt::Continue => {
                if self.loop_depth == 0 {
                    return Err(self.error_keyword("E4103", "continue outside loop", "continue"));
                }
                Ok(())
            }
            Stmt::Namespace { name: _, body } => self.check_block(body),
            Stmt::Struct { name: _, fields } => {
                let mut seen = HashSet::new();
                for field in fields {
                    if !seen.insert(&field.name) {
                        return Err(self.error_name(
                            "E4102",
                            format!("duplicate struct field '{}'", field.name),
                            &field.name,
                        ));
                    }
                    if let Some(default) = &field.default {
                        self.check_expr(default)?;
                    }
                }
                Ok(())
            }
            Stmt::Enum { name: _, variants } => {
                let mut seen = HashSet::new();
                for variant in variants {
                    if !seen.insert(variant) {
                        return Err(self.error_name(
                            "E4102",
                            format!("duplicate enum variant '{variant}'"),
                            variant,
                        ));
                    }
                }
                Ok(())
            }
            Stmt::TypeAlias { name: _, target: _ } => Ok(()),
        }
    }

    pub(super) fn check_block(&mut self, statements: &[Stmt]) -> NodiaResult<()> {
        self.push_scope();
        let result = self.check_statements(statements, ScopeMode::Nested);
        self.pop_scope();
        result
    }

    pub(super) fn check_function(
        &mut self,
        params: &[FuncParam],
        body: &[Stmt],
    ) -> NodiaResult<()> {
        self.function_depth += 1;
        self.push_scope();
        let mut seen = HashSet::new();
        for param in params {
            if !seen.insert(&param.name) {
                return Err(self.error_name(
                    "E4102",
                    format!("duplicate parameter '{}'", param.name),
                    &param.name,
                ));
            }
            if let Some(default) = &param.default {
                self.check_expr(default)?;
            }
            self.declare(&param.name, Symbol::unknown(true))?;
        }
        let result = self.check_block(body);
        self.pop_scope();
        self.function_depth -= 1;
        result
    }

    fn validate_match_pattern(
        &self,
        pattern: &MatchPattern,
        bindings: &mut HashSet<String>,
    ) -> NodiaResult<()> {
        match pattern {
            MatchPattern::Wildcard | MatchPattern::Literal(_) => Ok(()),
            MatchPattern::Capture(name) => {
                if !bindings.insert(name.clone()) {
                    return Err(self.error_name(
                        "E4102",
                        format!("duplicate match binding '{name}'"),
                        name,
                    ));
                }
                Ok(())
            }
            MatchPattern::List(items) => {
                for item in items {
                    self.validate_match_pattern(item, bindings)?;
                }
                Ok(())
            }
            MatchPattern::Map(entries) => {
                let mut keys = HashSet::new();
                for (key, pattern) in entries {
                    if !keys.insert(key.clone()) {
                        return Err(semantic(
                            "E4109",
                            format!("duplicate match key '{key}'"),
                            None,
                        ));
                    }
                    self.validate_match_pattern(pattern, bindings)?;
                }
                Ok(())
            }
        }
    }

    fn declare_match_pattern_bindings(&mut self, pattern: &MatchPattern) -> NodiaResult<()> {
        match pattern {
            MatchPattern::Wildcard | MatchPattern::Literal(_) => Ok(()),
            MatchPattern::Capture(name) => self.declare(name, Symbol::unknown(false)),
            MatchPattern::List(items) => {
                for item in items {
                    self.declare_match_pattern_bindings(item)?;
                }
                Ok(())
            }
            MatchPattern::Map(entries) => {
                for (_, pattern) in entries {
                    self.declare_match_pattern_bindings(pattern)?;
                }
                Ok(())
            }
        }
    }
}
