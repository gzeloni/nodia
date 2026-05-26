# Your First Program

This page walks through a minimal Nodia program: declaring values, reading
CLI input, and emitting structured output.

## Inline With `eval`

The fastest way to run Nodia code is `eval`:

```bash
./target/release/nodia eval 'emit "hello, world"'
```

```text
hello, world
```

`eval` takes a source string, lexes, parses, checks, and runs it. It is
intended for ad-hoc shell use and for the examples on this site.

## A Real File

Create `hello.nod`:

```nodia
val name = input.name

emit "hello, {capitalize(name)}"
```

Run it with one variable:

```bash
./target/release/nodia run hello.nod --var name=ana
```

```text
hello, Ana
```

Several variables:

```bash
./target/release/nodia run hello.nod --vars name=ana env=prod
```

`input` is a read-only map populated by the CLI. See
[Variables](../language/variables.md#cli-input) for the full input model.

## Multiple Statements

Nodia statements are separated by newlines (or `;`):

```bash
./target/release/nodia eval '
val a = 2
val b = 3
emit "a + b = {a + b}"
emit "a * b = {a * b}"
'
```

```text
a + b = 5
a * b = 6
```

## Reading Stdin

Pass `-` as the file to read source from stdin:

```bash
printf 'emit upper("nodia")\n' | ./target/release/nodia run -
```

```text
NODIA
```

## Catching Errors Early

Use `check` to lex, parse, resolve uses, and run the v0.5 semantic checks
without executing the program:

```bash
./target/release/nodia check hello.nod
```

```text
ok hello.nod
```

Errors come back as structured diagnostics with a stable code (`E1000`,
`E2000`, `E3001`, `E4100`, ...). See [Diagnostics](../reference/diagnostics.md).

## Next Steps

* [Project layout](projects.md)
* [Language tour](../language/source.md)
* [Standard library](../stdlib/index.md)
