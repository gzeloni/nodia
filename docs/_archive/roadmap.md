# Nodia Text-First Roadmap

This roadmap replaces the earlier static-first and self-hosting-first plan.
Nodia's primary goal is now clear: become the best language for working with
text.

That means reaching Perl-level practical power while being meaningfully better
than Perl in readability, semantic clarity, diagnostics, and long-term
maintainability.

Nodia should win where text work is messy in real life:

- generation of structured text;
- transformation of raw text into clean output;
- parsing and classification of semi-structured input;
- regex-heavy workflows that still need to remain readable;
- large-file processing without accidental memory blowups;
- predictable scripting for files, prompts, reports, logs, config, and data
  interchange.

## 1. Product Objective

Nodia is not trying to become a broad general-purpose platform first. It is not
trying to become a certification-oriented language first. It is not trying to
become self-hosted first.

Its first serious objective is narrower and more valuable:

> Nodia should be the most coherent language for text work, from one-liners and
> generators to robust transformation pipelines.

The language should feel:

- as practical as Perl for text-heavy tasks;
- more legible than Perl by default;
- more semantically explicit than Python-style ad hoc scripting;
- more predictable than shell pipelines glued together with fragile quoting;
- more specialized for text than general-purpose languages.

## 2. Strategic Positioning

Nodia should compete on four fronts at the same time:

| Front | Required outcome |
|---|---|
| Raw text power | Handle real-world text problems, not only clean demo strings. |
| Semantic clarity | Avoid magical coercions, hidden lossy behavior, and implicit text assumptions. |
| Readable regex and parsing | Keep regex first-class, but do not force regex to solve everything. |
| Tooling discipline | Make formatting, diagnostics, and testing part of the product value. |

If Nodia becomes only elegant, it loses to Perl in capability.
If Nodia becomes only powerful, it risks repeating Perl's worst tradeoffs.
The roadmap therefore prioritizes power and semantic discipline together.

## 3. Current Baseline: Nodia 0.6.2

Nodia 0.6.2 already has a credible text-oriented base:

- `emit` as a dedicated output channel;
- interpolated strings, raw strings, and triple-quoted blocks;
- a readable native `regex { ... }` DSL;
- regex builtins such as `find`, `find_all`, `replace`, `split`, and
  `full_match`;
- modules, closures, maps, lists, JSON, CSV, and file IO;
- a semantic checker that already catches common script mistakes before runtime;
- deterministic formatting and deterministic map ordering.

But the current baseline still has important limits:

- text is still mostly a flat runtime string model;
- there is no recoverable error model for serious pipelines;
- large-file processing is still too materialized;
- parsing beyond regex is weak;
- real-world text formats are under-covered;
- Unicode and encoding semantics are not yet strong enough to be a defining
  advantage.

## 4. Non-Negotiable Principles

Every future feature should reinforce these principles.

### 4.1 Text Semantics Must Be Explicit

Nodia must define what a text value is, how it is indexed, how it is sliced,
what happens with invalid encoding, and when a transformation is lossy.

No hidden ambiguity should remain around:

- bytes versus text;
- character count versus byte count;
- scalar values versus grapheme clusters;
- normalization and case-folding behavior;
- newline conventions;
- regex offsets and slice boundaries.

### 4.2 Text Pipelines Must Be Recoverable

Real text work fails in the middle:

- invalid UTF-8;
- malformed JSON or CSV;
- mismatched captures;
- missing files;
- partial lines;
- invalid dates;
- broken external command output.

Nodia needs structured recoverable errors, not only fatal runtime stops.

### 4.3 Regex Must Stay A Differentiator

The regex DSL is already one of Nodia's strongest ideas. It should remain a
signature feature.

But regex must be surrounded by the right ecosystem:

- better match objects;
- streaming regex operations;
- diagnostics for patterns and captures;
- parser-friendly APIs when regex is the wrong tool.

### 4.4 Streaming Must Be A First-Class Mode

Text languages fail when every task assumes whole-file materialization.

Nodia must support both:

- small, simple whole-value text scripting;
- scalable streaming for large or unbounded input.

### 4.5 The Standard Library Must Solve Real Formats

Being good at text means handling the formats people actually meet:

- JSON;
- CSV and TSV;
- Markdown;
- HTML;
- XML;
- TOML and INI;
- URLs and query strings;
- front matter;
- log lines;
- email-like headers;
- diffs and patches.

### 4.6 Tooling Is Part Of The Language

If Nodia wants to beat older text languages, it must not stop at syntax.

It needs:

- excellent diagnostics;
- canonical formatting;
- test support for text-heavy output;
- a strong REPL or inspector loop;
- editor feedback that understands text workflows.

## 5. What Nodia Must Beat Perl At

Nodia does not need to imitate Perl. It needs to outperform Perl in the right
dimensions.

| Dimension | Nodia target |
|---|---|
| Readability | A script should stay understandable after six months. |
| Semantics | Text, regex, errors, and conversions should behave by explicit rules. |
| Diagnostics | Failures should point to exact locations and likely fixes. |
| Structure | Medium-sized text projects should remain maintainable. |
| Safety | Fewer silent traps in parsing, replacement, indexing, and IO. |
| Tooling | Formatting, checking, testing, and exploration should feel native. |

Perl may still remain shorter in some micro-scripts. That is acceptable. The
goal is not shortest syntax. The goal is the best end-to-end experience for
serious text work.

## 6. Release Discipline

This roadmap assumes release trains, not heroic jumps.

The problem to avoid is simple: if every new concept gets promoted into the next
headline version immediately, the project accumulates half-finished semantics,
temporary APIs, stale docs, and unverified interactions between features.

Nodia therefore needs a stricter release policy.

### 6.1 Versioning Model

| Version form | Purpose |
|---|---|
| `0.6.x` | Hardening, diagnostics, docs, tests, low-risk stdlib growth, and groundwork that does not force a new semantic chapter. |
| `0.7.0`, `0.8.0`, ... | Opening release of a new roadmap line after the previous line is closed. |
| `1.0` | Compatibility milestone, not a marketing milestone. |

In other words, `0.x.0` releases are roadmap chapters, not places to dump every
unfinished idea that happened to be under discussion.

### 6.2 Required States For Every Roadmap Line

Each roadmap line must pass through four states:

1. Foundation: introduce the minimum coherent primitives.
2. Adoption: use those primitives in stdlib, diagnostics, and examples.
3. Stabilization: remove awkward edges, close semantic gaps, and harden tests.
4. Closure: publish migration guidance, freeze the surface, and only then move
   to the next line.

No line is complete while it still contains:

- provisional syntax with known replacement plans;
- duplicated temporary APIs left behind for convenience;
- undocumented behavioral edge cases;
- checker gaps that make the new surface fragile in practice;
- cookbook and reference drift.

### 6.3 Gates Between Lines

Nodia should not move from one roadmap line to the next unless all of the
following are true:

- the previous line has complete reference documentation;
- formatter behavior is settled for the new syntax, if any;
- diagnostics cover the most common misuse cases;
- regression tests exist for both normal and messy input;
- migration notes exist for any user-visible semantic change;
- at least one stabilization release has already shipped for that line.

This is especially important between the text-semantics, error-model, and
streaming lines. Those three areas interact too strongly to be treated as
single-release jumps.

## 7. Roadmap Lines

The roadmap below is ordered by strategic necessity, but paced so that each line
has room for cleanup before the next one begins.

### 7.1 `0.6.x`: Baseline Hardening And Contract Cleanup

Purpose: make the current language trustworthy before introducing new semantic
weight.

`0.6.3`

- synchronize public docs with the real `0.6.2` behavior;
- fix version drift and stale examples;
- strengthen regression tests for strings, regex, JSON, CSV, IO, and format.

`0.6.4`

- document current text indexing, string slicing, and regex offset behavior in
  precise terms;
- classify current behavior into stable, provisional, and known-limitation
  buckets;
- tighten diagnostics around interpolation, indexing, and regex placeholders.

`0.6.5`

- clean up current regex and text inconsistencies that would block later text
  semantics work;
- add missing cookbook cases for messy input, not only idealized examples;
- benchmark current whole-text workflows to establish a baseline.

`0.6.6`

- harden JSON, CSV, and IO behavior where current text semantics are already
  clear;
- reduce undocumented edge cases in file reading, invalid Unicode handling, and
  replacement behavior;
- freeze the `0.6` contract that future lines will build on.

Exit criteria:

- no major documentation drift against the implementation;
- no ambiguity around the current string and regex baseline;
- a regression corpus exists for both happy-path and messy text inputs;
- the project can start `0.7.0` without carrying unresolved `0.6` cleanup debt.

### 7.2 `0.7.x`: Text Semantics Line

Purpose: make text behavior explicit, predictable, and better than the default
string semantics inherited from the host runtime.

`0.7.0`

- introduce the first explicit text-semantics primitives without redesigning the
  whole stdlib at once;
- define the official conceptual model for text, bytes, characters, and
  boundaries;
- establish compatibility rules for old APIs that currently expose plain string
  behavior.

`0.7.1`

- add normalization and case-folding helpers;
- define equality and comparison expectations for normalization-aware text
  operations;
- improve docs and examples so users understand when normalization matters.

`0.7.2`

- add safer slicing and indexing modes for byte, scalar, and grapheme-aware
  access where appropriate;
- standardize offset terminology across regex, slicing, and diagnostics;
- add better failure messages for invalid boundaries.

`0.7.3`

- introduce explicit encode/decode APIs and newline normalization helpers;
- define how invalid decoding is represented and reported;
- add sanitation helpers for messy input pipelines.

`0.7.4`

- adopt the new text semantics across stdlib text, regex, JSON, CSV, and format
  APIs where needed;
- remove or deprecate transitional helpers that no longer make sense;
- update cookbook coverage to reflect the new recommended idioms.

`0.7.5`

- stabilization release for the whole text-semantics line;
- close naming inconsistencies;
- finish migration notes and compatibility guidance.

Exit criteria:

- important text transformations no longer depend on unspecified behavior;
- lossy text operations are explicit at the call site;
- terminology is consistent across docs, checker, runtime, and stdlib;
- no unresolved transitional API remains open before `0.8.0`.

### 7.3 `0.8.x`: Recoverable Errors And Reliable Pipelines

Purpose: let real text pipelines survive bad input without degenerating into
silent failure or fatal exits.

`0.8.0`

- introduce the core recoverable error model through `Result`, `try/catch`, or
  an equivalent structured mechanism;
- define the difference between fatal runtime failure and recoverable pipeline
  failure;
- establish the canonical error shape for text-oriented operations.

`0.8.1`

- integrate the new error model into IO, regex execution, decoding, JSON, CSV,
  and datetime parsing;
- remove ad hoc failure behavior where structured recovery is now expected;
- add checker and runtime diagnostics for the most common misuse patterns.

`0.8.2`

- add pipeline helpers such as `ok`, `err`, `unwrap_or`, `map_err`, and
  predictable fallback patterns;
- document idiomatic recovery flows for partial parse, skip-and-continue, and
  classify-and-report pipelines;
- add examples using messy input, not only ideal input.

`0.8.3`

- improve error context, spans, and related diagnostics for nested text
  transformations;
- refine regex capture and replacement failures;
- tighten interoperability between structured errors and the output model.

`0.8.4`

- stabilization release for the error line;
- remove temporary duplication between old fatal behavior and new recoverable
  behavior where migration is complete;
- publish firm guidance for library authors and script authors.

Exit criteria:

- non-trivial text pipelines can recover, skip, classify, or report failures;
- structured parsing failures no longer require fatal termination by default;
- error behavior is consistent enough that streaming can build on it cleanly.

### 7.4 `0.9.x`: Streaming And Bounded-Memory Text Processing

Purpose: scale Nodia from small scripts to large-file and long-running text
workflows.

`0.9.0`

- introduce stream-oriented text primitives and chunk or line iteration;
- define the relationship between whole-text APIs and streaming APIs;
- make memory expectations explicit in docs and diagnostics where practical.

`0.9.1`

- add lazy or stream-oriented `map`, `filter`, and reduction patterns;
- support stream transforms for line-based and record-based workflows;
- ensure the model works with the recoverable error surface from `0.8.x`.

`0.9.2`

- add streaming regex search, splitting, and replacement where the semantics are
  clear and safe;
- integrate streaming with CSV-like and log-like parsing workflows;
- benchmark memory behavior on representative text workloads.

`0.9.3`

- harden backpressure, flushing, chunk-boundary, and partial-record behavior;
- improve diagnostics for misuse of stream and whole-text APIs;
- document where streaming semantics differ intentionally from materialized
  semantics.

`0.9.4`

- stabilization release for the streaming line;
- remove redundant early APIs if the final model made them obsolete;
- publish performance and memory guidance.

Exit criteria:

- Nodia can process large inputs without reading everything into memory first;
- streaming and whole-text APIs feel like one language, not two competing
  styles;
- the line is stable enough that parser and format work can depend on it.

### 7.5 `0.10.x`: Pattern Matching And Structural Text Modeling

Purpose: make extracted text easier to classify, branch on, and validate.

`0.10.0`

- introduce `match` / `case` for structured branching;
- define the minimal semantics for branching on text-derived structures;
- keep the first surface intentionally small.

`0.10.1`

- add destructuring for lists, maps, and regex match objects;
- improve ergonomics for optional and partial fields;
- tighten checker support for obvious shape errors.

`0.10.2`

- add light structural modeling for parsed text records;
- improve safe field access and branch exhaustiveness where feasible;
- expand cookbook coverage for routing semi-structured data.

`0.10.3`

- stabilization release for the structural line;
- remove awkward syntax corners and inconsistent branch behavior;
- finish migration guidance for code that previously relied on nested `if`
  chains.

Exit criteria:

- routing semi-structured text no longer relies on fragile nested conditionals;
- parsed text shapes are easier to understand and safer to evolve.

### 7.6 `0.11.x`: Parsing Toolkit Beyond Regex

Purpose: give Nodia a serious parsing story when regex is not enough.

`0.11.0`

- introduce tokenizer or scanner primitives;
- standardize span-carrying parse outputs;
- define the relationship between regex parsing and token parsing.

`0.11.1`

- add parser combinators or a lightweight PEG-style layer;
- support layered parsing workflows from text to tokens to structured output;
- keep the API small enough to remain readable.

`0.11.2`

- improve parse diagnostics, error spans, and recovery stories;
- add helpers for delimited records, front matter, header blocks, and other
  common textual structures;
- validate interaction with the `0.8.x` error model and `0.9.x` streaming
  model.

`0.11.3`

- stabilization release for the parsing line;
- remove regex abuse cases from official examples where better parsing tools now
  exist;
- publish style guidance on when to use regex, scanners, or parser combinators.

Exit criteria:

- users are no longer forced to abuse regex for every parsing problem;
- parser-oriented code is readable, diagnosable, and stream-compatible.

### 7.7 `0.12.x`: Real-World Formats Standard Library

Purpose: cover the formats that define practical text work.

`0.12.0`

- choose the first official format modules with the highest practical value;
- likely priorities are Markdown, TOML, URL handling, and richer log parsing;
- define consistent parse and serialize conventions across format modules.

`0.12.1`

- add HTML and XML support with careful scope control;
- improve schema-like and messy-input ergonomics for JSON and CSV;
- add structured header and key-value helpers.

`0.12.2`

- add front matter, INI, and adjacent small text formats;
- improve deterministic serializers for generated text output;
- expand cookbook coverage for cross-format workflows.

`0.12.3`

- stabilization release for the formats line;
- remove naming drift and inconsistent option patterns between modules;
- publish guidance on what belongs in the core stdlib versus future packages.

Exit criteria:

- common text-automation work can stay inside Nodia's official toolbox;
- the format modules feel coherent instead of like unrelated utilities.

### 7.8 `0.13.x`: Text-Centric Tooling

Purpose: make the workflow around the language strong enough to compete with
older scripting ecosystems.

`0.13.0`

- introduce `nodia test` with snapshot and golden-output support;
- ensure test tooling fits generated-text workflows naturally;
- define a stable output-diff experience.

`0.13.1`

- add a REPL or inspector focused on text, regex, and parsed structures;
- support quick exploration of intermediate values, captures, and formatted
  output;
- integrate existing checker and formatter behavior.

`0.13.2`

- improve editor tooling for regex blocks, captures, text pipelines, and format
  modules;
- add benchmark fixtures for representative text workloads;
- publish comparisons against Python, Perl, and shell for real use cases.

`0.13.3`

- stabilization release for the tooling line;
- remove rough workflow edges that prevent daily usage;
- ensure examples, docs, and editor feedback all teach the same idioms.

Exit criteria:

- users can iterate on text transformations quickly and confidently;
- Nodia is easier to validate and refine than traditional text scripting
  stacks.

### 7.9 `1.0`: Text-First Leadership Release

Purpose: freeze the identity only after the text story is coherent end to end.

`1.0` should happen only after the previous lines are not merely implemented,
but cleaned up.

Release criteria:

- stable semantics for text, regex, parsing, streaming, and recoverable
  pipeline errors;
- a mature text-focused standard library with consistent naming and error
  behavior;
- strong docs and cookbook coverage for real-world text tasks;
- performance and memory guidance for both small and large workflows;
- compatibility guarantees for the core text surface;
- no large transitional API layer still waiting to be removed in `1.1`.

`1.0` is the point where Nodia can honestly claim:

- Perl-level leverage for text-heavy work;
- clearer semantics than Perl;
- better maintainability than ad hoc scripting stacks;
- a text workflow that feels complete rather than promising.

## 8. Deferred Until After Text Leadership

These topics may matter later, but they are not current roadmap drivers:

- self-hosting as a primary milestone;
- certification-oriented subsets;
- generalized high-integrity profiles;
- ownership-heavy language design as a headline feature;
- bytecode or native backends as identity work;
- broad general-purpose platform goals unrelated to text dominance.

They should only be prioritized when they clearly strengthen the text-first
mission rather than compete with it.

## 9. Success Definition

The roadmap succeeds when the answer to these questions is "yes":

1. Can Nodia handle messy real-world text, not only clean examples?
2. Does it provide Perl-level leverage without Perl-level semantic chaos?
3. Can medium-sized text projects remain readable and maintainable?
4. Can users process large inputs without accidentally writing memory-heavy
   scripts?
5. Does the standard library cover the formats text engineers actually face?
6. Do diagnostics and tooling make iteration faster than older scripting
   languages?

If the answer is still "no" to any of these, the roadmap is not complete.
