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
- `{expr}` interpolation inside strings
- Numbers, operators, punctuation
- Stdlib-aware completions for `use`
- Member completions for imported stdlib namespaces such as `json.read()` and `csv.write()`
- Regex DSL suggestions inside `regex { ... }`
- `nodia fmt` on save for `.nod` files
- `nodia check` diagnostics while editing and after save
- `input` global
- `#` and `//` line comments

## Stdlib Notes

The extension follows the current modular stdlib model and the `0.7.5`
parameterized API surface:

```nodia
use text
use io
use format
use datetime
use json
use csv
use re

emit text.upper("ana")
emit text.decode(io.read("payload.bin", io.bytes), text.utf8, text.lossy)
emit format.pad("42", 5, format.left, "0")
emit datetime.parse("2024-01-31T23:00:00Z", datetime.as_datetime)
emit json.read(r'{"name":"Ana"}').name
emit re.test("abc-42", "^[a-z]+-\\d+$", re.full)
emit re.find("ana 42", regex { one_or_more digit }, re.all)
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
