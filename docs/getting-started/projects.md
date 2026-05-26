# Project Layout

A Nodia project is any directory containing a `nodia.toml` file. When a CLI
command does not receive a file path, Nodia walks up from the current directory
looking for `nodia.toml` and uses the `entry` field.

## Scaffolding

```bash
./target/release/nodia init demo
```

This generates:

```text
demo/
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

Running `init` in an existing directory only creates missing files; it never
overwrites an existing `nodia.toml` or `src/main.nod`.

## Discovery

With `nodia.toml` in place, you can run the entry without naming it:

```bash
cd demo
../target/release/nodia run --var name=Project
```

```text
Hello, Project
```

This works because `run` resolved `entry = "src/main.nod"` from `nodia.toml`.

## Supported Keys

| Key     | Meaning                                          |
| ------- | ------------------------------------------------ |
| `name`  | Project name. Currently informational only.      |
| `entry` | Entry `.nod` file used when no file path is given. |

Additional keys are reserved for future versions and are not consumed today.

## Recommended Layout

```text
project/
  nodia.toml
  src/
    main.nod        # entry
  lib/
    text.nod        # local module
    format.nod      # local module
  data/
    fixtures.json
```

Use relative `use` paths inside `.nod` sources to import modules — see
[Modules (use)](../language/modules.md).

## Formatter Across The Project

`fmt` accepts a single file or a directory:

```bash
./target/release/nodia fmt .
./target/release/nodia fmt --check .
```

When formatting a directory, Nodia recursively visits `.nod` files and skips
the `target/` directory.
