// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Regex AST rendering to textual patterns.

use super::validation::validate_flag_delta;
use super::*;

pub(super) fn render_sequence(items: &[RegexNode]) -> NodiaResult<String> {
    let mut out = String::new();
    for item in items {
        out.push_str(&render_node(item)?);
    }
    Ok(out)
}

pub(super) fn render_node(node: &RegexNode) -> NodiaResult<String> {
    match node {
        RegexNode::Sequence(items) => render_sequence(items),
        RegexNode::Literal(value) => Ok(escape_regex_literal(value)),
        RegexNode::Raw(value) => Ok(value.clone()),
        RegexNode::Property { name, negated } => Ok(render_property(name, *negated)),
        RegexNode::Anchor(anchor) => Ok(anchor.render().to_string()),
        RegexNode::Class(class) => Ok(class.render().to_string()),
        RegexNode::AnyChar => Ok(".".to_string()),
        RegexNode::AnyCodepoint => Ok("[\\s\\S]".to_string()),
        RegexNode::Quantifier { target, kind, mode } => {
            let inner = render_repeat_target(target)?;
            Ok(format!(
                "{}{quantifier}{}",
                inner,
                mode.suffix(),
                quantifier = kind.suffix()
            ))
        }
        RegexNode::Group { kind, body } => {
            let rendered_body = render_group_body(body)?;
            let prefix = match kind {
                RegexGroupKind::Capture => "(",
                RegexGroupKind::NonCapture => "(?:",
                RegexGroupKind::Named(name) => return Ok(format!("(?<{name}>{rendered_body})")),
                RegexGroupKind::Atomic => "(?>",
            };
            Ok(format!("{prefix}{rendered_body})"))
        }
        RegexNode::Alternation(branches) => {
            let mut out = String::from("(?:");
            for (index, branch) in branches.iter().enumerate() {
                if index > 0 {
                    out.push('|');
                }
                out.push_str(&render_sequence(branch)?);
            }
            out.push(')');
            Ok(out)
        }
        RegexNode::CharSet(set) => render_char_set(set),
        RegexNode::Lookaround { kind, body } => {
            Ok(format!("{}{})", kind.prefix(), render_sequence(body)?))
        }
        RegexNode::Reference(reference) => match reference {
            RegexReference::Named(name) => Ok(format!("\\k<{name}>")),
            RegexReference::Group(index) => Ok(format!("\\{index}")),
        },
        RegexNode::Condition(condition) => {
            let mut out = String::from("(?(");
            out.push_str(&render_condition(condition)?);
            out.push_str("))");
            Ok(out)
        }
        RegexNode::Conditional {
            condition,
            then_branch,
            else_branch,
        } => render_conditional(condition, then_branch, else_branch),
        RegexNode::SubroutineCall(reference) => match reference {
            RegexReference::Named(name) => Ok(format!("\\g<{name}>")),
            RegexReference::Group(index) => Ok(format!("\\g<{index}>")),
        },
        RegexNode::BacktrackingVerb(verb) => Ok(verb.render().to_string()),
        RegexNode::Until { limit, body } => {
            let limit = render_sequence(limit)?;
            if let Some(body) = body {
                return Ok(format!("(?~|{limit}|{})", render_sequence(body)?));
            }
            Ok(format!("(?~{limit})"))
        }
        RegexNode::UntilStop(limit) => Ok(format!("(?~|{})", render_sequence(limit)?)),
        RegexNode::UntilClear => Ok("(?~|)".to_string()),
        RegexNode::DefineGroup { body } => Ok(format!("(?(DEFINE){})", render_sequence(body)?)),
        RegexNode::ScopedFlags {
            enable,
            disable,
            body,
        } => Ok(format!(
            "{}{})",
            render_scoped_flag_prefix(enable, disable)?,
            render_sequence(body)?
        )),
    }
}

pub(super) fn render_char_set(set: &RegexCharSet) -> NodiaResult<String> {
    let mut out = String::from("[");
    if set.negated {
        out.push('^');
    }
    for item in &set.items {
        out.push_str(&render_char_set_item(item)?);
    }
    out.push(']');
    Ok(out)
}

fn render_group_body(items: &[RegexNode]) -> NodiaResult<String> {
    if let [RegexNode::Alternation(branches)] = items {
        let mut out = String::new();
        for (index, branch) in branches.iter().enumerate() {
            if index > 0 {
                out.push('|');
            }
            out.push_str(&render_sequence(branch)?);
        }
        Ok(out)
    } else {
        render_sequence(items)
    }
}

pub(super) fn render_char_set_item(item: &RegexCharSetItem) -> NodiaResult<String> {
    match item {
        RegexCharSetItem::Char(ch) => Ok(escape_char_set_char(*ch)),
        RegexCharSetItem::Range(start, end) => Ok(format!(
            "{}-{}",
            escape_char_set_char(*start),
            escape_char_set_char(*end)
        )),
        RegexCharSetItem::Class(class) => Ok(class.render_in_set().to_string()),
        RegexCharSetItem::Property { name, negated } => Ok(render_property(name, *negated)),
        RegexCharSetItem::Raw(value) => Ok(value.clone()),
    }
}

pub(super) fn render_repeat_target(target: &RegexNode) -> NodiaResult<String> {
    let rendered = render_node(target)?;
    if repeat_target_is_atomic(target) {
        Ok(rendered)
    } else {
        Ok(format!("(?:{rendered})"))
    }
}

pub(super) fn repeat_target_is_atomic(target: &RegexNode) -> bool {
    match target {
        RegexNode::Literal(value) => value.chars().count() == 1,
        RegexNode::Anchor(_)
        | RegexNode::Property { .. }
        | RegexNode::Class(_)
        | RegexNode::AnyChar
        | RegexNode::AnyCodepoint
        | RegexNode::Quantifier { .. }
        | RegexNode::Group { .. }
        | RegexNode::Alternation(_)
        | RegexNode::CharSet(_)
        | RegexNode::Reference(_)
        | RegexNode::Condition(_)
        | RegexNode::Conditional { .. }
        | RegexNode::SubroutineCall(_)
        | RegexNode::BacktrackingVerb(_)
        | RegexNode::Until { .. }
        | RegexNode::UntilStop(_)
        | RegexNode::UntilClear
        | RegexNode::DefineGroup { .. }
        | RegexNode::ScopedFlags { .. } => true,
        RegexNode::Sequence(_) | RegexNode::Raw(_) | RegexNode::Lookaround { .. } => false,
    }
}

fn render_conditional(
    condition: &RegexCondition,
    then_branch: &[RegexNode],
    else_branch: &[RegexNode],
) -> NodiaResult<String> {
    if let RegexCondition::Lookaround { kind, body } = condition {
        return render_assertion_conditional(*kind, body, then_branch, else_branch);
    }

    let mut out = String::from("(?(");
    out.push_str(&render_condition(condition)?);
    out.push(')');
    out.push_str(&render_sequence(then_branch)?);
    if !else_branch.is_empty() {
        out.push('|');
        out.push_str(&render_sequence(else_branch)?);
    }
    out.push(')');
    Ok(out)
}

fn render_condition(condition: &RegexCondition) -> NodiaResult<String> {
    match condition {
        RegexCondition::Capture(RegexReference::Named(name)) => Ok(format!("<{name}>")),
        RegexCondition::Capture(RegexReference::Group(index)) => Ok(index.to_string()),
        RegexCondition::Expression(body) => render_sequence(body),
        RegexCondition::Lookaround { .. } => {
            unreachable!("lookaround condition is handled earlier")
        }
    }
}

fn render_assertion_conditional(
    kind: RegexLookaroundKind,
    condition_body: &[RegexNode],
    then_branch: &[RegexNode],
    else_branch: &[RegexNode],
) -> NodiaResult<String> {
    let positive = format!("{}{})", kind.prefix(), render_sequence(condition_body)?);
    let then_rendered = render_sequence(then_branch)?;

    if else_branch.is_empty() {
        return Ok(format!("{positive}{then_rendered}"));
    }

    let negative = format!(
        "{}{})",
        negate_lookaround_kind(kind).prefix(),
        render_sequence(condition_body)?
    );
    let else_rendered = render_sequence(else_branch)?;
    Ok(format!(
        "(?:{positive}{then_rendered}|{negative}{else_rendered})"
    ))
}

fn negate_lookaround_kind(kind: RegexLookaroundKind) -> RegexLookaroundKind {
    match kind {
        RegexLookaroundKind::FollowedBy => RegexLookaroundKind::NotFollowedBy,
        RegexLookaroundKind::NotFollowedBy => RegexLookaroundKind::FollowedBy,
        RegexLookaroundKind::PrecededBy => RegexLookaroundKind::NotPrecededBy,
        RegexLookaroundKind::NotPrecededBy => RegexLookaroundKind::PrecededBy,
    }
}

fn render_property(name: &str, negated: bool) -> String {
    if negated {
        format!(r"\P{{{name}}}")
    } else {
        format!(r"\p{{{name}}}")
    }
}

pub(super) fn render_global_flags(flags: &[RegexFlag]) -> String {
    let mut out = String::from("(?");
    for flag in flags {
        out.push(flag.code());
    }
    out.push(')');
    out
}

pub(super) fn render_scoped_flag_prefix(
    enable: &[RegexFlag],
    disable: &[RegexFlag],
) -> NodiaResult<String> {
    validate_flag_delta(enable, disable)?;

    let mut out = String::from("(?");
    for flag in enable {
        out.push(flag.code());
    }
    if !disable.is_empty() {
        out.push('-');
        for flag in disable {
            out.push(flag.code());
        }
    }
    out.push(':');
    Ok(out)
}

pub(super) fn escape_regex_literal(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' | '.' | '^' | '$' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | ']' | '{' | '}' => {
                out.push('\\');
                out.push(ch);
            }
            '\u{0007}' => out.push_str("\\a"),
            '\u{0008}' => out.push_str("\\x08"),
            '\u{000c}' => out.push_str("\\f"),
            '\u{000b}' => out.push_str("\\v"),
            '\u{001b}' => out.push_str("\\e"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

pub(super) fn escape_char_set_char(ch: char) -> String {
    match ch {
        '\\' | '[' | ']' | '^' | '-' => format!("\\{ch}"),
        '\u{0007}' => "\\a".to_string(),
        '\u{0008}' => "\\x08".to_string(),
        '\u{000c}' => "\\f".to_string(),
        '\u{000b}' => "\\v".to_string(),
        '\u{001b}' => "\\e".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        other => other.to_string(),
    }
}
