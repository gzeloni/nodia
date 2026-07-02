# Diff Builtins (REMOVED from stdlib)

> **Deprecated**: The `diff` module has been removed from the Rust standard
> library. It will be reimplemented in Nodia itself in the next major version.
> This documentation is kept for reference only.

Nodia exposes unified patch text through `use diff`.

```nodia
use diff
```

## `diff.read(text_or_bytes)`

Parses a practical unified diff subset into one document map:

```nodia
{
  files: [
    {
      old: "a.txt",
      new: "b.txt",
      hunks: [
        {
          header: "-1,2 +1,2",
          lines: [
            {kind: "context", text: "keep"},
            {kind: "delete", text: "old"},
            {kind: "add", text: "new"},
          ],
        },
      ],
    },
  ],
}
```

```bash
./target/release/nodia eval '
use diff

val patch = diff.read("""--- a.txt
+++ b.txt
@@ -1 +1 @@
-old
+new
""")
emit patch.files[0].new
emit patch.files[0].hunks[0].lines[1].kind
emit diff.write(patch)
'
```

```text
b.txt
delete
--- a.txt
+++ b.txt
@@ -1 +1 @@
-old
+new
```

Supported today:

* `--- old` / `+++ new` file headers;
* unified hunk headers `@@ ... @@`;
* context, add, and delete lines;
* ignored `\ No newline at end of file` markers.

Deliberately not supported in this first cut:

* context diff format;
* binary patch payloads;
* git metadata as structured fields.

Caught diff parse failures expose `context = ["diff.read"]` and nested `span`
details.

## `diff.write(document)`

Serializes the same unified diff document shape back into patch text.
