# Nodia Design Principles

This document defines the design foundation for Nodia. Language features, tooling,
standard library additions, and syntax changes should be evaluated against these principles.

## 1. Readability Over Concision

Nodia favors readability and canonical formatting over concision.

A feature that saves a few characters but makes code harder to scan, format, teach,
or diagnose should not enter the language. Nodia code should be obvious before it is clever.

Simplicity belongs primarily in syntax and structure. Builtin names should be short, technical,
and predictable instead of overly humanized.

## 2. Canonical Formatting Is Part Of The Language

There should be one canonical way to format Nodia code. The formatter is not only a
convenience tool; it is part of the language contract.

Style choices should not become project-level debates. If valid Nodia code can be formatted,
the formatter decides the layout.

## 3. Explicit Syntax Beats Magic

Nodia should avoid hidden context, implicit mode switches, special sigils, and syntax that
changes meaning based on where it appears.

Values should not behave differently because of invisible scalar/list/string contexts. Imports,
bindings, control flow, and output should stay explicit enough to read without memorizing tricks.

## 4. Tooling Is A First-Class Contract

The CLI, formatter, checker, diagnostics, token output, and AST output are part of Nodia's
public surface. Tooling behavior should be predictable enough for editors, CI, and automation.

A language feature is incomplete until it has reasonable formatting, diagnostics, and tests.

## 5. Focused Power, Not General Purpose

Nodia is a focused high-level language for mathematical and textual automation.

It may grow deeply in those domains without trying to become a universal application platform,
systems language, or kitchen-sink scripting environment.

Features are welcome when they strengthen text generation, mathematical reasoning, data
transformation, reproducibility, and tooling. Features that mainly push Nodia toward systems
programming, broad application frameworks, or an oversized standard library should be rejected.

## 6. One Good Path

Nodia should avoid adding multiple equivalent ways to express the same idea. As the language
grows, it benefits from stronger conventions and less stylistic drift.

When two designs are both viable, prefer the one that is easier to format, explain, and debug.
