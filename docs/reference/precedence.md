# Operator Precedence

From **lowest** to **highest**:

| Level | Operators                                | Associativity |
| ----- | ---------------------------------------- | ------------- |
| 1     | `or`                                     | left          |
| 2     | `and`                                    | left          |
| 3     | `==`, `!=`                               | left          |
| 4     | `<`, `<=`, `>`, `>=`                     | left          |
| 5     | `+`, `-`                                 | left          |
| 6     | `*`, `/`, `%`                            | left          |
| 7     | unary `-`, `not`                         | right         |
| 8     | call `f(...)`, field `.x`, index `[...]` | left          |
| 9     | literals, identifiers, grouped exprs     | n/a           |

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
