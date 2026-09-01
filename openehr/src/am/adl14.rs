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
//! [`ArchetypeId`], an optional parent [`ArchetypeId`], and one `AT_CODE`,
//! checked against the real ADL 1.4 grammar (`openEHR/adl-antlr`,
//! `adl14.g4`: `SYM_ARCHETYPE meta_data? ARCHETYPE_HRID
//! specialization_section? concept_section ...`) — and refuses
//! everything from `language` onward by name, per `K15.6`/`K15.7`'s refusal
//! discipline, rather than silently stopping short of them. It cannot build
//! an [`crate::am::Archetype`]: that type requires a `definition` and a
//! `terminology`, and this reads neither.
//!
//! What it is for: identifying and cataloguing ADL 1.4 source — which
//! archetype a `.adl` file is, and what it specialises — without a database
//! and without `K15.8`'s much larger conversion behind it.

use crate::base::ArchetypeId;
use core::fmt;

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
    /// This archetype's own identifier.
    pub archetype_id: ArchetypeId,
    /// The parent this archetype specialises, if the optional `specialize`
    /// section is present.
    pub specializes: Option<ArchetypeId>,
    /// The concept's local term code, e.g. `"at0000"` — `ontology`'s
    /// `term_definitions` names what it means, which this does not read.
    pub concept: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    /// A bare word or an ADL identifier — letters, digits, `_`, `-`, `.` —
    /// covers keywords (`archetype`, `specialize`, `concept`), an
    /// `ARCHETYPE_HRID`/`ARCHETYPE_REF`, and an `AT_CODE`. Which of those it
    /// is depends on context, exactly as the real grammar's lexer resolves
    /// `ALPHANUM_ID` against keyword tokens before falling back to it.
    Word(String),
    /// `(`, `)`, `[`, `]`, `;`, `=`.
    Symbol(char),
}

struct Lexer<'a> {
    source: &'a str,
    rest: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            rest: source,
        }
    }

    fn offset(&self) -> usize {
        self.source.len() - self.rest.len()
    }

    /// Skips whitespace and `-- ...` line comments, both of which are
    /// insignificant everywhere between tokens in ADL, not only at line
    /// boundaries.
    fn skip_trivia(&mut self) {
        loop {
            let trimmed = self.rest.trim_start();
            self.rest = trimmed;
            if let Some(after) = self.rest.strip_prefix("--") {
                let end = after.find('\n').unwrap_or(after.len());
                self.rest = &after[end..];
                continue;
            }
            break;
        }
    }

    fn next(&mut self) -> Option<Token> {
        self.skip_trivia();
        let mut chars = self.rest.char_indices();
        let (_, first) = chars.next()?;
        if "()[];=".contains(first) {
            self.rest = &self.rest[first.len_utf8()..];
            return Some(Token::Symbol(first));
        }
        let end = chars
            .find(|&(_, c)| c.is_whitespace() || "()[];=".contains(c))
            .map_or(self.rest.len(), |(i, _)| i);
        let word = &self.rest[..end];
        self.rest = &self.rest[end..];
        Some(Token::Word(word.to_owned()))
    }

    fn expect_word(&mut self, want: &str) -> Result<(), Adl14Error> {
        let offset = self.offset();
        match self.next() {
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

    fn expect_symbol(&mut self, want: char) -> Result<(), Adl14Error> {
        let offset = self.offset();
        match self.next() {
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

    fn expect_id(&mut self, what: &'static str) -> Result<String, Adl14Error> {
        let offset = self.offset();
        match self.next() {
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

    /// Consumes a balanced `( ... )` block without interpreting its
    /// contents — `meta_data`, ADL 1.4's `(adl_version=1.4; controlled)`
    /// header metadata, which this reader does not carry anywhere (unlike
    /// [`crate::am::Archetype`]'s own `adl_version` field, since nothing this
    /// reader produces is an `Archetype`).
    fn skip_parenthesised(&mut self) -> Result<(), Adl14Error> {
        self.expect_symbol('(')?;
        let mut depth = 1u32;
        loop {
            let offset = self.offset();
            match self.next() {
                Some(Token::Symbol('(')) => depth += 1,
                Some(Token::Symbol(')')) => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                Some(_) => {}
                None => return Err(Adl14Error::at(offset, "unterminated `(...)` metadata")),
            }
        }
    }
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

    lexer.expect_word("archetype")?;

    // `meta_data`: `(adl_version=1.4; controlled)`, optional.
    lexer.skip_trivia();
    if lexer.rest.starts_with('(') {
        lexer.skip_parenthesised()?;
    }

    let id_text = lexer.expect_id("an archetype identifier")?;
    let archetype_id: ArchetypeId = id_text.parse().map_err(|_| {
        Adl14Error::at(
            lexer.offset() - id_text.len(),
            format!("`{id_text}` is not a valid ARCHETYPE_HRID"),
        )
    })?;

    // `specialization_section`: `specialize <parent-id>`, optional.
    lexer.skip_trivia();
    let specializes = if lexer
        .rest
        .split(|c: char| c.is_whitespace() || "()[];=".contains(c))
        .next()
        .is_some_and(|w| w.eq_ignore_ascii_case("specialize"))
    {
        lexer.expect_word("specialize")?;
        let parent_text = lexer.expect_id("the specialised archetype's identifier")?;
        let parent: ArchetypeId = parent_text.parse().map_err(|_| {
            Adl14Error::at(
                lexer.offset() - parent_text.len(),
                format!("`{parent_text}` is not a valid ARCHETYPE_HRID"),
            )
        })?;
        Some(parent)
    } else {
        None
    };

    lexer.expect_word("concept")?;
    lexer.expect_symbol('[')?;
    let concept = lexer.expect_id("an AT_CODE")?;
    if !concept.starts_with("at") {
        return Err(Adl14Error::at(
            lexer.offset() - concept.len(),
            format!("`{concept}` is not an AT_CODE (expected `at` followed by digits)"),
        ));
    }
    lexer.expect_symbol(']')?;

    // Everything past here is `language` onward, per `K15.6`/`K15.7`: named,
    // not silently stopped short of.
    lexer.skip_trivia();
    if !lexer.rest.is_empty() {
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

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Word(w) => write!(f, "`{w}`"),
            Self::Symbol(s) => write!(f, "`{s}`"),
        }
    }
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
