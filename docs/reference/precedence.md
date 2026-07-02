# Operator Precedence

From **lowest** to **highest**:

| Level | Operators                                | Associativity |
| ----- | ---------------------------------------- | ------------- |
| 1     | `or`                                     | left          |
| 2     | `and`                                    | left          |
| 3     | `==`, `!=`                               | left          |
| 4     | `<`, `<=`, `>`, `>=`                     | left          |
| 5     | `|`                                      | left          |
| 6     | `^`                                      | left          |
| 7     | `&`                                      | left          |
| 8     | `<<`, `>>`                               | left          |
| 9     | `+`, `-`                                 | left          |
| 10    | `*`, `/`, `%`                            | left          |
| 11    | unary `-`, `not`, `~`                    | right         |
| 12    | call `f(...)`, field `.x`, index `[...]` | left          |
| 13    | literals, identifiers, grouped exprs     | n/a           |

Use parentheses to force order:

```bash
./target/release/nodia eval '
emit 1 + 2 * 3
emit (1 + 2) * 3
'
```

```text
7
9
```

For narrative examples of each operator, see
[Operators](../language/operators.md).
