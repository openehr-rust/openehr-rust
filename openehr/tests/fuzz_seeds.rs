//! The committed fuzz seed corpus still means what it claimed to mean.
//!
//! `W0.30` requires a fuzz target over a **structured** input to carry a seed
//! corpus of real instances, because random bytes are never a valid
//! `COMPOSITION` and an unseeded target exercises the JSON lexer and stops —
//! reporting the same green as one that works.
//!
//! A committed seed that no longer parses fails the same way, and more quietly:
//! the target still runs, the corpus is still there, and the file contributes
//! nothing. Nobody would notice, because a fuzz run's output does not
//! distinguish "22 seeds" from "22 files, 3 of which the deserializer rejects".
//! So the seeds are checked here, by the crate whose types they are seeds for.
//!
//! This is `C0.13` one level down: a check whose subject is absent reports the
//! silence as success.

use openehr::rm::data_types::DataValue;
use openehr::rm::ehr::Composition;
use std::path::{Path, PathBuf};

/// The sibling fuzz crate, or `None` when this crate is being tested from a
/// package rather than from the repository.
///
/// Distinguished deliberately. A packaged `openehr` on docs.rs has no sibling
/// and this test has nothing to check; a repository checkout that has lost the
/// corpus is a defect. Returning `None` for the first and a present-but-empty
/// directory for the second keeps those apart, instead of skipping on both.
fn fuzz_crate() -> Option<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../openehr-fuzz");
    dir.join("Cargo.toml").is_file().then_some(dir)
}

/// Every `seed-*` in one target's corpus, sorted so failures name a stable file.
fn seeds(target: &str) -> Vec<(String, Vec<u8>)> {
    let Some(root) = fuzz_crate() else {
        return Vec::new();
    };
    let dir = root.join("corpus").join(target);
    let mut found: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} is not readable: {e}", dir.display()))
        .map(|entry| entry.expect("a readable directory entry").path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("seed-"))
        })
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            (name, std::fs::read(&p).expect("a readable seed"))
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(
        !found.is_empty(),
        "{} has no seed-* files; `W0.30` requires a seeded corpus for a \
         structured target, and an empty one is the case it exists to prevent",
        dir.display()
    );
    found
}

/// Counts how many of a target's seeds the deserializer accepts.
fn split<T: serde::de::DeserializeOwned>(target: &str) -> (Vec<String>, Vec<String>) {
    let (mut parsed, mut rejected) = (Vec::new(), Vec::new());
    for (name, bytes) in seeds(target) {
        if serde_json::from_slice::<T>(&bytes).is_ok() {
            parsed.push(name);
        } else {
            rejected.push(name);
        }
    }
    (parsed, rejected)
}

/// The corpus spans both answers: instances, and inputs the parser refuses.
///
/// **Not** "every seed parses". A parser fuzzer wants malformed input too, and
/// this corpus has it deliberately — `null`, `{}`, `[]`, and a `COMPOSITION`
/// with only a `_type` are committed on purpose, and they drive the error
/// paths. Requiring all of them to parse would delete the half of the corpus
/// that exercises rejection.
///
/// What `W0.30` actually asks is that the target gets **past the lexer at
/// least sometimes**: "random bytes are never a valid `COMPOSITION`, so an
/// unseeded target exercises the lexer and stops". A corpus with no instance in
/// it is an unseeded target wearing a corpus. So: at least one of each.
#[test]
fn the_data_value_seeds_span_instances_and_refusals() {
    let Some(_) = fuzz_crate() else { return };
    let (parsed, rejected) = split::<DataValue>("data_value");
    assert!(
        !parsed.is_empty(),
        "no data_value seed deserializes; the target cannot reach validate()"
    );
    // Every hand-authored seed here is meant to be a real instance, so a
    // rejection is a typo rather than a design. This target's malformed inputs
    // come from mutation; the seeds are the shapes mutation is slow to build.
    assert!(
        rejected.is_empty(),
        "these data_value seeds no longer parse, so they contribute nothing: {rejected:?}"
    );
}

#[test]
fn the_canonical_json_seeds_span_instances_and_refusals() {
    let Some(_) = fuzz_crate() else { return };
    let (parsed, rejected) = split::<Composition>("canonical_json");
    assert!(
        !parsed.is_empty(),
        "no canonical_json seed deserializes as a COMPOSITION, so the target \
         exercises the JSON lexer and stops — which is exactly what the seed \
         corpus exists to prevent (`W0.30`)"
    );
    assert!(
        !rejected.is_empty(),
        "no canonical_json seed is refused; the deserializer's error paths are \
         reached only by mutation"
    );
}

/// The `uri` seeds are text rather than JSON, so "parses" is not the property.
///
/// What matters is that the corpus still spans **both** answers: a seed set in
/// which every string is well formed drives one branch of the constructor and
/// leaves the other to random mutation, which is slow to produce a colon in the
/// right place. The target's whole subject is the disagreement between the two
/// gates (`A-36`), so it needs both kinds.
#[test]
fn the_uri_seeds_still_span_accepted_and_rejected() {
    let Some(_) = fuzz_crate() else { return };
    let (mut accepted, mut rejected) = (0, 0);
    for (_, bytes) in seeds("uri") {
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        if openehr::rm::data_types::DvUri::new(text).is_ok() {
            accepted += 1;
        } else {
            rejected += 1;
        }
    }
    assert!(
        accepted > 0 && rejected > 0,
        "the uri corpus must contain both well-formed and malformed seeds, \
         got {accepted} accepted and {rejected} rejected"
    );
}
