# `init` & `version`

## `nodia init`

Scaffold a new Nodia project.

```bash
nodia init
nodia init demo
nodia init demo --json
```

In the target directory (current directory if omitted) `init` creates:

```text
nodia.toml
src/
  main.nod
```

`nodia.toml`:

```toml
name = "nodia-project"
entry = "src/main.nod"
```

`src/main.nod`:

```nodia
val name = input.name

emit "Hello, {name}"
```

`init` is idempotent: it creates only files that don't already exist and
never overwrites an existing `nodia.toml` or `src/main.nod`.

JSON output:

```bash
./target/release/nodia init demo --json
```

```json
{"ok":true,"path":"demo"}
```

See [Project Layout](../getting-started/projects.md) for what `nodia.toml`
controls today.

## `nodia version`

Print version metadata.

```bash
./target/release/nodia version
```

```text
nodia 0.7.1
```

JSON form:

```bash
./target/release/nodia version --json
```

```json
{"name":"nodia","version":"0.7.1","rust_std_only":false}
```

`rust_std_only: false` confirms that this build of Nodia uses targeted
third-party crates for regex execution and explicit Unicode
normalization/case-folding.
