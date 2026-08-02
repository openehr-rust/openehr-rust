# assets

**Generated. Do not edit by hand.**

```sh
cd openehr-assets
cargo run -- write   # regenerate
cargo run -- check   # fail if what is committed is stale
```

CI runs `check` on every push, so a schema or dialect change that forgets to
regenerate fails the build rather than leaving a stale file in the tree.

| File | Is |
| --- | --- |
| `schema.json` | the shared five-table schema as data — `schema.rs` derives `Serialize` for exactly this |
| `ddl/<engine>.sql` | the DDL each dialect emits, one file per engine |
| `column-type-matrix.md` | every logical column type as all six engines spell it, side by side |

## Why these are committed

A diff is the cheapest review there is, and this repository's recurring defect is
a change whose consequences nobody saw.

The schema is declared in Rust (`db:G2.7`) and each dialect renders it. Nothing
in a pull request shows what the *SQL* became — a reviewer would have to run six
commands, and reviewers do not. With the DDL committed, a changed type mapping is
a changed `.sql` file.

**And two dialects becoming the same thing shows up as two identical files.**
That is the point. `openehr-mariadb` shipped as a name-substituted copy of
`openehr-mysql`, and the thing that would have revealed it at review time was
seeing the two scripts side by side (`spec/audit.md` **W-01**).

The matrix serves the same end more directly: `mysql` and `mariadb` share every
type spelling, which is legal and documented — they are separated by index
idempotence and trigger syntax, not by types — but it should be a *deliberate*
agreement a reader can see, not a surprise.

## What this is not

Not `db:G2.1`, which required generating DDL **from** openEHR specification
packages and was withdrawn: openEHR has nothing to generate a schema from
(`db:S1.5`). This tool renders a schema that already exists in code. Nothing
reads these files back — a schema read from JSON would be a second source of
truth.
