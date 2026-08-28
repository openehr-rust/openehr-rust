# 13. Conformance testing

Requirement prefix: `T13`.

## What must be tested

- **T13.1** Every requirement claimed as satisfied in the
  [conformance matrix](conformance-matrix.md) MUST have at least one test cited
  against it.
- **T13.2** Every such test MUST have been shown to **fail** when the behaviour
  it checks is removed or inverted. A test that cannot fail verifies nothing.
- **T13.3** Each test MUST state, in a comment or its name, the failure mode it
  guards against. A test whose failure mode nobody wrote down is a test nobody
  will maintain.

## The categories

- **T13.4** **Round trip.** One fixture covering every modelled class MUST
  round-trip through canonical JSON, and MUST be byte-identical on a second pass
  (`J9.1`, `J9.2`).
- **T13.5** **Cross-cutting guarantees.** The properties that hold across the
  crate — no PHI in `Display` or errors, partial orders that stay partial, the
  four null flavours staying four, unimplemented operations refusing — MUST be
  tested at the crate boundary, not only inside the module that implements them.
  These are the properties that regress when someone adds a type by copying the
  shape of the one next to it.
- **T13.6** **Distinctive markers.** Tests for PHI leakage MUST use a string
  that cannot occur by accident, so "does this output contain patient data?" is
  answerable by substring search.
- **T13.7** **Interoperability.** At least one test MUST read a payload in the
  divergent forms other implementations emit (`J9.8`, `J9.9`) and assert that
  what the crate writes back is canonical.
- **T13.8** **Doctests.** Every public type and every non-trivial public
  function MUST carry a doctest, and the doctests MUST run in CI. A documented
  example that does not compile is worse than none, because it costs the reader
  the time to find out.

## Recording what was not verified

- **T13.9** A test that self-skips MUST say so and MUST fail if it ends up
  checking nothing.
- **T13.10** A requirement implemented but untested is `?` in the matrix, never
  `•` (`C0.8`).
- **T13.11** A measurement quoted in documentation or in a comment MUST name
  what measured it and when (`J9.15` is the example: the stack figure carries
  its toolchain version and its method).

## Lints as tests

- **T13.12** `cargo clippy --all-targets` MUST pass with `clippy::pedantic`
  enabled and with `missing_docs`, `missing_errors_doc`, and
  `missing_panics_doc` denied. A fallible function whose errors are undocumented
  and a panicking one whose panics are undocumented are both defects in a
  clinical library, because a caller who does not know cannot guard.
- **T13.13** `unsafe_code` MUST be forbidden.
- **T13.14** Where a lint is suppressed, the suppression MUST carry a comment
  giving the reason. An unexplained `#[allow]` is indistinguishable from one
  added to make a build pass.
