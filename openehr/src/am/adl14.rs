//! A minimal ADL 1.4 reader: the archetype header and the concept line, and
//! nothing past them.
//!
//! # This is not `K15.8`, and does not claim to be
//!
//! `K15.8` requires the crate to parse an ADL 1.4 archetype **and convert it
//! to AOM2** — header, specialisation, language, description, the full cADL
//! `definition`, `invariant`, and the `ontology`. `K15.9` prohibits an
//! approximate conversion outright: a 1.4 construct with no faithful AOM2
//! equivalent must fail naming the construct, not be dropped or guessed at.
//!
//! [`parse_header`] does none of that. It recognises exactly the
//! `archetype`/`specialize`/`concept` lines — this archetype's own
//! [`ArchetypeHrid`], an optional parent [`ArchetypeId`], and one `AT_CODE`,
//! checked against the real ADL 1.4 grammar (`openEHR/adl-antlr`,
//! `adl14.g4`: `SYM_ARCHETYPE meta_data? ARCHETYPE_HRID
//! specialization_section? concept_section ...`) — and refuses
//! everything from `language` onward by name, per `K15.6`/`K15.7`'s refusal
//! discipline, rather than silently stopping short of them. It cannot build
//! an [`crate::am::Archetype`]: that type requires a `definition` and a
//! `terminology`, and this reads neither.
//!
//! # Two different identifier grammars, on purpose
//!
//! The archetype's own line is the lexer's `ARCHETYPE_HRID` token — read as
//! [`ArchetypeHrid`], which models it faithfully (namespace prefix,
//! three-part version, prerelease suffix). `specialization_section` is a
//! plain `ARCHETYPE_REF` (`adl14.g4`: `SYM_SPECIALIZE ARCHETYPE_REF`) — a
//! different, narrower token, read here as [`ArchetypeId`], which is close
//! to it but not exact (`ARCHETYPE_REF` also allows a namespace prefix and
//! an unbounded chain of version segments; `ArchetypeId` allows neither).
//! See [`super::ArchetypeHrid`]'s own module documentation and
//! `spec/audit.md` **A-49**'s residual for exactly what that leaves open.
//!
//! What it is for: identifying and cataloguing ADL 1.4 source — which
//! archetype a `.adl` file is, and what it specialises — without a database
//! and without `K15.8`'s much larger conversion behind it.

use super::ArchetypeHrid;
use super::adl_lexer::{Lexer, Token};
use crate::base::ArchetypeId;

/// A failure to read an ADL 1.4 header.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ADL 1.4 header error at offset {offset}: {reason}")]
pub struct Adl14Error {
    /// Byte offset into the source where reading stopped.
    pub offset: usize,
    /// What was expected, or what was found and is not supported.
    pub reason: String,
}

impl Adl14Error {
    fn at(offset: usize, reason: impl Into<String>) -> Self {
        Self {
            offset,
            reason: reason.into(),
        }
    }
}

/// What a minimal ADL 1.4 header names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adl14Header {
    /// This archetype's own identifier — the header's `ARCHETYPE_HRID`
    /// token, in full (namespace, prerelease suffix, and all).
    pub archetype_id: ArchetypeHrid,
    /// The parent this archetype specialises, if the optional `specialize`
    /// section is present. Narrower than the grammar allows there — see the
    /// module documentation.
    pub specializes: Option<ArchetypeId>,
    /// The concept's local term code, e.g. `"at0000"` — `ontology`'s
    /// `term_definitions` names what it means, which this does not read.
    pub concept: String,
}

fn expect_word(lexer: &mut Lexer<'_>, want: &str) -> Result<(), Adl14Error> {
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::Word(w)) if w.eq_ignore_ascii_case(want) => Ok(()),
        Some(other) => Err(Adl14Error::at(
            offset,
            format!("expected `{want}`, found {other}"),
        )),
        None => Err(Adl14Error::at(
            offset,
            format!("expected `{want}`, found end of input"),
        )),
    }
}

fn expect_symbol(lexer: &mut Lexer<'_>, want: char) -> Result<(), Adl14Error> {
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::Symbol(s)) if s == want => Ok(()),
        Some(other) => Err(Adl14Error::at(
            offset,
            format!("expected `{want}`, found {other}"),
        )),
        None => Err(Adl14Error::at(
            offset,
            format!("expected `{want}`, found end of input"),
        )),
    }
}

fn expect_id(lexer: &mut Lexer<'_>, what: &'static str) -> Result<String, Adl14Error> {
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::Word(w)) => Ok(w),
        Some(other) => Err(Adl14Error::at(
            offset,
            format!("expected {what}, found {other}"),
        )),
        None => Err(Adl14Error::at(
            offset,
            format!("expected {what}, found end of input"),
        )),
    }
}

fn expect_archetype_hrid(
    lexer: &mut Lexer<'_>,
    what: &'static str,
) -> Result<ArchetypeHrid, Adl14Error> {
    let id_text = expect_id(lexer, what)?;
    id_text.parse().map_err(|_| {
        Adl14Error::at(
            lexer.offset() - id_text.len(),
            format!("`{id_text}` is not a valid ARCHETYPE_HRID"),
        )
    })
}

fn expect_archetype_id(
    lexer: &mut Lexer<'_>,
    what: &'static str,
) -> Result<ArchetypeId, Adl14Error> {
    let id_text = expect_id(lexer, what)?;
    id_text.parse().map_err(|_| {
        Adl14Error::at(
            lexer.offset() - id_text.len(),
            format!(
                "`{id_text}` does not parse as this reader's ArchetypeId — narrower than the \
                 ARCHETYPE_REF grammar allows here, see the module documentation"
            ),
        )
    })
}

/// Reads an ADL 1.4 archetype's header and concept line.
///
/// Refuses, naming the offset, as soon as it reaches anything past
/// `concept [<code>]` — `language`, `description`, `definition`, `invariant`,
/// and `ontology` are not read by this function at all (see the module
/// documentation for why). A source ending exactly after the concept
/// section's closing `]` is accepted; anything else after it is a refusal,
/// per `K15.6`/`K15.7`'s discipline against silently stopping short.
///
/// # Errors
///
/// Returns [`Adl14Error`] if the header does not match `archetype
/// meta_data? <id> (specialize <id>)? concept [<code>]`, or if anything
/// follows the concept section.
pub fn parse_header(source: &str) -> Result<Adl14Header, Adl14Error> {
    let mut lexer = Lexer::new(source);

    expect_word(&mut lexer, "archetype")?;

    // `meta_data`: `(adl_version=1.4; controlled)`, optional.
    if lexer.peek_symbol_is('(') {
        lexer
            .skip_parenthesised()
            .map_err(|offset| Adl14Error::at(offset, "unterminated `(...)` metadata"))?;
    }

    let archetype_id = expect_archetype_hrid(&mut lexer, "an archetype identifier")?;

    // `specialization_section`: `specialize <parent-id>`, optional.
    let specializes = if lexer
        .peek_word()
        .is_some_and(|w| w.eq_ignore_ascii_case("specialize"))
    {
        expect_word(&mut lexer, "specialize")?;
        Some(expect_archetype_id(
            &mut lexer,
            "the specialised archetype's identifier",
        )?)
    } else {
        None
    };

    expect_word(&mut lexer, "concept")?;
    expect_symbol(&mut lexer, '[')?;
    let concept = expect_id(&mut lexer, "an AT_CODE")?;
    if !concept.starts_with("at") {
        return Err(Adl14Error::at(
            lexer.offset() - concept.len(),
            format!("`{concept}` is not an AT_CODE (expected `at` followed by digits)"),
        ));
    }
    expect_symbol(&mut lexer, ']')?;

    // Everything past here is `language` onward, per `K15.6`/`K15.7`: named,
    // not silently stopped short of.
    if !lexer.at_end() {
        return Err(Adl14Error::at(
            lexer.offset(),
            "the concept section is the last thing this reader parses; `language`, \
             `description`, `definition`, `invariant`, and `ontology` are not \
             implemented (K15.8, not this function's scope)",
        ));
    }

    Ok(Adl14Header {
        archetype_id,
        specializes,
        concept,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real header of `openEHR-EHR-OBSERVATION.blood_pressure.v1`
    /// (`openEHR/adl-tools`, `Release-1.4` branch, fetched and read byte for
    /// byte — CRLF line endings and tabs, exactly as published, not
    /// retyped). This is `S1.4`'s own kind of evidence: a real published
    /// archetype, not a fixture invented to make the parser look good.
    const BLOOD_PRESSURE_HEADER: &str = "archetype\r\n\topenEHR-EHR-OBSERVATION.blood_pressure.v1\r\n\r\nconcept\r\n\t[at0000]\t-- blood pressure measurement\r\n";

    #[test]
    fn reads_a_real_published_archetypes_header() {
        let header = parse_header(BLOOD_PRESSURE_HEADER).expect("should parse");
        assert_eq!(
            header.archetype_id.to_string(),
            "openEHR-EHR-OBSERVATION.blood_pressure.v1"
        );
        assert_eq!(header.specializes, None);
        assert_eq!(header.concept, "at0000");
    }

    /// The same real archetype, in full, is legacy ADL 1.4: no `language`
    /// section at all, straight from `concept` into `description` — exactly
    /// the "non-conforming legacy archetype" the ADL 1.4 spec itself warns
    /// implementations must tolerate. `parse_header` does not tolerate it —
    /// it stops at `concept` by design — so this confirms the refusal fires
    /// cleanly on real text rather than only on an invented one.
    #[test]
    fn refuses_by_name_rather_than_silently_stopping_at_the_real_archetypes_description() {
        let full = format!("{BLOOD_PRESSURE_HEADER}description\r\n\tauthor = <\"Sam Heard\">\r\n");
        let err = parse_header(&full).unwrap_err();
        assert!(
            err.reason.contains("K15.8"),
            "the refusal should name what it does not implement: {err}"
        );
        // The offset lands exactly where `description` starts, not at EOF or
        // at the start of the file -- a refusal that cannot say where is not
        // much better than a silent one.
        assert_eq!(&full[err.offset..][..11], "description");
    }

    #[test]
    fn reads_specialisation_when_present() {
        let source = "archetype\n\topenEHR-EHR-OBSERVATION.blood_pressure.v2\nspecialize\n\topenEHR-EHR-OBSERVATION.blood_pressure.v1\nconcept\n\t[at0000]\n";
        let header = parse_header(source).expect("should parse");
        assert_eq!(
            header.specializes.map(|id| id.to_string()),
            Some("openEHR-EHR-OBSERVATION.blood_pressure.v1".to_owned())
        );
    }

    #[test]
    fn skips_archetype_header_metadata() {
        let source = "archetype (adl_version=1.4; controlled)\n\topenEHR-EHR-OBSERVATION.blood_pressure.v2\nconcept\n\t[at0000]\n";
        let header = parse_header(source).expect("should parse");
        assert_eq!(
            header.archetype_id.to_string(),
            "openEHR-EHR-OBSERVATION.blood_pressure.v2"
        );
    }

    #[test]
    fn a_malformed_archetype_id_is_refused_naming_it() {
        let source = "archetype\n\tnot an archetype id\nconcept\n\t[at0000]\n";
        let err = parse_header(source).unwrap_err();
        assert!(err.reason.contains("ARCHETYPE_HRID"), "{err}");
    }

    #[test]
    fn a_code_that_is_not_an_at_code_is_refused() {
        let source =
            "archetype\n\topenEHR-EHR-OBSERVATION.blood_pressure.v1\nconcept\n\t[ac0001]\n";
        let err = parse_header(source).unwrap_err();
        assert!(err.reason.contains("AT_CODE"), "{err}");
    }

    #[test]
    fn missing_concept_section_is_refused_not_defaulted() {
        let source = "archetype\n\topenEHR-EHR-OBSERVATION.blood_pressure.v1\n";
        let err = parse_header(source).unwrap_err();
        assert!(err.reason.contains("concept"), "{err}");
    }

    #[test]
    fn the_archetype_keyword_is_case_insensitive_but_concept_and_specialize_are_recognised_by_position()
     {
        // The lexer's SYM_ARCHETYPE token is documented case-insensitive;
        // this crate matches that for `archetype` and, since the same lexer
        // rule shape applies, for `concept` and `specialize` too.
        let source =
            "ARCHETYPE\n\topenEHR-EHR-OBSERVATION.blood_pressure.v1\nCONCEPT\n\t[at0000]\n";
        assert!(parse_header(source).is_ok());
    }
}
