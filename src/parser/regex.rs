// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Parsing support for the native regex DSL.

use super::*;

impl Parser {
    pub(super) fn regex_literal(&mut self) -> NodiaResult<Expr> {
        let mut flags = if self.match_kind(&TokenKind::LeftParen) {
            self.regex_flags()?
        } else {
            Vec::new()
        };
        self.skip_separators();
        let mut items =
            self.regex_braced_sequence("expected '{' after regex", "expected '}' after regex")?;
        if let [RegexNode::ScopedFlags {
            enable,
            disable,
            body,
        }] = items.as_slice()
        {
            if disable.is_empty() {
                flags.extend(enable.iter().copied());
                items = body.clone();
            }
        }
        Ok(Expr::Regex(RegexPattern { flags, body: items }))
    }

    pub(super) fn regex_flags(&mut self) -> NodiaResult<Vec<RegexFlag>> {
        let mut flags = Vec::new();
        self.skip_separators();
        if !self.check(&TokenKind::RightParen) {
            loop {
                let token = self.advance().clone();
                let name = match token.kind {
                    TokenKind::Identifier(name) => name,
                    _ => {
                        return Err(NodiaError::new(
                            "expected regex flag name",
                            token.line,
                            token.column,
                        ))
                    }
                };
                let Some(flag) = RegexFlag::from_name(&name) else {
                    return Err(NodiaError::new(
                        format!("unknown regex flag '{name}'"),
                        token.line,
                        token.column,
                    ));
                };
                flags.push(flag);
                self.skip_separators();
                if !self.match_kind(&TokenKind::Comma) {
                    break;
                }
                self.skip_separators();
                if self.check(&TokenKind::RightParen) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightParen, "expected ')' after regex flags")?;
        Ok(flags)
    }

    pub(super) fn regex_braced_sequence(
        &mut self,
        start_message: &str,
        end_message: &str,
    ) -> NodiaResult<Vec<RegexNode>> {
        self.expect(TokenKind::LeftBrace, start_message)?;
        let items = self.regex_sequence()?;
        self.expect(TokenKind::RightBrace, end_message)?;
        Ok(items)
    }

    pub(super) fn regex_sequence(&mut self) -> NodiaResult<Vec<RegexNode>> {
        let mut items = Vec::new();
        self.skip_separators();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            items.extend(self.regex_sequence_item()?);
            self.skip_separators();
        }
        Ok(items)
    }

    pub(super) fn regex_sequence_item(&mut self) -> NodiaResult<Vec<RegexNode>> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::String(value) => Ok(vec![RegexNode::Literal(value)]),
            TokenKind::RawString(value) => {
                self.regex_text_sequence_item(value, token.line, token.column)
            }
            TokenKind::Identifier(name) => Ok(vec![self.regex_identifier_item(
                name,
                token.line,
                token.column,
            )?]),
            _ => Err(NodiaError::new(
                "expected regex item",
                token.line,
                token.column,
            )),
        }
    }

    pub(super) fn regex_text_sequence_item(
        &self,
        value: String,
        line: usize,
        column: usize,
    ) -> NodiaResult<Vec<RegexNode>> {
        let pattern = regex_api::parse_text(&value).map_err(|error| {
            let crate::error::NodiaError {
                code,
                message,
                context,
                span,
                ..
            } = error;
            let mut mapped = NodiaError::semantic_at(message, line, column).with_code(code);
            for context in context {
                mapped = mapped.with_context(context);
            }
            if let Some(span) = span {
                mapped = mapped.with_span(span.line, span.column);
            }
            mapped
        })?;
        Ok(self.regex_embed_pattern(pattern))
    }

    pub(super) fn regex_embed_pattern(&self, pattern: RegexPattern) -> Vec<RegexNode> {
        if pattern.flags.is_empty() {
            pattern.body
        } else {
            vec![RegexNode::ScopedFlags {
                enable: pattern.flags,
                disable: Vec::new(),
                body: pattern.body,
            }]
        }
    }

    pub(super) fn regex_identifier_item(
        &mut self,
        name: String,
        line: usize,
        column: usize,
    ) -> NodiaResult<RegexNode> {
        if let Some(anchor) = RegexAnchor::from_name(&name) {
            return Ok(RegexNode::Anchor(anchor));
        }
        if let Some(class) = RegexClass::from_name(&name) {
            return Ok(RegexNode::Class(class));
        }

        match name.as_str() {
            "any_char" => Ok(RegexNode::AnyChar),
            "any_codepoint" => Ok(RegexNode::AnyCodepoint),
            "literal" => Ok(RegexNode::Literal(self.regex_parenthesized_string(
                "expected '(' after literal",
                "expected string inside literal()",
                "expected ')' after literal()",
            )?)),
            "raw_regex" => Ok(RegexNode::Raw(
                self.regex_expect_string("expected string after raw_regex")?,
            )),
            "property" => Ok(RegexNode::Property {
                name: self.regex_expect_string("expected property name after property")?,
                negated: false,
            }),
            "not_property" => Ok(RegexNode::Property {
                name: self.regex_expect_string("expected property name after not_property")?,
                negated: true,
            }),
            "optional" => self.regex_quantified(RegexQuantifierKind::Optional),
            "zero_or_more" => self.regex_quantified(RegexQuantifierKind::ZeroOrMore),
            "one_or_more" => self.regex_quantified(RegexQuantifierKind::OneOrMore),
            "exactly" => {
                let count = self.regex_expect_usize("expected integer after exactly")?;
                self.regex_quantified(RegexQuantifierKind::Exactly(count))
            }
            "at_least" => {
                let count = self.regex_expect_usize("expected integer after at_least")?;
                self.regex_quantified(RegexQuantifierKind::AtLeast(count))
            }
            "between" => {
                let min = self.regex_expect_usize("expected minimum after between")?;
                self.expect(TokenKind::And, "expected 'and' in between quantifier")?;
                let max = self.regex_expect_usize("expected maximum after 'and'")?;
                self.regex_quantified(RegexQuantifierKind::Between(min, max))
            }
            "group" | "capture" => self.regex_group(RegexGroupKind::Capture),
            "non_capture" => self.regex_group(RegexGroupKind::NonCapture),
            "named" => {
                let name = self.expect_name_like("expected group name after named")?;
                self.regex_group(RegexGroupKind::Named(name))
            }
            "atomic" => self.regex_group(RegexGroupKind::Atomic),
            "either" => self.regex_either(),
            "char_set" => self.regex_char_set(false),
            "not_char_set" => self.regex_char_set(true),
            "followed_by" => self.regex_lookaround(RegexLookaroundKind::FollowedBy),
            "not_followed_by" => self.regex_lookaround(RegexLookaroundKind::NotFollowedBy),
            "preceded_by" => self.regex_lookaround(RegexLookaroundKind::PrecededBy),
            "not_preceded_by" => self.regex_lookaround(RegexLookaroundKind::NotPrecededBy),
            "if_capture" => self.regex_conditional_capture(),
            "if_followed_by" => self.regex_conditional_lookaround(RegexLookaroundKind::FollowedBy),
            "if_not_followed_by" => {
                self.regex_conditional_lookaround(RegexLookaroundKind::NotFollowedBy)
            }
            "if_preceded_by" => self.regex_conditional_lookaround(RegexLookaroundKind::PrecededBy),
            "if_not_preceded_by" => {
                self.regex_conditional_lookaround(RegexLookaroundKind::NotPrecededBy)
            }
            "if_matches" => self.regex_conditional_expression(),
            "same_as" => Ok(RegexNode::Reference(RegexReference::Named(
                self.expect_name_like("expected named group after same_as")?,
            ))),
            "same_as_group" => Ok(RegexNode::Reference(RegexReference::Group(
                self.regex_expect_usize("expected group index after same_as_group")?,
            ))),
            "call" => Ok(RegexNode::SubroutineCall(RegexReference::Named(
                self.expect_name_like("expected subroutine name after call")?,
            ))),
            "call_group" => Ok(RegexNode::SubroutineCall(RegexReference::Group(
                self.regex_expect_usize("expected subroutine index after call_group")?,
            ))),
            "fail" => Ok(RegexNode::BacktrackingVerb(RegexBacktrackingVerb::Fail)),
            "accept" => Ok(RegexNode::BacktrackingVerb(RegexBacktrackingVerb::Accept)),
            "commit" => Ok(RegexNode::BacktrackingVerb(RegexBacktrackingVerb::Commit)),
            "skip" => Ok(RegexNode::BacktrackingVerb(RegexBacktrackingVerb::Skip)),
            "prune" => Ok(RegexNode::BacktrackingVerb(RegexBacktrackingVerb::Prune)),
            "until" => self.regex_until(),
            "until_stop" => self.regex_until_stop(),
            "until_clear" => Ok(RegexNode::UntilClear),
            "define" => self.regex_define(),
            "with_flags" => self.regex_scoped_flags(true),
            "without_flags" => self.regex_scoped_flags(false),
            "branch" => Err(NodiaError::new(
                "branch is only valid inside either",
                line,
                column,
            )),
            "range" => Err(NodiaError::new(
                "range is only valid inside char_set",
                line,
                column,
            )),
            "lazy" | "possessive" => Err(NodiaError::new(
                "repeat mode must follow a quantifier",
                line,
                column,
            )),
            "char" => Err(NodiaError::new(
                "char() is only valid inside char_set",
                line,
                column,
            )),
            other => Err(NodiaError::new(
                format!("unknown regex item '{other}'"),
                line,
                column,
            )),
        }
    }

    pub(super) fn regex_quantified(
        &mut self,
        quantifier: RegexQuantifierKind,
    ) -> NodiaResult<RegexNode> {
        let mode = self.regex_repeat_mode();
        let target = self.regex_target()?;
        Ok(RegexNode::Quantifier {
            target: Box::new(target),
            kind: quantifier,
            mode,
        })
    }

    pub(super) fn regex_repeat_mode(&mut self) -> RegexQuantifierMode {
        match &self.peek().kind {
            TokenKind::Identifier(name) if name == "lazy" => {
                self.advance();
                RegexQuantifierMode::Lazy
            }
            TokenKind::Identifier(name) if name == "possessive" => {
                self.advance();
                RegexQuantifierMode::Possessive
            }
            _ => RegexQuantifierMode::Greedy,
        }
    }

    pub(super) fn regex_target(&mut self) -> NodiaResult<RegexNode> {
        self.skip_separators();
        if self.match_kind(&TokenKind::LeftBrace) {
            let items = self.regex_sequence()?;
            self.expect(
                TokenKind::RightBrace,
                "expected '}' after regex block target",
            )?;
            return Ok(RegexNode::Sequence(items));
        }
        let token = self.advance().clone();
        match token.kind {
            TokenKind::String(value) => Ok(RegexNode::Literal(value)),
            TokenKind::RawString(value) => {
                let items = self.regex_text_sequence_item(value, token.line, token.column)?;
                if items.len() == 1 {
                    Ok(items.into_iter().next().unwrap())
                } else {
                    Ok(RegexNode::Sequence(items))
                }
            }
            TokenKind::Identifier(name) => {
                self.regex_identifier_item(name, token.line, token.column)
            }
            _ => Err(NodiaError::new(
                "expected regex item",
                token.line,
                token.column,
            )),
        }
    }

    pub(super) fn regex_group(&mut self, kind: RegexGroupKind) -> NodiaResult<RegexNode> {
        let items = self.regex_braced_sequence(
            "expected '{' after regex group",
            "expected '}' after regex group",
        )?;
        Ok(RegexNode::Group { kind, body: items })
    }

    pub(super) fn regex_lookaround(&mut self, kind: RegexLookaroundKind) -> NodiaResult<RegexNode> {
        let items = self.regex_braced_sequence(
            "expected '{' after regex lookaround",
            "expected '}' after regex lookaround",
        )?;
        Ok(RegexNode::Lookaround { kind, body: items })
    }

    pub(super) fn regex_conditional_capture(&mut self) -> NodiaResult<RegexNode> {
        let reference = match self.peek().kind.clone() {
            TokenKind::Int(value) if value >= 0 => {
                self.advance();
                RegexReference::Group(value as usize)
            }
            _ => RegexReference::Named(
                self.expect_name_like("expected capture name or group index after if_capture")?,
            ),
        };
        self.regex_conditional(RegexCondition::Capture(reference))
    }

    pub(super) fn regex_conditional_lookaround(
        &mut self,
        kind: RegexLookaroundKind,
    ) -> NodiaResult<RegexNode> {
        let body = self.regex_braced_sequence(
            "expected '{' after conditional regex assertion",
            "expected '}' after conditional regex assertion",
        )?;
        self.regex_conditional(RegexCondition::Lookaround { kind, body })
    }

    pub(super) fn regex_conditional(
        &mut self,
        condition: RegexCondition,
    ) -> NodiaResult<RegexNode> {
        self.skip_separators();
        if matches!(&self.peek().kind, TokenKind::Identifier(name) if name == "then") {
            self.advance();
            self.skip_separators();
            let then_branch = self.regex_braced_sequence(
                "expected '{' after then",
                "expected '}' after then block",
            )?;
            self.skip_separators();
            let else_branch = if self.match_kind(&TokenKind::Else) {
                self.skip_separators();
                self.regex_braced_sequence(
                    "expected '{' after else",
                    "expected '}' after else block",
                )?
            } else {
                Vec::new()
            };
            Ok(RegexNode::Conditional {
                condition,
                then_branch,
                else_branch,
            })
        } else {
            Ok(RegexNode::Condition(condition))
        }
    }

    pub(super) fn regex_conditional_expression(&mut self) -> NodiaResult<RegexNode> {
        let body = self.regex_braced_sequence(
            "expected '{' after if_matches",
            "expected '}' after if_matches condition",
        )?;
        self.regex_conditional(RegexCondition::Expression(body))
    }

    pub(super) fn regex_either(&mut self) -> NodiaResult<RegexNode> {
        self.expect(TokenKind::LeftBrace, "expected '{' after either")?;
        let mut branches = Vec::new();
        self.skip_separators();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let branch = self.expect_identifier("expected branch inside either")?;
            if branch != "branch" {
                return Err(NodiaError::new(
                    "expected branch inside either",
                    self.previous().line,
                    self.previous().column,
                ));
            }
            branches.push(
                self.regex_braced_sequence(
                    "expected '{' after branch",
                    "expected '}' after branch",
                )?,
            );
            self.skip_separators();
        }
        self.expect(TokenKind::RightBrace, "expected '}' after either")?;
        Ok(RegexNode::Alternation(branches))
    }

    pub(super) fn regex_char_set(&mut self, negated: bool) -> NodiaResult<RegexNode> {
        self.expect(TokenKind::LeftBrace, "expected '{' after char_set")?;
        let mut items = Vec::new();
        self.skip_separators();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            items.push(self.regex_char_set_item()?);
            self.skip_separators();
        }
        self.expect(TokenKind::RightBrace, "expected '}' after char_set")?;
        Ok(RegexNode::CharSet(RegexCharSet { negated, items }))
    }

    pub(super) fn regex_char_set_item(&mut self) -> NodiaResult<RegexCharSetItem> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::String(value) | TokenKind::RawString(value) => {
                self.regex_char_set_char_sugar(value, token.line, token.column)
            }
            TokenKind::Identifier(name) => {
                if let Some(class) = RegexClass::from_name(&name) {
                    return Ok(RegexCharSetItem::Class(class));
                }
                match name.as_str() {
                    "property" => Ok(RegexCharSetItem::Property {
                        name: self.regex_expect_string("expected property name after property")?,
                        negated: false,
                    }),
                    "not_property" => Ok(RegexCharSetItem::Property {
                        name: self
                            .regex_expect_string("expected property name after not_property")?,
                        negated: true,
                    }),
                    "char" => {
                        let value = self.regex_parenthesized_string(
                            "expected '(' after char",
                            "expected string inside char()",
                            "expected ')' after char()",
                        )?;
                        self.regex_char_set_char_sugar(value, token.line, token.column)
                    }
                    "raw_regex" => Ok(RegexCharSetItem::Raw(
                        self.regex_expect_string("expected string after raw_regex")?,
                    )),
                    "range" => {
                        let start =
                            self.regex_expect_char_literal("expected char literal after range")?;
                        let token = self.advance().clone();
                        let keyword = match token.kind {
                            TokenKind::Identifier(name) => name,
                            _ => {
                                return Err(NodiaError::new(
                                    "expected 'to' in range",
                                    token.line,
                                    token.column,
                                ))
                            }
                        };
                        if keyword != "to" {
                            return Err(NodiaError::new(
                                "expected 'to' in range",
                                token.line,
                                token.column,
                            ));
                        }
                        let end =
                            self.regex_expect_char_literal("expected char literal after to")?;
                        Ok(RegexCharSetItem::Range(start, end))
                    }
                    other if RegexAnchor::from_name(other).is_some() => Err(NodiaError::new(
                        format!("'{}' is not valid inside char_set", other),
                        token.line,
                        token.column,
                    )),
                    "any_char" | "any_codepoint" | "with_flags" | "without_flags" => {
                        Err(NodiaError::new(
                            format!("'{}' is not valid inside char_set", name),
                            token.line,
                            token.column,
                        ))
                    }
                    other => Err(NodiaError::new(
                        format!("unknown char_set item '{other}'"),
                        token.line,
                        token.column,
                    )),
                }
            }
            _ => Err(NodiaError::new(
                "expected char_set item",
                token.line,
                token.column,
            )),
        }
    }

    pub(super) fn regex_scoped_flags(&mut self, enable_only: bool) -> NodiaResult<RegexNode> {
        self.expect(TokenKind::LeftParen, "expected '(' after scoped flags")?;
        let flags = self.regex_flags()?;
        self.skip_separators();
        let body = self.regex_braced_sequence(
            "expected '{' after scoped flags",
            "expected '}' after scoped flags",
        )?;
        Ok(RegexNode::ScopedFlags {
            enable: if enable_only {
                flags.clone()
            } else {
                Vec::new()
            },
            disable: if enable_only { Vec::new() } else { flags },
            body,
        })
    }

    pub(super) fn regex_until(&mut self) -> NodiaResult<RegexNode> {
        let limit =
            self.regex_braced_sequence("expected '{' after until", "expected '}' after until")?;
        self.skip_separators();
        let body = if matches!(&self.peek().kind, TokenKind::Identifier(name) if name == "then") {
            self.advance();
            self.skip_separators();
            Some(self.regex_braced_sequence(
                "expected '{' after then",
                "expected '}' after then block",
            )?)
        } else {
            None
        };
        Ok(RegexNode::Until { limit, body })
    }

    pub(super) fn regex_until_stop(&mut self) -> NodiaResult<RegexNode> {
        Ok(RegexNode::UntilStop(self.regex_braced_sequence(
            "expected '{' after until_stop",
            "expected '}' after until_stop",
        )?))
    }

    pub(super) fn regex_define(&mut self) -> NodiaResult<RegexNode> {
        Ok(RegexNode::DefineGroup {
            body: self
                .regex_braced_sequence("expected '{' after define", "expected '}' after define")?,
        })
    }

    pub(super) fn regex_parenthesized_string(
        &mut self,
        start_message: &str,
        value_message: &str,
        end_message: &str,
    ) -> NodiaResult<String> {
        self.expect(TokenKind::LeftParen, start_message)?;
        self.skip_separators();
        let value = self.regex_expect_string(value_message)?;
        self.skip_separators();
        self.expect(TokenKind::RightParen, end_message)?;
        Ok(value)
    }

    pub(super) fn regex_char_set_char_sugar(
        &self,
        value: String,
        line: usize,
        column: usize,
    ) -> NodiaResult<RegexCharSetItem> {
        let mut chars = value.chars();
        let Some(ch) = chars.next() else {
            return Err(NodiaError::new(
                "char_set string sugar expects exactly one character",
                line,
                column,
            ));
        };
        if chars.next().is_some() {
            return Err(NodiaError::new(
                "char_set string sugar expects exactly one character; use multiple entries or raw_regex",
                line,
                column,
            ));
        }
        Ok(RegexCharSetItem::Char(ch))
    }

    pub(super) fn regex_expect_string(&mut self, message: &str) -> NodiaResult<String> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::String(value) | TokenKind::RawString(value) => Ok(value),
            _ => Err(NodiaError::new(message, token.line, token.column)),
        }
    }

    pub(super) fn regex_expect_char_literal(&mut self, message: &str) -> NodiaResult<char> {
        let token = self.advance().clone();
        let value = match token.kind {
            TokenKind::String(value) | TokenKind::RawString(value) => value,
            _ => return Err(NodiaError::new(message, token.line, token.column)),
        };
        let mut chars = value.chars();
        let Some(ch) = chars.next() else {
            return Err(NodiaError::new(
                "expected single character literal",
                token.line,
                token.column,
            ));
        };
        if chars.next().is_some() {
            return Err(NodiaError::new(
                "expected single character literal",
                token.line,
                token.column,
            ));
        }
        Ok(ch)
    }

    pub(super) fn regex_expect_usize(&mut self, message: &str) -> NodiaResult<usize> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Int(value) if value >= 0 => Ok(value as usize),
            _ => Err(NodiaError::new(message, token.line, token.column)),
        }
    }
}
