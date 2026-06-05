// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Statement-level semantic checks and scope management.

use super::*;

impl<'a> State<'a> {
    pub(super) fn new(
        checker: &'a mut Checker,
        base_dir: Option<PathBuf>,
        positions: PositionIndex,
    ) -> Self {
        let mut root = HashMap::new();
        root.insert("input".to_string(), Symbol::unknown(false));
        Self {
            checker,
            scopes: vec![root],
            base_dir,
            positions,
            loop_depth: 0,
            function_depth: 0,
        }
    }

    pub(super) fn predeclare_top_level(&mut self, program: &Program) -> DobraResult<()> {
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
                    self.declare(name, Symbol::function(params.len(), false))?
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
    ) -> DobraResult<()> {
        for statement in statements {
            self.check_statement(statement, mode)?;
        }
        Ok(())
    }

    pub(super) fn check_statement(&mut self, statement: &Stmt, mode: ScopeMode) -> DobraResult<()> {
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
                    self.declare(name, Symbol::function(params.len(), false))?;
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
            Stmt::Emit(expr) | Stmt::Expr(expr) => self.check_expr(expr),
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
        }
    }

    pub(super) fn check_block(&mut self, statements: &[Stmt]) -> DobraResult<()> {
        self.push_scope();
        let result = self.check_statements(statements, ScopeMode::Nested);
        self.pop_scope();
        result
    }

    pub(super) fn check_function(&mut self, params: &[String], body: &[Stmt]) -> DobraResult<()> {
        self.function_depth += 1;
        self.push_scope();
        let mut seen = HashSet::new();
        for param in params {
            if !seen.insert(param) {
                return Err(self.error_name(
                    "E4102",
                    format!("duplicate parameter '{param}'"),
                    param,
                ));
            }
            self.declare(param, Symbol::unknown(true))?;
        }
        let result = self.check_block(body);
        self.pop_scope();
        self.function_depth -= 1;
        result
    }
}
