# Nodia Technical Evolution Roadmap

This document defines the technical direction for Nodia after v0.3. It is not a
feature wishlist. It is the engineering path required to evolve Nodia from its
current interpreter-oriented implementation into a statically checked,
predictable, self-hosted language with a credible path toward high-integrity and
certifiable software development.

Nodia's long-term objective is to be simple at the source level and rigorous
under the surface. The language should remain approachable to non-specialists,
but its implementation, semantics, tooling, and runtime model must be designed
with the discipline expected from serious production languages.

The first major milestone is self-hosting: a Nodia compiler written in Nodia
must be able to compile itself and produce reproducible equivalent artifacts
across bootstrap stages.

## 1. Language Thesis

Nodia is a statically checked, safe, high-level language for textual automation,
mathematical work, structured data transformation, tooling, and eventually
high-integrity software subsets.

The language should optimize for:

- simple and readable syntax;
- canonical formatting;
- static correctness before execution;
- deterministic behavior;
- explicit effects;
- absence of undefined behavior;
- strong diagnostics;
- strong REPL ergonomics;
- reproducible builds;
- self-contained static executables;
- a future certifiable subset suitable for safety-critical domains.

Nodia should not optimize for syntactic cleverness, implicit context, dynamic
metaprogramming, runtime magic, or compatibility with existing scripting language
habits when those habits weaken predictability.

## 2. Non-Negotiable Semantic Properties

Nodia must define all observable behavior. A valid Nodia program may fail with a
well-defined diagnostic or runtime error, but it must not enter undefined
behavior.

Required properties:

| Property | Requirement |
|---|---|
| Undefined behavior | Not permitted by the language semantics. |
| Integer overflow | Defined by profile: checked by default, optionally proven absent. |
| Division by zero | Defined runtime error or proven absent. |
| Null access | Only possible through explicit optional types. |
| Index access | Checked or statically proven within bounds. |
| Memory safety | No use-after-free, double free, dangling reference, or data race. |
| IO | Explicitly modeled as an effect. |
| Concurrency | Structured and data-race-free by construction. |
| Formatting | Canonical and non-configurable. |
| Build output | Reproducible for the same compiler, input, target, and profile. |

## 3. Memory Model

Nodia should not rely on a tracing garbage collector as a core language
requirement. The memory model must be deterministic enough to support static
analysis, predictable resource usage, and high-integrity profiles.

The target direction is automatic memory management through static ownership,
region-based allocation, borrow-like checking, and compiler-inserted destruction
where appropriate.

The user-facing model should remain simple:

```nod
val text = read("input.txt")
val rows = lines(text)
val result = transform(rows)
write("output.txt", result)
```

The implementation model may use:

- ownership for values with exclusive lifetime;
- immutable sharing where safe;
- affine or move-only resources for files, streams, locks, and handles;
- lexical destruction for resources;
- arenas or regions for short-lived compiler/runtime allocations;
- escape analysis for stack or region allocation;
- reference counting only where explicitly justified;
- no mandatory global GC.

High-integrity profiles must be able to restrict allocation after initialization,
require bounded memory regions, and reject programs whose memory behavior cannot
be statically bounded or proven acceptable.

## 4. Language Profiles

Nodia should support multiple profiles instead of forcing one execution model for
all programs.

| Profile | Purpose | Characteristics |
|---|---|---|
| `script` | Everyday automation and REPL usage. | Broad standard library, IO available, dynamic conveniences where still statically checked. |
| `core` | General Nodia programs. | Static types, modules, effects, deterministic semantics. |
| `safe` | High-assurance application code. | Restricted effects, stronger checks, no uncontrolled allocation or concurrency. |
| `flight` | Certifiable subset. | Bounded memory, explicit effects, formal contracts, traceable artifacts, restricted runtime. |

Profiles are not dialects. They are progressively stricter subsets of the same
language. Code accepted by a stricter profile should remain valid in broader
profiles unless profile-specific target constraints apply.

## 5. Syntax Direction

Nodia's current syntax is serviceable, but it carries too many habits from
Python-like languages. Future syntax should preserve approachability while giving
Nodia its own identity.

Preferred direction:

| Concept | Direction |
|---|---|
| Immutable binding | `val` |
| Mutable binding | `var` |
| Function | `func` |
| Module use | `use` |
| Namespace alias | `as` |
| Selective use | `pick` |
| Exclusion use | `hide` |
| Output | `emit` |
| Pattern branch | `match` / `case` |
| Optional value | `T?` |
| Error handling | typed `try` / `catch` / `raise` or result-style equivalent |
| Contracts | `requires`, `ensures`, `invariant` |

Example target style:

```nod
use "./stats" as stats
use "./text" pick title, slug

func report(name: Text, values: List<Int>) -> Text
  requires len(values) > 0

  val label = title(name)
  val total = stats.sum(values)
  val mean = stats.avg(values)

  emit """
  Report: {label}
  Slug: {slug(label)}
  Total: {total}
  Mean: {mean}
  """
end
```

The syntax must remain easy to format. A feature that cannot be formatted
canonically should not be accepted.

## 6. Type System

Nodia must move from runtime-shaped values toward a static type system with local
inference.

Initial type universe:

| Type | Notes |
|---|---|
| `Null` | Only inhabits optional types directly. |
| `Bool` | Boolean logic, no truthiness for arbitrary values. |
| `Int` | Defined width or arbitrary precision by profile decision. |
| `Float` | IEEE behavior explicitly documented or replaced by decimal/rational profiles. |
| `Text` | Unicode semantics explicitly defined. |
| `List<T>` | Homogeneous list. |
| `Map<K, V>` | Typed keys and values. |
| `Stream<T>` | Linear or affine resource type. |
| `Func<A, B>` | Function type. |
| `Result<T, E>` | Typed recoverable error channel if selected. |
| `Option<T>` / `T?` | Explicit absence. |

Rules:

- inference should be local and explainable;
- public APIs should prefer explicit types;
- implicit numeric coercions should be minimal or absent;
- no implicit string/list/scalar context;
- nullability must be explicit;
- mutation must be visible through `var` or mutable containers;
- resources such as streams should not be freely copyable.

## 7. Effects And IO

IO must be real, stream-capable, and statically visible. Nodia should not hide
side effects behind ordinary-looking pure functions.

The compiler should eventually track effects such as:

- filesystem read;
- filesystem write;
- standard input;
- standard output;
- environment access;
- time;
- randomness;
- network access;
- process spawning;
- unsafe foreign calls, if ever introduced.

A possible function signature direction:

```nod
func copy(input: Path, output: Path) -> Result<Int, IoError>
  effects fs.read, fs.write
end
```

Profiles may restrict effects. For example, `flight` may disallow filesystem IO
in critical code or require it to be represented through target-specific certified
interfaces.

## 8. Error Model

Errors must be typed and explicit. Nodia should avoid unchecked exception-style
control flow as the primary model.

The error model should support:

- parse errors;
- semantic errors;
- type errors;
- effect errors;
- runtime errors;
- recoverable application errors;
- internal compiler errors.

Diagnostics must include:

- stable error code;
- source file;
- line and column;
- short message;
- primary span;
- optional related spans;
- optional machine-readable output;
- suggestion when the compiler can be precise.

## 9. Module System

Modules must be deterministic, resilient, and suitable for large projects.

Requirements:

- uses are resolved from the source file containing the use;
- package uses are resolved through `nodia.toml`;
- use cycles are allowed only where semantically safe;
- declaration cycles and initialization cycles are distinct;
- used names are explicit;
- namespace uses are preferred for larger modules;
- module initialization order is defined;
- build graphs are reproducible.

Target syntax:

```nod
use "./parser" as parser
use "./token" pick Token, TokenKind
use "./internal" hide debug_only
```

## 10. Compiler Architecture

The compiler should evolve into a staged pipeline with stable internal contracts.

Target pipeline:

```text
source
  -> tokens
  -> concrete syntax tree
  -> abstract syntax tree
  -> resolved module graph
  -> typed AST
  -> checked effect graph
  -> canonical IR
  -> optimized IR
  -> bytecode or native backend
  -> linked artifact
```

Each stage should be testable and serializable where practical. This is required
for bootstrapping, editor tooling, regression testing, and long-term
certifiability.

The IR must be:

- deterministic;
- textual or binary with a stable debug representation;
- independent from parser implementation details;
- suitable for bytecode generation;
- suitable for future native code generation;
- simple enough to be emitted by a compiler written in Nodia.

## 11. Runtime And Distribution

Nodia should produce self-contained static binaries where supported by the target
platform. Running a Nodia program should not require a C runtime as an external
language-level dependency.

Runtime responsibilities:

- value representation;
- text representation;
- stream/resource management;
- panic/runtime error reporting;
- math primitives;
- deterministic collections;
- platform abstraction;
- optional REPL support;
- bytecode VM, until native backends are mature.

The runtime must remain small, auditable, and profile-aware. The `flight` profile
must be able to use a restricted runtime with bounded behavior.

## 12. REPL Requirements

The REPL is not a toy shell. It must use the same parser, type checker, module
resolver, and evaluator/compiler pipeline as normal files.

Required REPL capabilities:

- incremental compilation;
- persistent typed session state;
- imports;
- function and type definitions;
- multiline editing;
- inspect type of expression;
- inspect formatted representation;
- inspect generated IR;
- run tests or examples inline;
- recover cleanly from parse/type errors;
- no semantic drift from file execution.

## 13. Tooling Contract

Tooling is part of the language, not an accessory.

Required commands:

| Command | Purpose |
|---|---|
| `nodia run` | Compile and execute a source file. |
| `nodia build` | Produce an artifact. |
| `nodia check` | Parse, resolve, type-check, and effect-check. |
| `nodia fmt` | Apply canonical formatting. |
| `nodia test` | Run tests. |
| `nodia repl` | Start the interactive environment. |
| `nodia doc` | Generate documentation. |
| `nodia pkg` | Package management operations. |
| `nodia lsp` | Language server entrypoint. |
| `nodia ir` | Emit compiler IR for debugging and tests. |

The formatter must remain non-configurable. The checker must become fast enough
to be used continuously by editors.

## 14. Mathematical Direction

Nodia should become strong in practical mathematics without becoming a large
scientific framework by default.

Core direction:

- exact integers where practical;
- explicit fixed-width integers where needed;
- decimal or rational types for exact user-facing arithmetic;
- matrices and vectors as library-level abstractions first;
- statistical primitives;
- ranges and sequences;
- deterministic random number generation when requested;
- clear floating-point semantics;
- optional proof support for bounds and invariants.

Mathematical code must preserve the language principles: explicitness,
readability, deterministic formatting, and defined behavior.

## 15. Text Direction

Text is a primary domain for Nodia. Unicode behavior must be explicit and stable.

Required direction:

- define `Text` encoding and indexing semantics;
- distinguish bytes, scalar values, and grapheme clusters where needed;
- provide safe slicing APIs;
- provide parser-friendly text primitives;
- provide structured templates;
- support streaming text processing;
- avoid implicit lossy conversions.

Text APIs must be short, technical, and predictable.

## 16. High-Integrity And Certification Path

Nodia cannot declare itself certified. It can be designed to support certification
efforts by producing evidence and by restricting programs to analyzable subsets.

The long-term high-integrity path should align with ideas used by safety-critical
software ecosystems: requirements traceability, formal contracts, static analysis,
coverage evidence, reproducible builds, and small auditable runtimes.

Required artifacts for stricter profiles:

- compiler version and profile metadata;
- resolved dependency graph;
- formatted source snapshot;
- typed AST or equivalent semantic representation;
- effect report;
- allocation report;
- proof obligations;
- test report;
- coverage report where applicable;
- build provenance;
- reproducible artifact hash.

The `flight` profile should eventually support:

- no undefined behavior;
- no implicit allocation in critical sections;
- bounded loops or termination evidence where required;
- contracts for public functions;
- absence of runtime exceptions where provable;
- restricted standard library;
- deterministic compilation;
- target-specific certified runtime integration.

## 17. Version Roadmap

### v0.4: Language Specification Baseline

- Freeze and document the current grammar.
- Define precedence, scopes, modules, imports, mutability, and errors.
- Establish a formal AST schema.
- Make formatter behavior part of the language contract.
- Build an official corpus of valid and invalid programs.

Exit criterion: all implemented behavior has a documented language-level rule.

### v0.5: Identity And Syntax Revision

- Introduce the target syntax direction: `val`, `var`, `func`, `use`.
- Keep compatibility shims for current syntax where reasonable.
- Define reserved words for the next language era.
- Remove Python-like decisions that do not fit Nodia's identity.
- Update formatter and diagnostics for the revised syntax.

Exit criterion: new code can be written in the Nodia identity without depending
on legacy syntax.

### v0.6: Static Type System

- Add explicit type annotations.
- Add local type inference.
- Define primitive and collection types.
- Add optional types.
- Remove arbitrary truthiness.
- Add typed function signatures.

Exit criterion: ordinary programs are checked before execution and type errors do
not reach runtime.

### v0.7: Functions, Resources, And Ownership Foundation

- Stabilize named functions, recursion, and return typing.
- Introduce resource types for streams and file handles.
- Prevent accidental copying of resources.
- Add lexical destruction for resources.
- Define the first ownership and borrowing rules.

Exit criterion: IO-heavy programs can be safe without a global garbage collector.

### v0.8: Module Graph And Package Manifest

- Make `nodia.toml` a real package manifest.
- Implement deterministic module resolution.
- Separate declaration cycles from initialization cycles.
- Add package-level build graph validation.
- Prepare module metadata for editor tooling.

Exit criterion: multi-file projects compile predictably and incrementally.

### v0.9: Effects And Typed Errors

- Track IO and other side effects in the compiler.
- Introduce typed recoverable errors.
- Separate semantic errors from runtime failures.
- Add effect-aware diagnostics.
- Add profile-based effect restrictions.

Exit criterion: the compiler can explain what a program may do, not only what it
computes.

### v0.10: REPL And Incremental Compiler

- Build the REPL on top of the real compiler pipeline.
- Preserve typed session state.
- Support imports, multiline definitions, and diagnostics recovery.
- Add inspection commands for types, formatting, and IR.

Exit criterion: the REPL behaves like an incremental Nodia project, not a
separate interpreter mode.

### v0.11: Canonical IR

- Introduce a stable intermediate representation.
- Lower typed AST into IR.
- Add textual IR snapshots.
- Implement conservative IR validation.
- Make IR simple enough to generate from Nodia later.

Exit criterion: execution no longer depends directly on AST walking.

### v0.12: Bytecode And VM

- Define Nodia bytecode.
- Implement a deterministic VM.
- Compile IR to bytecode.
- Link modules into bytecode artifacts.
- Make `run` equivalent to compile plus execute.

Exit criterion: bytecode is the primary execution format.

### v0.13: Static Artifact Generation

- Produce self-contained executable artifacts where supported.
- Separate runtime, VM, standard library, and user bytecode cleanly.
- Add reproducible artifact hashing.
- Add release-mode checks for determinism.

Exit criterion: Nodia programs can be distributed without requiring the source
tree or host language toolchain.

### v0.14: Mathematical Core

- Add exact and profile-defined numeric types.
- Add deterministic numeric semantics.
- Add vectors, matrices, ranges, and statistical primitives.
- Add overflow and bounds proof hooks.

Exit criterion: mathematical code is expressive, predictable, and statically
checked.

### v0.15: Text Core

- Define Unicode behavior precisely.
- Add streaming text APIs.
- Add safe slicing and parsing primitives.
- Add structured templates.
- Add text-focused diagnostics and tests.

Exit criterion: Nodia becomes excellent at text without relying on ad hoc string
behavior.

### v0.16: `safe` Profile

- Restrict effects.
- Restrict allocation behavior.
- Require stronger typing on public APIs.
- Add contract checking.
- Add concurrency rules if concurrency exists by this stage.

Exit criterion: a meaningful subset can be audited and reasoned about statically.

### v0.17: `flight` Profile Prototype

- Add profile-specific standard library restrictions.
- Add allocation reports.
- Add effect reports.
- Add proof obligation generation.
- Add traceability metadata.
- Add deterministic build reports.

Exit criterion: Nodia can produce the kinds of artifacts a high-integrity process
would need, even before full certification maturity.

### v0.18: Compiler In Nodia, Stage 1

- Implement lexer in Nodia.
- Implement parser in Nodia.
- Define Nodia-native AST structures.
- Compare Rust compiler AST against Nodia compiler AST.
- Run the official corpus through both implementations.

Exit criterion: Nodia can parse itself using code written in Nodia.

### v0.19: Compiler In Nodia, Stage 2

- Implement module resolution in Nodia.
- Implement name resolution in Nodia.
- Implement type checking in Nodia.
- Implement effect checking in Nodia.
- Emit canonical IR from Nodia.

Exit criterion: Nodia compiler code can produce the same checked IR as the host
compiler for the official corpus.

### v0.20: Compiler In Nodia, Stage 3

- Implement IR validation in Nodia.
- Implement bytecode generation in Nodia.
- Implement module linking in Nodia.
- Build real projects with the Nodia compiler.

Exit criterion: a compiler written in Nodia can compile Nodia programs to
bytecode.

### v0.21: Bootstrap

- Use the host compiler to compile the Nodia compiler.
- Use the resulting compiler to compile the Nodia compiler again.
- Compare stage artifacts.
- Require deterministic equivalent output.
- Document the bootstrap chain.

Exit criterion: stage 1 and stage 2 produce equivalent reproducible compiler
artifacts.

### v1.0: First Self-Hosted Release

- Make the Nodia compiler the official compiler implementation.
- Keep the previous host compiler as a reference and recovery path.
- Publish the bootstrap process.
- Publish language, IR, bytecode, and runtime specifications.
- Freeze compatibility expectations for the v1 series.

Exit criterion: Nodia is self-hosted and has a stable language contract.

## 18. Bootstrap Definition

Nodia reaches the end of its first stage when this chain works reliably:

```text
host compiler
  -> compiles compiler written in Nodia
    -> generated compiler compiles compiler written in Nodia
      -> generated compiler artifact is reproducibly equivalent
```

This is the first major language maturity boundary. After that point, the project
can evolve as a self-hosted language rather than as a language implemented only in
its original host.
