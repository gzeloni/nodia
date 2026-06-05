// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Semantic validation for regex AST nodes.

use super::support::*;
use super::*;

pub(super) fn validate_sequence(
    items: &[RegexNode],
    named_groups: &mut HashSet<String>,
) -> NodiaResult<()> {
    for item in items {
        validate_node(item, named_groups)?;
    }
    Ok(())
}

pub(super) fn validate_node(
    node: &RegexNode,
    named_groups: &mut HashSet<String>,
) -> NodiaResult<()> {
    match node {
        RegexNode::Sequence(items) => {
            if items.is_empty() {
                return Err(regex_error("regex block target cannot be empty"));
            }
            validate_sequence(items, named_groups)
        }
        RegexNode::Literal(_)
        | RegexNode::Raw(_)
        | RegexNode::Anchor(_)
        | RegexNode::Class(_)
        | RegexNode::AnyChar
        | RegexNode::AnyCodepoint => Ok(()),
        RegexNode::Reference(reference) => validate_reference(reference),
        RegexNode::Quantifier { target, kind, .. } => {
            validate_node(target, named_groups)?;
            validate_quantifier(*kind)?;
            validate_repeat_target(target)
        }
        RegexNode::Group {
            kind: RegexGroupKind::Named(name),
            body,
        } => {
            if !named_groups.insert(name.clone()) {
                return Err(regex_error(format!("duplicate named capture '{name}'")));
            }
            validate_sequence(body, named_groups)
        }
        RegexNode::Group { body, .. } | RegexNode::Lookaround { body, .. } => {
            validate_sequence(body, named_groups)
        }
        RegexNode::Alternation(branches) => {
            if branches.is_empty() {
                return Err(regex_error("either block requires at least one branch"));
            }
            for branch in branches {
                validate_sequence(branch, named_groups)?;
            }
            Ok(())
        }
        RegexNode::CharSet(set) => validate_char_set(set),
        RegexNode::ScopedFlags {
            enable,
            disable,
            body,
        } => {
            validate_flag_delta(enable, disable)?;
            validate_sequence(body, named_groups)
        }
    }
}

pub(super) fn validate_target_sequence(
    items: &[RegexNode],
    target: RegexTarget,
) -> NodiaResult<()> {
    for item in items {
        validate_target_node(item, target)?;
    }
    Ok(())
}

pub(super) fn validate_target_node(node: &RegexNode, target: RegexTarget) -> NodiaResult<()> {
    match node {
        RegexNode::Group {
            kind: RegexGroupKind::Atomic,
            body,
        } => {
            if matches!(target, RegexTarget::Javascript | RegexTarget::Re2) {
                return Err(regex_error(format!(
                    "atomic groups are not supported by {}",
                    target.name()
                )));
            }
            validate_target_sequence(body, target)
        }
        RegexNode::Group { body, .. } => validate_target_sequence(body, target),
        RegexNode::Lookaround { kind, body } => {
            if target == RegexTarget::Re2 {
                return Err(regex_error(format!(
                    "{} is not supported by {}",
                    kind.name(),
                    target.name()
                )));
            }
            validate_target_sequence(body, target)
        }
        RegexNode::ScopedFlags { body, .. } => {
            if matches!(target, RegexTarget::Javascript | RegexTarget::Re2) {
                return Err(regex_error(format!(
                    "scoped flags are not supported by {}",
                    target.name()
                )));
            }
            validate_target_sequence(body, target)
        }
        RegexNode::Sequence(items) => validate_target_sequence(items, target),
        RegexNode::Alternation(branches) => {
            for branch in branches {
                validate_target_sequence(branch, target)?;
            }
            Ok(())
        }
        RegexNode::Quantifier {
            target: inner,
            mode,
            ..
        } => {
            if *mode == RegexQuantifierMode::Possessive
                && matches!(target, RegexTarget::Javascript | RegexTarget::Re2)
            {
                return Err(regex_error(format!(
                    "possessive quantifiers are not supported by {}",
                    target.name()
                )));
            }
            validate_target_node(inner, target)
        }
        RegexNode::Reference(_) if target == RegexTarget::Re2 => Err(regex_error(format!(
            "backreferences are not supported by {}",
            target.name()
        ))),
        RegexNode::CharSet(_) => Ok(()),
        _ => Ok(()),
    }
}

pub(super) fn validate_flags(flags: &[RegexFlag], context: &str) -> NodiaResult<()> {
    let mut seen = HashSet::new();
    for flag in flags {
        if !seen.insert(*flag) {
            return Err(regex_error(format!(
                "duplicate {context} flag '{}'",
                flag.name()
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_flag_delta(enable: &[RegexFlag], disable: &[RegexFlag]) -> NodiaResult<()> {
    validate_flags(enable, "with_flags")?;
    validate_flags(disable, "without_flags")?;
    if enable.is_empty() && disable.is_empty() {
        return Err(regex_error(
            "scoped flags require at least one enabled or disabled flag",
        ));
    }
    for flag in enable {
        if disable.contains(flag) {
            return Err(regex_error(format!(
                "flag '{}' cannot be enabled and disabled in the same scope",
                flag.name()
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_reference(reference: &RegexReference) -> NodiaResult<()> {
    if matches!(reference, RegexReference::Group(0)) {
        Err(regex_error("same_as_group expects an index starting at 1"))
    } else {
        Ok(())
    }
}

pub(super) fn validate_quantifier(kind: RegexQuantifierKind) -> NodiaResult<()> {
    if let RegexQuantifierKind::Between(min, max) = kind {
        if min > max {
            return Err(regex_error(
                "between minimum cannot be greater than maximum",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_repeat_target(target: &RegexNode) -> NodiaResult<()> {
    match target {
        RegexNode::Anchor(anchor) => Err(regex_error(format!(
            "'{}' cannot be quantified",
            anchor.name()
        ))),
        RegexNode::Lookaround { kind, .. } => Err(regex_error(format!(
            "'{}' cannot be quantified",
            kind.name()
        ))),
        RegexNode::Literal(value) if value.is_empty() => {
            Err(regex_error("empty regex literal cannot be quantified"))
        }
        _ => Ok(()),
    }
}

pub(super) fn validate_char_set(set: &RegexCharSet) -> NodiaResult<()> {
    if set.items.is_empty() {
        return Err(regex_error("char_set requires at least one item"));
    }
    for item in &set.items {
        match item {
            RegexCharSetItem::Char(_) | RegexCharSetItem::Raw(_) | RegexCharSetItem::Class(_) => {}
            RegexCharSetItem::Range(start, end) => {
                if start > end {
                    return Err(regex_error(
                        "char_set range start cannot be greater than end",
                    ));
                }
            }
        }
    }
    Ok(())
}
