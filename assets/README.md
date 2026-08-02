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
| `rm-1.1.0-invariants.json` | all 155 openEHR RM 1.1.0 invariants, distilled from the BMM — **expressions**, not just names |
| `invariant-coverage.md` | which of those the crate names, **and which names diverge from openEHR's** (`lib:L10.4`) |

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

## The divergence table is now a short list of *additions*

Fifteen citations named an invariant openEHR does not have and were renamed to
its own (`lib:A-20`). What is left are rules the crate **adds** where openEHR
states none for that class — `DV_PROPORTION.Parts_finite`,
`INTERVAL_EVENT.Width_non_negative`, and eleven others. Those are strengthenings,
not defects, and they need a name openEHR has not taken.

The distinction is the whole value of the table: without it, a rename and an
addition look identical, and the earlier by-hand sweep treated both as
divergences.

## A caution about `invariant-coverage.md`

**Naming an invariant is not enforcing it.** That report is a grep, and it says
so at the top. 60 of 155 named is not "60 satisfied": the 95 unnamed mix three
different things — out of scope by a declared exclusion, vacuous in Rust
(`X /= Void implies not X.is_empty`, where an empty `Vec` *is* the absent case),
and genuinely unenforced. Separating them needs a human.

It is committed anyway because before it existed the question had no answer at
all, and the two invariants that turned out to be neither enforced nor declared
— `COMPOSITION.Territory_valid` and `Language_valid`, now `lib:A-19` — were
invisible. A weak measure honest about being weak beats no measure, provided
nobody promotes it into a conformance claim (`db:C0.20`). Verified status lives
in the [conformance matrix](../spec/databases/conformance-matrix.md).

The invariants themselves come from the openEHR BMM, which carries the
**expressions** and not merely the names — which is what made any of this
checkable.

## What this is not

Not `db:G2.1`, which required generating DDL **from** openEHR specification
packages and was withdrawn: openEHR has nothing to generate a schema from
(`db:S1.5`). This tool renders a schema that already exists in code. Nothing
reads these files back — a schema read from JSON would be a second source of
truth.
