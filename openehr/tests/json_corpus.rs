//! Runs `serde_json → Composition → Validate` over an external corpus of
//! real canonical JSON compositions and reports, per file, whether it
//! deserialized, and — if it did — whether [`Validate::validate`] found it
//! clean or listed what it found wrong.
//!
//! **This is a discovery run, not a gate.** It asserts nothing about the
//! corpus except that every file produced a verdict; the point is the table
//! it prints, which [`openehr/spec/corpus.md`](../spec/corpus.md) records
//! with the corpus commit and the date, the same discipline
//! [`adl_corpus`](../tests/adl_corpus.rs) already applies to the archetype
//! half of `tasks.md`'s "run an external corpus" item. It is `#[ignore]`d
//! because it needs a corpus on disk this repository does not vendor: the
//! source is Apache-2.0 (unlike `openEHR/adl-archetypes`, which carries no
//! licence at all), so vendoring it here would be permitted, but reading it
//! where it is rather than committing someone else's fixtures is this
//! session's own conservative default, consistent with `adl_corpus.rs`
//! rather than required by the licence.
//!
//! ```sh
//! git clone --depth 1 https://github.com/ehrbase/openEHR_SDK /some/where
//! OPENEHR_JSON_CORPUS=/some/where/test-data/src/main/resources/composition/canonical_json \
//!   cargo test --test json_corpus -- --ignored --nocapture
//! # optionally, a per-file TSV:
//! OPENEHR_JSON_CORPUS_REPORT=/tmp/json_corpus.tsv ...
//! ```
//!
//! Only `COMPOSITION` documents are read here — the directory's own name —
//! not `EHR_STATUS`, `CONTRIBUTION`, or any other canonical form this crate
//! also round-trips (`tests/canonical_json.rs` covers those against
//! fixtures built in-process). A file whose top-level `_type` names a
//! different class is not this run's concern and is reported as its own
//! verdict rather than forced through the wrong reader.

use openehr::rm::ehr::Composition;
use openehr::validation::Validate;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

fn walk(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, ext, out);
        } else if path.extension().is_some_and(|e| e == ext) {
            out.push(path);
        }
    }
}

/// Collapses a `serde_json` parse error into a category: the line, column,
/// and any quoted token vary per file, the construct it names does not —
/// the same reasoning [`adl_corpus`](../tests/adl_corpus.rs)'s own
/// `category` applies to a `CadlError`'s reason.
fn parse_error_category(reason: &str) -> String {
    let head = reason.split(" at line ").next().unwrap_or(reason);
    let mut out = String::new();
    let mut in_quote = false;
    for c in head.chars() {
        match c {
            '`' | '"' => {
                in_quote = !in_quote;
                if !in_quote {
                    out.push('…');
                }
            }
            _ if in_quote => {}
            c => out.push(c),
        }
    }
    out.trim().chars().take(120).collect()
}

fn run(corpus: &Path, report: Option<&mut fs::File>) -> (usize, usize, usize) {
    let mut files = Vec::new();
    walk(corpus, "json", &mut files);
    files.sort();
    let mut parsed_clean = 0;
    let mut parsed_with_violations = 0;
    let mut parse_errors: BTreeMap<String, usize> = BTreeMap::new();
    let mut violation_kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut report = report;
    for path in &files {
        let rel = path.strip_prefix(corpus).unwrap_or(path).display().to_string();
        let source = fs::read_to_string(path).unwrap_or_default();
        let verdict = match serde_json::from_str::<Composition>(&source) {
            Err(e) => {
                let cat = parse_error_category(&e.to_string());
                *parse_errors.entry(cat.clone()).or_default() += 1;
                format!("parse-error\t{cat}")
            }
            Ok(composition) => {
                let report = composition.validate();
                if report.is_empty() {
                    parsed_clean += 1;
                    "valid".to_owned()
                } else {
                    parsed_with_violations += 1;
                    let mut names: Vec<&str> = report.violations().iter().map(|v| v.invariant).collect();
                    names.sort_unstable();
                    names.dedup();
                    for name in &names {
                        *violation_kinds.entry((*name).to_owned()).or_default() += 1;
                    }
                    format!("invalid\t{} violations\t{}", report.violations().len(), names.join(","))
                }
            }
        };
        if let Some(r) = report.as_deref_mut() {
            writeln!(r, "{rel}\t{verdict}").expect("report write");
        }
    }
    println!(
        "\n== {} files, {} parsed clean, {} parsed with violations, {} did not parse ==",
        files.len(),
        parsed_clean,
        parsed_with_violations,
        parse_errors.values().sum::<usize>()
    );
    if !parse_errors.is_empty() {
        println!("-- parse errors --");
        let mut by_count: Vec<_> = parse_errors.iter().collect();
        by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (cat, n) in by_count {
            println!("{n:>6}  {cat}");
        }
    }
    if !violation_kinds.is_empty() {
        println!("-- invariants that fired, by file count --");
        let mut by_count: Vec<_> = violation_kinds.iter().collect();
        by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (name, n) in by_count {
            println!("{n:>6}  {name}");
        }
    }
    (files.len(), parsed_clean, parsed_with_violations)
}

#[test]
#[ignore = "needs OPENEHR_JSON_CORPUS=<directory of canonical COMPOSITION .json files>; see the module documentation"]
fn every_corpus_file_gets_a_verdict() {
    let corpus = PathBuf::from(
        std::env::var("OPENEHR_JSON_CORPUS").expect("OPENEHR_JSON_CORPUS must name the corpus directory"),
    );
    let mut report = std::env::var("OPENEHR_JSON_CORPUS_REPORT")
        .ok()
        .map(|p| fs::File::create(p).expect("report file"));
    let (n, _, _) = run(&corpus, report.as_mut());
    assert!(n > 0, "no .json files under {}", corpus.display());
}
