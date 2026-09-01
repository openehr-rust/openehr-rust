//! A minimal ADL 2 reader: the archetype header only.
//!
//! # This is not `K15.5`, and does not claim to be
//!
//! `K15.5` requires the crate to parse **ADL 2** into the AOM2 model in
//! full: header, specialisation, language, description, definition, rules,
//! terminology, and annotations. `K15.6`/`K15.7` require that any construct
//! the parser does not implement be a named refusal, never a silent skip or
//! a resync past it.
//!
//! [`parse_header`] reads only `archetype [meta_data] <id> [specialize
//! <id>]` — this archetype's own [`ArchetypeHrid`] and, if present, its
//! parent's [`ArchetypeId`] — checked against the real ADL 2 grammar
//! (`openEHR/adl-antlr`, `adl2.g4`: `authored_archetype: SYM_ARCHETYPE
//! meta_data? archetypeHrid specialize_section? language_section
//! description_section definition_section rules_section? terminology_section
//! annotations_section?`), and refuses everything from `language` onward by
//! name. It cannot build an [`crate::am::Archetype`], for the same reason
//! [`super::adl14::parse_header`] cannot: no `definition`, no `terminology`.
//!
//! # Two different identifier grammars, on purpose
//!
//! The archetype's own line is the lexer's `ARCHETYPE_HRID` token — read as
//! [`ArchetypeHrid`], which models it faithfully (namespace prefix,
//! three-part version, prerelease suffix). The `specialize` line is
//! `archetype_ref: ARCHETYPE_HRID | ARCHETYPE_REF` (`cadl2.g4`) — either
//! form is legal there, but this reader accepts only the narrower
//! [`ArchetypeId`] shape for it, which is neither of the two exactly. See
//! [`super::ArchetypeHrid`]'s own module documentation and `spec/audit.md`
//! **A-49**'s residual for exactly what that leaves open.
//!
//! # ADL 2 has no `concept` section
//!
//! ADL 1.4's header carries a `concept [<at-code>]` line naming the root
//! concept; ADL 2's grammar removes it entirely — the root concept is
//! expressed through the definition's own root node instead, which this
//! reader does not read. So [`Adl2Header`] carries no concept code where
//! [`super::adl14::Adl14Header`] does; that is a real difference between the
//! two ADL versions, not an oversight here.
//!
//! Node identifiers changed too — ADL 2 uses `id1`/`id1.1` rather than ADL
//! 1.4's `at0001`/`ac0001` (with `at`-code support re-added as an
//! alternative in AM 2.4.0's amendment record) — but the header this reader
//! parses does not carry one, so that difference does not surface here.

use super::ArchetypeHrid;
use super::adl_lexer::{Lexer, Token};
use crate::base::ArchetypeId;

/// A failure to read an ADL 2 header.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ADL 2 header error at offset {offset}: {reason}")]
pub struct Adl2Error {
    /// Byte offset into the source where reading stopped.
    pub offset: usize,
    /// What was expected, or what was found and is not supported.
    pub reason: String,
}

impl Adl2Error {
    fn at(offset: usize, reason: impl Into<String>) -> Self {
        Self {
            offset,
            reason: reason.into(),
        }
    }
}

/// What a minimal ADL 2 header names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Adl2Header {
    /// This archetype's own identifier — the header's `ARCHETYPE_HRID`
    /// token, in full (namespace, prerelease suffix, and all).
    pub archetype_id: ArchetypeHrid,
    /// The parent this archetype specialises, if the optional
    /// `specialize_section` is present. Narrower than the grammar allows
    /// there — see the module documentation.
    pub specializes: Option<ArchetypeId>,
}

fn expect_word(lexer: &mut Lexer<'_>, want: &str) -> Result<(), Adl2Error> {
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::Word(w)) if w.eq_ignore_ascii_case(want) => Ok(()),
        Some(other) => Err(Adl2Error::at(
            offset,
            format!("expected `{want}`, found {other}"),
        )),
        None => Err(Adl2Error::at(
            offset,
            format!("expected `{want}`, found end of input"),
        )),
    }
}

fn expect_id(lexer: &mut Lexer<'_>, what: &'static str) -> Result<String, Adl2Error> {
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::Word(w)) => Ok(w),
        Some(other) => Err(Adl2Error::at(
            offset,
            format!("expected {what}, found {other}"),
        )),
        None => Err(Adl2Error::at(
            offset,
            format!("expected {what}, found end of input"),
        )),
    }
}

fn expect_archetype_hrid(
    lexer: &mut Lexer<'_>,
    what: &'static str,
) -> Result<ArchetypeHrid, Adl2Error> {
    let id_text = expect_id(lexer, what)?;
    id_text.parse().map_err(|_| {
        Adl2Error::at(
            lexer.offset() - id_text.len(),
            format!("`{id_text}` is not a valid ARCHETYPE_HRID"),
        )
    })
}

fn expect_archetype_id(
    lexer: &mut Lexer<'_>,
    what: &'static str,
) -> Result<ArchetypeId, Adl2Error> {
    let id_text = expect_id(lexer, what)?;
    id_text.parse().map_err(|_| {
        Adl2Error::at(
            lexer.offset() - id_text.len(),
            format!(
                "`{id_text}` does not parse as this reader's ArchetypeId — narrower than the \
                 `archetype_ref: ARCHETYPE_HRID | ARCHETYPE_REF` grammar allows here, see the \
                 module documentation"
            ),
        )
    })
}

/// Reads an ADL 2 archetype's header: `archetype`, optional `meta_data`,
/// the archetype's own identifier, and an optional `specialize` clause.
///
/// Refuses, naming the offset, as soon as it reaches anything past the
/// header — `language_section` is the next, mandatory production in the
/// real grammar, and this reader implements none of it (see the module
/// documentation for why).
///
/// # Errors
///
/// Returns [`Adl2Error`] if the header does not match `archetype
/// meta_data? <id> (specialize <id>)?`, or if anything follows it.
pub fn parse_header(source: &str) -> Result<Adl2Header, Adl2Error> {
    let mut lexer = Lexer::new(source);

    expect_word(&mut lexer, "archetype")?;

    // `meta_data`: `(adl_version=2.4.0; uid=...; ...)`, optional.
    if lexer.peek_symbol_is('(') {
        lexer
            .skip_parenthesised()
            .map_err(|offset| Adl2Error::at(offset, "unterminated `(...)` metadata"))?;
    }

    let archetype_id = expect_archetype_hrid(&mut lexer, "an archetype identifier")?;

    // `specialize_section`: `specialize <parent-id>`, optional.
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

    // Everything past here is `language_section` onward, per `K15.6`/
    // `K15.7`: named, not silently stopped short of.
    if !lexer.at_end() {
        return Err(Adl2Error::at(
            lexer.offset(),
            "the header is the last thing this reader parses; `language`, \
             `description`, `definition`, `rules`, `terminology`, and \
             `annotations` are not implemented (K15.5, not this function's scope)",
        ));
    }

    Ok(Adl2Header {
        archetype_id,
        specializes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_bare_header() {
        let header =
            parse_header("archetype\n\topenEHR-EHR-OBSERVATION.blood_pressure.v2\n").unwrap();
        assert_eq!(
            header.archetype_id.to_string(),
            "openEHR-EHR-OBSERVATION.blood_pressure.v2"
        );
        assert_eq!(header.specializes, None);
    }

    #[test]
    fn reads_meta_data_and_specialisation() {
        let source = "archetype (adl_version=2.4.0; uid=b5f2a220-5967-4d0c-b246-static)\n\
                       \topenEHR-EHR-OBSERVATION.blood_pressure.v2\n\
                       specialize\n\topenEHR-EHR-OBSERVATION.blood_pressure.v1\n";
        let header = parse_header(source).unwrap();
        assert_eq!(
            header.specializes.map(|id| id.to_string()),
            Some("openEHR-EHR-OBSERVATION.blood_pressure.v1".to_owned())
        );
    }

    /// ADL 2 has no `concept` section — confirming the absence is
    /// deliberate, not a stalled parser: a source shaped like ADL 1.4's
    /// header (with a trailing `concept [at0000]`) is refused, naming the
    /// offset where the unexpected `concept` token sits, not silently
    /// accepted as if this reader recognised it.
    #[test]
    fn a_trailing_concept_section_is_refused_not_silently_accepted() {
        let source =
            "archetype\n\topenEHR-EHR-OBSERVATION.blood_pressure.v1\nconcept\n\t[at0000]\n";
        let err = parse_header(source).unwrap_err();
        assert!(err.reason.contains("K15.5"), "{err}");
    }

    #[test]
    fn a_malformed_archetype_id_is_refused_naming_it() {
        let err = parse_header("archetype\n\tnot an archetype id\n").unwrap_err();
        assert!(err.reason.contains("ARCHETYPE_HRID"), "{err}");
    }

    #[test]
    fn missing_archetype_id_is_refused_not_defaulted() {
        let err = parse_header("archetype\n").unwrap_err();
        assert!(err.reason.contains("end of input"), "{err}");
    }
}
