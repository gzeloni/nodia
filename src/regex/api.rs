// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Public regex validation, rendering, and compilation helpers.

use super::rendering::*;
use super::support::*;
use super::validation::*;
use super::*;

/// Validates a regex AST for semantic correctness.
pub fn validate(pattern: &RegexPattern) -> NodiaResult<()> {
    validate_flags(&pattern.flags, "regex")?;
    let mut named_groups = HashSet::new();
    validate_sequence(&pattern.body, &mut named_groups)
}

/// Validates a regex AST against a specific output target.
pub fn validate_for_target(pattern: &RegexPattern, target: RegexTarget) -> NodiaResult<()> {
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
pub fn render(pattern: &RegexPattern) -> NodiaResult<String> {
    render_for_target(pattern, RegexTarget::Classic)
}

/// Renders a regex AST for a specific target after validation.
pub fn render_for_target(pattern: &RegexPattern, target: RegexTarget) -> NodiaResult<String> {
    validate_for_target(pattern, target)?;
    let mut out = String::new();
    if !pattern.flags.is_empty() {
        out.push_str(&render_global_flags(&pattern.flags));
    }
    out.push_str(&render_sequence(&pattern.body)?);
    Ok(out)
}

/// Compiles a validated regex AST into a runtime regex value.
pub fn compile(pattern: &RegexPattern) -> NodiaResult<RuntimeRegex> {
    let rendered = render(pattern)?;
    compile_text(&rendered)
}

/// Compiles raw regex text into a runtime regex value.
pub fn compile_text(rendered: &str) -> NodiaResult<RuntimeRegex> {
    let engine = Regex::new(rendered)
        .map_err(|err| NodiaError::runtime(format!("cannot compile regex '{rendered}': {err}")))?;
    Ok(RuntimeRegex {
        rendered: rendered.to_string(),
        engine: Rc::new(engine),
    })
}

/// Validates raw regex text using semantic regex diagnostics.
pub fn validate_text(rendered: &str) -> NodiaResult<()> {
    Regex::new(rendered)
        .map(|_| ())
        .map_err(|err| regex_error(format!("cannot compile regex '{rendered}': {err}")))
}

/// Validates replacement placeholder syntax without capture-shape checks.
pub fn validate_replacement_syntax(replacement: &str) -> NodiaResult<()> {
    parse_replacement_chunks(replacement)
        .map(|_| ())
        .map_err(regex_error)
}

/// Validates replacement placeholders against a regex pattern's capture contract.
pub fn validate_replacement(pattern: &RegexPattern, replacement: &str) -> NodiaResult<()> {
    validate(pattern)?;
    let chunks = parse_replacement_chunks(replacement).map_err(regex_error)?;
    let mut names = HashSet::new();
    let mut capture_len = 1usize;
    collect_capture_contract(&pattern.body, &mut capture_len, &mut names);
    validate_chunks_against_capture_contract(chunks, capture_len, &names)
}

fn validate_chunks_against_capture_contract(
    chunks: Vec<ReplacementChunk>,
    capture_len: usize,
    names: &HashSet<String>,
) -> NodiaResult<()> {
    for chunk in chunks {
        match chunk {
            ReplacementChunk::CaptureIndex { index, .. } if index >= capture_len => {
                return Err(regex_error(format!(
                    "regex replacement refers to missing capture group {index}"
                )));
            }
            ReplacementChunk::CaptureName(name) if !names.contains(&name) => {
                return Err(regex_error(format!(
                    "regex replacement refers to missing named capture '{name}'"
                )));
            }
            _ => {}
        }
    }

    Ok(())
}

fn collect_capture_contract(
    items: &[RegexNode],
    capture_len: &mut usize,
    names: &mut HashSet<String>,
) {
    for item in items {
        match item {
            RegexNode::Sequence(items) => collect_capture_contract(items, capture_len, names),
            RegexNode::Quantifier { target, .. } => {
                collect_capture_contract(std::slice::from_ref(target.as_ref()), capture_len, names);
            }
            RegexNode::Group { kind, body } => {
                match kind {
                    RegexGroupKind::Capture => *capture_len += 1,
                    RegexGroupKind::Named(name) => {
                        *capture_len += 1;
                        names.insert(name.clone());
                    }
                    RegexGroupKind::NonCapture | RegexGroupKind::Atomic => {}
                }
                collect_capture_contract(body, capture_len, names);
            }
            RegexNode::Alternation(branches) => {
                for branch in branches {
                    collect_capture_contract(branch, capture_len, names);
                }
            }
            RegexNode::Lookaround { body, .. } | RegexNode::ScopedFlags { body, .. } => {
                collect_capture_contract(body, capture_len, names);
            }
            RegexNode::Literal(_)
            | RegexNode::Raw(_)
            | RegexNode::Anchor(_)
            | RegexNode::Class(_)
            | RegexNode::AnyChar
            | RegexNode::AnyCodepoint
            | RegexNode::CharSet(_)
            | RegexNode::Reference(_) => {}
        }
    }
}
