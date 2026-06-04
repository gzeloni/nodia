use super::validation::validate_flag_delta;
use super::*;

pub(super) fn render_sequence(items: &[RegexNode]) -> DobraResult<String> {
    let mut out = String::new();
    for item in items {
        out.push_str(&render_node(item)?);
    }
    Ok(out)
}

pub(super) fn render_node(node: &RegexNode) -> DobraResult<String> {
    match node {
        RegexNode::Sequence(items) => render_sequence(items),
        RegexNode::Literal(value) => Ok(escape_regex_literal(value)),
        RegexNode::Raw(value) => Ok(value.clone()),
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
            let prefix = match kind {
                RegexGroupKind::Capture => "(",
                RegexGroupKind::NonCapture => "(?:",
                RegexGroupKind::Named(name) => {
                    return Ok(format!("(?<{name}>{})", render_sequence(body)?))
                }
                RegexGroupKind::Atomic => "(?>",
            };
            Ok(format!("{prefix}{})", render_sequence(body)?))
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

pub(super) fn render_char_set(set: &RegexCharSet) -> DobraResult<String> {
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

pub(super) fn render_char_set_item(item: &RegexCharSetItem) -> DobraResult<String> {
    match item {
        RegexCharSetItem::Char(ch) => Ok(escape_char_set_char(*ch)),
        RegexCharSetItem::Range(start, end) => Ok(format!(
            "{}-{}",
            escape_char_set_char(*start),
            escape_char_set_char(*end)
        )),
        RegexCharSetItem::Class(class) => Ok(class.render_in_set().to_string()),
        RegexCharSetItem::Raw(value) => Ok(value.clone()),
    }
}

pub(super) fn render_repeat_target(target: &RegexNode) -> DobraResult<String> {
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
        | RegexNode::Class(_)
        | RegexNode::AnyChar
        | RegexNode::AnyCodepoint
        | RegexNode::Quantifier { .. }
        | RegexNode::Group { .. }
        | RegexNode::Alternation(_)
        | RegexNode::CharSet(_)
        | RegexNode::Reference(_)
        | RegexNode::ScopedFlags { .. } => true,
        RegexNode::Sequence(_) | RegexNode::Raw(_) | RegexNode::Lookaround { .. } => false,
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
) -> DobraResult<String> {
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
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        other => other.to_string(),
    }
}
