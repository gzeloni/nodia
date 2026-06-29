// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023-2025 Gustavo Zeloni <gustavo@gzeloni.dev>

//! Regression tests for the regex DSL and runtime.

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
fn renders_conditional_regex_nodes() {
    let pattern = RegexPattern {
        flags: Vec::new(),
        body: vec![
            RegexNode::Quantifier {
                target: Box::new(RegexNode::Group {
                    kind: RegexGroupKind::Capture,
                    body: vec![RegexNode::Literal("a".to_string())],
                }),
                kind: RegexQuantifierKind::Optional,
                mode: RegexQuantifierMode::Greedy,
            },
            RegexNode::Literal("b".to_string()),
            RegexNode::Conditional {
                condition: RegexCondition::Capture(RegexReference::Group(1)),
                then_branch: vec![RegexNode::Literal("c".to_string())],
                else_branch: vec![RegexNode::Literal("d".to_string())],
            },
        ],
    };

    let rendered = render(&pattern).unwrap();
    assert_eq!(rendered, "(a)?b(?(1)c|d)");
}

#[test]
fn compiled_regex_reports_matches_with_scalar_offsets() {
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

#[test]
fn regex_replace_all_supports_nodia_placeholders() {
    let regex = compile_text("(?<name>[A-Za-z]+)").unwrap();

    let output = regex.replace_all("ana bob", "<$(name):$(0)>").unwrap();

    assert_eq!(output, "<ana:ana> <bob:bob>");
}

#[test]
fn regex_replace_all_rejects_unknown_named_capture() {
    let regex = compile_text("(?<name>[A-Za-z]+)").unwrap();

    let err = regex.replace_all("ana", "$(missing)").unwrap_err();

    assert_eq!(
        err.span.as_ref().map(|span| (span.line, span.column)),
        Some((1, 1))
    );
    assert!(err
        .to_string()
        .contains("regex replacement refers to missing named capture 'missing'"));
}

#[test]
fn regex_replace_all_uses_empty_string_for_unmatched_branch_capture() {
    let regex = compile_text("(?:(?<word>[A-Za-z]+)|(?<num>\\d+))").unwrap();

    let output = regex.replace_all("ana 42", "<$(word):$(num)>").unwrap();

    assert_eq!(output, "<ana:> <:42>");
}

#[test]
fn regex_split_returns_unmatched_segments() {
    let regex = compile_text("\\s+").unwrap();

    let parts = regex.split("ana   bruno\tcarla").unwrap();

    assert_eq!(parts, vec!["ana", "bruno", "carla"]);
}

#[test]
fn duplicate_named_groups_are_rejected() {
    let pattern = RegexPattern {
        flags: Vec::new(),
        body: vec![
            RegexNode::Group {
                kind: RegexGroupKind::Named("x".to_string()),
                body: vec![RegexNode::Literal("a".to_string())],
            },
            RegexNode::Group {
                kind: RegexGroupKind::Named("x".to_string()),
                body: vec![RegexNode::Literal("b".to_string())],
            },
        ],
    };

    let err = validate(&pattern).unwrap_err();

    assert_eq!(err.code, "E4200");
    assert!(err.message.contains("duplicate named capture 'x'"));
}

#[test]
fn parses_classic_regex_text_back_into_native_ast() {
    let pattern = parse_text(r"(?i)^\d{2}(?:-|/)(?<month>\d{2})$").unwrap();

    assert_eq!(pattern.flags, vec![RegexFlag::CaseInsensitive]);
    assert_eq!(
        render(&pattern).unwrap(),
        r"(?i)^\d{2}(?:-|/)(?<month>\d{2})$"
    );
}

#[test]
fn parses_classic_regex_conditionals_and_python_named_forms() {
    let conditional = parse_text(r"(a)?b(?(1)c|d)").unwrap();
    assert_eq!(render(&conditional).unwrap(), r"(a)?b(?(1)c|d)");

    let assertion = parse_text(r"(?(?=foo)foo|bar)").unwrap();
    assert_eq!(render(&assertion).unwrap(), r"(?:(?=foo)foo|(?!foo)bar)");

    let python_named = parse_text(r"(?P<word>[A-Za-z]+)\s+(?P=word)").unwrap();
    assert_eq!(
        render(&python_named).unwrap(),
        r"(?<word>[A-Za-z]+)\s+\k<word>"
    );
}

#[test]
fn parses_classic_regex_text_canonicalizes_known_classes() {
    let pattern = parse_text(r"[\s\S]+[A-Za-z]").unwrap();

    assert_eq!(
        pattern.body,
        vec![
            RegexNode::Quantifier {
                target: Box::new(RegexNode::AnyCodepoint),
                kind: RegexQuantifierKind::OneOrMore,
                mode: RegexQuantifierMode::Greedy,
            },
            RegexNode::Class(RegexClass::Letter),
        ]
    );
}

#[test]
fn parses_classic_regex_text_with_properties_quotes_and_strong_anchors() {
    let pattern = parse_text(r"\A\p{Greek}+\x41\Q.+\E\z").unwrap();

    assert_eq!(
        pattern.body,
        vec![
            RegexNode::Anchor(RegexAnchor::StartText),
            RegexNode::Quantifier {
                target: Box::new(RegexNode::Property {
                    name: "Greek".to_string(),
                    negated: false,
                }),
                kind: RegexQuantifierKind::OneOrMore,
                mode: RegexQuantifierMode::Greedy,
            },
            RegexNode::Literal("A.+".to_string()),
            RegexNode::Anchor(RegexAnchor::EndText),
        ]
    );
    assert_eq!(render(&pattern).unwrap(), r"\A\p{Greek}+A\.\+\z");
}

#[test]
fn parses_fancy_only_regex_features_via_fallback_converter() {
    let toggled = parse_text(r"abc(?i)def").unwrap();
    assert_eq!(render(&toggled).unwrap(), r"abc(?i:def)");

    let subroutine = parse_text(r"(?<num>\d+) x \g<num>").unwrap();
    assert_eq!(render(&subroutine).unwrap(), r"(?<num>\d+) x \g<num>");

    let until = parse_text(r"(?~END)").unwrap();
    assert_eq!(render(&until).unwrap(), r"(?~END)");

    let boundary = parse_text(r"\b{start}\b{end-half}").unwrap();
    assert_eq!(render(&boundary).unwrap(), r"\b{start}\b{end-half}");

    let defined = parse_text(r"(?(DEFINE)(?<word>[A-Za-z]+))\g<word>").unwrap();
    assert_eq!(
        render(&defined).unwrap(),
        r"(?(DEFINE)(?<word>[A-Za-z]+))\g<word>"
    );
}

#[test]
fn rejects_unsupported_classic_regex_features_instead_of_silently_degrading() {
    let err = parse_text(r"\X").unwrap_err();
    assert!(err.message.contains(r"\X"));
}
