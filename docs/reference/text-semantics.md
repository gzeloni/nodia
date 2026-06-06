# Text Positions & Slices

This page defines the current `0.6.4` baseline for text positions in Nodia:

* direct indexing with `value[index]`;
* sequence slicing with `slice(value, start, end)`;
* regex match offsets returned by `find(...)` and `find_all(...)`.

It documents the implementation **as it exists today**. It is not a preview of
the future `0.7.x` text-semantics line.

## Stable In `0.6.x`

These behaviors are part of the `0.6.x` baseline and should be treated as the
current contract.

| Surface | Current rule |
| --- | --- |
| `list[index]` | zero-based; negative indexes count from the end |
| `get(list, index, default)` | same index normalization as `list[index]`, but returns `default` instead of raising |
| `slice(list, start, end)` | `start` inclusive, `end` exclusive; negative bounds count from the end; bounds are clamped into the valid range |
| `slice(string, start, end)` | same bound rules as list slicing |
| `find(...).start` / `find(...).end` | offsets use the same position counting as `len(string)` and `slice(string, ...)` |

`slice(...)` returns an empty list or empty string when the normalized `end`
falls before the normalized `start`.

## Provisional In `0.6.x`

These behaviors are exposed today but are intentionally documented as
provisional because the later text-semantics line may revisit them.

| Surface | Current rule | Why provisional |
| --- | --- | --- |
| `string[index]` | zero-based and returns a one-character string | unlike list indexing, negative indexes do **not** count from the end |
| text position counting | `len(string)`, `slice(string, ...)`, and regex offsets count **Unicode scalar values** | this is explicit now, but grapheme-cluster semantics remain future work |

In practice, that means a decomposed text sequence such as `é` currently counts
as two positions, not one grapheme cluster.

## Known Limitations

These are real limitations of the current `0.6.x` model, not hidden behavior.

| Area | Current limitation |
| --- | --- |
| direct string indexing | `text[-1]` is out of bounds; use `get(text, -1, default)` or `slice(text, start, end)` when you need from-end behavior |
| grapheme awareness | there is no grapheme-cluster indexing, slicing, or regex offset API yet |
| byte awareness | there is no byte-offset or byte-slice API in the public language surface |
| interpolation parser | interpolation currently scans until the next `}`; expressions that themselves need nested balanced braces are still fragile in `0.6.x` |

## Worked Baseline

```bash
./target/release/nodia eval '
emit ["a", "b", "c"][-1]
emit get("nodia", -1, "?")
emit slice("nodia", -99, 99)
emit len(slice("nodia", 4, 2))
'
```

```text
c
a
nodia
0
```

Regex offsets align with the same string-position model:

```bash
./target/release/nodia eval '
val text = "éx"
val hit = find(text, regex { "x" })
emit hit.start
emit hit.end
emit slice(text, 0, hit.start)
'
```

```text
2
3
é
```
