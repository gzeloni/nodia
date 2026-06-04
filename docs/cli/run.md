# `nodia run`

Execute a `.nod` file (or stdin source).

```bash
nodia run file.nod
nodia run file.nod --var key=value
nodia run file.nod --vars key1=value1 key2=value2 ...
nodia run file.nod --out output.txt
nodia run file.nod --allow-write
nodia run file.nod --allow-env
nodia run file.nod --allow-process
nodia run file.nod -- one two
nodia run -                          # read source from stdin
nodia run                            # uses nodia.toml entry
```

## Minimal Example

`hello.nod`:

```nodia
val name = input.name
emit "Hello, {name}"
```

```bash
./target/release/nodia run hello.nod --var name=Ana
```

```text
Hello, Ana
```

## CLI Variables

### `--var`

Pass a single `key=value` pair. Repeatable:

```bash
./target/release/nodia run hello.nod --var name=Ana --var env=prod
```

### `--vars`

Pass multiple `key=value` pairs after a single flag:

```bash
./target/release/nodia run hello.nod --vars name=Ana env=prod owner=gzeloni
```

### Variables File

`--vars` also accepts a JSON or YAML file path. The file must be a flat
key/value document:

`vars.json`:

```json
{ "app": "nodia", "limit": 3, "enabled": true }
```

```bash
./target/release/nodia run app.nod --vars vars.json
```

`vars.yaml`:

```yaml
app: nodia
env: prod
```

```bash
./target/release/nodia run app.nod --vars vars.yaml
```

### Access From Code

All variables land in the read-only `input` map:

```nodia
emit input.app
emit input.env
```

JSON files preserve typed scalars (`string`, `int`, `float`, `bool`, `null`).
`--var` / inline `--vars` always pass strings.

## Script Arguments

Arguments after `--` are exposed through the read-only `args` list:

```bash
./target/release/nodia run script.nod -- one two
```

`script.nod`:

```nodia
emit args
emit args[1]
```

```text
["one", "two"]
two
```

## Source From Stdin

Pass `-` as the file argument:

```bash
printf 'emit upper("nodia")\n' | ./target/release/nodia run -
```

```text
NODIA
```

## Writing The Output Channel To A File

`emit` writes to Nodia's program output channel. By default that is the
process stdout. The `--out` / `-o` flag redirects it to a file:

```bash
./target/release/nodia run report.nod --out report.txt
./target/release/nodia run report.nod -o report.txt
```

When `--out` is given without a path, Nodia writes beside the source path with
the suffix `.out`:

```bash
./target/release/nodia run report.nod --out
# writes report.nod.out
```

`--stdout` is the explicit form of the default behavior:

```bash
./target/release/nodia run report.nod --stdout
```

`--out` does **not** require `--allow-write`. It is a CLI feature, not a
language-level write.

## Writing Files From Code

Functions like `write(path, text)`, `append(path, text)`, and
`open(path, "write")` are gated by `--allow-write`:

```bash
./target/release/nodia run transform.nod --allow-write
```

Without it, the program fails with `E3001`.

## Environment Access

`env(...)` requires `--allow-env`:

```bash
HOME=/tmp ./target/release/nodia run script.nod --allow-env
```

## Process Execution

`exec(...)` requires `--allow-process`:

```bash
./target/release/nodia run script.nod --allow-process
```

## Project Entry

Without a file argument, `nodia run` walks upward looking for `nodia.toml` and
runs its `entry`:

```bash
cd demo
../target/release/nodia run --var name=Project
```

See [Project Layout](../getting-started/projects.md).
