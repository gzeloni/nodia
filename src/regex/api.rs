// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Public regex validation, rendering, and compilation helpers.

use super::rendering::*;
use super::support::*;
use super::validation::*;
use super::*;

/// Validates a regex AST for semantic correctness.
pub fn validate(pattern: &RegexPattern) -> DobraResult<()> {
    validate_flags(&pattern.flags, "regex")?;
    let mut named_groups = HashSet::new();
    validate_sequence(&pattern.body, &mut named_groups)
}

/// Validates a regex AST against a specific output target.
pub fn validate_for_target(pattern: &RegexPattern, target: RegexTarget) -> DobraResult<()> {
    validate(pattern)?;
    validate_target_sequence(&pattern.body, target)?;

    if pattern.flags.contains(&RegexFlag::Ungreedy)
        && matches!(
            target,
            RegexTarget::Javascript | RegexTarget::Python | RegexTarget::DotNet | RegexTarget::Re2
        )
    {
        return Err(regex_error(format!(
            "flag '{}' is not supported by {}",
            RegexFlag::Ungreedy.name(),
            target.name()
        )));
    }

    Ok(())
}

/// Renders a regex AST to classic regex text.
pub fn render(pattern: &RegexPattern) -> DobraResult<String> {
    render_for_target(pattern, RegexTarget::Classic)
}

/// Renders a regex AST for a specific target after validation.
pub fn render_for_target(pattern: &RegexPattern, target: RegexTarget) -> DobraResult<String> {
    validate_for_target(pattern, target)?;
    let mut out = String::new();
    if !pattern.flags.is_empty() {
        out.push_str(&render_global_flags(&pattern.flags));
    }
    out.push_str(&render_sequence(&pattern.body)?);
    Ok(out)
}

/// Compiles a validated regex AST into a runtime regex value.
pub fn compile(pattern: &RegexPattern) -> DobraResult<RuntimeRegex> {
    let rendered = render(pattern)?;
    compile_text(&rendered)
}

/// Compiles raw regex text into a runtime regex value.
pub fn compile_text(rendered: &str) -> DobraResult<RuntimeRegex> {
    let engine = Regex::new(rendered)
        .map_err(|err| DobraError::runtime(format!("cannot compile regex '{rendered}': {err}")))?;
    Ok(RuntimeRegex {
        rendered: rendered.to_string(),
        engine: Rc::new(engine),
    })
}
