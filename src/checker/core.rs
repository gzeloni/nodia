// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Core checker orchestration and module-loading support.

use super::helpers::*;
use super::*;

impl Checker {
    pub(super) fn new() -> Self {
        Self {
            modules: HashMap::new(),
            loading: HashSet::new(),
        }
    }

    pub(super) fn check_program(
        &mut self,
        program: &Program,
        base_dir: Option<PathBuf>,
        positions: PositionIndex,
    ) -> NodiaResult<()> {
        let mut state = State::new(self, base_dir, positions);
        state.predeclare_top_level(program)?;
        state.check_statements(&program.statements, ScopeMode::Top)
    }

    pub(super) fn load_module(
        &mut self,
        path: &str,
        base_dir: Option<&Path>,
    ) -> NodiaResult<ModuleInfo> {
        let resolved = resolve_use(path, base_dir)?;
        if let Some(info) = self.modules.get(&resolved) {
            return Ok(info.clone());
        }

        let source = fs::read_to_string(&resolved).map_err(|err| {
            NodiaError::io(format!("cannot read use '{}': {err}", resolved.display()))
        })?;
        let tokens = Lexer::new(&source)
            .tokenize()
            .map_err(|err| err.with_file(resolved.display().to_string()))?;
        let positions = PositionIndex::from_tokens(&tokens);
        let program = Parser::new(tokens)
            .parse_program()
            .map_err(|err| err.with_file(resolved.display().to_string()))?;

        let info = ModuleInfo {
            symbols: declared_exports(&program),
        };
        self.modules.insert(resolved.clone(), info.clone());

        if self.loading.insert(resolved.clone()) {
            let result = self
                .check_program(
                    &program,
                    resolved.parent().map(Path::to_path_buf),
                    positions,
                )
                .map_err(|err| err.with_file_if_missing(resolved.display().to_string()));
            self.loading.remove(&resolved);
            result?;
        }

        Ok(info)
    }
}
