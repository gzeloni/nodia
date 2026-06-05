// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Regex AST, validation, rendering, and runtime helpers for the native regex DSL.

use crate::error::{DobraError, DobraResult};
use fancy_regex::{Captures, Regex};
use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::rc::Rc;

mod api;
mod engine;
mod rendering;
mod support;
mod validation;

pub use self::api::{
    compile, compile_text, render, render_for_target, validate, validate_for_target,
};

/// Full regex literal made of top-level flags and body nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct RegexPattern {
    /// Flags enabled for the whole pattern.
    pub flags: Vec<RegexFlag>,
    /// Top-level regex nodes in evaluation order.
    pub body: Vec<RegexNode>,
}

/// Regex flags supported by the DSL and renderer.
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
    /// Resolves a DSL flag name into its enum representation.
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

    /// Returns the stable DSL name for the flag.
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

/// Regex AST node used by the parser and renderer.
#[derive(Debug, Clone, PartialEq)]
pub enum RegexNode {
    /// Nested sequence used to preserve grouping in the AST.
    Sequence(Vec<RegexNode>),
    /// Literal text that must be escaped for the target regex engine.
    Literal(String),
    /// Raw regex text inserted as-is.
    Raw(String),
    /// Zero-width anchor.
    Anchor(RegexAnchor),
    /// Character class shorthand.
    Class(RegexClass),
    /// Classic `.` wildcard.
    AnyChar,
    /// Dot-all wildcard that matches any codepoint.
    AnyCodepoint,
    /// Quantified target with an explicit greediness mode.
    Quantifier {
        target: Box<RegexNode>,
        kind: RegexQuantifierKind,
        mode: RegexQuantifierMode,
    },
    /// Capturing, non-capturing, named, or atomic group.
    Group {
        kind: RegexGroupKind,
        body: Vec<RegexNode>,
    },
    /// Alternation represented as explicit branches.
    Alternation(Vec<Vec<RegexNode>>),
    /// Explicit character set.
    CharSet(RegexCharSet),
    /// Lookaround assertion.
    Lookaround {
        kind: RegexLookaroundKind,
        body: Vec<RegexNode>,
    },
    /// Backreference by index or name.
    Reference(RegexReference),
    /// Scoped flag delta for a sub-sequence.
    ScopedFlags {
        enable: Vec<RegexFlag>,
        disable: Vec<RegexFlag>,
        body: Vec<RegexNode>,
    },
}

/// Anchor kinds accepted by the regex DSL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegexAnchor {
    Start,
    End,
    WordBoundary,
    NotWordBoundary,
}

impl RegexAnchor {
    /// Resolves a DSL anchor name into its enum representation.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "start" => Self::Start,
            "end" => Self::End,
            "word_boundary" => Self::WordBoundary,
            "not_word_boundary" => Self::NotWordBoundary,
            _ => return None,
        })
    }

    /// Returns the stable DSL name for the anchor.
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

/// Character classes supported by the regex DSL.
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
    /// Resolves a DSL class name into its enum representation.
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

    /// Returns the stable DSL name for the class.
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

/// Quantifier evaluation modes supported by the regex engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexQuantifierMode {
    Greedy,
    Lazy,
    Possessive,
}

impl RegexQuantifierMode {
    /// Returns the stable DSL name for the quantifier mode.
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

/// Quantifier shapes supported by the regex DSL.
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
    /// Formats the quantifier as it appears in the DSL.
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

/// Group kinds supported by the regex DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum RegexGroupKind {
    Capture,
    NonCapture,
    Named(String),
    Atomic,
}

impl RegexGroupKind {
    /// Returns the stable DSL name for the group kind.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Capture => "group",
            Self::NonCapture => "non_capture",
            Self::Named(_) => "named",
            Self::Atomic => "atomic",
        }
    }
}

/// Lookaround assertion kinds supported by the regex DSL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegexLookaroundKind {
    FollowedBy,
    NotFollowedBy,
    PrecededBy,
    NotPrecededBy,
}

impl RegexLookaroundKind {
    /// Returns the stable DSL name for the lookaround kind.
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

/// Character set literal used by [`RegexNode::CharSet`].
#[derive(Debug, Clone, PartialEq)]
pub struct RegexCharSet {
    /// Whether the set is negated.
    pub negated: bool,
    /// Items included in the set.
    pub items: Vec<RegexCharSetItem>,
}

/// Item inside a character set literal.
#[derive(Debug, Clone, PartialEq)]
pub enum RegexCharSetItem {
    /// Single character member.
    Char(char),
    /// Inclusive character range.
    Range(char, char),
    /// Nested character class shorthand.
    Class(RegexClass),
    /// Raw target-specific set fragment.
    Raw(String),
}

/// Backreference target in the regex DSL.
#[derive(Debug, Clone, PartialEq)]
pub enum RegexReference {
    /// Named capturing group reference.
    Named(String),
    /// Numeric capturing group reference.
    Group(usize),
}

/// Rendering target used for cross-engine validation.
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
    /// Returns the stable target name used by diagnostics and docs.
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

/// Compiled regex value used by the runtime.
#[derive(Clone)]
pub struct RuntimeRegex {
    rendered: String,
    engine: Rc<Regex>,
}

/// Rich match result returned by runtime regex operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegexMatch {
    /// Full matched text.
    pub text: String,
    /// Start offset in Unicode scalar values.
    pub start: usize,
    /// End offset in Unicode scalar values.
    pub end: usize,
    /// Indexed capture groups, excluding the whole match.
    pub groups: Vec<Option<String>>,
    /// Named capture groups.
    pub named: BTreeMap<String, Option<String>>,
}

#[cfg(test)]
mod tests;
