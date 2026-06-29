# Nodia Language for VS Code

Local VS Code editor support for `.nod` files.

## Install Locally

1. Open VS Code.
2. Run `Developer: Install Extension from Location...`.
3. Select this folder:

```text
vscode/nodia-language
```

## Supported

- `.nod` file association
- Current language keywords
- Raw strings and triple-quoted strings
- Byte strings
- `{expr}` interpolation inside strings
- Numbers, operators, punctuation
- Stdlib-aware completions for `use`
- Member completions for imported stdlib namespaces such as `result.raise()`, `json.read()`, and `csv.write()`
- Builtin regex completions such as `regex.find()`, `regex.test()`, `regex.replace()`, and `regex.split()` without any import
- Regex DSL suggestions inside `regex { ... }`, including `property`, `define`, `call`, `until`, and backtracking verbs
- `nodia fmt` on save for `.nod` files
- `nodia check` diagnostics while editing and after save
- `input` global
- `#` and `//` line comments

## Editor Matrix

This repository also ships a local Zed integration under:

```text
zed/nodia
```

VS Code remains the richer editor integration today because it already shells
out to `nodia fmt` and `nodia check`. The Zed side currently focuses on
Tree-sitter parsing, highlighting, brackets, indentation, and outline support.

## Stdlib Notes

The extension follows the current modular stdlib model and the current
parameterized API surface:

```nodia
use text
use io
use format
use result
use datetime
use json
use csv

emit text.upper("ana")
emit result.raise(text.decode(result.raise(io.read("payload.bin", io.bytes)), text.utf8, text.lossy))
emit format.pad("42", 5, format.left, "0")
emit result.raise(datetime.parse("2024-01-31T23:00:00Z", datetime.as_datetime))
emit result.raise(json.read(r'{"name":"Ana"}')).name
emit result.raise(regex.test("abc-42", "^[a-z]+-\\d+$", regex.full))
emit result.raise(regex.find("ana 42", regex { one_or_more digit }, regex.all))
```

Global helpers such as `json_parse`, `csv_read`, `upper`, `read`, `stdin`, and
`stdout` are intentionally not suggested anymore because they are no longer the
public language surface.

## CLI Integration

The extension uses the real `nodia` executable for formatting and checking.

Configuration:

- `nodia.executablePath`: explicit path to the binary. When empty, the extension
  tries `target/debug/nodia`, then `target/release/nodia`, then `nodia` from `PATH`.
- `nodia.formatOnSave`: run `nodia fmt` before saving.
- `nodia.enableChecker`: run `nodia check` and publish diagnostics.
- `nodia.checkerDelayMs`: debounce for checker reruns after edits.
