# Markup Builtins (REMOVED from stdlib)

> **Deprecated**: The `markdown`, `html`, and `xml` modules have been removed
> from the Rust standard library. They will be reimplemented in Nodia itself in
> the next major version. This documentation is kept for reference only.

Nodia now exposes lightweight readers and writers for Markdown, HTML, and XML.
These are intentionally small structural surfaces, not full spec implementations.

## Markdown

```nodia
use markdown
```

### `markdown.read(text_or_bytes)`

Parses a small block-level Markdown subset into a list of block maps.

Supported block kinds:

* heading: `{kind: "heading", level: 1, text: "Title"}`
* paragraph: `{kind: "paragraph", text: "..." }`
* list: `{kind: "list", items: ["a", "b"]}`
* code: `{kind: "code", lang: "nodia", text: "..." }`

```bash
./target/release/nodia eval '
use markdown

val doc = markdown.read("""
# Title

one
two

- alpha
- beta
""")
emit doc[0].kind
emit doc[1].text
emit markdown.write(doc)
'
```

Caught parse failures expose `context = ["markdown.read"]` and nested `span`
details.

### `markdown.write(blocks)`

Serializes the same block list shape back into Markdown text.

## HTML / XML

```nodia
use html
use xml
```

### `html.read(text_or_bytes)` / `xml.read(text_or_bytes)`

Parses a shared XML-style node model:

* element node:
  `{kind: "element", name: "div", attrs: {class: "note"}, children: [...]}`
* text node:
  `{kind: "text", text: "hello"}`
* comment node:
  `{kind: "comment", text: "done"}`

```bash
./target/release/nodia eval '
use html

val doc = html.read(r"<div class=\"note\"><span>hi</span><!--done--></div>")
emit doc[0].name
emit doc[0].attrs.class
emit html.write(doc)
'
```

```text
div
note
<div class="note"><span>hi</span><!--done--></div>
```

Supported today:

* nested elements;
* quoted attributes;
* self-closing tags;
* comments;
* common entities such as `&lt;`, `&gt;`, and `&amp;`.

Deliberately not supported in this first cut:

* HTML void-tag inference;
* doctypes as structured nodes;
* mixed lax browser parsing rules.

### `html.write(nodes)` / `xml.write(nodes)`

Serializes either one node map or a list of node maps back into text.

Both writers share the same structural contract.
