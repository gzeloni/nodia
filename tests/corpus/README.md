# Dobra v0.4 Corpus

This directory contains the official v0.4 language corpus.

The corpus is intentionally small at first. It exists to make language changes
explicit: every new syntax or semantic rule should add or update corpus files.

## Layout

| Path | Meaning |
|---|---|
| `valid/` | Programs that should lex and parse successfully. Some may require runtime flags to execute. |
| `invalid/lex/` | Programs that should fail during lexing. |
| `invalid/parse/` | Programs that should fail during parsing. |
| `invalid/semantic/` | Programs that should parse successfully but fail v0.4 semantic checks. |
| `invalid/runtime/` | Programs that pass `check` but should fail at runtime. |

## v0.4 Policy

`dobra check` performs v0.4 semantic checks. Files under `invalid/runtime/`
are expected to pass `check` and fail only when executed.
