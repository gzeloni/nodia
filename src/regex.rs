use crate::error::{DobraError, DobraResult};
use fancy_regex::{Captures, Regex};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct RegexPattern {
    pub flags: Vec<RegexFlag>,
    pub body: Vec<RegexNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegexFlag {
    CaseInsensitive,
    Multiline,
    DotAll,
    Unicode,
    IgnoreWhitespace,
    Ungreedy,
}

impl RegexFlag {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "case_insensitive" => Self::CaseInsensitive,
            "multiline" => Self::Multiline,
            "dot_all" => Self::DotAll,
            "unicode" => Self::Unicode,
            "ignore_whitespace" => Self::IgnoreWhitespace,
            "ungreedy" => Self::Ungreedy,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::CaseInsensitive => "case_insensitive",
            Self::Multiline => "multiline",
            Self::DotAll => "dot_all",
            Self::Unicode => "unicode",
            Self::IgnoreWhitespace => "ignore_whitespace",
            Self::Ungreedy => "ungreedy",
        }
    }

    fn code(self) -> char {
        match self {
            Self::CaseInsensitive => 'i',
            Self::Multiline => 'm',
            Self::DotAll => 's',
            Self::Unicode => 'u',
            Self::IgnoreWhitespace => 'x',
            Self::Ungreedy => 'U',
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegexNode {
    Sequence(Vec<RegexNode>),
    Literal(String),
    Raw(String),
    Anchor(RegexAnchor),
    Class(RegexClass),
    AnyChar,
    AnyCodepoint,
    Quantifier {
        target: Box<RegexNode>,
        kind: RegexQuantifierKind,
        mode: RegexQuantifierMode,
    },
    Group {
        kind: RegexGroupKind,
        body: Vec<RegexNode>,
    },
    Alternation(Vec<Vec<RegexNode>>),
    CharSet(RegexCharSet),
    Lookaround {
        kind: RegexLookaroundKind,
        body: Vec<RegexNode>,
    },
    Reference(RegexReference),
    ScopedFlags {
        enable: Vec<RegexFlag>,
        disable: Vec<RegexFlag>,
        body: Vec<RegexNode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegexAnchor {
    Start,
    End,
    WordBoundary,
    NotWordBoundary,
}

impl RegexAnchor {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "start" => Self::Start,
            "end" => Self::End,
            "word_boundary" => Self::WordBoundary,
            "not_word_boundary" => Self::NotWordBoundary,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::End => "end",
            Self::WordBoundary => "word_boundary",
            Self::NotWordBoundary => "not_word_boundary",
        }
    }

    fn render(self) -> &'static str {
        match self {
            Self::Start => "^",
            Self::End => "$",
            Self::WordBoundary => "\\b",
            Self::NotWordBoundary => "\\B",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegexClass {
    Digit,
    NotDigit,
    Whitespace,
    NotWhitespace,
    WordChar,
    NotWordChar,
    Letter,
    Lowercase,
    Uppercase,
    HexDigit,
    Alnum,
    Space,
    Tab,
    Newline,
}

impl RegexClass {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "digit" => Self::Digit,
            "not_digit" => Self::NotDigit,
            "whitespace" => Self::Whitespace,
            "not_whitespace" => Self::NotWhitespace,
            "word_char" => Self::WordChar,
            "not_word_char" => Self::NotWordChar,
            "letter" => Self::Letter,
            "lowercase" => Self::Lowercase,
            "uppercase" => Self::Uppercase,
            "hex_digit" => Self::HexDigit,
            "alnum" => Self::Alnum,
            "space" => Self::Space,
            "tab" => Self::Tab,
            "newline" => Self::Newline,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Digit => "digit",
            Self::NotDigit => "not_digit",
            Self::Whitespace => "whitespace",
            Self::NotWhitespace => "not_whitespace",
            Self::WordChar => "word_char",
            Self::NotWordChar => "not_word_char",
            Self::Letter => "letter",
            Self::Lowercase => "lowercase",
            Self::Uppercase => "uppercase",
            Self::HexDigit => "hex_digit",
            Self::Alnum => "alnum",
            Self::Space => "space",
            Self::Tab => "tab",
            Self::Newline => "newline",
        }
    }

    fn render(self) -> &'static str {
        match self {
            Self::Digit => "\\d",
            Self::NotDigit => "\\D",
            Self::Whitespace => "\\s",
            Self::NotWhitespace => "\\S",
            Self::WordChar => "\\w",
            Self::NotWordChar => "\\W",
            Self::Letter => "[A-Za-z]",
            Self::Lowercase => "[a-z]",
            Self::Uppercase => "[A-Z]",
            Self::HexDigit => "[0-9A-Fa-f]",
            Self::Alnum => "[A-Za-z0-9]",
            Self::Space => " ",
            Self::Tab => "\\t",
            Self::Newline => "\\n",
        }
    }

    fn render_in_set(self) -> &'static str {
        match self {
            Self::Digit => "0-9",
            Self::NotDigit => "\\D",
            Self::Whitespace => "\\s",
            Self::NotWhitespace => "\\S",
            Self::WordChar => "\\w",
            Self::NotWordChar => "\\W",
            Self::Letter => "A-Za-z",
            Self::Lowercase => "a-z",
            Self::Uppercase => "A-Z",
            Self::HexDigit => "0-9A-Fa-f",
            Self::Alnum => "A-Za-z0-9",
            Self::Space => " ",
            Self::Tab => "\\t",
            Self::Newline => "\\n",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexQuantifierMode {
    Greedy,
    Lazy,
    Possessive,
}

impl RegexQuantifierMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Greedy => "greedy",
            Self::Lazy => "lazy",
            Self::Possessive => "possessive",
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Greedy => "",
            Self::Lazy => "?",
            Self::Possessive => "+",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexQuantifierKind {
    Optional,
    ZeroOrMore,
    OneOrMore,
    Exactly(usize),
    AtLeast(usize),
    Between(usize, usize),
}

impl RegexQuantifierKind {
    pub fn format_keyword(self) -> String {
        match self {
            Self::Optional => "optional".to_string(),
            Self::ZeroOrMore => "zero_or_more".to_string(),
            Self::OneOrMore => "one_or_more".to_string(),
            Self::Exactly(count) => format!("exactly {count}"),
            Self::AtLeast(count) => format!("at_least {count}"),
            Self::Between(min, max) => format!("between {min} and {max}"),
        }
    }

    fn suffix(self) -> String {
        match self {
            Self::Optional => "?".to_string(),
            Self::ZeroOrMore => "*".to_string(),
            Self::OneOrMore => "+".to_string(),
            Self::Exactly(count) => format!("{{{count}}}"),
            Self::AtLeast(count) => format!("{{{count},}}"),
            Self::Between(min, max) => format!("{{{min},{max}}}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegexGroupKind {
    Capture,
    NonCapture,
    Named(String),
    Atomic,
}

impl RegexGroupKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Capture => "group",
            Self::NonCapture => "non_capture",
            Self::Named(_) => "named",
            Self::Atomic => "atomic",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexLookaroundKind {
    FollowedBy,
    NotFollowedBy,
    PrecededBy,
    NotPrecededBy,
}

impl RegexLookaroundKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::FollowedBy => "followed_by",
            Self::NotFollowedBy => "not_followed_by",
            Self::PrecededBy => "preceded_by",
            Self::NotPrecededBy => "not_preceded_by",
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::FollowedBy => "(?=",
            Self::NotFollowedBy => "(?!",
            Self::PrecededBy => "(?<=",
            Self::NotPrecededBy => "(?<!",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegexCharSet {
    pub negated: bool,
    pub items: Vec<RegexCharSetItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegexCharSetItem {
    Char(char),
    Range(char, char),
    Class(RegexClass),
    Raw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegexReference {
    Named(String),
    Group(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexTarget {
    Classic,
    Pcre,
    Javascript,
    Python,
    DotNet,
    Re2,
}

impl RegexTarget {
    pub fn name(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Pcre => "pcre",
            Self::Javascript => "javascript",
            Self::Python => "python",
            Self::DotNet => "dotnet",
            Self::Re2 => "re2",
        }
    }
}

#[derive(Clone)]
pub struct RuntimeRegex {
    rendered: String,
    engine: Rc<Regex>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexMatch {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub groups: Vec<Option<String>>,
    pub named: BTreeMap<String, Option<String>>,
}

impl RuntimeRegex {
    pub fn rendered(&self) -> &str {
        &self.rendered
    }

    pub fn is_match(&self, text: &str) -> DobraResult<bool> {
        self.engine.is_match(text).map_err(regex_engine_error)
    }

    pub fn is_full_match(&self, text: &str) -> DobraResult<bool> {
        let captures = self.engine.captures(text).map_err(regex_engine_error)?;
        Ok(captures
            .and_then(|captures| captures.get(0))
            .is_some_and(|matched| matched.start() == 0 && matched.end() == text.len()))
    }

    pub fn find(&self, text: &str) -> DobraResult<Option<RegexMatch>> {
        let captures = self.engine.captures(text).map_err(regex_engine_error)?;
        captures
            .map(|captures| self.capture_to_match(text, &captures))
            .transpose()
    }

    pub fn find_all(&self, text: &str) -> DobraResult<Vec<RegexMatch>> {
        let mut matches = Vec::new();
        for captures in self.engine.captures_iter(text) {
            let captures = captures.map_err(regex_engine_error)?;
            matches.push(self.capture_to_match(text, &captures)?);
        }
        Ok(matches)
    }

    fn capture_to_match(&self, text: &str, captures: &Captures<'_>) -> DobraResult<RegexMatch> {
        let matched = captures
            .get(0)
            .ok_or_else(|| DobraError::runtime("regex engine returned a match without group 0"))?;
        let groups = (1..captures.len())
            .map(|index| captures.get(index).map(|value| value.as_str().to_string()))
            .collect();
        let mut named = BTreeMap::new();
        for name in self.engine.capture_names().flatten() {
            named
                .entry(name.to_string())
                .or_insert_with(|| captures.name(name).map(|value| value.as_str().to_string()));
        }
        Ok(RegexMatch {
            text: matched.as_str().to_string(),
            start: char_offset(text, matched.start()),
            end: char_offset(text, matched.end()),
            groups,
            named,
        })
    }
}

impl PartialEq for RuntimeRegex {
    fn eq(&self, other: &Self) -> bool {
        self.rendered == other.rendered
    }
}

impl fmt::Debug for RuntimeRegex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RuntimeRegex")
            .field("rendered", &self.rendered)
            .finish()
    }
}

pub fn validate(pattern: &RegexPattern) -> DobraResult<()> {
    validate_flags(&pattern.flags, "regex")?;
    validate_sequence(&pattern.body)
}

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

pub fn render(pattern: &RegexPattern) -> DobraResult<String> {
    render_for_target(pattern, RegexTarget::Classic)
}

pub fn render_for_target(pattern: &RegexPattern, target: RegexTarget) -> DobraResult<String> {
    validate_for_target(pattern, target)?;
    let mut out = String::new();
    if !pattern.flags.is_empty() {
        out.push_str(&render_global_flags(&pattern.flags));
    }
    out.push_str(&render_sequence(&pattern.body)?);
    Ok(out)
}

pub fn compile(pattern: &RegexPattern) -> DobraResult<RuntimeRegex> {
    let rendered = render(pattern)?;
    compile_text(&rendered)
}

pub fn compile_text(rendered: &str) -> DobraResult<RuntimeRegex> {
    let engine = Regex::new(rendered)
        .map_err(|err| DobraError::runtime(format!("cannot compile regex '{rendered}': {err}")))?;
    Ok(RuntimeRegex {
        rendered: rendered.to_string(),
        engine: Rc::new(engine),
    })
}

fn validate_sequence(items: &[RegexNode]) -> DobraResult<()> {
    for item in items {
        validate_node(item)?;
    }
    Ok(())
}

fn validate_node(node: &RegexNode) -> DobraResult<()> {
    match node {
        RegexNode::Sequence(items) => {
            if items.is_empty() {
                return Err(regex_error("regex block target cannot be empty"));
            }
            validate_sequence(items)
        }
        RegexNode::Literal(_)
        | RegexNode::Raw(_)
        | RegexNode::Anchor(_)
        | RegexNode::Class(_)
        | RegexNode::AnyChar
        | RegexNode::AnyCodepoint => Ok(()),
        RegexNode::Reference(reference) => validate_reference(reference),
        RegexNode::Quantifier { target, kind, .. } => {
            validate_node(target)?;
            validate_quantifier(*kind)?;
            validate_repeat_target(target)
        }
        RegexNode::Group { body, .. } | RegexNode::Lookaround { body, .. } => {
            validate_sequence(body)
        }
        RegexNode::Alternation(branches) => {
            if branches.is_empty() {
                return Err(regex_error("either block requires at least one branch"));
            }
            for branch in branches {
                validate_sequence(branch)?;
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
            validate_sequence(body)
        }
    }
}

fn validate_target_sequence(items: &[RegexNode], target: RegexTarget) -> DobraResult<()> {
    for item in items {
        validate_target_node(item, target)?;
    }
    Ok(())
}

fn validate_target_node(node: &RegexNode, target: RegexTarget) -> DobraResult<()> {
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

fn validate_flags(flags: &[RegexFlag], context: &str) -> DobraResult<()> {
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

fn validate_flag_delta(enable: &[RegexFlag], disable: &[RegexFlag]) -> DobraResult<()> {
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

fn validate_reference(reference: &RegexReference) -> DobraResult<()> {
    if matches!(reference, RegexReference::Group(0)) {
        Err(regex_error("same_as_group expects an index starting at 1"))
    } else {
        Ok(())
    }
}

fn validate_quantifier(kind: RegexQuantifierKind) -> DobraResult<()> {
    if let RegexQuantifierKind::Between(min, max) = kind {
        if min > max {
            return Err(regex_error(
                "between minimum cannot be greater than maximum",
            ));
        }
    }
    Ok(())
}

fn validate_repeat_target(target: &RegexNode) -> DobraResult<()> {
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

fn validate_char_set(set: &RegexCharSet) -> DobraResult<()> {
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

fn render_sequence(items: &[RegexNode]) -> DobraResult<String> {
    let mut out = String::new();
    for item in items {
        out.push_str(&render_node(item)?);
    }
    Ok(out)
}

fn render_node(node: &RegexNode) -> DobraResult<String> {
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

fn render_char_set(set: &RegexCharSet) -> DobraResult<String> {
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

fn render_char_set_item(item: &RegexCharSetItem) -> DobraResult<String> {
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

fn render_repeat_target(target: &RegexNode) -> DobraResult<String> {
    let rendered = render_node(target)?;
    if repeat_target_is_atomic(target) {
        Ok(rendered)
    } else {
        Ok(format!("(?:{rendered})"))
    }
}

fn repeat_target_is_atomic(target: &RegexNode) -> bool {
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

fn render_global_flags(flags: &[RegexFlag]) -> String {
    let mut out = String::from("(?");
    for flag in flags {
        out.push(flag.code());
    }
    out.push(')');
    out
}

fn render_scoped_flag_prefix(enable: &[RegexFlag], disable: &[RegexFlag]) -> DobraResult<String> {
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

fn escape_regex_literal(value: &str) -> String {
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

fn escape_char_set_char(ch: char) -> String {
    match ch {
        '\\' | '[' | ']' | '^' | '-' => format!("\\{ch}"),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        other => other.to_string(),
    }
}

fn char_offset(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset].chars().count()
}

fn regex_engine_error(err: fancy_regex::Error) -> DobraError {
    DobraError::runtime(format!("regex engine error: {err}"))
}

fn regex_error(message: impl Into<String>) -> DobraError {
    DobraError::semantic(message).with_code("E4200")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_regex_with_groups_sets_and_lookarounds() {
        let pattern = RegexPattern {
            flags: vec![RegexFlag::CaseInsensitive, RegexFlag::Multiline],
            body: vec![
                RegexNode::Anchor(RegexAnchor::Start),
                RegexNode::Group {
                    kind: RegexGroupKind::Named("year".to_string()),
                    body: vec![RegexNode::Quantifier {
                        target: Box::new(RegexNode::Class(RegexClass::Digit)),
                        kind: RegexQuantifierKind::Exactly(4),
                        mode: RegexQuantifierMode::Greedy,
                    }],
                },
                RegexNode::Literal("-".to_string()),
                RegexNode::CharSet(RegexCharSet {
                    negated: false,
                    items: vec![
                        RegexCharSetItem::Range('a', 'z'),
                        RegexCharSetItem::Class(RegexClass::Digit),
                    ],
                }),
                RegexNode::Lookaround {
                    kind: RegexLookaroundKind::FollowedBy,
                    body: vec![RegexNode::Literal(".log".to_string())],
                },
                RegexNode::Anchor(RegexAnchor::End),
            ],
        };

        let rendered = render(&pattern).unwrap();
        assert_eq!(rendered, "(?im)^(?<year>\\d{4})-[a-z0-9](?=\\.log)$");
    }

    #[test]
    fn rejects_anchor_quantifiers() {
        let pattern = RegexPattern {
            flags: Vec::new(),
            body: vec![RegexNode::Quantifier {
                target: Box::new(RegexNode::Anchor(RegexAnchor::Start)),
                kind: RegexQuantifierKind::OneOrMore,
                mode: RegexQuantifierMode::Greedy,
            }],
        };

        assert!(render(&pattern).is_err());
    }

    #[test]
    fn supports_scoped_flags_and_any_codepoint() {
        let pattern = RegexPattern {
            flags: Vec::new(),
            body: vec![RegexNode::ScopedFlags {
                enable: vec![RegexFlag::CaseInsensitive],
                disable: vec![],
                body: vec![RegexNode::Quantifier {
                    target: Box::new(RegexNode::AnyCodepoint),
                    kind: RegexQuantifierKind::ZeroOrMore,
                    mode: RegexQuantifierMode::Lazy,
                }],
            }],
        };

        let rendered = render(&pattern).unwrap();
        assert_eq!(rendered, "(?i:[\\s\\S]*?)");
    }

    #[test]
    fn compiled_regex_reports_matches_with_char_offsets() {
        let pattern = RegexPattern {
            flags: vec![RegexFlag::CaseInsensitive],
            body: vec![RegexNode::Group {
                kind: RegexGroupKind::Named("word".to_string()),
                body: vec![RegexNode::Quantifier {
                    target: Box::new(RegexNode::Class(RegexClass::Letter)),
                    kind: RegexQuantifierKind::OneOrMore,
                    mode: RegexQuantifierMode::Greedy,
                }],
            }],
        };

        let regex = compile(&pattern).unwrap();
        let matched = regex.find("é ana").unwrap().unwrap();

        assert_eq!(matched.text, "ana");
        assert_eq!((matched.start, matched.end), (2, 5));
        assert_eq!(matched.named.get("word"), Some(&Some("ana".to_string())));
    }

    #[test]
    fn target_validation_rejects_re2_backreferences() {
        let pattern = RegexPattern {
            flags: Vec::new(),
            body: vec![RegexNode::Reference(RegexReference::Group(1))],
        };

        assert!(validate_for_target(&pattern, RegexTarget::Re2).is_err());
    }
}
