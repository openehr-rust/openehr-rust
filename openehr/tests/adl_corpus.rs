//! Runs `openehr::am::parse_definition` over an external corpus of real
//! archetypes and reports, per file, whether the `definition` section
//! parsed, was refused by name (`K15.6`), or failed for a reason this crate
//! does not yet state.
//!
//! **This is a discovery run, not a gate.** It asserts nothing about the
//! corpus except that every file produced a verdict; the point is the table
//! it prints, which `openehr/spec/corpus.md` records with the corpus commit
//! and the date. It is `#[ignore]`d because it needs a corpus on disk that
//! this repository does not vendor — `openEHR/adl-archetypes` carries no
//! licence file, so its archetypes are read where they are and never copied
//! into this tree.
//!
//! ```sh
//! OPENEHR_ADL_CORPUS=/path/to/adl-archetypes \
//!   cargo test --test adl_corpus -- --ignored --nocapture
//! # optionally, a per-file TSV:
//! OPENEHR_ADL_CORPUS_REPORT=/tmp/corpus.tsv ...
//! ```
//!
//! Only the `definition` section is read: `parse_definition` reads
//! `c_complex_object` and nothing else (`am::cadl`'s own module
//! documentation), and the header readers are exercised separately. The
//! section is cut by the top-level keyword lines ADL 2 and ADL 1.4 both use
//! at column zero, so a file is never partially parsed by accident — an
//! extraction that misses the boundary is a refusal, not a pass (`K15.7`).

use openehr::am::parse_definition;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Every top-level section that can follow `definition` in either ADL
/// dialect. Anything else at column zero is the definition's own text.
const SECTION_AFTER_DEFINITION: &[&str] = &[
    "rules",
    "terminology",
    "annotations",
    "component_terminologies",
    "invariant",
    "ontology",
];

fn definition_section(source: &str) -> Option<String> {
    let mut lines = source.lines();
    lines.by_ref().find(|l| l.trim_end() == "definition")?;
    let body: Vec<&str> = lines
        .take_while(|l| !SECTION_AFTER_DEFINITION.contains(&l.trim_end()))
        .collect();
    Some(body.join("\n"))
}

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

/// Collapses a refusal into a category: quoted tokens and offsets vary per
/// file, the construct or rule named does not.
fn category(reason: &str) -> String {
    let mut out = String::new();
    let mut in_tick = false;
    for c in reason.chars() {
        match c {
            '`' => {
                in_tick = !in_tick;
                if !in_tick {
                    out.push('…');
                }
            }
            _ if in_tick => {}
            d if d.is_ascii_digit() => {}
            c => out.push(c),
        }
    }
    let out = out.split(" — ").next().unwrap_or(&out).trim().to_owned();
    out.chars().take(110).collect()
}

fn run(corpus: &Path, ext: &str, report: Option<&mut fs::File>) -> (usize, usize, BTreeMap<String, usize>) {
    let mut files = Vec::new();
    walk(corpus, ext, &mut files);
    files.sort();
    let mut parsed = 0;
    let mut no_definition = 0;
    let mut not_utf8 = 0;
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    let mut report = report;
    for path in &files {
        let rel = path.strip_prefix(corpus).unwrap_or(path).display().to_string();
        // A few corpus files are Latin-1 (a `©` in the description). ADL 2
        // is UTF-8 by specification, so that is a corpus fact worth its own
        // count, but it is not a fact about the `definition` section: decode
        // lossily and still give the definition a verdict.
        let bytes = fs::read(path).unwrap_or_default();
        let source = String::from_utf8_lossy(&bytes);
        let lossy = std::str::from_utf8(&bytes).is_err();
        if lossy {
            not_utf8 += 1;
        }
        let verdict = match definition_section(&source) {
            None => {
                no_definition += 1;
                "no-definition-section".to_owned()
            }
            Some(def) => match parse_definition(&def) {
                Ok(_) => {
                    parsed += 1;
                    "parsed".to_owned()
                }
                Err(e) => {
                    let cat = category(&e.reason);
                    *refusals.entry(cat.clone()).or_default() += 1;
                    // The category for the summary, the raw reason and offset
                    // for the per-file report — the raw text is what a reader
                    // drilling into a category actually needs.
                    format!("refused: {cat}\t{}\t{}", e.offset, e.reason)
                }
            },
        };
        if let Some(r) = report.as_deref_mut() {
            let encoding = if lossy { "not-utf8" } else { "utf8" };
            writeln!(r, "{ext}\t{rel}\t{encoding}\t{verdict}").expect("report write");
        }
    }
    println!(
        "\n== .{ext}: {} files, {} parsed, {} refused, {} with no definition section, {} not UTF-8 ==",
        files.len(),
        parsed,
        files.len() - parsed - no_definition,
        no_definition,
        not_utf8
    );
    let mut by_count: Vec<_> = refusals.iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (cat, n) in by_count {
        println!("{n:>6}  {cat}");
    }
    (files.len(), parsed, refusals)
}

#[test]
#[ignore = "needs OPENEHR_ADL_CORPUS=<directory of .adls/.adl files>; see the module documentation"]
fn every_corpus_file_gets_a_verdict() {
    let corpus = PathBuf::from(
        std::env::var("OPENEHR_ADL_CORPUS").expect("OPENEHR_ADL_CORPUS must name the corpus directory"),
    );
    let mut report = std::env::var("OPENEHR_ADL_CORPUS_REPORT")
        .ok()
        .map(|p| fs::File::create(p).expect("report file"));
    let (n2, _, _) = run(&corpus, "adls", report.as_mut());
    let (n14, _, _) = run(&corpus, "adl", report.as_mut());
    assert!(n2 + n14 > 0, "no .adls or .adl files under {}", corpus.display());
}

#[test]
fn the_definition_section_is_cut_at_the_next_top_level_keyword() {
    let source = "archetype (adl_version=2.0.6)\n\tx\n\nlanguage\n\toriginal_language = <[ISO_639-1::en]>\n\ndefinition\n\tCLUSTER[id1] matches {\n\t\titems matches {\n\t\t\tELEMENT[id2] occurrences matches {0..1}\n\t\t}\n\t}\n\nterminology\n\tterm_definitions = <...>\n";
    let def = definition_section(source).expect("has a definition");
    assert!(def.trim_start().starts_with("CLUSTER[id1]"));
    assert!(!def.contains("terminology"));
    assert!(parse_definition(&def).is_ok());
    assert_eq!(definition_section("archetype\n\tx\nterminology\n"), None);
}
