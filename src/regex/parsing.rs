// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Parsing from classic regex text back into the native regex DSL AST.

use fancy_regex::{
    Absent as FancyAbsent, Assertion as FancyAssertion, BacktrackingControlVerb as FancyVerb,
    Error as FancyError, Expr as FancyExpr,
};
use std::collections::BTreeMap;

use super::support::regex_error;
use super::*;

pub(super) fn parse_text_pattern(rendered: &str) -> NodiaResult<RegexPattern> {
    if let Ok(pattern) = TextRegexParser::new(rendered).parse() {
        return Ok(pattern);
    }

    let tree =
        FancyExpr::parse_tree(rendered).map_err(|error| map_fancy_parse_error(rendered, error))?;
    ExprRegexConverter::new(tree.named_groups).convert(tree.expr)
}

struct ExprRegexConverter {
    names_by_group: BTreeMap<usize, String>,
    next_group: usize,
}

impl ExprRegexConverter {
    fn new<I>(named_groups: I) -> Self
    where
        I: IntoIterator<Item = (String, usize)>,
    {
        let mut names_by_group = BTreeMap::new();
        for (name, group) in named_groups {
            names_by_group.insert(group, name);
        }
        Self {
            names_by_group,
            next_group: 1,
        }
    }

    fn convert(mut self, expr: FancyExpr) -> NodiaResult<RegexPattern> {
        Ok(RegexPattern {
            flags: Vec::new(),
            body: self.convert_expr(&expr)?,
        })
    }

    fn convert_expr(&mut self, expr: &FancyExpr) -> NodiaResult<Vec<RegexNode>> {
        match expr {
            FancyExpr::Empty => Ok(Vec::new()),
            FancyExpr::Any { newline, crlf } => {
                let node = if *newline {
                    RegexNode::AnyCodepoint
                } else {
                    RegexNode::AnyChar
                };
                Ok(self.wrap_crlf(vec![node], *crlf && !newline))
            }
            FancyExpr::Assertion(assertion) => self.convert_assertion(*assertion),
            FancyExpr::GeneralNewline { .. } => Ok(vec![RegexNode::Class(RegexClass::GeneralNewline)]),
            FancyExpr::Literal { val, casei } => {
                Ok(self.wrap_case_insensitive(vec![canonical_literal_node(val)], *casei))
            }
            FancyExpr::Concat(items) => {
                let mut out = Vec::new();
                for item in items {
                    for node in self.convert_expr(item)? {
                        push_regex_node(&mut out, node);
                    }
                }
                Ok(out)
            }
            FancyExpr::Alt(branches) => {
                let mut out = Vec::with_capacity(branches.len());
                for branch in branches {
                    out.push(self.convert_expr(branch)?);
                }
                Ok(vec![RegexNode::Alternation(out)])
            }
            FancyExpr::Group(body) => {
                let group = self.next_group;
                self.next_group += 1;
                let kind = match self.names_by_group.get(&group) {
                    Some(name) => RegexGroupKind::Named(name.clone()),
                    None => RegexGroupKind::Capture,
                };
                Ok(vec![RegexNode::Group {
                    kind,
                    body: self.convert_expr(body)?,
                }])
            }
            FancyExpr::LookAround(body, kind) => Ok(vec![RegexNode::Lookaround {
                kind: match kind {
                    fancy_regex::LookAround::LookAhead => RegexLookaroundKind::FollowedBy,
                    fancy_regex::LookAround::LookAheadNeg => RegexLookaroundKind::NotFollowedBy,
                    fancy_regex::LookAround::LookBehind => RegexLookaroundKind::PrecededBy,
                    fancy_regex::LookAround::LookBehindNeg => RegexLookaroundKind::NotPrecededBy,
                },
                body: self.convert_expr(body)?,
            }]),
            FancyExpr::Repeat {
                child,
                lo,
                hi,
                greedy,
            } => Ok(vec![RegexNode::Quantifier {
                target: Box::new(self.convert_target(child)?),
                kind: if *lo == 0 && *hi == 1 {
                    RegexQuantifierKind::Optional
                } else if *lo == 0 && *hi == usize::MAX {
                    RegexQuantifierKind::ZeroOrMore
                } else if *lo == 1 && *hi == usize::MAX {
                    RegexQuantifierKind::OneOrMore
                } else if lo == hi {
                    RegexQuantifierKind::Exactly(*lo)
                } else if *hi == usize::MAX {
                    RegexQuantifierKind::AtLeast(*lo)
                } else {
                    RegexQuantifierKind::Between(*lo, *hi)
                },
                mode: if *greedy {
                    RegexQuantifierMode::Greedy
                } else {
                    RegexQuantifierMode::Lazy
                },
            }]),
            FancyExpr::Delegate { inner, casei } => Ok(self.wrap_case_insensitive(
                vec![parse_delegate_node(inner)?],
                *casei,
            )),
            FancyExpr::Backref { group, casei } => Ok(self.wrap_case_insensitive(
                vec![RegexNode::Reference(self.reference_for_group(*group))],
                *casei,
            )),
            FancyExpr::BackrefWithRelativeRecursionLevel { .. } => Err(regex_error(
                "relative recursion-level backreferences are not yet supported in regex DSL reversal",
            )),
            FancyExpr::AtomicGroup(body) => Ok(vec![RegexNode::Group {
                kind: RegexGroupKind::Atomic,
                body: self.convert_expr(body)?,
            }]),
            FancyExpr::KeepOut => Ok(vec![RegexNode::Anchor(RegexAnchor::KeepOut)]),
            FancyExpr::ContinueFromPreviousMatchEnd => {
                Ok(vec![RegexNode::Anchor(RegexAnchor::PreviousMatchEnd)])
            }
            FancyExpr::BackrefExistsCondition {
                group,
                relative_recursion_level,
            } => {
                if relative_recursion_level.is_some() {
                    return Err(regex_error(
                        "relative recursion-level capture conditions are not yet supported in regex DSL reversal",
                    ));
                }
                Ok(vec![RegexNode::Condition(RegexCondition::Capture(
                    self.reference_for_group(*group),
                ))])
            }
            FancyExpr::Conditional {
                condition,
                true_branch,
                false_branch,
            } => {
                let condition = self.convert_condition(condition)?;
                let true_branch = self.convert_expr(true_branch)?;
                let false_branch = self.convert_expr(false_branch)?;
                if true_branch.is_empty() && false_branch.is_empty() {
                    Ok(vec![RegexNode::Condition(condition)])
                } else {
                    Ok(vec![RegexNode::Conditional {
                        condition,
                        then_branch: true_branch,
                        else_branch: false_branch,
                    }])
                }
            }
            FancyExpr::SubroutineCall(group) => {
                Ok(vec![RegexNode::SubroutineCall(self.reference_for_group(*group))])
            }
            FancyExpr::BacktrackingControlVerb(verb) => Ok(vec![RegexNode::BacktrackingVerb(
                match verb {
                    FancyVerb::Fail => RegexBacktrackingVerb::Fail,
                    FancyVerb::Accept => RegexBacktrackingVerb::Accept,
                    FancyVerb::Commit => RegexBacktrackingVerb::Commit,
                    FancyVerb::Skip => RegexBacktrackingVerb::Skip,
                    FancyVerb::Prune => RegexBacktrackingVerb::Prune,
                },
            )]),
            FancyExpr::Absent(absent) => self.convert_absent(absent),
            FancyExpr::DefineGroup { definitions } => Ok(vec![RegexNode::DefineGroup {
                body: self.convert_expr(definitions)?,
            }]),
            FancyExpr::AstNode(..) => Err(regex_error(
                "unresolved regex AST node encountered while reversing raw regex",
            )),
        }
    }

    fn convert_assertion(&mut self, assertion: FancyAssertion) -> NodiaResult<Vec<RegexNode>> {
        match assertion {
            FancyAssertion::StartText => Ok(vec![RegexNode::Anchor(RegexAnchor::StartText)]),
            FancyAssertion::EndText => Ok(vec![RegexNode::Anchor(RegexAnchor::EndText)]),
            FancyAssertion::EndTextIgnoreTrailingNewlines { crlf } => Ok(self.wrap_crlf(
                vec![RegexNode::Anchor(RegexAnchor::EndTextBeforeNewlines)],
                crlf,
            )),
            FancyAssertion::StartLine { crlf } => {
                Ok(self.wrap_crlf(vec![RegexNode::Anchor(RegexAnchor::Start)], crlf))
            }
            FancyAssertion::EndLine { crlf } => {
                Ok(self.wrap_crlf(vec![RegexNode::Anchor(RegexAnchor::End)], crlf))
            }
            FancyAssertion::LeftWordBoundary => {
                Ok(vec![RegexNode::Anchor(RegexAnchor::LeftWordBoundary)])
            }
            FancyAssertion::LeftWordHalfBoundary => {
                Ok(vec![RegexNode::Anchor(RegexAnchor::LeftWordHalfBoundary)])
            }
            FancyAssertion::RightWordBoundary => {
                Ok(vec![RegexNode::Anchor(RegexAnchor::RightWordBoundary)])
            }
            FancyAssertion::RightWordHalfBoundary => {
                Ok(vec![RegexNode::Anchor(RegexAnchor::RightWordHalfBoundary)])
            }
            FancyAssertion::WordBoundary => Ok(vec![RegexNode::Anchor(RegexAnchor::WordBoundary)]),
            FancyAssertion::NotWordBoundary => {
                Ok(vec![RegexNode::Anchor(RegexAnchor::NotWordBoundary)])
            }
        }
    }

    fn convert_condition(&mut self, expr: &FancyExpr) -> NodiaResult<RegexCondition> {
        match expr {
            FancyExpr::BackrefExistsCondition {
                group,
                relative_recursion_level,
            } => {
                if relative_recursion_level.is_some() {
                    return Err(regex_error(
                        "relative recursion-level capture conditions are not yet supported in regex DSL reversal",
                    ));
                }
                Ok(RegexCondition::Capture(self.reference_for_group(*group)))
            }
            FancyExpr::LookAround(body, kind) => Ok(RegexCondition::Lookaround {
                kind: match kind {
                    fancy_regex::LookAround::LookAhead => RegexLookaroundKind::FollowedBy,
                    fancy_regex::LookAround::LookAheadNeg => RegexLookaroundKind::NotFollowedBy,
                    fancy_regex::LookAround::LookBehind => RegexLookaroundKind::PrecededBy,
                    fancy_regex::LookAround::LookBehindNeg => RegexLookaroundKind::NotPrecededBy,
                },
                body: self.convert_expr(body)?,
            }),
            other => Ok(RegexCondition::Expression(self.convert_expr(other)?)),
        }
    }

    fn convert_absent(&mut self, absent: &FancyAbsent) -> NodiaResult<Vec<RegexNode>> {
        match absent {
            FancyAbsent::Repeater(limit) => Ok(vec![RegexNode::Until {
                limit: self.convert_expr(limit)?,
                body: None,
            }]),
            FancyAbsent::Expression { absent, exp } => Ok(vec![RegexNode::Until {
                limit: self.convert_expr(absent)?,
                body: Some(self.convert_expr(exp)?),
            }]),
            FancyAbsent::Stopper(limit) => {
                Ok(vec![RegexNode::UntilStop(self.convert_expr(limit)?)])
            }
            FancyAbsent::Clear => Ok(vec![RegexNode::UntilClear]),
        }
    }

    fn convert_target(&mut self, expr: &FancyExpr) -> NodiaResult<RegexNode> {
        let items = self.convert_expr(expr)?;
        Ok(if items.len() == 1 {
            items.into_iter().next().unwrap()
        } else {
            RegexNode::Sequence(items)
        })
    }

    fn wrap_case_insensitive(&self, body: Vec<RegexNode>, yes: bool) -> Vec<RegexNode> {
        if !yes || body.is_empty() {
            body
        } else {
            vec![RegexNode::ScopedFlags {
                enable: vec![RegexFlag::CaseInsensitive],
                disable: Vec::new(),
                body,
            }]
        }
    }

    fn wrap_crlf(&self, body: Vec<RegexNode>, yes: bool) -> Vec<RegexNode> {
        if !yes || body.is_empty() {
            body
        } else {
            vec![RegexNode::ScopedFlags {
                enable: vec![RegexFlag::Crlf],
                disable: Vec::new(),
                body,
            }]
        }
    }

    fn reference_for_group(&self, group: usize) -> RegexReference {
        match self.names_by_group.get(&group) {
            Some(name) => RegexReference::Named(name.clone()),
            None => RegexReference::Group(group),
        }
    }
}

fn canonical_literal_node(value: &str) -> RegexNode {
    let mut chars = value.chars();
    let Some(ch) = chars.next() else {
        return RegexNode::Literal(String::new());
    };
    if chars.next().is_some() {
        return RegexNode::Literal(value.to_string());
    }
    match ch {
        '\u{0007}' => RegexNode::Class(RegexClass::Bell),
        '\u{001b}' => RegexNode::Class(RegexClass::Escape),
        '\u{000c}' => RegexNode::Class(RegexClass::FormFeed),
        '\u{000b}' => RegexNode::Class(RegexClass::VerticalTab),
        '\r' => RegexNode::Class(RegexClass::CarriageReturn),
        '\n' => RegexNode::Class(RegexClass::Newline),
        '\t' => RegexNode::Class(RegexClass::Tab),
        ' ' => RegexNode::Class(RegexClass::Space),
        _ => RegexNode::Literal(value.to_string()),
    }
}

fn parse_delegate_node(inner: &str) -> NodiaResult<RegexNode> {
    let pattern = TextRegexParser::new(inner).parse()?;
    if !pattern.flags.is_empty() {
        return Err(regex_error(
            "delegate regex fragments cannot carry top-level flags in DSL reversal",
        ));
    }
    let mut body = pattern.body;
    if body.is_empty() {
        return Err(regex_error("delegate regex fragment cannot be empty"));
    }
    Ok(if body.len() == 1 {
        body.remove(0)
    } else {
        RegexNode::Sequence(body)
    })
}

fn map_fancy_parse_error(rendered: &str, error: FancyError) -> NodiaError {
    match error {
        FancyError::ParseError(index, kind) => {
            let (line, column) = line_column_for_text(rendered, index);
            regex_error(kind.to_string()).with_span(line, column)
        }
        other => regex_error(other.to_string()),
    }
}

fn line_column_for_text(text: &str, index: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    for ch in text.chars().take(index) {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

struct TextRegexParser {
    chars: Vec<char>,
    pos: usize,
}

impl TextRegexParser {
    fn new(rendered: &str) -> Self {
        Self {
            chars: rendered.chars().collect(),
            pos: 0,
        }
    }

    fn parse(mut self) -> NodiaResult<RegexPattern> {
        let flags = self.parse_leading_global_flags()?;
        let ignore_whitespace = flags.contains(&RegexFlag::IgnoreWhitespace);
        let body = self.parse_expression(None, ignore_whitespace)?;
        self.skip_ignored(ignore_whitespace);
        if !self.is_at_end() {
            return Err(self.error_here("unexpected trailing regex text"));
        }
        Ok(RegexPattern { flags, body })
    }

    fn parse_leading_global_flags(&mut self) -> NodiaResult<Vec<RegexFlag>> {
        let mut flags = Vec::new();

        loop {
            let start = self.pos;
            if !self.consume('(') || !self.consume('?') {
                self.pos = start;
                break;
            }
            if !matches!(self.peek(), Some(ch) if is_flag_code(ch)) {
                self.pos = start;
                break;
            }

            let (enable, disable) = self.parse_flag_delta()?;
            if !disable.is_empty() {
                return Err(self.error_at(
                    start,
                    "leading regex flags cannot disable modes; use scoped flags",
                ));
            }
            if !self.consume(')') {
                self.pos = start;
                break;
            }
            flags.extend(enable);
        }

        Ok(flags)
    }

    fn parse_expression(
        &mut self,
        terminator: Option<char>,
        ignore_whitespace: bool,
    ) -> NodiaResult<Vec<RegexNode>> {
        let mut branches = vec![self.parse_sequence(terminator, ignore_whitespace)?];
        self.skip_ignored(ignore_whitespace);

        while self.consume('|') {
            branches.push(self.parse_sequence(terminator, ignore_whitespace)?);
            self.skip_ignored(ignore_whitespace);
        }

        if let Some(terminator) = terminator {
            if !self.consume(terminator) {
                return Err(self.error_here(&format!("expected '{terminator}'")));
            }
        }

        if branches.len() == 1 {
            Ok(branches.pop().unwrap())
        } else {
            Ok(vec![RegexNode::Alternation(branches)])
        }
    }

    fn parse_sequence(
        &mut self,
        terminator: Option<char>,
        ignore_whitespace: bool,
    ) -> NodiaResult<Vec<RegexNode>> {
        let mut items = Vec::new();

        loop {
            self.skip_ignored(ignore_whitespace);
            if self.is_at_end() || self.peek() == Some('|') || self.peek() == terminator {
                break;
            }

            let node = self.parse_atom(ignore_whitespace)?;
            let node = self.parse_quantifier(node, ignore_whitespace)?;
            push_regex_node(&mut items, node);
        }

        Ok(items)
    }

    fn parse_atom(&mut self, ignore_whitespace: bool) -> NodiaResult<RegexNode> {
        let Some(ch) = self.advance() else {
            return Err(self.error_here("expected regex item"));
        };

        match ch {
            '^' => Ok(RegexNode::Anchor(RegexAnchor::Start)),
            '$' => Ok(RegexNode::Anchor(RegexAnchor::End)),
            '.' => Ok(RegexNode::AnyChar),
            '[' => self.parse_char_set(),
            '(' => self.parse_group(ignore_whitespace),
            '\\' => self.parse_escape(),
            '*' | '+' | '?' => Err(self.error_previous("quantifier is missing a target")),
            other => Ok(RegexNode::Literal(other.to_string())),
        }
    }

    fn parse_quantifier(
        &mut self,
        target: RegexNode,
        ignore_whitespace: bool,
    ) -> NodiaResult<RegexNode> {
        self.skip_ignored(ignore_whitespace);

        let Some(kind) = self.parse_quantifier_kind()? else {
            return Ok(target);
        };

        let mode = if self.consume('?') {
            RegexQuantifierMode::Lazy
        } else if self.consume('+') {
            RegexQuantifierMode::Possessive
        } else {
            RegexQuantifierMode::Greedy
        };

        Ok(RegexNode::Quantifier {
            target: Box::new(target),
            kind,
            mode,
        })
    }

    fn parse_quantifier_kind(&mut self) -> NodiaResult<Option<RegexQuantifierKind>> {
        if self.consume('?') {
            return Ok(Some(RegexQuantifierKind::Optional));
        }
        if self.consume('*') {
            return Ok(Some(RegexQuantifierKind::ZeroOrMore));
        }
        if self.consume('+') {
            return Ok(Some(RegexQuantifierKind::OneOrMore));
        }

        let start = self.pos;
        if !self.consume('{') {
            return Ok(None);
        }

        let Some(min) = self.parse_unsigned() else {
            self.pos = start;
            return Ok(None);
        };

        if self.consume('}') {
            return Ok(Some(RegexQuantifierKind::Exactly(min)));
        }

        if !self.consume(',') {
            self.pos = start;
            return Ok(None);
        }

        if self.consume('}') {
            return Ok(Some(RegexQuantifierKind::AtLeast(min)));
        }

        let Some(max) = self.parse_unsigned() else {
            self.pos = start;
            return Ok(None);
        };

        if !self.consume('}') {
            self.pos = start;
            return Ok(None);
        }

        Ok(Some(RegexQuantifierKind::Between(min, max)))
    }

    fn parse_group(&mut self, ignore_whitespace: bool) -> NodiaResult<RegexNode> {
        if !self.consume('?') {
            let body = self.parse_expression(Some(')'), ignore_whitespace)?;
            return Ok(RegexNode::Group {
                kind: RegexGroupKind::Capture,
                body,
            });
        }

        if self.consume('(') {
            return self.parse_conditional_group(ignore_whitespace);
        }
        if self.consume(':') {
            return Ok(RegexNode::Group {
                kind: RegexGroupKind::NonCapture,
                body: self.parse_expression(Some(')'), ignore_whitespace)?,
            });
        }
        if self.consume('>') {
            return Ok(RegexNode::Group {
                kind: RegexGroupKind::Atomic,
                body: self.parse_expression(Some(')'), ignore_whitespace)?,
            });
        }
        if self.consume('=') {
            return Ok(RegexNode::Lookaround {
                kind: RegexLookaroundKind::FollowedBy,
                body: self.parse_expression(Some(')'), ignore_whitespace)?,
            });
        }
        if self.consume('!') {
            return Ok(RegexNode::Lookaround {
                kind: RegexLookaroundKind::NotFollowedBy,
                body: self.parse_expression(Some(')'), ignore_whitespace)?,
            });
        }
        if self.consume('<') {
            if self.consume('=') {
                return Ok(RegexNode::Lookaround {
                    kind: RegexLookaroundKind::PrecededBy,
                    body: self.parse_expression(Some(')'), ignore_whitespace)?,
                });
            }
            if self.consume('!') {
                return Ok(RegexNode::Lookaround {
                    kind: RegexLookaroundKind::NotPrecededBy,
                    body: self.parse_expression(Some(')'), ignore_whitespace)?,
                });
            }

            let name = self.parse_capture_name()?;
            if !self.consume('>') {
                return Err(self.error_here("expected '>' after named capture"));
            }
            return Ok(RegexNode::Group {
                kind: RegexGroupKind::Named(name),
                body: self.parse_expression(Some(')'), ignore_whitespace)?,
            });
        }
        if self.consume('\'') {
            let name = self.parse_capture_name()?;
            if !self.consume('\'') {
                return Err(self.error_here("expected '\\'' after named capture"));
            }
            return Ok(RegexNode::Group {
                kind: RegexGroupKind::Named(name),
                body: self.parse_expression(Some(')'), ignore_whitespace)?,
            });
        }
        if self.consume('P') {
            if self.consume('<') {
                let name = self.parse_capture_name()?;
                if !self.consume('>') {
                    return Err(self.error_here("expected '>' after named capture"));
                }
                return Ok(RegexNode::Group {
                    kind: RegexGroupKind::Named(name),
                    body: self.parse_expression(Some(')'), ignore_whitespace)?,
                });
            }
            if self.consume('=') {
                let name = self.parse_capture_name()?;
                if !self.consume(')') {
                    return Err(self.error_here("expected ')' after named backreference"));
                }
                return Ok(RegexNode::Reference(RegexReference::Named(name)));
            }
        }

        if matches!(self.peek(), Some(ch) if is_flag_code(ch) || ch == '-') {
            let (enable, disable) = self.parse_flag_delta()?;
            if !self.consume(':') {
                return Err(self.error_here(
                    "plain mode toggles are only supported at the start of the pattern; use scoped flags",
                ));
            }
            let nested_ignore_whitespace =
                toggle_ignore_whitespace(ignore_whitespace, &enable, &disable);
            return Ok(RegexNode::ScopedFlags {
                enable,
                disable,
                body: self.parse_expression(Some(')'), nested_ignore_whitespace)?,
            });
        }

        Err(self.error_here("unsupported regex group syntax"))
    }

    fn parse_conditional_group(&mut self, ignore_whitespace: bool) -> NodiaResult<RegexNode> {
        if self.starts_with_keyword("DEFINE)") {
            self.pos += "DEFINE)".chars().count();
            return Ok(RegexNode::DefineGroup {
                body: self.parse_expression(Some(')'), ignore_whitespace)?,
            });
        }

        let (condition, condition_closed) = self.parse_conditional_condition(ignore_whitespace)?;
        if !condition_closed && !self.consume(')') {
            return Err(self.error_here("expected ')' after regex condition"));
        }

        let then_branch = self.parse_sequence(Some(')'), ignore_whitespace)?;
        let else_branch = if self.consume('|') {
            let branch = self.parse_sequence(Some(')'), ignore_whitespace)?;
            if !self.consume(')') {
                return Err(self.error_here("expected ')' after conditional branch"));
            }
            branch
        } else {
            if !self.consume(')') {
                return Err(self.error_here("expected ')' after conditional branch"));
            }
            Vec::new()
        };

        Ok(RegexNode::Conditional {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_conditional_condition(
        &mut self,
        ignore_whitespace: bool,
    ) -> NodiaResult<(RegexCondition, bool)> {
        if let Some(index) = self.parse_unsigned() {
            return Ok((RegexCondition::Capture(RegexReference::Group(index)), false));
        }
        if self.consume('<') {
            let name = self.parse_capture_name()?;
            if !self.consume('>') {
                return Err(self.error_here("expected '>' after named conditional capture"));
            }
            return Ok((RegexCondition::Capture(RegexReference::Named(name)), false));
        }
        if self.consume('\'') {
            let name = self.parse_capture_name()?;
            if !self.consume('\'') {
                return Err(self.error_here("expected '\\'' after named conditional capture"));
            }
            return Ok((RegexCondition::Capture(RegexReference::Named(name)), false));
        }
        if self.consume('?') {
            if self.consume('=') {
                return Ok((
                    RegexCondition::Lookaround {
                        kind: RegexLookaroundKind::FollowedBy,
                        body: self.parse_expression(Some(')'), ignore_whitespace)?,
                    },
                    true,
                ));
            }
            if self.consume('!') {
                return Ok((
                    RegexCondition::Lookaround {
                        kind: RegexLookaroundKind::NotFollowedBy,
                        body: self.parse_expression(Some(')'), ignore_whitespace)?,
                    },
                    true,
                ));
            }
            if self.consume('<') {
                if self.consume('=') {
                    return Ok((
                        RegexCondition::Lookaround {
                            kind: RegexLookaroundKind::PrecededBy,
                            body: self.parse_expression(Some(')'), ignore_whitespace)?,
                        },
                        true,
                    ));
                }
                if self.consume('!') {
                    return Ok((
                        RegexCondition::Lookaround {
                            kind: RegexLookaroundKind::NotPrecededBy,
                            body: self.parse_expression(Some(')'), ignore_whitespace)?,
                        },
                        true,
                    ));
                }
            }
            return Err(self.error_here("unsupported regex conditional assertion"));
        }

        Ok((
            RegexCondition::Capture(RegexReference::Named(self.parse_capture_name()?)),
            false,
        ))
    }

    fn parse_char_set(&mut self) -> NodiaResult<RegexNode> {
        let negated = self.consume('^');
        let mut items = Vec::new();
        let mut first = true;

        loop {
            let Some(ch) = self.peek() else {
                return Err(self.error_here("unterminated char_set"));
            };
            if ch == ']' && !first {
                self.advance();
                break;
            }
            if ch == ']' && first {
                self.advance();
                items.push(RegexCharSetItem::Char(']'));
                first = false;
                continue;
            }

            let start = self.parse_char_set_atom()?;
            first = false;

            if let RegexCharSetItem::Char(range_start) = start {
                let dash = self.pos;
                if self.consume('-') {
                    if self.peek() == Some(']') {
                        self.pos = dash;
                        items.push(RegexCharSetItem::Char(range_start));
                        continue;
                    }

                    match self.parse_char_set_atom()? {
                        RegexCharSetItem::Char(range_end) => {
                            items.push(RegexCharSetItem::Range(range_start, range_end));
                            continue;
                        }
                        _ => {
                            return Err(self.error_at(
                                dash,
                                "char_set ranges only support literal character bounds",
                            ))
                        }
                    }
                }

                items.push(RegexCharSetItem::Char(range_start));
                continue;
            }

            items.push(start);
        }

        let set = RegexCharSet { negated, items };
        Ok(canonicalize_char_set(set))
    }

    fn parse_char_set_atom(&mut self) -> NodiaResult<RegexCharSetItem> {
        let Some(ch) = self.advance() else {
            return Err(self.error_here("unterminated char_set"));
        };

        if ch == '\\' {
            return self.parse_char_set_escape();
        }

        Ok(RegexCharSetItem::Char(ch))
    }

    fn parse_escape(&mut self) -> NodiaResult<RegexNode> {
        let Some(ch) = self.advance() else {
            return Err(self.error_previous("unterminated regex escape"));
        };

        Ok(match ch {
            'd' => RegexNode::Class(RegexClass::Digit),
            'D' => RegexNode::Class(RegexClass::NotDigit),
            's' => RegexNode::Class(RegexClass::Whitespace),
            'S' => RegexNode::Class(RegexClass::NotWhitespace),
            'w' => RegexNode::Class(RegexClass::WordChar),
            'W' => RegexNode::Class(RegexClass::NotWordChar),
            'h' => RegexNode::Class(RegexClass::HexDigit),
            'H' => RegexNode::Class(RegexClass::NotHexDigit),
            'N' => RegexNode::Class(RegexClass::NotNewline),
            'R' => RegexNode::Class(RegexClass::GeneralNewline),
            'O' => RegexNode::AnyCodepoint,
            'a' => RegexNode::Class(RegexClass::Bell),
            'b' => {
                if self.peek() == Some('{') {
                    self.parse_word_boundary_brace()?
                } else {
                    RegexNode::Anchor(RegexAnchor::WordBoundary)
                }
            }
            'B' => {
                if self.peek() == Some('{') {
                    return Err(self.error_here("extended '\\B{...}' boundaries are not supported"));
                }
                RegexNode::Anchor(RegexAnchor::NotWordBoundary)
            }
            'A' => RegexNode::Anchor(RegexAnchor::StartText),
            'z' => RegexNode::Anchor(RegexAnchor::EndText),
            'Z' => RegexNode::Anchor(RegexAnchor::EndTextBeforeNewlines),
            'G' => RegexNode::Anchor(RegexAnchor::PreviousMatchEnd),
            'K' => RegexNode::Anchor(RegexAnchor::KeepOut),
            'e' => RegexNode::Class(RegexClass::Escape),
            'f' => RegexNode::Class(RegexClass::FormFeed),
            'r' => RegexNode::Class(RegexClass::CarriageReturn),
            't' => RegexNode::Class(RegexClass::Tab),
            'n' => RegexNode::Class(RegexClass::Newline),
            'v' => RegexNode::Class(RegexClass::VerticalTab),
            'k' => RegexNode::Reference(RegexReference::Named(self.parse_named_reference()?)),
            'g' => RegexNode::SubroutineCall(self.parse_subroutine_reference()?),
            'p' | 'P' => {
                let (name, negated) = self.parse_property_escape(ch == 'P')?;
                RegexNode::Property { name, negated }
            }
            'x' | 'u' | 'U' => canonical_literal_node(&self.parse_hex_escape(ch)?),
            '0' => {
                return Err(self.error_previous(
                    "unsupported '\\0' escape in regex text; use raw_regex for opaque patterns",
                ))
            }
            digit if digit.is_ascii_digit() => {
                let mut raw = String::from(digit);
                while matches!(self.peek(), Some(next) if next.is_ascii_digit()) {
                    raw.push(self.advance().unwrap());
                }
                RegexNode::Reference(RegexReference::Group(
                    raw.parse::<usize>().expect("ascii digits always parse"),
                ))
            }
            'Q' => RegexNode::Literal(self.parse_quoted_literal()?),
            'E' => return Err(self.error_previous("unexpected '\\E' without '\\Q'")),
            other if other.is_ascii_alphabetic() => {
                return Err(
                    self.error_previous(format!("unsupported '\\{other}' escape in regex text"))
                )
            }
            other => RegexNode::Literal(other.to_string()),
        })
    }

    fn parse_char_set_escape(&mut self) -> NodiaResult<RegexCharSetItem> {
        let Some(ch) = self.advance() else {
            return Err(self.error_previous("unterminated regex escape"));
        };

        Ok(match ch {
            'd' => RegexCharSetItem::Class(RegexClass::Digit),
            'D' => RegexCharSetItem::Class(RegexClass::NotDigit),
            's' => RegexCharSetItem::Class(RegexClass::Whitespace),
            'S' => RegexCharSetItem::Class(RegexClass::NotWhitespace),
            'w' => RegexCharSetItem::Class(RegexClass::WordChar),
            'W' => RegexCharSetItem::Class(RegexClass::NotWordChar),
            'h' => RegexCharSetItem::Class(RegexClass::HexDigit),
            'H' => RegexCharSetItem::Class(RegexClass::NotHexDigit),
            'N' => RegexCharSetItem::Class(RegexClass::NotNewline),
            'R' => RegexCharSetItem::Class(RegexClass::GeneralNewline),
            'a' => RegexCharSetItem::Class(RegexClass::Bell),
            'e' => RegexCharSetItem::Class(RegexClass::Escape),
            'f' => RegexCharSetItem::Class(RegexClass::FormFeed),
            't' => RegexCharSetItem::Class(RegexClass::Tab),
            'n' => RegexCharSetItem::Class(RegexClass::Newline),
            'r' => RegexCharSetItem::Class(RegexClass::CarriageReturn),
            'v' => RegexCharSetItem::Class(RegexClass::VerticalTab),
            'b' => RegexCharSetItem::Char('\u{0008}'),
            'p' | 'P' => {
                let (name, negated) = self.parse_property_escape(ch == 'P')?;
                RegexCharSetItem::Property { name, negated }
            }
            'x' | 'u' | 'U' => {
                let value = self.parse_hex_escape(ch)?;
                let mut chars = value.chars();
                let Some(ch) = chars.next() else {
                    return Err(self.error_previous("hex escape resolved to empty character"));
                };
                if chars.next().is_some() {
                    return Err(self.error_previous(
                        "hex escape inside char_set must resolve to exactly one character",
                    ));
                }
                RegexCharSetItem::Char(ch)
            }
            'A' | 'z' | 'Z' | 'G' | 'Q' | 'E' | 'k' | 'K' | 'O' => {
                return Err(self.error_previous(format!(
                    "unsupported '\\{ch}' escape in char_set; use raw_regex for opaque patterns"
                )))
            }
            other if other.is_ascii_alphabetic() => {
                return Err(
                    self.error_previous(format!("unsupported '\\{other}' escape in char_set"))
                )
            }
            other => RegexCharSetItem::Char(other),
        })
    }

    fn parse_property_escape(&mut self, already_negated: bool) -> NodiaResult<(String, bool)> {
        if !self.consume('{') {
            return Err(self.error_here("expected '{' after unicode property escape"));
        }
        let mut raw = String::new();
        while let Some(ch) = self.advance() {
            if ch == '}' {
                if raw.is_empty() {
                    return Err(self.error_previous("unicode property name cannot be empty"));
                }
                let mut negated = already_negated;
                let name = if let Some(stripped) = raw.strip_prefix('^') {
                    negated = !negated;
                    stripped.to_string()
                } else {
                    raw
                };
                return Ok((name, negated));
            }
            raw.push(ch);
        }
        Err(self.error_previous("unterminated unicode property escape"))
    }

    fn parse_hex_escape(&mut self, prefix: char) -> NodiaResult<String> {
        let expected_digits = match prefix {
            'x' => 2usize,
            'u' => 4usize,
            'U' => 8usize,
            _ => unreachable!("hex escape helper only accepts x/u/U"),
        };

        if self.consume('{') {
            let mut raw = String::new();
            while let Some(ch) = self.advance() {
                if ch == '}' {
                    if raw.is_empty() {
                        return Err(self.error_previous("invalid hex escape"));
                    }
                    return decode_hex_escape(&raw)
                        .ok_or_else(|| self.error_previous("invalid codepoint in hex escape"));
                }
                if !ch.is_ascii_hexdigit() || raw.len() >= 8 {
                    return Err(self.error_previous("invalid hex escape"));
                }
                raw.push(ch);
            }
            return Err(self.error_previous("unterminated hex escape"));
        }

        let start = self.pos;
        for _ in 0..expected_digits {
            if !matches!(self.peek(), Some(ch) if ch.is_ascii_hexdigit()) {
                return Err(self.error_at(start, "invalid hex escape"));
            }
            self.advance();
        }
        let raw = self.chars[start..self.pos].iter().collect::<String>();
        decode_hex_escape(&raw)
            .ok_or_else(|| self.error_at(start, "invalid codepoint in hex escape"))
    }

    fn parse_quoted_literal(&mut self) -> NodiaResult<String> {
        let mut out = String::new();
        while let Some(ch) = self.advance() {
            if ch == '\\' && self.peek() == Some('E') {
                self.advance();
                return Ok(out);
            }
            out.push(ch);
        }
        Err(self.error_previous("unterminated '\\Q...\\E' quote block"))
    }

    fn parse_word_boundary_brace(&mut self) -> NodiaResult<RegexNode> {
        if !self.consume('{') {
            return Err(self.error_here("expected '{' after '\\b'"));
        }
        let start = self.pos;
        while !self.is_at_end() && self.peek() != Some('}') {
            self.advance();
        }
        if !self.consume('}') {
            return Err(self.error_here("unterminated '\\b{...}' boundary"));
        }
        let content = self.chars[start..self.pos - 1].iter().collect::<String>();
        match content.as_str() {
            "start" => Ok(RegexNode::Anchor(RegexAnchor::LeftWordBoundary)),
            "start-half" => Ok(RegexNode::Anchor(RegexAnchor::LeftWordHalfBoundary)),
            "end" => Ok(RegexNode::Anchor(RegexAnchor::RightWordBoundary)),
            "end-half" => Ok(RegexNode::Anchor(RegexAnchor::RightWordHalfBoundary)),
            other => Err(self.error_at(start, format!("unsupported '\\b{{{other}}}' boundary"))),
        }
    }

    fn parse_named_reference(&mut self) -> NodiaResult<String> {
        if !self.consume('<') {
            return Err(self.error_here("expected '<' after '\\k'"));
        }
        let name = self.parse_capture_name()?;
        if !self.consume('>') {
            return Err(self.error_here("expected '>' after named backreference"));
        }
        Ok(name)
    }

    fn parse_subroutine_reference(&mut self) -> NodiaResult<RegexReference> {
        if self.consume('<') {
            let reference =
                self.parse_capture_reference("expected '>' after subroutine reference")?;
            if !self.consume('>') {
                return Err(self.error_here("expected '>' after subroutine reference"));
            }
            return Ok(reference);
        }
        if self.consume('\'') {
            let reference =
                self.parse_capture_reference("expected '\\'' after subroutine reference")?;
            if !self.consume('\'') {
                return Err(self.error_here("expected '\\'' after subroutine reference"));
            }
            return Ok(reference);
        }
        Err(self.error_here("expected '<' or '\\'' after '\\g'"))
    }

    fn parse_capture_reference(&mut self, _message: &str) -> NodiaResult<RegexReference> {
        if let Some(index) = self.parse_unsigned() {
            return Ok(RegexReference::Group(index));
        }
        Ok(RegexReference::Named(self.parse_capture_name()?))
    }

    fn parse_capture_name(&mut self) -> NodiaResult<String> {
        let start = self.pos;
        let Some(first) = self.peek() else {
            return Err(self.error_here("expected capture name"));
        };
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return Err(self.error_here("expected capture name"));
        }

        let mut name = String::new();
        name.push(self.advance().unwrap());
        while matches!(self.peek(), Some(ch) if ch == '_' || ch.is_ascii_alphanumeric()) {
            name.push(self.advance().unwrap());
        }

        if name.is_empty() {
            return Err(self.error_at(start, "expected capture name"));
        }

        Ok(name)
    }

    fn parse_flag_delta(&mut self) -> NodiaResult<(Vec<RegexFlag>, Vec<RegexFlag>)> {
        let mut enable = Vec::new();
        let mut disable = Vec::new();
        let mut writing_disable = false;
        let mut saw_flag = false;

        loop {
            match self.peek() {
                Some('-') if !writing_disable => {
                    writing_disable = true;
                    self.advance();
                }
                Some(ch) if is_flag_code(ch) => {
                    saw_flag = true;
                    let flag = flag_from_code(self.advance().unwrap())?;
                    if writing_disable {
                        disable.push(flag);
                    } else {
                        enable.push(flag);
                    }
                }
                _ => break,
            }
        }

        if !saw_flag {
            return Err(self.error_here("expected regex flag"));
        }

        Ok((enable, disable))
    }

    fn parse_unsigned(&mut self) -> Option<usize> {
        let start = self.pos;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) {
            self.advance();
        }
        if self.pos == start {
            return None;
        }
        self.chars[start..self.pos]
            .iter()
            .collect::<String>()
            .parse::<usize>()
            .ok()
    }

    fn skip_ignored(&mut self, ignore_whitespace: bool) {
        if !ignore_whitespace {
            return;
        }

        loop {
            let mut advanced = false;

            while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
                self.advance();
                advanced = true;
            }

            if self.peek() == Some('#') {
                advanced = true;
                while let Some(ch) = self.advance() {
                    if ch == '\n' {
                        break;
                    }
                }
            }

            if !advanced {
                break;
            }
        }
    }

    fn error_here(&self, message: impl Into<String>) -> NodiaError {
        let (line, column) = self.line_column(self.pos);
        regex_error(message).with_span(line, column)
    }

    fn error_previous(&self, message: impl Into<String>) -> NodiaError {
        let index = self.pos.saturating_sub(1);
        let (line, column) = self.line_column(index);
        regex_error(message).with_span(line, column)
    }

    fn error_at(&self, index: usize, message: impl Into<String>) -> NodiaError {
        let (line, column) = self.line_column(index);
        regex_error(message).with_span(line, column)
    }

    fn line_column(&self, index: usize) -> (usize, usize) {
        let mut line = 1usize;
        let mut column = 1usize;
        for ch in self.chars.iter().take(index) {
            if *ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        (line, column)
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.is_at_end() {
            None
        } else {
            let ch = self.chars[self.pos];
            self.pos += 1;
            Some(ch)
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn starts_with_keyword(&self, keyword: &str) -> bool {
        let expected = keyword.chars().collect::<Vec<_>>();
        self.chars
            .get(self.pos..self.pos + expected.len())
            .is_some_and(|slice| slice == expected.as_slice())
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }
}

fn push_regex_node(items: &mut Vec<RegexNode>, node: RegexNode) {
    match node {
        RegexNode::Sequence(children) => {
            for child in children {
                push_regex_node(items, child);
            }
        }
        RegexNode::Literal(value) => {
            if let Some(RegexNode::Literal(current)) = items.last_mut() {
                current.push_str(&value);
            } else {
                items.push(RegexNode::Literal(value));
            }
        }
        RegexNode::ScopedFlags {
            enable,
            disable,
            body,
        } => {
            if let Some(RegexNode::ScopedFlags {
                enable: current_enable,
                disable: current_disable,
                body: current_body,
            }) = items.last_mut()
            {
                if *current_enable == enable && *current_disable == disable {
                    for child in body {
                        push_regex_node(current_body, child);
                    }
                    return;
                }
            }
            items.push(RegexNode::ScopedFlags {
                enable,
                disable,
                body,
            });
        }
        other => items.push(other),
    }
}

fn flag_from_code(ch: char) -> NodiaResult<RegexFlag> {
    match ch {
        'i' => Ok(RegexFlag::CaseInsensitive),
        'm' => Ok(RegexFlag::Multiline),
        'R' => Ok(RegexFlag::Crlf),
        's' => Ok(RegexFlag::DotAll),
        'u' => Ok(RegexFlag::Unicode),
        'x' => Ok(RegexFlag::IgnoreWhitespace),
        'U' => Ok(RegexFlag::Ungreedy),
        _ => Err(regex_error(format!("unsupported regex flag '{ch}'"))),
    }
}

fn is_flag_code(ch: char) -> bool {
    matches!(ch, 'i' | 'm' | 'R' | 's' | 'u' | 'x' | 'U')
}

fn toggle_ignore_whitespace(inherited: bool, enable: &[RegexFlag], disable: &[RegexFlag]) -> bool {
    if disable.contains(&RegexFlag::IgnoreWhitespace) {
        false
    } else if enable.contains(&RegexFlag::IgnoreWhitespace) {
        true
    } else {
        inherited
    }
}

fn decode_hex_escape(raw: &str) -> Option<String> {
    let codepoint = u32::from_str_radix(raw, 16).ok()?;
    char::from_u32(codepoint).map(|ch| ch.to_string())
}

fn canonicalize_char_set(set: RegexCharSet) -> RegexNode {
    if !set.negated {
        if matches!(
            set.items.as_slice(),
            [
                RegexCharSetItem::Class(RegexClass::Whitespace),
                RegexCharSetItem::Class(RegexClass::NotWhitespace),
            ] | [
                RegexCharSetItem::Class(RegexClass::NotWhitespace),
                RegexCharSetItem::Class(RegexClass::Whitespace),
            ]
        ) {
            return RegexNode::AnyCodepoint;
        }

        let class = match set.items.as_slice() {
            [RegexCharSetItem::Range('A', 'Z'), RegexCharSetItem::Range('a', 'z')] => {
                Some(RegexClass::Letter)
            }
            [RegexCharSetItem::Range('a', 'z')] => Some(RegexClass::Lowercase),
            [RegexCharSetItem::Range('A', 'Z')] => Some(RegexClass::Uppercase),
            [RegexCharSetItem::Range('0', '9'), RegexCharSetItem::Range('A', 'F'), RegexCharSetItem::Range('a', 'f')] => {
                Some(RegexClass::HexDigit)
            }
            [RegexCharSetItem::Range('A', 'Z'), RegexCharSetItem::Range('a', 'z'), RegexCharSetItem::Range('0', '9')]
            | [RegexCharSetItem::Range('0', '9'), RegexCharSetItem::Range('A', 'Z'), RegexCharSetItem::Range('a', 'z')]
            | [RegexCharSetItem::Range('A', 'Z'), RegexCharSetItem::Range('0', '9'), RegexCharSetItem::Range('a', 'z')]
            | [RegexCharSetItem::Range('a', 'z'), RegexCharSetItem::Range('A', 'Z'), RegexCharSetItem::Range('0', '9')]
            | [RegexCharSetItem::Range('a', 'z'), RegexCharSetItem::Range('0', '9'), RegexCharSetItem::Range('A', 'Z')]
            | [RegexCharSetItem::Range('0', '9'), RegexCharSetItem::Range('a', 'z'), RegexCharSetItem::Range('A', 'Z')] => {
                Some(RegexClass::Alnum)
            }
            _ => None,
        };

        if let Some(class) = class {
            return RegexNode::Class(class);
        }
    }

    RegexNode::CharSet(set)
}
