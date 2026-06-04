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
- `input` global
- `#` and `//` line comments

## Stdlib Notes

The extension follows the current modular stdlib model:

```nodia
use text
use json
use csv
use re

emit text.upper("ana")
emit json.read(r'{"name":"Ana"}').name
emit re.find("ana 42", regex { one_or_more digit }).text
```

Global helpers such as `json_parse`, `csv_read`, `upper`, `read`, `stdin`, and
`stdout` are intentionally not suggested anymore because they are no longer the
public language surface.
