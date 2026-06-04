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

#[cfg(test)]
mod tests;
