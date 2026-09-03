//! A minimal cADL parser: `definition`'s own grammar rule, `c_complex_object`,
//! and nothing else.
//!
//! # This is not `K15.5`, and does not claim to be
//!
//! `K15.5` requires the crate to parse **ADL 2** into the AOM2 model in full:
//! header, specialisation, language, description, definition, rules,
//! terminology, and annotations. [`parse_definition`] reads only the
//! `definition` section's own body — one `c_complex_object`
//! (`openEHR/adl-antlr`, `cadl2.g4`) — and cannot build an [`crate::am::Archetype`],
//! which also needs `terminology` (a node id `[at0004]` means nothing without
//! the term it names) and, to be checked at all, the rest of the header.
//! `spec/audit.md`'s **A-40** residual already scopes the full grammar at
//! several weeks of work; this is the smallest slice of it that is real —
//! not a toy subset invented for this parser, but the actual grammar for the
//! actual constraint tree, with the actual node kinds this crate does not yet
//! model refused by name rather than silently accepted or dropped.
//!
//! # What this parser refuses, and why each one is a real boundary
//!
//! Every refusal below names the construct; none is a silent skip
//! (`K15.6`), and none resynchronises past what it could not parse
//! (`K15.7`) — a `CadlError` stops the parse where it happens.
//!
//! - **`C_ATTRIBUTE_TUPLE`** (`[units, magnitude] matches { [...], ... }`)
//!   is implemented (`A-67`), wired to [`crate::am::CAttributeTuple`]
//!   (`A-50`). Its own row items (`c_primitive_tuple_item`) are always the
//!   unwrapped shorthand — the grammar gives a tuple row no room for a
//!   wrapping `rm_type_id` — so an interval item like `{|0..300|}` relies
//!   on the unwrapped interval's kind being decided by its first bound's
//!   token (`A-72`, a few bullets below), which it is.
//! - **`C_ARCHETYPE_ROOT`** (`use_archetype`) and **`C_COMPLEX_OBJECT_PROXY`**
//!   (`use_node`) are fully implemented: the former's `archetype_ref`
//!   (`ARCHETYPE_HRID` or `ARCHETYPE_REF`) is reconstructed by slicing the
//!   source between two token boundaries rather than lexed atomically (see
//!   [`super::cadl_lexer::Lexer::text_since`]'s own documentation for why),
//!   and the latter's trailing `ADL_PATH` is read as raw, un-tokenized text
//!   up to the next whitespace ([`super::cadl_lexer::Lexer::read_raw_path`]).
//! - **`ARCHETYPE_SLOT`** (`allow_archetype`) is implemented for the
//!   unrestricted form and for `include`/`exclude` assertions shaped
//!   `bound_path SYM_MATCHES CONTAINED_REGEXP` (`A-66`) — the shape every
//!   real `ARCHETYPE_SLOT` assertion this repository has found actually
//!   uses, and for `closed` (`A-73`): its grammar production carries no
//!   `c_occurrences` at all, which was a refusal while
//!   [`crate::am::ArchetypeSlot`] could only hold a stated interval and is
//!   a plain `None` since `A-71` made `occurrences` optional on every
//!   `C_OBJECT`. An assertion
//!   richer than the one shape above — a boolean operator, a quantifier, a
//!   function call, or even the same `constraint_expr`'s own
//!   `'{' c_inline_primitive_object '}'` alternative — is refused by name
//!   ([`parse_slot_assertion`](self::parse_slot_assertion)'s own
//!   documentation): the full BEOM `boolean_expr` grammar (`K15.10`) is not
//!   implemented, and this crate makes no claim to have implemented more of
//!   it than the one slice that is real.
//! - **`SIBLING_ORDER`** (`after [at0004]` / `before [at0004]` prefixing a
//!   node) — meaningless outside a specialised archetype
//!   (`crate::am::rm_overlay`'s own `SIBLING_ORDER` note in `A-50`/`A-52`),
//!   and this parser builds unspecialised archetypes only.
//! - **`default_value`** (`_default = <...>`) — needs the ODIN grammar,
//!   which this parser does not implement any part of.
//! - **A `C_STRING` regex** (`CONTAINED_REGEXP`, `{/…/}` or `{^…^}`) is now
//!   recognised (`A-66`), both as `c_attribute`'s own shorthand — an
//!   unwrapped `C_STRING` whose `list` holds the delimited pattern
//!   (`A-63`'s single-list shape) — and inside an `ARCHETYPE_SLOT`
//!   assertion above. A **date pattern** (`yyyy-mm-??`, `DATE_CONSTRAINT_PATTERN`)
//!   is a different lexical token this parser still does not recognise, so
//!   a source using one fails to lex as anything this parser expects
//!   rather than being silently dropped.
//! - **More than one `C_INTEGER`/`C_REAL`/temporal range** (`|0..10|,
//!   |20..30|`) — [`crate::am::CPrimitive::Integer`] and its siblings hold
//!   one `Option<Interval<_>>` each, not a list of them; AOM2 itself allows
//!   several disjoint ranges, and representing that is a shape change to
//!   those variants this parser does not make on its own.
//! - **An unwrapped interval's kind is decided, not refused** (`A-72`).
//!   `A-67` refused `attr matches {|0..100|}` as undecidable between
//!   `C_INTEGER` and `C_REAL` without a wrapping type name; `odin_values.g4`
//!   decides it by token — `integer_interval_value` is built from `INTEGER`
//!   bounds, `real_interval_value` from `REAL` — so this parser reads the
//!   first bound's token ([`super::cadl_lexer::Lexer::peek_interval_bound`])
//!   and dispatches. Two refusals remain, both by name: a bound mixing the
//!   kinds (`|0..100.0|`, which the grammar refuses too), and an **unwrapped
//!   temporal interval** (`|2024-01-01..2024-12-31|`, `|PT0S..PT1H|`), whose
//!   bound begins like an ISO 8601 literal and whose kind — date, time,
//!   date-time, or duration — this parser does not decide unwrapped.
//! - **The `+/-` interval spelling** (`|5 +/- 1|`) is refused by name; the
//!   range (`|lower..upper|`, either end optionally open with `>`/`<`) and
//!   the relop form (`|>=0.0|`, `|<10|`, `|5|`; `A-74`) are both read. No
//!   corpus file uses `+/-`; 134 use `|>=0.0|`.
//! - **Generic RM type parameters** (`LIST<DV_TEXT>`) — `rm_type_id` accepts
//!   only a bare `ALPHA_UC_ID` here.
//!
//! # Some literals need their own lexer scan
//!
//! `2024-01-01`, `12:30:00`, and `P1Y2M` all contain `-`/`:`, which
//! [`super::cadl_lexer::Lexer`]'s ordinary word-scanner treats as `Symbol`s
//! (needed for archetype identifiers and interval syntax elsewhere), so
//! none of the four temporal kinds could be parsed at all until `A-65`
//! added [`super::cadl_lexer::Lexer::read_iso8601`] — a second, dedicated
//! scan reached only from [`expect_temporal`](self::expect_temporal),
//! where a temporal literal is grammatically expected, never from the
//! ordinary tokenizer, so there is no ambiguity with a plain
//! `INTEGER`/`REAL` for it to resolve.
//!
//! `CONTAINED_REGEXP` (`{/…/}`, `{^…^}`) needs the same treatment for a
//! different reason: its body may contain almost any character —
//! `,`, `"`, even a bare `{` — none of which the ordinary tokenizer could
//! ever treat as part of one token. `A-66` added
//! [`super::cadl_lexer::Lexer::try_read_contained_regexp`], reached from
//! both `c_attribute_def`'s own shorthand and
//! [`parse_slot_assertion`](self::parse_slot_assertion), each already
//! committed to expecting one by grammar position before calling it —
//! `try_`, not `expect_`, only because a bare `{` is ambiguous with an
//! ordinary `{c_objects}` block until the character after it is seen.
//!
//! # Occurrences: stated, or carried unstated — never guessed
//!
//! AOM2's `C_OBJECT.occurrences` is `0..1`; `Void` means
//! `effective_occurrences()` — lower bound `0`, upper bound the owning
//! `C_ATTRIBUTE`'s cardinality upper bound if it has one, else the
//! Reference Model's own multiplicity for that attribute. Every
//! [`crate::am::CObject`] variant stores `occurrences` as
//! `Option<MultiplicityInterval>` (`A-71`, `K15.32`; before it, only
//! [`crate::am::CComplexObjectProxy`] could, `A-54`), and this parser
//! carries an omitted one as `None` — neither refused, as it was before
//! `A-71`, nor filled in, since a round trip must not invent what the
//! author omitted (`K15.3`) and specialisation reads "set" as "overrides"
//! (`K15.13`). [`crate::am::CObject::effective_occurrences`] infers the
//! value from the owning attribute when asked. Two nodes are different: the
//! definition's own root, which AOM2 fixes at exactly one and which has no
//! owning attribute, is stated as [`crate::am::ROOT_OCCURRENCES`] when
//! omitted; and a `use_node`'s `None` means `use_target_occurrences()`, a
//! deferral to a target this crate does not resolve, not an inference.
//! The corpus run that motivated this (`openehr/spec/corpus.md`) found the
//! old refusal to be two thirds of every refusal over 1,972 real files.
//!
//! # Container or single: decided by whether `cardinality` was written
//!
//! [`crate::am::CAttribute::single`] and
//! [`crate::am::CAttribute::container`] are different constructors in this
//! crate, and choosing between them needs to know whether the Reference
//! Model's own attribute is multiple-valued — again, a fact this parser has
//! no table for. The rule here is syntactic instead: an attribute whose text
//! states `cardinality matches {...}` is built as a container; one that does
//! not is built as single-valued, even where the underlying RM attribute is
//! actually a `List` or `Set` whose archetype simply never narrowed its
//! default cardinality. A real archetype relying on that default would
//! parse as single-valued here, which is a misreading of the artefact, not
//! a refusal of it — stated here because it is the one simplification in
//! this module that can produce a *wrong* tree rather than an honest
//! refusal, and a reader needs to know that going in.

use super::archetype_hrid::is_identifier;
use super::cadl_lexer::{Lexer, Token};
use super::{
    ArchetypeSlot, CArchetypeRoot, CAttribute, CAttributeTuple, CComplexObject,
    CComplexObjectProxy, CObject, Cardinality, MultiplicityInterval, NodeIdSyntax,
    ROOT_OCCURRENCES,
};
use crate::am::{CPrimitive, CPrimitiveObject, CPrimitiveTuple, PrimitiveValue};
use crate::base::{Date, DateTime, Duration, Interval, Real, SemanticOrd, Time};
use core::str::FromStr;

/// A failure to parse cADL — a construct outside this parser's scope (see
/// the module documentation for the boundary), or malformed text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("cADL error at offset {offset}: {reason}")]
pub struct CadlError {
    /// Byte offset into the source where parsing stopped.
    pub offset: usize,
    /// What was expected, or what was found and is not supported.
    pub reason: String,
}

impl CadlError {
    fn at(offset: usize, reason: impl Into<String>) -> Self {
        Self {
            offset,
            reason: reason.into(),
        }
    }
}

/// Parses a `definition` section's own body — one `c_complex_object` — into
/// a [`CComplexObject`]. See the module documentation for exactly what this
/// does and does not implement, and why.
///
/// # Errors
///
/// Returns [`CadlError`] naming the offset and the construct, at the first
/// point the source uses anything outside this parser's scope, or is
/// malformed. Never a partial tree (`K15.7`).
pub fn parse_definition(source: &str) -> Result<CComplexObject, CadlError> {
    let mut lexer = Lexer::new(source);
    let root = c_complex_object(&mut lexer, true)?;
    if !lexer.at_end() {
        return Err(CadlError::at(
            lexer.offset(),
            "unexpected content after the definition's root object",
        ));
    }
    Ok(root)
}

// ---------------------------------------------------------------------
// Token-level helpers
// ---------------------------------------------------------------------

fn expect_symbol(lexer: &mut Lexer<'_>, want: char) -> Result<(), CadlError> {
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::Symbol(c)) if c == want => Ok(()),
        Some(other) => Err(CadlError::at(
            offset,
            format!("expected `{want}`, found {other}"),
        )),
        None => Err(CadlError::at(
            offset,
            format!("expected `{want}`, found end of input"),
        )),
    }
}

fn consume_symbol_if(lexer: &mut Lexer<'_>, want: char) -> bool {
    if matches!(lexer.peek(), Some(Token::Symbol(c)) if c == want) {
        lexer.next();
        true
    } else {
        false
    }
}

fn expect_dotdot(lexer: &mut Lexer<'_>) -> Result<(), CadlError> {
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::DotDot) => Ok(()),
        Some(other) => Err(CadlError::at(offset, format!("expected `..`, found {other}"))),
        None => Err(CadlError::at(offset, "expected `..`, found end of input")),
    }
}

fn peek_keyword(lexer: &mut Lexer<'_>, want: &str) -> bool {
    matches!(lexer.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(want))
}

fn expect_keyword(lexer: &mut Lexer<'_>, want: &str) -> Result<(), CadlError> {
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::Word(w)) if w.eq_ignore_ascii_case(want) => Ok(()),
        Some(other) => Err(CadlError::at(
            offset,
            format!("expected `{want}`, found {other}"),
        )),
        None => Err(CadlError::at(
            offset,
            format!("expected `{want}`, found end of input"),
        )),
    }
}

fn expect_word(lexer: &mut Lexer<'_>, what: &'static str) -> Result<String, CadlError> {
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::Word(w)) => Ok(w),
        Some(other) => Err(CadlError::at(offset, format!("expected {what}, found {other}"))),
        None => Err(CadlError::at(
            offset,
            format!("expected {what}, found end of input"),
        )),
    }
}

/// `rm_type_id: ALPHA_UC_ID ( '<' rm_type_id (',' rm_type_id)* '>' )?` — the
/// generic-parameter form is refused (module documentation).
fn expect_rm_type_id(lexer: &mut Lexer<'_>) -> Result<String, CadlError> {
    let name = expect_word(lexer, "an RM type name")?;
    if matches!(lexer.peek(), Some(Token::Symbol('<'))) {
        return Err(CadlError::at(
            lexer.offset(),
            "generic RM type parameters (`LIST<DV_TEXT>`) are not implemented by this parser",
        ));
    }
    Ok(name)
}

fn expect_node_id(lexer: &mut Lexer<'_>) -> Result<String, CadlError> {
    let offset = lexer.offset();
    let code = expect_word(lexer, "an id-, at-, or ac-code")?;
    if NodeIdSyntax::of(&code).is_none() {
        return Err(CadlError::at(
            offset,
            format!("`{code}` is not a valid id-, at-, or ac-code"),
        ));
    }
    Ok(code)
}

fn expect_signed_integer(lexer: &mut Lexer<'_>) -> Result<i64, CadlError> {
    let offset = lexer.offset();
    let negative = consume_symbol_if(lexer, '-');
    if !negative {
        consume_symbol_if(lexer, '+');
    }
    match lexer.next() {
        Some(Token::Integer(n)) => n
            .parse::<i64>()
            .map(|v| if negative { -v } else { v })
            .map_err(|_| CadlError::at(offset, format!("`{n}` does not fit an INTEGER"))),
        Some(other) => Err(CadlError::at(offset, format!("expected an integer, found {other}"))),
        None => Err(CadlError::at(offset, "expected an integer, found end of input")),
    }
}

/// `real_value`, accepting an `INTEGER`-shaped token too (`|0..100.0|`'s
/// `0`) — `REAL` strictly requires a `.`, but nothing about a whole-number
/// bound in a real-valued range is meaningless, and refusing it would
/// refuse real archetype text over a lexical technicality this parser does
/// not need to enforce.
fn expect_signed_real(lexer: &mut Lexer<'_>) -> Result<Real, CadlError> {
    let offset = lexer.offset();
    let negative = consume_symbol_if(lexer, '-');
    if !negative {
        consume_symbol_if(lexer, '+');
    }
    let text = match lexer.next() {
        Some(Token::Real(n) | Token::Integer(n)) => n,
        Some(other) => return Err(CadlError::at(offset, format!("expected a real number, found {other}"))),
        None => return Err(CadlError::at(offset, "expected a real number, found end of input")),
    };
    let text = if negative { format!("-{text}") } else { text };
    Real::from_str(&text).map_err(|_| CadlError::at(offset, format!("`{text}` is not a valid REAL")))
}

// ---------------------------------------------------------------------
// c_complex_object, c_attribute, c_objects
// ---------------------------------------------------------------------

/// `c_complex_object: rm_type_id '[' (ROOT_ID_CODE|ID_CODE) ']' c_occurrences?
/// ( SYM_MATCHES '{' c_attribute_def+ default_value? '}' )? ;`
fn c_complex_object(lexer: &mut Lexer<'_>, is_root: bool) -> Result<CComplexObject, CadlError> {
    let rm_type_name = expect_rm_type_id(lexer)?;
    expect_symbol(lexer, '[')?;
    let node_id = expect_node_id(lexer)?;
    expect_symbol(lexer, ']')?;
    let occurrences = parse_occurrences(lexer, is_root)?;

    let mut attributes = Vec::new();
    let mut attribute_tuples = Vec::new();
    if peek_keyword(lexer, "matches") || peek_keyword(lexer, "is_in") {
        lexer.next();
        expect_symbol(lexer, '{')?;
        loop {
            if consume_symbol_if(lexer, '}') {
                break;
            }
            if peek_keyword(lexer, "_default") {
                return Err(CadlError::at(
                    lexer.offset(),
                    "default_value (`_default = <...>`) is not implemented by this parser",
                ));
            }
            match c_attribute_def(lexer)? {
                AttributeDef::Attribute(a) => attributes.push(a),
                AttributeDef::Tuple(t) => attribute_tuples.push(t),
            }
        }
    }

    CComplexObject::new(rm_type_name, Some(node_id), occurrences, attributes)
        .map(|c| c.with_attribute_tuples(attribute_tuples))
        .map_err(|e| CadlError::at(lexer.offset(), format!("invalid C_COMPLEX_OBJECT: {e}")))
}

/// Reads `c_occurrences?`, applying the rule the module documentation
/// states: the root defaults to [`ROOT_OCCURRENCES`] when absent — AOM2
/// fixes it at exactly one and there is no owning attribute to infer from
/// — and any other node with it absent is carried as `None`, AOM2's own
/// `Void`, for [`crate::am::CObject::effective_occurrences`] to infer from
/// the owning attribute when asked (`A-71`, `K15.32`). Nothing is guessed
/// here: the stated/unstated distinction survives the parse.
fn parse_occurrences(lexer: &mut Lexer<'_>, is_root: bool) -> Result<Option<MultiplicityInterval>, CadlError> {
    let stated = parse_optional_occurrences(lexer)?;
    Ok(match (stated, is_root) {
        (None, true) => Some(ROOT_OCCURRENCES),
        (stated, _) => stated,
    })
}

/// `multiplicity: INTEGER | '*' | INTEGER '..' ( INTEGER | '*' ) ;`
fn parse_multiplicity(lexer: &mut Lexer<'_>) -> Result<MultiplicityInterval, CadlError> {
    let offset = lexer.offset();
    let (lower, upper) = match lexer.next() {
        Some(Token::Symbol('*')) => (0, None),
        Some(Token::Integer(n)) => {
            let lower: u32 = n
                .parse()
                .map_err(|_| CadlError::at(offset, format!("`{n}` does not fit a multiplicity bound")))?;
            if matches!(lexer.peek(), Some(Token::DotDot)) {
                lexer.next();
                let upper_offset = lexer.offset();
                match lexer.next() {
                    Some(Token::Symbol('*')) => (lower, None),
                    Some(Token::Integer(u)) => {
                        let upper: u32 = u.parse().map_err(|_| {
                            CadlError::at(upper_offset, format!("`{u}` does not fit a multiplicity bound"))
                        })?;
                        (lower, Some(upper))
                    }
                    Some(other) => {
                        return Err(CadlError::at(
                            upper_offset,
                            format!("expected an integer or `*`, found {other}"),
                        ));
                    }
                    None => {
                        return Err(CadlError::at(
                            upper_offset,
                            "expected an integer or `*`, found end of input",
                        ));
                    }
                }
            } else {
                (lower, Some(lower))
            }
        }
        Some(other) => {
            return Err(CadlError::at(
                offset,
                format!("expected a multiplicity (an integer, `*`, or a range), found {other}"),
            ));
        }
        None => return Err(CadlError::at(offset, "expected a multiplicity, found end of input")),
    };
    MultiplicityInterval::new(lower, upper).map_err(|e| CadlError::at(offset, e.to_string()))
}

/// `existence: INTEGER | INTEGER '..' INTEGER ;` — no `*`.
fn parse_existence(lexer: &mut Lexer<'_>) -> Result<MultiplicityInterval, CadlError> {
    let offset = lexer.offset();
    let lower_i = expect_signed_integer(lexer)?;
    let lower = u32::try_from(lower_i)
        .map_err(|_| CadlError::at(offset, "an existence bound must not be negative"))?;
    let interval = if matches!(lexer.peek(), Some(Token::DotDot)) {
        lexer.next();
        let upper_offset = lexer.offset();
        let upper_i = expect_signed_integer(lexer)?;
        let upper = u32::try_from(upper_i)
            .map_err(|_| CadlError::at(upper_offset, "an existence bound must not be negative"))?;
        MultiplicityInterval::new(lower, Some(upper))
    } else {
        MultiplicityInterval::new(lower, Some(lower))
    };
    interval.map_err(|e| CadlError::at(offset, e.to_string()))
}

/// `cardinality: multiplicity ( multiplicity_mod multiplicity_mod? )? ;`
fn parse_cardinality(lexer: &mut Lexer<'_>) -> Result<Cardinality, CadlError> {
    let interval = parse_multiplicity(lexer)?;
    let mut cardinality = Cardinality::new(interval);
    for _ in 0..2 {
        if !consume_symbol_if(lexer, ';') {
            break;
        }
        let offset = lexer.offset();
        match lexer.next() {
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("ordered") => cardinality = cardinality.ordered(),
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("unordered") => {}
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("unique") => cardinality = cardinality.unique(),
            Some(other) => {
                return Err(CadlError::at(
                    offset,
                    format!("expected `ordered`, `unordered`, or `unique`, found {other}"),
                ));
            }
            None => {
                return Err(CadlError::at(
                    offset,
                    "expected `ordered`, `unordered`, or `unique`, found end of input",
                ));
            }
        }
    }
    Ok(cardinality)
}

/// The two kinds of child a `C_COMPLEX_OBJECT`'s own `matches {...}` block
/// can name — `c_attribute_def: c_attribute | c_attribute_tuple ;` — kept
/// distinct here because [`CComplexObject`] itself holds them in two
/// separate fields (`attributes`, `attribute_tuples`), not one list.
enum AttributeDef {
    Attribute(CAttribute),
    Tuple(CAttributeTuple),
}

/// `c_attribute_def: c_attribute | c_attribute_tuple ;` — a leading `[`
/// means the tuple form, dispatched to [`c_attribute_tuple`]: nothing in
/// `c_attribute` (`ADL_PATH | rm_attribute_id`) starts with `[`, so this
/// is exact, not a guess.
///
/// `c_attribute: (ADL_PATH | rm_attribute_id) c_existence? c_cardinality?
/// ( SYM_MATCHES ( '{' c_objects '}' | CONTAINED_REGEXP) )? ;`. An
/// `ADL_PATH` is an attribute in differential form — `/data/events
/// cardinality matches {2..8; ordered}` in a specialised archetype — and is
/// one whitespace-bounded run containing `/`, which a bare
/// `rm_attribute_id` never is; so the choice is exact. The path is split at
/// its last `/`: the tail is the attribute, the head is AOM2's
/// `differential_path` (`A-70`).
///
/// Before `A-70` the lexer's own fallback made `/` a one-character "word"
/// and this parser accepted it as an attribute name, so a differential
/// path mis-parsed into several attributes and was refused later as a
/// `VOKU` duplicate — a refusal naming the wrong thing (`K15.6`). Now a
/// name that is not an `IDENTIFIER` is refused by name here.
fn c_attribute_def(lexer: &mut Lexer<'_>) -> Result<AttributeDef, CadlError> {
    let before = lexer.offset();
    let def = c_attribute_def_inner(lexer)?;
    // Every attribute definition consumes at least its name, so this is
    // unreachable for real input; it exists because both callers loop on
    // this function until `}`, and a loop over a sub-parser that did not
    // advance is an infinite loop. cargo-mutants reached exactly that by
    // replacing `Lexer::read_raw_path`'s body with a constant, and the run
    // hung for its full timeout instead of failing — the same shape the
    // `ARCHETYPE_SLOT` assertion loop guards against.
    if lexer.offset() == before {
        return Err(CadlError::at(before, "a C_ATTRIBUTE definition consumed no input"));
    }
    Ok(def)
}

fn c_attribute_def_inner(lexer: &mut Lexer<'_>) -> Result<AttributeDef, CadlError> {
    if matches!(lexer.peek(), Some(Token::Symbol('['))) {
        return c_attribute_tuple(lexer).map(AttributeDef::Tuple);
    }
    let offset = lexer.offset();
    // The split *is* the test for a path: `rsplit_once('/')` answers `None`
    // for a bare name, so there is no separate "contains `/`" guard that a
    // mutation could turn into `true` and quietly give every plain
    // attribute a differential path of `/` (found by cargo-mutants on the
    // first shape of this code).
    let (differential_path, attr_name) = match lexer.peek_raw_path().and_then(|raw| raw.rsplit_once('/')) {
        Some((parent, name)) => {
            let (parent, name) = (parent.to_owned(), name.to_owned());
            // Consume what was peeked: the whole whitespace-bounded run.
            let _ = lexer.read_raw_path();
            // `/events` is an attribute of the root object itself, whose
            // path is `/`; `items[id2]/value` is relative to the enclosing
            // object and keeps its predicates in the parent part.
            let parent = if parent.is_empty() { "/".to_owned() } else { parent };
            (Some(parent), name)
        }
        None => (None, expect_word(lexer, "an attribute name")?),
    };
    if !is_identifier(&attr_name) {
        return Err(CadlError::at(
            offset,
            format!("`{attr_name}` is not an attribute name (an identifier: a letter, then letters, digits, `_`)"),
        ));
    }

    let existence = if peek_keyword(lexer, "existence") {
        lexer.next();
        expect_keyword(lexer, "matches")?;
        expect_symbol(lexer, '{')?;
        let e = parse_existence(lexer)?;
        expect_symbol(lexer, '}')?;
        e
    } else {
        // AOM2's own stated default for an unstated `existence`: required.
        MultiplicityInterval::MANDATORY
    };

    let cardinality = if peek_keyword(lexer, "cardinality") {
        lexer.next();
        expect_keyword(lexer, "matches")?;
        expect_symbol(lexer, '{')?;
        let c = parse_cardinality(lexer)?;
        expect_symbol(lexer, '}')?;
        Some(c)
    } else {
        None
    };

    let mut children = Vec::new();
    if peek_keyword(lexer, "matches") || peek_keyword(lexer, "is_in") {
        lexer.next();
        let regexp_offset = lexer.offset();
        match lexer.try_read_contained_regexp() {
            // `c_attribute`'s own shorthand: `SYM_MATCHES CONTAINED_REGEXP`
            // in place of `SYM_MATCHES '{' c_objects '}'`, constraining the
            // attribute's single value directly by regex rather than
            // naming a wrapped `C_STRING` node — built as exactly that
            // node anyway (`CPrimitive::String { list: [pattern] }`, `A-63`'s
            // own single-list shape), the same unwrapped-primitive shape
            // `c_objects`'s own shorthand already produces.
            Ok(Some(pattern)) => {
                let constraint = CPrimitive::String { list: vec![pattern.to_owned()] };
                let assumed = finish_contained_regexp(lexer)?.map(PrimitiveValue::Text);
                children = vec![unwrapped_primitive_object(constraint, assumed)];
            }
            Ok(None) => {
                expect_symbol(lexer, '{')?;
                children = c_objects(lexer)?;
                expect_symbol(lexer, '}')?;
            }
            Err(()) => {
                return Err(CadlError::at(
                    regexp_offset,
                    "malformed CONTAINED_REGEXP: no closing delimiter found before a newline or \
                     the end of input",
                ));
            }
        }
    }

    let result = match cardinality {
        Some(cardinality) => CAttribute::container(attr_name.clone(), existence, cardinality, children),
        None => CAttribute::single(attr_name.clone(), existence, children),
    };
    result
        .and_then(|attribute| match differential_path {
            Some(parent) => attribute.with_differential_path(parent),
            None => Ok(attribute),
        })
        .map(AttributeDef::Attribute)
        .map_err(|e| CadlError::at(offset, format!("invalid C_ATTRIBUTE `{attr_name}`: {e}")))
}

/// `c_attribute_tuple : '[' rm_attribute_id ( ',' rm_attribute_id )* ']'
/// SYM_MATCHES '{' c_primitive_tuple ( ',' c_primitive_tuple )* '}' ;`
///
/// Each co-varying attribute is built `CAttribute::single`, `MANDATORY` —
/// the grammar states no `c_existence`/`c_cardinality` of its own for one,
/// and AOM2's own tuple examples (`{units, magnitude}`,
/// `{value, symbol}`) are always single, mandatory values, which is the
/// whole reason a tuple exists rather than two independent attributes.
fn c_attribute_tuple(lexer: &mut Lexer<'_>) -> Result<CAttributeTuple, CadlError> {
    let offset = lexer.offset();
    expect_symbol(lexer, '[')?;
    let mut member_names = Vec::new();
    loop {
        member_names.push(expect_word(lexer, "an RM attribute name")?);
        if !consume_symbol_if(lexer, ',') {
            break;
        }
    }
    expect_symbol(lexer, ']')?;
    expect_keyword(lexer, "matches")?;
    expect_symbol(lexer, '{')?;
    let mut rows = Vec::new();
    loop {
        rows.push(c_primitive_tuple(lexer)?);
        if !consume_symbol_if(lexer, ',') {
            break;
        }
    }
    expect_symbol(lexer, '}')?;

    let members = member_names
        .iter()
        .map(|name| {
            CAttribute::single(name.clone(), MultiplicityInterval::MANDATORY, Vec::new())
                .map_err(|e| CadlError::at(offset, format!("invalid C_ATTRIBUTE `{name}`: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    CAttributeTuple::new(members, rows)
        .map_err(|e| CadlError::at(offset, format!("invalid C_ATTRIBUTE_TUPLE: {e}")))
}

/// `c_primitive_tuple : '[' c_primitive_tuple_item ( ','
/// c_primitive_tuple_item )* ']' ;`
fn c_primitive_tuple(lexer: &mut Lexer<'_>) -> Result<CPrimitiveTuple, CadlError> {
    let offset = lexer.offset();
    expect_symbol(lexer, '[')?;
    let mut items = Vec::new();
    loop {
        items.push(c_primitive_tuple_item(lexer)?);
        if !consume_symbol_if(lexer, ',') {
            break;
        }
    }
    expect_symbol(lexer, ']')?;
    CPrimitiveTuple::new(items).map_err(|e| CadlError::at(offset, format!("invalid C_PRIMITIVE_TUPLE: {e}")))
}

/// `c_primitive_tuple_item: '{' c_inline_primitive_object '}' |
/// CONTAINED_REGEXP ;` — the same two shapes, and the same reasoning,
/// `c_attribute_def`'s own `CONTAINED_REGEXP` shorthand already
/// implements above; the real grammar's own comment states why a tuple
/// row's regex item is written this way rather than as a `C_STRING`:
/// "the only workable solution to match a regex unambiguously appears to
/// be to match with enclosing `{}`... as a `C_OBJECT` alternative, not as
/// a `C_STRING`."
fn c_primitive_tuple_item(lexer: &mut Lexer<'_>) -> Result<CPrimitiveObject, CadlError> {
    let regexp_offset = lexer.offset();
    match lexer.try_read_contained_regexp() {
        Ok(Some(pattern)) => {
            let constraint = CPrimitive::String { list: vec![pattern.to_owned()] };
            let assumed = finish_contained_regexp(lexer)?.map(PrimitiveValue::Text);
            Ok(unwrapped_primitive(constraint, assumed))
        }
        Ok(None) => {
            expect_symbol(lexer, '{')?;
            let (constraint, assumed) = parse_inline_primitive(lexer, None)?;
            expect_symbol(lexer, '}')?;
            Ok(unwrapped_primitive(constraint, assumed))
        }
        Err(()) => Err(CadlError::at(
            regexp_offset,
            "malformed CONTAINED_REGEXP: no closing delimiter found before a newline or the end \
             of input",
        )),
    }
}

/// Builds the `C_PRIMITIVE_OBJECT` an unwrapped shorthand produces —
/// [`CPrimitiveObject::PRIMITIVE_NODE_ID`], no rm-type-name of its own —
/// the one shape shared by `c_objects`'s own inline-primitive shorthand,
/// `c_attribute`'s `CONTAINED_REGEXP` shorthand, and each
/// `c_primitive_tuple_item` in a `C_ATTRIBUTE_TUPLE` row (`lib:A-33`: one
/// place, not three, for a repeated construction).
fn unwrapped_primitive(constraint: CPrimitive, assumed: Option<PrimitiveValue>) -> CPrimitiveObject {
    let mut object = CPrimitiveObject::new("primitive", Some(MultiplicityInterval::MANDATORY), constraint)
        .with_node_id(CPrimitiveObject::PRIMITIVE_NODE_ID)
        .expect("PRIMITIVE_NODE_ID is always accepted by with_node_id");
    if let Some(value) = assumed {
        object = object.with_assumed_value(value);
    }
    object
}

/// [`unwrapped_primitive`], wrapped as the `CObject::Primitive` most call
/// sites actually need — a `C_ATTRIBUTE_TUPLE` row is the one exception,
/// which wants the bare `CPrimitiveObject` itself.
fn unwrapped_primitive_object(constraint: CPrimitive, assumed: Option<PrimitiveValue>) -> CObject {
    CObject::Primitive(unwrapped_primitive(constraint, assumed))
}

/// `c_objects: c_regular_object_ordered+ | c_inline_primitive_object ;`
fn c_objects(lexer: &mut Lexer<'_>) -> Result<Vec<CObject>, CadlError> {
    if starts_inline_primitive(lexer) {
        let (primitive, assumed) = parse_inline_primitive(lexer, None)?;
        return Ok(vec![unwrapped_primitive_object(primitive, assumed)]);
    }
    let mut objects = Vec::new();
    loop {
        objects.push(c_regular_object_ordered(lexer)?);
        if matches!(lexer.peek(), Some(Token::Symbol('}'))) || lexer.peek().is_none() {
            break;
        }
    }
    Ok(objects)
}

/// Whether the next token can only begin `c_inline_primitive_object`
/// (module documentation: the unwrapped shorthand, supported for
/// `Boolean`/`String`/`Integer`/`Real`/`Terminology_code` only).
/// Whether raw text begins like an ISO 8601 literal rather than a number:
/// four digits then `-` (a date or date-time), two digits then `:` (a time),
/// or `P`/`-P` (a duration) — the opening shapes `base_lexer.g4`'s
/// `ISO8601_DATE`, `ISO8601_TIME`, `ISO8601_DATE_TIME`, and
/// `ISO8601_DURATION` share. A `0..100` bound has a `.` after its digits and
/// is a number.
fn looks_iso8601(raw: &str) -> bool {
    let digits = raw.bytes().take_while(u8::is_ascii_digit).count();
    let after = raw.as_bytes().get(digits).copied();
    (digits == 4 && after == Some(b'-'))
        || (digits == 2 && after == Some(b':'))
        || raw.starts_with('P')
        || raw.starts_with("-P")
}

fn starts_inline_primitive(lexer: &mut Lexer<'_>) -> bool {
    matches!(
        lexer.peek(),
        Some(Token::Str(_) | Token::Integer(_) | Token::Real(_) | Token::Symbol('[' | '|'))
    ) || peek_keyword(lexer, "true")
        || peek_keyword(lexer, "false")
}

/// `c_regular_object_ordered: sibling_order? c_regular_object ;`
fn c_regular_object_ordered(lexer: &mut Lexer<'_>) -> Result<CObject, CadlError> {
    if peek_keyword(lexer, "after") || peek_keyword(lexer, "before") {
        return Err(CadlError::at(
            lexer.offset(),
            "SIBLING_ORDER (`after`/`before [at0004]`) is not implemented by this parser",
        ));
    }
    c_regular_object(lexer)
}

/// `c_regular_object: c_complex_object | c_archetype_root |
/// c_complex_object_proxy | archetype_slot | c_regular_primitive_object ;`
fn c_regular_object(lexer: &mut Lexer<'_>) -> Result<CObject, CadlError> {
    if peek_keyword(lexer, "use_archetype") {
        return c_archetype_root(lexer);
    }
    if peek_keyword(lexer, "use_node") {
        return c_complex_object_proxy(lexer);
    }
    if peek_keyword(lexer, "allow_archetype") {
        return archetype_slot(lexer);
    }

    let offset = lexer.offset();
    let rm_type_name = expect_rm_type_id(lexer)?;
    expect_symbol(lexer, '[')?;
    let node_id = expect_node_id(lexer)?;
    expect_symbol(lexer, ']')?;
    let is_primitive = primitive_kind(&rm_type_name).is_some();

    if is_primitive {
        let occurrences = parse_occurrences(lexer, false)?;
        let mut constraint = CPrimitive::Boolean {
            allow_true: true,
            allow_false: true,
        };
        let mut assumed = None;
        if peek_keyword(lexer, "matches") || peek_keyword(lexer, "is_in") {
            lexer.next();
            expect_symbol(lexer, '{')?;
            (constraint, assumed) = parse_inline_primitive(lexer, Some(&rm_type_name))?;
            expect_symbol(lexer, '}')?;
        }
        let mut object = CPrimitiveObject::new(rm_type_name, occurrences, constraint)
            .with_node_id(node_id)
            .map_err(|e| CadlError::at(offset, format!("invalid C_PRIMITIVE_OBJECT: {e}")))?;
        if let Some(value) = assumed {
            object = object.with_assumed_value(value);
        }
        return Ok(CObject::Primitive(object));
    }

    // `c_complex_object`'s own tail, resumed from just after `']'` — the
    // rm_type_id/node id were already read above to decide primitive vs
    // complex, so this repeats only the occurrences/attributes tail rather
    // than calling `c_complex_object` itself and re-reading them.
    let occurrences = parse_occurrences(lexer, false)?;
    let mut attributes = Vec::new();
    let mut attribute_tuples = Vec::new();
    if peek_keyword(lexer, "matches") || peek_keyword(lexer, "is_in") {
        lexer.next();
        expect_symbol(lexer, '{')?;
        loop {
            if consume_symbol_if(lexer, '}') {
                break;
            }
            if peek_keyword(lexer, "_default") {
                return Err(CadlError::at(
                    lexer.offset(),
                    "default_value (`_default = <...>`) is not implemented by this parser",
                ));
            }
            match c_attribute_def(lexer)? {
                AttributeDef::Attribute(a) => attributes.push(a),
                AttributeDef::Tuple(t) => attribute_tuples.push(t),
            }
        }
    }
    let complex = CComplexObject::new(rm_type_name, Some(node_id), occurrences, attributes)
        .map(|c| c.with_attribute_tuples(attribute_tuples))
        .map_err(|e| CadlError::at(offset, format!("invalid C_COMPLEX_OBJECT: {e}")))?;
    Ok(CObject::Complex(complex))
}

/// Reads text up to (not including) the next `]`, trimmed — `archetype_ref:
/// ARCHETYPE_HRID | ARCHETYPE_REF`'s own source, reconstructed by slicing
/// rather than by tokenizing (see [`Lexer::text_since`]'s own module
/// documentation for why). `c_archetype_root`'s grammar puts nothing else
/// between the reference and the closing bracket, so "everything up to
/// `]`" is exact for this one call site, not a guess.
///
/// # Errors
///
/// Returns [`CadlError`] if `]` is the very next token — an archetype
/// reference naming nothing.
fn expect_archetype_ref(lexer: &mut Lexer<'_>) -> Result<String, CadlError> {
    let start = lexer.offset();
    // `while let`, not `loop`/`match`: end of input ends this loop by the
    // loop's own shape, not by an arm that can be deleted. cargo-mutants
    // deleted the `']' | None => break` arm of the previous `loop` and the
    // test run hung for its full 300 s timeout instead of failing (CI run
    // 33775455452) — a parser loop must terminate on *any* mutation of its
    // body, or a survivor shows up as a timeout nobody reads.
    while let Some(token) = lexer.peek() {
        if matches!(token, Token::Symbol(']')) {
            break;
        }
        lexer.next();
    }
    let text = lexer.text_since(start).trim();
    if text.is_empty() {
        return Err(CadlError::at(
            start,
            "expected an archetype reference (ARCHETYPE_HRID or ARCHETYPE_REF), found `]`",
        ));
    }
    Ok(text.to_owned())
}

/// Reads `c_occurrences?` without defaulting an absent one — unlike
/// [`parse_occurrences`], where every non-root absence is refused.
/// `C_COMPLEX_OBJECT_PROXY.occurrences` is the one `C_OBJECT` field in this
/// crate an absence is meaningful for: `None` is AOM2's own `Void`, meaning
/// `use_target_occurrences()` (`crate::am::CComplexObjectProxy::new`'s own
/// documentation), not something to guess a value for or refuse.
fn parse_optional_occurrences(lexer: &mut Lexer<'_>) -> Result<Option<MultiplicityInterval>, CadlError> {
    if peek_keyword(lexer, "occurrences") {
        lexer.next();
        expect_keyword(lexer, "matches")?;
        expect_symbol(lexer, '{')?;
        let m = parse_multiplicity(lexer)?;
        expect_symbol(lexer, '}')?;
        Ok(Some(m))
    } else {
        Ok(None)
    }
}

/// `c_archetype_root: SYM_USE_ARCHETYPE rm_type_id '[' ID_CODE ','
/// archetype_ref ']' c_occurrences? ;`
fn c_archetype_root(lexer: &mut Lexer<'_>) -> Result<CObject, CadlError> {
    let offset = lexer.offset();
    expect_keyword(lexer, "use_archetype")?;
    let rm_type_name = expect_rm_type_id(lexer)?;
    expect_symbol(lexer, '[')?;
    let node_id = expect_node_id(lexer)?;
    expect_symbol(lexer, ',')?;
    let archetype_ref = expect_archetype_ref(lexer)?;
    expect_symbol(lexer, ']')?;
    let occurrences = parse_occurrences(lexer, false)?;

    let root = CArchetypeRoot::new(rm_type_name, archetype_ref, occurrences)
        .and_then(|r| r.with_node_id(node_id))
        .map_err(|e| CadlError::at(offset, format!("invalid C_ARCHETYPE_ROOT: {e}")))?;
    Ok(CObject::ArchetypeRoot(root))
}

/// `c_complex_object_proxy: SYM_USE_NODE rm_type_id '[' ID_CODE ']'
/// c_occurrences? ADL_PATH ;`
fn c_complex_object_proxy(lexer: &mut Lexer<'_>) -> Result<CObject, CadlError> {
    let offset = lexer.offset();
    expect_keyword(lexer, "use_node")?;
    let rm_type_name = expect_rm_type_id(lexer)?;
    expect_symbol(lexer, '[')?;
    let node_id = expect_node_id(lexer)?;
    expect_symbol(lexer, ']')?;
    let occurrences = parse_optional_occurrences(lexer)?;
    let path_offset = lexer.offset();
    let target_path = lexer.read_raw_path().ok_or_else(|| {
        CadlError::at(path_offset, "expected a target ADL_PATH, found end of input")
    })?;

    let proxy = CComplexObjectProxy::new(rm_type_name, Some(node_id), occurrences, target_path)
        .map_err(|e| CadlError::at(offset, format!("invalid C_COMPLEX_OBJECT_PROXY: {e}")))?;
    Ok(CObject::Proxy(proxy))
}

/// Consumes the tail of a `CONTAINED_REGEXP` after
/// [`Lexer::try_read_contained_regexp`] has already read its own delimited
/// body: an optional `';' STRING` assumed value, then the mandatory
/// closing `'}'`. Both are ordinary tokens, read through the lexer's own
/// tokenizer rather than more raw scanning.
fn finish_contained_regexp(lexer: &mut Lexer<'_>) -> Result<Option<String>, CadlError> {
    let mut assumed = None;
    if consume_symbol_if(lexer, ';') {
        let offset = lexer.offset();
        match lexer.next() {
            Some(Token::Str(s)) => assumed = Some(s),
            Some(other) => return Err(CadlError::at(offset, format!("expected a string literal, found {other}"))),
            None => return Err(CadlError::at(offset, "expected a string literal, found end of input")),
        }
    }
    expect_symbol(lexer, '}')?;
    Ok(assumed)
}

/// Maps [`Lexer::try_read_contained_regexp`]'s two failure shapes onto a
/// [`CadlError`] naming which one fired — shared between `c_attribute_def`
/// and `parse_slot_assertion`, the two call sites that need a
/// `CONTAINED_REGEXP` and disagree about nothing past this point.
fn expect_contained_regexp<'a>(lexer: &mut Lexer<'a>, context: &'static str) -> Result<&'a str, CadlError> {
    let offset = lexer.offset();
    match lexer.try_read_contained_regexp() {
        Ok(Some(pattern)) => Ok(pattern),
        Ok(None) => Err(CadlError::at(
            offset,
            format!(
                "expected a CONTAINED_REGEXP (`{{/…/}}` or `{{^…^}}`) {context} — a richer form is \
                 not implemented by this parser"
            ),
        )),
        Err(()) => Err(CadlError::at(
            offset,
            "malformed CONTAINED_REGEXP: no closing delimiter found before a newline or the end \
             of input",
        )),
    }
}

/// The one assertion shape this parser implements: `bound_path SYM_MATCHES
/// CONTAINED_REGEXP` (`constraint_expr` in `base_expressions.g4`) — the
/// shape every real `ARCHETYPE_SLOT` assertion this repository has found
/// actually uses (`archetype_id/value matches {/…/}`), and the narrowest
/// slice of `K15.10`'s own BEOM expression grammar that is real rather
/// than invented for this parser. Anything richer — a boolean operator, a
/// quantifier, a function call — is refused by
/// [`expect_contained_regexp`], not silently accepted or dropped.
///
/// The whole assertion's own source text is what `ArchetypeSlot::including`/
/// `excluding` carry (`K15.10`'s own residual: an assertion is carried, not
/// evaluated), reconstructed by slicing rather than by re-serialising the
/// parsed pieces — the same choice `expect_archetype_ref` makes for
/// `use_archetype`'s own reference.
fn parse_slot_assertion(lexer: &mut Lexer<'_>) -> Result<String, CadlError> {
    let start = lexer.offset();
    let path_offset = lexer.offset();
    lexer
        .read_raw_path()
        .ok_or_else(|| CadlError::at(path_offset, "expected an assertion path, found end of input"))?;
    expect_keyword(lexer, "matches")?;
    expect_contained_regexp(lexer, "after `matches` in an ARCHETYPE_SLOT assertion")?;
    finish_contained_regexp(lexer)?;
    Ok(lexer.text_since(start).trim().to_owned())
}

/// `archetype_slot: SYM_ALLOW_ARCHETYPE rm_type_id '[' ID_CODE ']'
/// (( c_occurrences? ( SYM_MATCHES '{' c_includes? c_excludes? '}' )? ) |
/// SYM_CLOSED ) ;`, `c_includes: SYM_INCLUDE assertion+ ;`,
/// `c_excludes: SYM_EXCLUDE assertion+ ;`.
///
/// `closed` is refused: its own grammar production carries no
/// `c_occurrences` at all, and every `C_OBJECT` variant this parser builds
/// except [`CComplexObjectProxy`] stores occurrences as a plain,
/// non-deferrable `MultiplicityInterval` (`A-54`'s own scope decision) —
/// [`ArchetypeSlot`] among them — so there is no way to build one for a
/// closed slot without inventing a value this parser has no grammar to
/// take it from. `matches { include ... exclude ... }` is built for the
/// one assertion shape [`parse_slot_assertion`] implements; a richer one
/// is refused there, by name, not silently accepted.
fn archetype_slot(lexer: &mut Lexer<'_>) -> Result<CObject, CadlError> {
    let offset = lexer.offset();
    expect_keyword(lexer, "allow_archetype")?;
    let rm_type_name = expect_rm_type_id(lexer)?;
    expect_symbol(lexer, '[')?;
    let node_id = expect_node_id(lexer)?;
    expect_symbol(lexer, ']')?;

    // `SYM_CLOSED`: the grammar's own alternative carries no `c_occurrences`
    // and no assertions. Refused until `A-71` made `occurrences` an
    // `Option` on every `C_OBJECT` — there was no value to build a slot
    // from without guessing one; now `None` is what the grammar says
    // (`A-73`), and `effective_occurrences` infers it like any other node.
    if peek_keyword(lexer, "closed") {
        lexer.next();
        let slot = ArchetypeSlot::new(rm_type_name, node_id, None)
            .map_err(|e| CadlError::at(offset, format!("invalid ARCHETYPE_SLOT: {e}")))?
            .closed();
        return Ok(CObject::Slot(slot));
    }

    let occurrences = parse_occurrences(lexer, false)?;
    let mut slot = ArchetypeSlot::new(rm_type_name, node_id, occurrences)
        .map_err(|e| CadlError::at(offset, format!("invalid ARCHETYPE_SLOT: {e}")))?;

    if peek_keyword(lexer, "matches") {
        lexer.next();
        expect_symbol(lexer, '{')?;
        loop {
            if consume_symbol_if(lexer, '}') {
                break;
            }
            let including = if peek_keyword(lexer, "include") {
                lexer.next();
                true
            } else if peek_keyword(lexer, "exclude") {
                lexer.next();
                false
            } else {
                return Err(CadlError::at(
                    lexer.offset(),
                    match lexer.peek() {
                        Some(other) => format!("expected `include`, `exclude`, or `}}`, found {other}"),
                        None => "expected `include`, `exclude`, or `}`, found end of input".to_owned(),
                    },
                ));
            };
            // `assertion+`: one or more, stopping only at the next keyword
            // or the closing `}` — not at a fixed count.
            loop {
                let before = lexer.offset();
                let assertion = parse_slot_assertion(lexer)?;
                // A sub-parser that returned `Ok` without consuming anything
                // would make this loop infinite: nothing below advances the
                // lexer, so the same text would be "parsed" forever. Real
                // `parse_slot_assertion` always consumes at least a path, so
                // this is unreachable today — cargo-mutants reached it by
                // replacing that function's body with a constant, and the
                // job timed out rather than failed (CI run 33775455452).
                if lexer.offset() == before {
                    return Err(CadlError::at(before, "an ARCHETYPE_SLOT assertion consumed no input"));
                }
                slot = if including { slot.including(assertion) } else { slot.excluding(assertion) };
                if peek_keyword(lexer, "include")
                    || peek_keyword(lexer, "exclude")
                    || matches!(lexer.peek(), Some(Token::Symbol('}')))
                {
                    break;
                }
            }
        }
    }

    Ok(CObject::Slot(slot))
}

// ---------------------------------------------------------------------
// c_inline_primitive_object and its per-kind productions
// ---------------------------------------------------------------------

/// The `CPrimitive` kind a wrapped `rm_type_id` names, if it is one of the
/// foundation-type names this parser recognises (`org.openehr.base
/// .foundation_types`), case-insensitively — real archetypes are not
/// perfectly consistent about capitalising `Terminology_code`.
fn primitive_kind(rm_type_name: &str) -> Option<&'static str> {
    const KINDS: &[&str] = &[
        "Boolean",
        "String",
        "Integer",
        "Real",
        "Date",
        "Time",
        "Date_time",
        "Duration",
        "Terminology_code",
    ];
    KINDS
        .iter()
        .find(|k| k.eq_ignore_ascii_case(rm_type_name))
        .copied()
}

/// `c_inline_primitive_object: c_integer | c_real | c_date | c_time |
/// c_date_time | c_duration | c_string | c_terminology_code | c_boolean ;`
///
/// `rm_type_hint`: `Some` for the wrapped form (`c_regular_primitive_object`,
/// dispatched by the RM type name already read), `None` for the unwrapped
/// shorthand under `c_objects` (dispatched by token shape instead — see
/// [`starts_inline_primitive`], which only ever calls this with `None` for
/// `Boolean`/`String`/`Integer`/`Real`/`Terminology_code` shapes — an
/// unwrapped interval's kind is its first bound's token, `A-72`; the four
/// temporal kinds are not reachable unwrapped in this parser at all).
///
/// The second element of the returned pair is the trailing assumed value
/// (`'; ' <value>`), every one of the eight kinds' own grammar production
/// (`cadl2_primitives.g4`) states as an optional suffix — except
/// `c_terminology_code`, whose `[ac3; at5]` is a second code inside its own
/// brackets rather than a value trailing them, read in
/// `parse_terminology_code_primitive` itself.
fn parse_inline_primitive(
    lexer: &mut Lexer<'_>,
    rm_type_hint: Option<&str>,
) -> Result<(CPrimitive, Option<PrimitiveValue>), CadlError> {
    match rm_type_hint {
        // A `match` on the lowered name, not a chain of `if` guards: a
        // guard can be mutated to `true` and route every kind to one
        // parser, and the last guard's mutant was reachable by no test
        // because `primitive_kind` only ever hands over these nine names.
        Some(kind) => match kind.to_ascii_lowercase().as_str() {
            "boolean" => parse_boolean_primitive(lexer),
            "string" => parse_string_primitive(lexer),
            "integer" => parse_integer_primitive(lexer),
            "real" => parse_real_primitive(lexer),
            "date" => parse_date_primitive(lexer),
            "time" => parse_time_primitive(lexer),
            "date_time" => parse_date_time_primitive(lexer),
            "duration" => parse_duration_primitive(lexer),
            "terminology_code" => parse_terminology_code_primitive(lexer),
            other => Err(CadlError::at(
                lexer.offset(),
                format!("`{other}` is not a primitive kind this parser recognises"),
            )),
        },
        None => match lexer.peek() {
            Some(Token::Str(_)) => parse_string_primitive(lexer),
            Some(Token::Symbol('[')) => parse_terminology_code_primitive(lexer),
            Some(Token::Real(_)) => parse_real_primitive(lexer),
            Some(Token::Integer(_)) => parse_integer_primitive(lexer),
            // `A-72`: the kind of an unwrapped interval is its first
            // bound's token kind — `INTEGER` for `integer_interval_value`,
            // `REAL` for `real_interval_value` (`odin_values.g4`) — not an
            // ambiguity a wrapping type name has to settle, which is what
            // `A-67` wrongly stated. A bound that begins like an ISO 8601
            // literal (digits then `-` or `:`, or `P`) is a temporal
            // interval, whose unwrapped form is refused by name.
            Some(Token::Symbol('|')) => match lexer.peek_interval_bound() {
                Some((_, raw)) if looks_iso8601(raw) => Err(CadlError::at(
                    lexer.offset(),
                    "an unwrapped temporal interval (a date, time, date-time, or duration bound with \
                     no wrapping rm_type_id) is not implemented by this parser",
                )),
                Some((Token::Integer(_), _)) => parse_integer_primitive(lexer),
                Some((Token::Real(_), _)) => parse_real_primitive(lexer),
                Some((other, _)) => Err(CadlError::at(
                    lexer.offset(),
                    format!("expected an integer or real bound after `|`, found {other}"),
                )),
                None => Err(CadlError::at(lexer.offset(), "expected an interval bound after `|`, found end of input")),
            },
            _ if peek_keyword(lexer, "true") || peek_keyword(lexer, "false") => parse_boolean_primitive(lexer),
            Some(other) => Err(CadlError::at(lexer.offset(), format!("expected a primitive value, found {other}"))),
            None => Err(CadlError::at(lexer.offset(), "expected a primitive value, found end of input")),
        },
    }
}

/// Reads an optional trailing `';' <value>` — `assumed_boolean_value`,
/// `assumed_string_value`, … in `cadl2_primitives.g4`, the same shape every
/// primitive kind but `C_TERMINOLOGY_CODE` shares (see
/// [`parse_inline_primitive`]'s own documentation).
fn parse_assumed_boolean(lexer: &mut Lexer<'_>) -> Result<Option<PrimitiveValue>, CadlError> {
    if !consume_symbol_if(lexer, ';') {
        return Ok(None);
    }
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::Word(w)) if w.eq_ignore_ascii_case("true") => Ok(Some(PrimitiveValue::Boolean(true))),
        Some(Token::Word(w)) if w.eq_ignore_ascii_case("false") => Ok(Some(PrimitiveValue::Boolean(false))),
        Some(other) => Err(CadlError::at(offset, format!("expected `true` or `false`, found {other}"))),
        None => Err(CadlError::at(offset, "expected `true` or `false`, found end of input")),
    }
}

fn parse_boolean_primitive(lexer: &mut Lexer<'_>) -> Result<(CPrimitive, Option<PrimitiveValue>), CadlError> {
    let (mut allow_true, mut allow_false) = (false, false);
    loop {
        let offset = lexer.offset();
        match lexer.next() {
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("true") => allow_true = true,
            Some(Token::Word(w)) if w.eq_ignore_ascii_case("false") => allow_false = true,
            Some(other) => return Err(CadlError::at(offset, format!("expected `true` or `false`, found {other}"))),
            None => return Err(CadlError::at(offset, "expected `true` or `false`, found end of input")),
        }
        if !consume_symbol_if(lexer, ',') {
            break;
        }
    }
    let assumed = parse_assumed_boolean(lexer)?;
    Ok((CPrimitive::Boolean { allow_true, allow_false }, assumed))
}

fn parse_assumed_string(lexer: &mut Lexer<'_>) -> Result<Option<PrimitiveValue>, CadlError> {
    if !consume_symbol_if(lexer, ';') {
        return Ok(None);
    }
    let offset = lexer.offset();
    match lexer.next() {
        Some(Token::Str(s)) => Ok(Some(PrimitiveValue::Text(s))),
        Some(other) => Err(CadlError::at(offset, format!("expected a string literal, found {other}"))),
        None => Err(CadlError::at(offset, "expected a string literal, found end of input")),
    }
}

fn parse_string_primitive(lexer: &mut Lexer<'_>) -> Result<(CPrimitive, Option<PrimitiveValue>), CadlError> {
    let mut list = Vec::new();
    loop {
        let offset = lexer.offset();
        match lexer.next() {
            Some(Token::Str(s)) => list.push(s),
            Some(other) => return Err(CadlError::at(offset, format!("expected a string literal, found {other}"))),
            None => return Err(CadlError::at(offset, "expected a string literal, found end of input")),
        }
        if !consume_symbol_if(lexer, ',') {
            break;
        }
    }
    let assumed = parse_assumed_string(lexer)?;
    Ok((CPrimitive::String { list }, assumed))
}

fn parse_assumed_integer(lexer: &mut Lexer<'_>) -> Result<Option<PrimitiveValue>, CadlError> {
    if !consume_symbol_if(lexer, ';') {
        return Ok(None);
    }
    Ok(Some(PrimitiveValue::Integer(expect_signed_integer(lexer)?)))
}

fn parse_integer_primitive(lexer: &mut Lexer<'_>) -> Result<(CPrimitive, Option<PrimitiveValue>), CadlError> {
    let constraint = if matches!(lexer.peek(), Some(Token::Symbol('|'))) {
        let range = parse_integer_interval(lexer)?;
        if matches!(lexer.peek(), Some(Token::Symbol(','))) {
            return Err(CadlError::at(
                lexer.offset(),
                "more than one C_INTEGER range is not representable by this crate's own \
                 CPrimitive::Integer (one Option<Interval<i64>>)",
            ));
        }
        CPrimitive::Integer { list: Vec::new(), range: Some(range) }
    } else {
        let mut list = Vec::new();
        loop {
            list.push(expect_signed_integer(lexer)?);
            if !consume_symbol_if(lexer, ',') {
                break;
            }
        }
        CPrimitive::Integer { list, range: None }
    };
    let assumed = parse_assumed_integer(lexer)?;
    Ok((constraint, assumed))
}

fn parse_integer_interval(lexer: &mut Lexer<'_>) -> Result<Interval<i64>, CadlError> {
    parse_numeric_interval(lexer, expect_signed_integer)
}

/// The integer and real interval spellings of `odin_values.g4`, which differ
/// only in the bound's own token:
///
/// ```text
/// '|' SYM_GT? v '..' SYM_LT? v '|'      a range, either end optionally open
/// '|' relop? v '|'                      one bound (`A-74`): `|>=0.0|`, `|<10|`, or `|5|`
/// '|' v SYM_PLUS_OR_MINUS v '|'         refused by name — not implemented
/// ```
///
/// Before `A-74` only the first was read, and the second — `|>=0.0|`, the
/// spelling 134 corpus files use for a non-negative magnitude — failed
/// inside the range reader as "expected a real number, found `=`", a
/// refusal naming a symptom, not the construct (`K15.6`).
fn parse_numeric_interval<T: SemanticOrd + Clone>(
    lexer: &mut Lexer<'_>,
    read_bound: fn(&mut Lexer<'_>) -> Result<T, CadlError>,
) -> Result<Interval<T>, CadlError> {
    let offset = lexer.offset();
    expect_symbol(lexer, '|')?;
    // `>` opens either a relop (`|>v|`, `|>=v|`) or a range's excluded
    // lower bound (`|>v..w|`); which one is settled by what follows `v`.
    let gt = consume_symbol_if(lexer, '>');
    let lt = !gt && consume_symbol_if(lexer, '<');
    let eq = (gt || lt) && consume_symbol_if(lexer, '=');
    let first = read_bound(lexer)?;
    let build = |interval: Result<Interval<T>, crate::error::ParseError>| {
        interval.map_err(|e| CadlError::at(offset, e.to_string()))
    };
    if consume_symbol_if(lexer, '|') {
        return build(if gt {
            Interval::new(Some(first), None, Some(eq), None)
        } else if lt {
            Interval::new(None, Some(first), None, Some(eq))
        } else {
            Interval::new(Some(first.clone()), Some(first), Some(true), Some(true))
        });
    }
    if lt || eq {
        return Err(CadlError::at(
            lexer.offset(),
            "a relop interval (`|<v|`, `|<=v|`, `|>=v|`) takes exactly one bound; expected `|` after it",
        ));
    }
    if matches!(lexer.peek(), Some(Token::Symbol('+'))) {
        return Err(CadlError::at(
            lexer.offset(),
            "the `|v +/- w|` interval spelling is not implemented by this parser",
        ));
    }
    expect_dotdot(lexer)?;
    let upper_excluded = consume_symbol_if(lexer, '<');
    let upper = read_bound(lexer)?;
    expect_symbol(lexer, '|')?;
    build(Interval::new(Some(first), Some(upper), Some(!gt), Some(!upper_excluded)))
}

fn parse_assumed_real(lexer: &mut Lexer<'_>) -> Result<Option<PrimitiveValue>, CadlError> {
    if !consume_symbol_if(lexer, ';') {
        return Ok(None);
    }
    Ok(Some(PrimitiveValue::Real(expect_signed_real(lexer)?)))
}

fn parse_real_primitive(lexer: &mut Lexer<'_>) -> Result<(CPrimitive, Option<PrimitiveValue>), CadlError> {
    let constraint = if matches!(lexer.peek(), Some(Token::Symbol('|'))) {
        let range = parse_real_interval(lexer)?;
        if matches!(lexer.peek(), Some(Token::Symbol(','))) {
            return Err(CadlError::at(
                lexer.offset(),
                "more than one C_REAL range is not representable by this crate's own \
                 CPrimitive::Real (one Option<Interval<Real>>)",
            ));
        }
        CPrimitive::Real { list: Vec::new(), range: Some(range) }
    } else {
        let mut list = Vec::new();
        loop {
            list.push(expect_signed_real(lexer)?);
            if !consume_symbol_if(lexer, ',') {
                break;
            }
        }
        CPrimitive::Real { list, range: None }
    };
    let assumed = parse_assumed_real(lexer)?;
    Ok((constraint, assumed))
}

fn parse_real_interval(lexer: &mut Lexer<'_>) -> Result<Interval<Real>, CadlError> {
    parse_numeric_interval(lexer, expect_signed_real)
}

/// `c_terminology_code: '[' ( AC_CODE ( ';' AT_CODE )? | AT_CODE ) ']' ;`
///
/// The assumed value is read here rather than through
/// [`parse_assumed_boolean`]'s siblings: it is a second code *inside* the
/// brackets, not a value trailing them, and the grammar's own note is
/// explicit that it "can only occur after an ac-code not after the single
/// at-code" — enforced below rather than accepted and left for
/// `Inv_valid_assumed_value` (`A-56`) to catch later.
fn parse_terminology_code_primitive(lexer: &mut Lexer<'_>) -> Result<(CPrimitive, Option<PrimitiveValue>), CadlError> {
    expect_symbol(lexer, '[')?;
    let offset = lexer.offset();
    let code = expect_word(lexer, "an at- or ac-code")?;
    if NodeIdSyntax::of(&code).is_none() {
        return Err(CadlError::at(offset, format!("`{code}` is not a valid at- or ac-code")));
    }
    let mut assumed = None;
    if consume_symbol_if(lexer, ';') {
        if !code.starts_with("ac") {
            return Err(CadlError::at(
                offset,
                "an assumed at-code (`[acN; atN]`) may only follow an ac-code, not a bare \
                 at-code — cadl2_primitives.g4's own note",
            ));
        }
        let at_offset = lexer.offset();
        let at_code = expect_word(lexer, "an assumed at-code")?;
        if !at_code.starts_with("at") || NodeIdSyntax::of(&at_code).is_none() {
            return Err(CadlError::at(at_offset, format!("`{at_code}` is not a valid at-code")));
        }
        assumed = Some(PrimitiveValue::Text(at_code));
    }
    expect_symbol(lexer, ']')?;
    Ok((
        CPrimitive::TerminologyCode {
            constraint: Some(code),
            constraint_status: None,
        },
        assumed,
    ))
}

/// Reads one ISO8601-shaped literal via [`Lexer::read_iso8601`] and parses
/// it via `T`'s own `FromStr` — the distinction between a real literal and
/// a malformed one is made entirely by whether `T::from_str` accepts it,
/// this function's own scan being a lexical boundary, not a grammar
/// (`A-65`, `read_iso8601`'s own documentation for why a dedicated scan
/// exists at all rather than the ordinary `Word`/`Integer` tokenizing every
/// other literal in this parser uses).
fn expect_temporal<T: FromStr>(lexer: &mut Lexer<'_>, what: &'static str) -> Result<T, CadlError> {
    let offset = lexer.offset();
    let Some(text) = lexer.read_iso8601() else {
        return Err(match lexer.peek() {
            Some(other) => CadlError::at(offset, format!("expected {what}, found {other}")),
            None => CadlError::at(offset, format!("expected {what}, found end of input")),
        });
    };
    T::from_str(text).map_err(|_| CadlError::at(offset, format!("`{text}` is not a valid {what}")))
}

/// Reads an optional trailing `';' <value>` for a temporal kind —
/// `assumed_date_value`/`assumed_time_value`/… in `cadl2_primitives.g4`,
/// the same shape [`parse_assumed_boolean`]'s siblings share. The text is
/// kept, not a re-serialised `T`: [`PrimitiveValue::Text`] stands in for
/// every non-numeric kind alike ([`PrimitiveValue`]'s own module
/// documentation), and validating through `T::from_str` without discarding
/// the original text is cheaper than parsing and reformatting.
fn parse_assumed_temporal<T: FromStr>(
    lexer: &mut Lexer<'_>,
    what: &'static str,
) -> Result<Option<PrimitiveValue>, CadlError> {
    if !consume_symbol_if(lexer, ';') {
        return Ok(None);
    }
    let offset = lexer.offset();
    let Some(text) = lexer.read_iso8601() else {
        return Err(match lexer.peek() {
            Some(other) => CadlError::at(offset, format!("expected {what}, found {other}")),
            None => CadlError::at(offset, format!("expected {what}, found end of input")),
        });
    };
    T::from_str(text).map_err(|_| CadlError::at(offset, format!("`{text}` is not a valid {what}")))?;
    Ok(Some(PrimitiveValue::Text(text.to_owned())))
}

macro_rules! temporal_primitive {
    ($fn_list:ident, $fn_interval:ident, $fn_primitive:ident, $ty:ty, $what:literal, $variant:ident) => {
        fn $fn_interval(lexer: &mut Lexer<'_>) -> Result<Interval<$ty>, CadlError> {
            let offset = lexer.offset();
            expect_symbol(lexer, '|')?;
            let lower_excluded = consume_symbol_if(lexer, '>');
            let lower: $ty = expect_temporal(lexer, $what)?;
            expect_dotdot(lexer)?;
            let upper_excluded = consume_symbol_if(lexer, '<');
            let upper: $ty = expect_temporal(lexer, $what)?;
            expect_symbol(lexer, '|')?;
            Interval::new(Some(lower), Some(upper), Some(!lower_excluded), Some(!upper_excluded))
                .map_err(|e| CadlError::at(offset, e.to_string()))
        }

        fn $fn_primitive(lexer: &mut Lexer<'_>) -> Result<(CPrimitive, Option<PrimitiveValue>), CadlError> {
            let constraint = if matches!(lexer.peek(), Some(Token::Symbol('|'))) {
                let range = $fn_interval(lexer)?;
                if matches!(lexer.peek(), Some(Token::Symbol(','))) {
                    return Err(CadlError::at(
                        lexer.offset(),
                        concat!(
                            "more than one range is not representable by this crate's own ",
                            "CPrimitive::",
                            stringify!($variant),
                            " (one Option<Interval<_>>)"
                        ),
                    ));
                }
                CPrimitive::$variant { range: vec![range], pattern: None }
            } else {
                let value: $ty = expect_temporal(lexer, $what)?;
                let mut range = vec![Interval::new(Some(value.clone()), Some(value), Some(true), Some(true))
                    .map_err(|e| CadlError::at(lexer.offset(), e.to_string()))?];
                while consume_symbol_if(lexer, ',') {
                    let value: $ty = expect_temporal(lexer, $what)?;
                    range.push(
                        Interval::new(Some(value.clone()), Some(value), Some(true), Some(true))
                            .map_err(|e| CadlError::at(lexer.offset(), e.to_string()))?,
                    );
                }
                CPrimitive::$variant { range, pattern: None }
            };
            let assumed = parse_assumed_temporal::<$ty>(lexer, $what)?;
            Ok((constraint, assumed))
        }
    };
}

temporal_primitive!(_unused_date_list, parse_date_interval, parse_date_primitive, Date, "ISO8601_DATE", Date);
temporal_primitive!(_unused_time_list, parse_time_interval, parse_time_primitive, Time, "ISO8601_TIME", Time);
temporal_primitive!(
    _unused_date_time_list,
    parse_date_time_interval,
    parse_date_time_primitive,
    DateTime,
    "ISO8601_DATE_TIME",
    DateTime
);
temporal_primitive!(
    _unused_duration_list,
    parse_duration_interval,
    parse_duration_primitive,
    Duration,
    "ISO8601_DURATION",
    Duration
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::am::CObject;

    #[test]
    fn a_bare_root_with_no_attributes_is_any_allowed() {
        let root = parse_definition("DV_TEXT[id1]").unwrap();
        assert_eq!(root.rm_type_name(), "DV_TEXT");
        assert_eq!(root.node_id(), Some("id1"));
        assert!(root.attributes().is_empty());
    }

    #[test]
    fn a_nested_element_with_a_string_constraint() {
        let source = r#"
            CLUSTER[id1] matches {
                items cardinality matches {0..*} matches {
                    ELEMENT[id2] occurrences matches {0..1} matches {
                        value matches {
                            DV_TEXT[id3] occurrences matches {1} matches {
                                value matches {"a", "b"}
                            }
                        }
                    }
                }
            }
        "#;
        let root = parse_definition(source).unwrap();
        assert_eq!(root.rm_type_name(), "CLUSTER");
        let items = &root.attributes()[0];
        assert_eq!(items.rm_attribute_name(), "items");
        assert!(items.cardinality().is_some());
        let CObject::Complex(element) = &items.children()[0] else {
            panic!("expected a complex ELEMENT");
        };
        assert_eq!(element.node_id(), Some("id2"));
        let CObject::Complex(dv_text) = &element.attributes()[0].children()[0] else {
            panic!("expected a complex DV_TEXT");
        };
        let CObject::Primitive(leaf) = &dv_text.attributes()[0].children()[0] else {
            panic!("expected a primitive String value");
        };
        assert_eq!(
            leaf.constraint(),
            &CPrimitive::String {
                list: vec!["a".to_owned(), "b".to_owned()],
            }
        );
    }

    #[test]
    fn a_quantity_with_a_real_range_and_a_terminology_code() {
        let source = r#"
            DV_QUANTITY[id1] matches {
                property matches {
                    Terminology_code[id2] occurrences matches {1} matches {[ac1]}
                }
                magnitude matches {
                    Real[id3] occurrences matches {1} matches {|0.0..1000.0|}
                }
                units matches {"mm[Hg]"}
            }
        "#;
        let root = parse_definition(source).unwrap();
        let property = &root.attributes()[0];
        let CObject::Primitive(code) = &property.children()[0] else {
            panic!("expected a primitive Terminology_code");
        };
        assert_eq!(
            code.constraint(),
            &CPrimitive::TerminologyCode {
                constraint: Some("ac1".to_owned()),
                constraint_status: None,
            }
        );
        let magnitude = &root.attributes()[1];
        let CObject::Primitive(real) = &magnitude.children()[0] else {
            panic!("expected a primitive Real");
        };
        assert_eq!(
            real.constraint(),
            &CPrimitive::Real {
                list: Vec::new(),
                range: Some(Interval::closed("0.0".parse().unwrap(), "1000.0".parse().unwrap()).unwrap()),
            }
        );
        // `units` uses the unwrapped shorthand: no `String[atN]` wrapper.
        let units = &root.attributes()[2];
        let CObject::Primitive(unwrapped) = &units.children()[0] else {
            panic!("expected an unwrapped primitive String");
        };
        assert_eq!(unwrapped.node_id(), Some(CPrimitiveObject::PRIMITIVE_NODE_ID));
    }

    /// The trailing `'; ' <value>` every primitive kind but
    /// `Terminology_code` shares (`cadl2_primitives.g4`'s own
    /// `assumed_*_value` productions) — one case per kind, wrapped, so each
    /// confirms both the parse and that `with_assumed_value` (`A-48`) is
    /// actually reached rather than the value being read and discarded.
    /// `Date` stands in for all four temporal kinds, which share one macro
    /// (`temporal_primitive!`) and, since `A-65`, one lexer scan
    /// (`Lexer::read_iso8601`); the other three get their own end-to-end
    /// coverage in `a_temporal_primitive_of_each_kind_is_parsed` below.
    #[test]
    fn a_wrapped_primitives_assumed_value_is_attached() {
        let cases: &[(&str, &str, PrimitiveValue)] = &[
            ("Boolean", "true; false", PrimitiveValue::Boolean(false)),
            ("String", "\"a\", \"b\"; \"a\"", PrimitiveValue::Text("a".to_owned())),
            ("Integer", "|0..10|; 5", PrimitiveValue::Integer(5)),
            ("Real", "|0.0..10.0|; 5.0", PrimitiveValue::Real("5.0".parse().unwrap())),
            (
                "Date",
                "2024-01-01; 2024-06-15",
                PrimitiveValue::Text("2024-06-15".to_owned()),
            ),
        ];
        for (kind, body, want) in cases {
            let source = format!(
                "ELEMENT[id1] matches {{ value matches {{ {kind}[id2] occurrences matches {{1}} \
                 matches {{{body}}} }} }}"
            );
            let root = parse_definition(&source).unwrap();
            let CObject::Primitive(leaf) = &root.attributes()[0].children()[0] else {
                panic!("expected a primitive {kind}, source: {source}");
            };
            assert_eq!(leaf.assumed_value(), Some(want), "kind {kind}, source: {source}");
        }
    }

    /// `A-65`: before `Lexer::read_iso8601` existed, none of these four
    /// kinds could parse a single literal — `Date`'s own coverage is above
    /// (a discrete value, via the assumed-value test); this covers a
    /// discrete `Time`, a ranged `Date_time`, and a `Duration`, so every
    /// kind the shared `temporal_primitive!` macro generates has been
    /// exercised at least once through the full `parse_definition` path.
    #[test]
    fn a_temporal_primitive_of_each_kind_is_parsed() {
        let cases: &[(&str, &str)] = &[
            ("Time", "12:30:00"),
            ("Date_time", "|2024-01-01T00:00:00..2024-12-31T23:59:59|"),
            ("Duration", "P1Y2M3DT4H5M6S"),
        ];
        for (kind, body) in cases {
            let source = format!(
                "ELEMENT[id1] matches {{ value matches {{ {kind}[id2] occurrences matches {{1}} \
                 matches {{{body}}} }} }}"
            );
            let root = parse_definition(&source).unwrap();
            let CObject::Primitive(leaf) = &root.attributes()[0].children()[0] else {
                panic!("expected a primitive {kind}, source: {source}");
            };
            assert_eq!(leaf.rm_type_name(), *kind);
        }
    }

    /// The unwrapped shorthand (`c_objects`'s own `parse_inline_primitive`
    /// call, `None` hint) reaches the same assumed-value path as the
    /// wrapped form above — a second call site, not a second
    /// implementation (`lib:A-33`).
    #[test]
    fn an_unwrapped_primitives_assumed_value_is_attached() {
        let source = r#"CLUSTER[id1] matches { units matches {"mm[Hg]", "kPa"; "mm[Hg]"} }"#;
        let root = parse_definition(source).unwrap();
        let CObject::Primitive(leaf) = &root.attributes()[0].children()[0] else {
            panic!("expected an unwrapped primitive String");
        };
        assert_eq!(
            leaf.assumed_value(),
            Some(&PrimitiveValue::Text("mm[Hg]".to_owned()))
        );
    }

    /// `[ac3; at5]`: an ac-code's assumed at-code, the one assumed-value
    /// shape with its own grammar production rather than a trailing `;
    /// <value>` — read inside `parse_terminology_code_primitive` itself.
    #[test]
    fn an_ac_codes_assumed_at_code_is_attached() {
        let source = "ELEMENT[id1] matches { value matches { Terminology_code[id2] \
                       occurrences matches {1} matches {[ac1; at5]} } }";
        let root = parse_definition(source).unwrap();
        let CObject::Primitive(leaf) = &root.attributes()[0].children()[0] else {
            panic!("expected a primitive Terminology_code");
        };
        assert_eq!(
            leaf.constraint(),
            &CPrimitive::TerminologyCode {
                constraint: Some("ac1".to_owned()),
                constraint_status: None,
            }
        );
        assert_eq!(
            leaf.assumed_value(),
            Some(&PrimitiveValue::Text("at5".to_owned()))
        );
    }

    /// The grammar's own note: an assumed at-code may only follow an
    /// ac-code, never a bare at-code — `[at5; at6]` is refused, not
    /// silently accepted with the second code ignored.
    #[test]
    fn an_at_codes_own_assumed_value_is_refused_not_silently_ignored() {
        let source = "ELEMENT[id1] matches { value matches { Terminology_code[id2] \
                       occurrences matches {1} matches {[at5; at6]} } }";
        let err = parse_definition(source).unwrap_err();
        assert!(err.reason.contains("ac-code"), "{err}");
    }

    /// `c_attribute`'s own `SYM_MATCHES CONTAINED_REGEXP` shorthand: no
    /// wrapping `String[id]` node, the attribute's single child built
    /// directly as an unwrapped `C_STRING` whose `list` holds the pattern
    /// with its own delimiters intact (`A-63`'s single-list shape).
    #[test]
    fn an_attributes_contained_regexp_shorthand_builds_an_unwrapped_c_string() {
        let source = r"CLUSTER[id1] matches { units matches {/mm\[Hg\]|kPa/} }";
        let root = parse_definition(source).unwrap();
        let CObject::Primitive(leaf) = &root.attributes()[0].children()[0] else {
            panic!("expected an unwrapped primitive String");
        };
        assert_eq!(leaf.node_id(), Some(CPrimitiveObject::PRIMITIVE_NODE_ID));
        assert_eq!(
            leaf.constraint(),
            &CPrimitive::String { list: vec![r"/mm\[Hg\]|kPa/".to_owned()] }
        );
        assert_eq!(leaf.assumed_value(), None);
    }

    /// `CONTAINED_REGEXP`'s own optional `'; ' STRING` assumed value,
    /// carried through to the same `C_PRIMITIVE_OBJECT.assumed_value`
    /// (`A-48`) every other assumed-value form in this parser attaches to.
    #[test]
    fn a_contained_regexps_own_assumed_value_is_attached() {
        let source = r#"CLUSTER[id1] matches { units matches {/mm\[Hg\]|kPa/; "mm[Hg]"} }"#;
        let root = parse_definition(source).unwrap();
        let CObject::Primitive(leaf) = &root.attributes()[0].children()[0] else {
            panic!("expected an unwrapped primitive String");
        };
        assert_eq!(
            leaf.assumed_value(),
            Some(&PrimitiveValue::Text("mm[Hg]".to_owned()))
        );
    }

    /// A `^…^`-delimited regex (`CARET_REGEXP`, `base_lexer.g4`'s other
    /// delimiter form) is recognised on the same terms as `/…/`.
    #[test]
    fn a_caret_delimited_contained_regexp_is_recognised() {
        let source = r"CLUSTER[id1] matches { units matches {^mm\^Hg\^^} }";
        let root = parse_definition(source).unwrap();
        let CObject::Primitive(leaf) = &root.attributes()[0].children()[0] else {
            panic!("expected an unwrapped primitive String");
        };
        assert_eq!(
            leaf.constraint(),
            &CPrimitive::String { list: vec![r"^mm\^Hg\^^".to_owned()] }
        );
    }

    /// A `CONTAINED_REGEXP` with no closing delimiter before the end of
    /// input is refused, naming the malformation, not silently treated as
    /// an ordinary `{c_objects}` block (which would then fail with a much
    /// less specific error, or in the worst case at the wrong offset).
    #[test]
    fn an_unterminated_contained_regexp_is_refused_naming_it() {
        let source = "CLUSTER[id1] matches { units matches {/mm[Hg]kPa }";
        let err = parse_definition(source).unwrap_err();
        assert!(err.reason.contains("malformed CONTAINED_REGEXP"), "{err}");
    }

    /// The other real slice of `K15.10` this parser implements: an
    /// `ARCHETYPE_SLOT`'s own `include`/`exclude` assertions, each the one
    /// shape `parse_slot_assertion` supports. Two assertions under one
    /// `include` (`assertion+`, not exactly one) and one under `exclude`,
    /// so both the "more than one" loop and the keyword switch are
    /// exercised in the same fixture.
    #[test]
    fn an_archetype_slots_include_and_exclude_assertions_are_carried() {
        let source = "CLUSTER[id1] matches { items matches { allow_archetype CLUSTER[id2] \
                       occurrences matches {0..1} matches { \
                       include \
                       archetype_id/value matches {/openEHR-EHR-CLUSTER\\.device\\..*/} \
                       archetype_id/value matches {/openEHR-EHR-CLUSTER\\.exposure\\..*/} \
                       exclude \
                       archetype_id/value matches {/.*\\.experimental\\..*/} \
                       } } }";
        let root = parse_definition(source).unwrap();
        let CObject::Slot(slot) = &root.attributes()[0].children()[0] else {
            panic!("expected an ARCHETYPE_SLOT");
        };
        assert_eq!(slot.includes().len(), 2);
        assert_eq!(
            slot.includes()[0],
            "archetype_id/value matches {/openEHR-EHR-CLUSTER\\.device\\..*/}"
        );
        assert_eq!(slot.excludes().len(), 1);
        assert!(!slot.any_allowed());
        assert!(!slot.is_closed());
    }

    /// `A-71`, `K15.32`: an omitted `occurrences` is AOM2's own `Void`,
    /// carried as `None` — not refused (the rule before `A-71`), and not
    /// filled in by the parser either, since a round trip must not invent
    /// what the author omitted. `effective_occurrences` infers it from the
    /// owning attribute when asked: `0..1` under a single-valued attribute,
    /// `0..upper` under a container.
    #[test]
    fn occurrences_omitted_on_a_non_root_node_is_carried_unstated_and_inferred_from_its_owner() {
        let root = parse_definition("CLUSTER[id1] matches { items matches { ELEMENT[id2] matches { } } }").unwrap();
        let items = &root.attributes()[0];
        assert_eq!(items.children()[0].occurrences(), None);
        assert_eq!(
            items.children()[0].effective_occurrences(items),
            Some(MultiplicityInterval::from_zero_to(Some(1)))
        );

        let root = parse_definition(
            "CLUSTER[id1] matches { items cardinality matches {0..3} matches { ELEMENT[id2] matches { } } }",
        )
        .unwrap();
        let items = &root.attributes()[0];
        assert_eq!(items.children()[0].occurrences(), None);
        assert_eq!(
            items.children()[0].effective_occurrences(items),
            Some(MultiplicityInterval::from_zero_to(Some(3)))
        );
        // The root itself: AOM2 fixes it at exactly one, and there is no
        // owning attribute to infer from, so the parser states it.
        assert_eq!(root.occurrences(), Some(&ROOT_OCCURRENCES));
    }

    /// The one assertion shape this parser still refuses: `constraint_expr`
    /// allows `'{' c_inline_primitive_object '}'` as an alternative to
    /// `CONTAINED_REGEXP`, and only the latter is implemented
    /// (`parse_slot_assertion`'s own documentation) — a quoted-string form
    /// is refused by name, not silently accepted as though it were a
    /// regex. Occurrences are stated so this refusal, not "occurrences
    /// omitted", is the one that fires.
    #[test]
    fn a_slot_assertion_using_a_quoted_string_instead_of_a_regex_is_refused() {
        let source = "CLUSTER[id1] matches { items matches { allow_archetype CLUSTER[id2] \
                       occurrences matches {0..1} matches { include \
                       archetype_id/value matches {\"literal\"} } } }";
        let err = parse_definition(source).unwrap_err();
        assert!(err.reason.contains("CONTAINED_REGEXP"), "{err}");
    }

    /// A closed slot's own grammar production carries no `c_occurrences`
    /// and no assertions. Until `A-71` gave [`crate::am::ArchetypeSlot`] an
    /// `Option` to hold that absence, this was refused by name (`A-62`);
    /// now (`A-73`) it is built closed, with occurrences unstated, and
    /// nothing after `closed` is consumed.
    #[test]
    fn a_closed_archetype_slot_is_parsed_closed_with_unstated_occurrences() {
        let source = "CLUSTER[id1] matches { items matches { allow_archetype CLUSTER[id2] \
                       closed } }";
        let root = parse_definition(source).unwrap();
        let CObject::Slot(slot) = &root.attributes()[0].children()[0] else {
            panic!("expected an ARCHETYPE_SLOT");
        };
        assert!(slot.is_closed());
        assert_eq!(slot.occurrences(), None);
        assert!(slot.includes().is_empty() && slot.excludes().is_empty());
        assert_eq!(slot.node_id(), "id2");
    }

    /// The one `ARCHETYPE_SLOT` form this parser does build: no `matches`
    /// clause at all, so `any_allowed()` is true and
    /// `am::validate::walk_slot` (`A-60`) can fully check whatever, if
    /// anything, fills it.
    #[test]
    fn an_unrestricted_archetype_slot_is_parsed() {
        let source = "CLUSTER[id1] matches { items matches { allow_archetype ELEMENT[id2] \
                       occurrences matches {0..1} } }";
        let root = parse_definition(source).unwrap();
        let CObject::Slot(slot) = &root.attributes()[0].children()[0] else {
            panic!("expected an ARCHETYPE_SLOT");
        };
        assert_eq!(slot.node_id(), "id2");
        assert!(!slot.is_closed());
        assert!(slot.any_allowed());
    }

    /// `c_archetype_root`'s `archetype_ref` is reconstructed by slicing the
    /// source, not by tokenizing it (`expect_archetype_ref`'s own
    /// documentation) — this is the fixture that proves the slice is exact
    /// across the `-`-separated `rm_publisher-rm_package-rm_class` prefix
    /// the lexer's own word-scanner splits into several tokens.
    #[test]
    fn a_use_archetype_is_parsed_into_a_c_archetype_root() {
        let source = "CLUSTER[id1] matches { items matches { \
                       use_archetype CLUSTER[id2, openEHR-EHR-CLUSTER.device.v1] \
                       occurrences matches {0..1} } }";
        let root = parse_definition(source).unwrap();
        let CObject::ArchetypeRoot(filled) = &root.attributes()[0].children()[0] else {
            panic!("expected a C_ARCHETYPE_ROOT");
        };
        assert_eq!(filled.node_id(), Some("id2"));
        assert_eq!(filled.archetype_ref(), "openEHR-EHR-CLUSTER.device.v1");
    }

    /// `c_complex_object_proxy`'s trailing `ADL_PATH` is read as raw text up
    /// to the next whitespace ([`super::cadl_lexer::Lexer::read_raw_path`]),
    /// and its `occurrences` builds `None` — AOM2's own `Void` meaning
    /// `use_target_occurrences()` — rather than being refused, unlike every
    /// other node kind an absent `occurrences` refuses.
    #[test]
    fn a_use_node_with_no_stated_occurrences_defers_to_its_target() {
        let source = "CLUSTER[id1] matches { items matches { \
                       use_node ELEMENT[id2] /items[id9] } }";
        let root = parse_definition(source).unwrap();
        let CObject::Proxy(proxy) = &root.attributes()[0].children()[0] else {
            panic!("expected a C_COMPLEX_OBJECT_PROXY");
        };
        assert_eq!(proxy.node_id(), Some("id2"));
        assert_eq!(proxy.target_path(), "/items[id9]");
        assert_eq!(proxy.occurrences(), None);
    }

    /// `A-70`: `c_attribute`'s `ADL_PATH` alternative — an attribute in
    /// differential form, the way a specialised archetype states only what
    /// it redefines. The corpus file that surfaced this
    /// (`openEHR-EHR-OBSERVATION.redefine_cardinality.v1.0.0.adls`) has
    /// exactly this shape: a path, a cardinality, no children.
    #[test]
    fn a_differential_path_attribute_is_parsed_with_its_parent_path_split_off() {
        let source = "OBSERVATION[id1.1] matches { /data/events cardinality matches {2..8; ordered} }";
        let root = parse_definition(source).unwrap();
        let attr = &root.attributes()[0];
        assert_eq!(attr.rm_attribute_name(), "events");
        assert_eq!(attr.differential_path(), Some("/data"));
        assert!(attr.cardinality().is_some());
        assert!(attr.children().is_empty());
    }

    /// The other way round: a bare name has no differential path at all.
    /// cargo-mutants turned the first shape's "is this a path?" guard into
    /// `true`, giving every plain attribute a parent of `/`, and nothing
    /// asserted the absence.
    #[test]
    fn a_plain_attribute_has_no_differential_path() {
        let root = parse_definition("CLUSTER[id1] matches { items cardinality matches {0..*} matches { \
                                     ELEMENT[id2] occurrences matches {0..1} } }")
        .unwrap();
        assert_eq!(root.attributes()[0].rm_attribute_name(), "items");
        assert_eq!(root.attributes()[0].differential_path(), None);
    }

    #[test]
    fn a_relative_differential_path_keeps_its_predicates_in_the_parent_part() {
        let source = "CLUSTER[id1.1] matches { items[id2]/value matches { \
                       DV_QUANTITY[id0.16] occurrences matches {0..1} } }";
        let root = parse_definition(source).unwrap();
        let attr = &root.attributes()[0];
        assert_eq!(attr.rm_attribute_name(), "value");
        assert_eq!(attr.differential_path(), Some("items[id2]"));
        assert_eq!(attr.children().len(), 1);
    }

    #[test]
    fn a_root_level_differential_path_names_the_root_as_its_parent() {
        let root = parse_definition("CLUSTER[id1.1] matches { /items cardinality matches {1..*} }").unwrap();
        assert_eq!(root.attributes()[0].differential_path(), Some("/"));
        assert_eq!(root.attributes()[0].rm_attribute_name(), "items");
    }

    /// Before `A-70` the lexer's fallback made `/` a one-character "word"
    /// and this parser accepted it as an attribute name, so `/data/events`
    /// became three attributes and failed later as a `VOKU` duplicate — a
    /// refusal naming the wrong thing (`K15.6`). A name that is not an
    /// identifier is now refused as one, and a path ending in `/` names no
    /// attribute at all.
    #[test]
    fn a_name_that_is_not_an_identifier_is_refused_by_name() {
        let err = parse_definition("CLUSTER[id1] matches { : matches {} }").unwrap_err();
        assert!(err.reason.contains("is not an attribute name"), "{err}");
        let err = parse_definition("CLUSTER[id1] matches { /data/ matches {} }").unwrap_err();
        assert!(err.reason.contains("is not an attribute name"), "{err}");
    }

    /// One wrapped primitive under `ELEMENT[id1]/value`, for the tests
    /// below that need the parsed `C_PRIMITIVE_OBJECT` itself.
    fn wrapped(kind: &str, body: &str) -> Result<CPrimitiveObject, CadlError> {
        let source = format!(
            "ELEMENT[id1] matches {{ value matches {{ {kind}[id2] occurrences matches {{1}} \
             matches {{{body}}} }} }}"
        );
        let root = parse_definition(&source)?;
        let CObject::Primitive(p) = &root.attributes()[0].children()[0] else {
            panic!("expected a C_PRIMITIVE_OBJECT");
        };
        Ok(p.clone())
    }

    /// The mutation-testing job (CI run 33775455452) found both of
    /// `parse_assumed_boolean`'s match guards reachable by no test: only
    /// `; false` was ever parsed, so `; true` refused, or `; maybe`
    /// accepted as `false`, went unnoticed.
    #[test]
    fn an_assumed_boolean_is_read_either_way_and_anything_else_is_refused() {
        assert_eq!(
            wrapped("Boolean", "true, false; true").unwrap().assumed_value(),
            Some(&PrimitiveValue::Boolean(true))
        );
        assert_eq!(
            wrapped("Boolean", "true, false; false").unwrap().assumed_value(),
            Some(&PrimitiveValue::Boolean(false))
        );
        let err = wrapped("Boolean", "true; maybe").unwrap_err();
        assert!(err.reason.contains("expected `true` or `false`"), "{err}");
    }

    /// Same run: no test parsed an integer *list*, so the `,` loop in
    /// `parse_integer_primitive` could be inverted unnoticed.
    #[test]
    fn an_integer_list_is_read_whole() {
        assert_eq!(
            wrapped("Integer", "1, 2, 3").unwrap().constraint(),
            &CPrimitive::Integer { list: vec![1, 2, 3], range: None }
        );
        assert_eq!(
            wrapped("Integer", "7").unwrap().constraint(),
            &CPrimitive::Integer { list: vec![7], range: None }
        );
    }

    /// Same run: `use_node`'s stated occurrences were only ever tested
    /// absent, so `parse_optional_occurrences` returning `None` for a
    /// stated one went unnoticed.
    #[test]
    fn a_use_nodes_stated_occurrences_are_kept() {
        let source = "CLUSTER[id1] matches { items cardinality matches {0..*} matches { \
                       use_node ELEMENT[id2] occurrences matches {0..3} /items[id9] } }";
        let root = parse_definition(source).unwrap();
        let CObject::Proxy(proxy) = &root.attributes()[0].children()[0] else {
            panic!("expected a C_COMPLEX_OBJECT_PROXY");
        };
        assert_eq!(
            proxy.occurrences(),
            Some(&MultiplicityInterval::new(0, Some(3)).unwrap())
        );
    }

    /// Same run: an assumed code that is well-formed but an `ac`-code, not
    /// an `at`-code — the second half of the check, which the malformed
    /// case alone never exercised.
    #[test]
    fn an_assumed_code_that_is_an_ac_code_is_refused_as_not_an_at_code() {
        let err = parse_definition("ELEMENT[id1] matches { value matches { [ac1; ac2] } }").unwrap_err();
        assert!(err.reason.contains("is not a valid at-code"), "{err}");
    }

    /// `A-67`: `C_ATTRIBUTE_TUPLE` syntax itself is now implemented, but a
    /// row's own items are always the unwrapped shorthand — the grammar
    /// gives `c_primitive_tuple_item` no room for a wrapping `rm_type_id`
    /// — so AOM2's own canonical `{units, magnitude}` example, whose
    /// magnitude column is a *range*, depends on the unwrapped interval's
    /// kind being decidable. Until `A-72` this test asserted the refusal
    /// `A-67` gave it; the grammar decides the kind by token
    /// (`odin_values.g4`), so it now asserts the parse: the row's second
    /// item is a `C_INTEGER` range.
    #[test]
    fn a_c_attribute_tuple_with_an_unwrapped_interval_item_is_parsed() {
        let source = r#"
            DV_QUANTITY[id1] matches {
                [units, magnitude] matches {
                    [{"mm[Hg]"}, {|0..300|}]
                }
            }
        "#;
        let root = parse_definition(source).unwrap();
        let tuple = &root.attribute_tuples()[0];
        let row = &tuple.tuples()[0];
        assert_eq!(
            row.members()[1].constraint(),
            &CPrimitive::Integer { list: Vec::new(), range: Some(Interval::closed(0, 300).unwrap()) }
        );
    }

    /// `A-72`: the kind of an unwrapped interval is its first bound's token
    /// — `INTEGER` builds `integer_interval_value`, `REAL` builds
    /// `real_interval_value` (`odin_values.g4`) — so `|0..100|` is a
    /// `C_INTEGER` and `|0.0..100.0|` a `C_REAL`, with or without an assumed
    /// value, and a bound mixing the two is refused by name as the grammar
    /// refuses it. The corpus file that surfaced this
    /// (`openehr-TEST_PKG-WHOLE.assumed_values.v1.0.0.adls`) writes
    /// `integer_attr3 matches {|0..100|; 10}`.
    #[test]
    fn an_unwrapped_intervals_kind_is_decided_by_its_first_bounds_token() {
        let parse = |body: &str| {
            let root = parse_definition(&format!("WHOLE[id1] matches {{ attr matches {{{body}}} }}")).unwrap();
            let CObject::Primitive(p) = &root.attributes()[0].children()[0] else {
                panic!("expected a C_PRIMITIVE_OBJECT");
            };
            p.clone()
        };
        let integer = parse("|0..100|; 10");
        assert_eq!(
            integer.constraint(),
            &CPrimitive::Integer { list: Vec::new(), range: Some(Interval::closed(0, 100).unwrap()) }
        );
        assert_eq!(integer.assumed_value(), Some(&PrimitiveValue::Integer(10)));
        let real = parse("|0.0..100.0|");
        assert!(matches!(real.constraint(), CPrimitive::Real { range: Some(_), .. }), "{real:?}");
        let negative = parse("|-5..5|");
        assert!(matches!(negative.constraint(), CPrimitive::Integer { range: Some(_), .. }));
        // Bounds that share a prefix with an ISO 8601 shape — four digits,
        // or two — are numbers all the same: `looks_iso8601` needs the
        // `-` or `:` too. cargo-mutants turned each `&&` and `==` in it
        // and nothing noticed until these two.
        assert!(matches!(parse("|1000..2000|").constraint(), CPrimitive::Integer { range: Some(_), .. }));
        assert!(matches!(parse("|10..20|").constraint(), CPrimitive::Integer { range: Some(_), .. }));
        // And the bare unwrapped value, which the same dispatcher reads
        // by the same token — never tested unwrapped before this.
        assert_eq!(parse("5").constraint(), &CPrimitive::Integer { list: vec![5], range: None });
        assert!(matches!(parse("5.5").constraint(), CPrimitive::Real { range: None, .. }));

        let err = parse_definition("WHOLE[id1] matches { attr matches {|0..100.0|} }").unwrap_err();
        assert!(err.reason.contains("expected an integer"), "{err}");
        let err = parse_definition("WHOLE[id1] matches { attr matches {|2024-01-01..2024-12-31|} }").unwrap_err();
        assert!(err.reason.contains("unwrapped temporal interval"), "{err}");
        let err = parse_definition("WHOLE[id1] matches { attr matches {|PT0S..PT1H|} }").unwrap_err();
        assert!(err.reason.contains("unwrapped temporal interval"), "{err}");
        let err = parse_definition("WHOLE[id1] matches { attr matches {|09:00:00..17:00:00|} }").unwrap_err();
        assert!(err.reason.contains("unwrapped temporal interval"), "{err}");
        let err = parse_definition(r#"WHOLE[id1] matches { attr matches {|"a".."b"|} }"#).unwrap_err();
        assert!(err.reason.contains("expected an integer or real bound"), "{err}");
    }

    /// `A-74`: `odin_values.g4`'s second interval spelling, one bound with
    /// a relop — `|>=0.0|` is how 134 corpus files say "non-negative" —
    /// read as a half-open interval; a bare `|5|` is the point `5..5`;
    /// a relop followed by `..` and the `+/-` spelling are refused by
    /// name, not as "expected a real number, found `=`".
    #[test]
    fn a_relop_interval_is_read_as_one_open_ended_bound() {
        let range = |body: &str| {
            let root = parse_definition(&format!("WHOLE[id1] matches {{ attr matches {{{body}}} }}")).unwrap();
            let CObject::Primitive(p) = &root.attributes()[0].children()[0] else {
                panic!("expected a C_PRIMITIVE_OBJECT");
            };
            p.constraint().clone()
        };
        let CPrimitive::Real { range: Some(r), .. } = range("|>=0.0|") else { panic!() };
        assert_eq!(r.lower().map(ToString::to_string), Some("0.0".to_owned()));
        assert_eq!(r.lower_included(), Some(true));
        assert!(r.upper_unbounded());
        let CPrimitive::Integer { range: Some(r), .. } = range("|>0|") else { panic!() };
        assert_eq!((r.lower(), r.lower_included(), r.upper()), (Some(&0), Some(false), None));
        let CPrimitive::Integer { range: Some(r), .. } = range("|<=10|") else { panic!() };
        assert_eq!((r.lower(), r.upper(), r.upper_included()), (None, Some(&10), Some(true)));
        let CPrimitive::Integer { range: Some(r), .. } = range("|<10|") else { panic!() };
        assert_eq!((r.upper(), r.upper_included()), (Some(&10), Some(false)));
        let CPrimitive::Integer { range: Some(r), .. } = range("|5|") else { panic!() };
        assert_eq!((r.lower(), r.upper()), (Some(&5), Some(&5)));
        // The range's own optional open lower bound is unchanged.
        let CPrimitive::Integer { range: Some(r), .. } = range("|>0..10|") else { panic!() };
        assert_eq!((r.lower_included(), r.upper()), (Some(false), Some(&10)));

        let err = parse_definition("WHOLE[id1] matches { attr matches {|>=0..10|} }").unwrap_err();
        assert!(err.reason.contains("relop interval"), "{err}");
        let err = parse_definition("WHOLE[id1] matches { attr matches {|5 +/- 1|} }").unwrap_err();
        assert!(err.reason.contains("+/-"), "{err}");
    }

    /// The unwrapped boolean shorthand, reached through a tuple row —
    /// the one path that hands `parse_inline_primitive` a token its
    /// `c_objects` gate has not already screened. `{true}` is a
    /// `C_BOOLEAN`; a bare word that is not a boolean is refused as "not a
    /// primitive value", not mis-read as one. cargo-mutants could turn the
    /// boolean guard to `true`, to `false`, or its `||` to `&&` unnoticed.
    #[test]
    fn an_unwrapped_boolean_in_a_tuple_row_is_a_c_boolean_and_a_bare_word_is_not() {
        let source = "
            SOME_TYPE[id1] matches {
                [flag, magnitude] matches {
                    [{true}, {5}],
                    [{false}, {6}]
                }
            }
        ";
        let root = parse_definition(source).unwrap();
        let rows = root.attribute_tuples()[0].tuples();
        assert_eq!(
            rows[0].members()[0].constraint(),
            &CPrimitive::Boolean { allow_true: true, allow_false: false }
        );
        assert_eq!(
            rows[1].members()[0].constraint(),
            &CPrimitive::Boolean { allow_true: false, allow_false: true }
        );
        let err = parse_definition("SOME_TYPE[id1] matches { [flag, magnitude] matches { [{xyz}, {5}] } }")
            .unwrap_err();
        assert!(err.reason.contains("expected a primitive value"), "{err}");
    }

    /// The tuple mechanism itself, proven end-to-end: two rows, each
    /// pairing a unit string with a discrete magnitude value (not a
    /// range, so no unwrapped-interval ambiguity — see the test above)
    /// — the same `{deg F, deg C}` shape
    /// `am::constraint::tests::a_units_magnitude_tuple_pairs_each_unit_with_its_own_range`
    /// builds directly, here read from real cADL text instead.
    #[test]
    fn a_c_attribute_tuple_with_discrete_values_is_parsed() {
        let source = r#"
            DV_QUANTITY[id1] matches {
                [units, magnitude] matches {
                    [{"deg F"}, {212.0}],
                    [{"deg C"}, {100.0}]
                }
            }
        "#;
        let root = parse_definition(source).unwrap();
        assert_eq!(root.attribute_tuples().len(), 1);
        let tuple = &root.attribute_tuples()[0];
        assert_eq!(
            tuple.members().iter().map(CAttribute::rm_attribute_name).collect::<Vec<_>>(),
            ["units", "magnitude"]
        );
        assert_eq!(tuple.tuples().len(), 2);
        assert_eq!(tuple.tuples()[0].members()[0].constraint(), &CPrimitive::String { list: vec!["deg F".to_owned()] });
        assert_eq!(
            tuple.tuples()[0].members()[1].constraint(),
            &CPrimitive::Real { list: vec!["212.0".parse().unwrap()], range: None }
        );
    }

    /// A regex tuple item — `c_primitive_tuple_item`'s own `CONTAINED_REGEXP`
    /// alternative, the same shorthand `c_attribute`'s own regex form uses.
    #[test]
    fn a_c_attribute_tuple_with_a_regex_item_is_parsed() {
        let source = r"
            DV_QUANTITY[id1] matches {
                [units, magnitude] matches {
                    [{/mm\[Hg\]|kPa/}, {300.0}]
                }
            }
        ";
        let root = parse_definition(source).unwrap();
        let tuple = &root.attribute_tuples()[0];
        assert_eq!(
            tuple.tuples()[0].members()[0].constraint(),
            &CPrimitive::String { list: vec![r"/mm\[Hg\]|kPa/".to_owned()] }
        );
    }

    /// `assumed_value` on the outer `C_COMPLEX_OBJECT` — `default_value`
    /// (`_default = <...>`) — is a different, still-unimplemented ODIN
    /// construct; a `C_ATTRIBUTE_TUPLE` is refused by name only when its
    /// own syntax is malformed, never silently skipped.
    #[test]
    fn a_malformed_c_attribute_tuple_is_refused_naming_it() {
        let source = r#"
            DV_QUANTITY[id1] matches {
                [units, magnitude] matches {
                    [{"deg F"}]
                }
            }
        "#;
        let err = parse_definition(source).unwrap_err();
        assert!(err.reason.contains("C_ATTRIBUTE_TUPLE"), "{err}");
    }

    #[test]
    fn trailing_content_after_the_root_is_refused() {
        let err = parse_definition("DV_TEXT[id1] DV_TEXT[id2]").unwrap_err();
        assert!(err.reason.contains("unexpected content"), "{err}");
    }

    /// A real, published archetype's own `definition` bytes
    /// (`openEHR/adl-archetypes`, `openEHR-EHR-CLUSTER.device.v1.0.0.adls`),
    /// not an invented fixture. When this test was written the parser could
    /// not consume it — `allow_archetype`'s `matches { include ... }` form
    /// (`A-66`) and omitted `occurrences` (`A-71`) were each refused by
    /// name — and the test asserted that named refusal, as `K15.6`/`K15.7`
    /// require. Both are implemented now, so it asserts the parse instead:
    /// three alternatives under `items`, the two that state no
    /// `occurrences` carried unstated, and the slot's own assertion carried
    /// as written.
    #[test]
    fn a_real_published_archetypes_definition_is_parsed_whole() {
        let source = r"
            CLUSTER[id1] matches {	-- Medical Device
                items matches {
                    ELEMENT[id2] matches {	-- Device name
                        value matches {
                            DV_TEXT[id29]
                        }
                    }
                    ELEMENT[id4] occurrences matches {0..1} matches {	-- Type
                        value matches {
                            DV_TEXT[id30]
                        }
                    }
                    allow_archetype CLUSTER[id10] matches {	-- Properties
                        include
                            archetype_id/value matches {/openEHR-EHR-CLUSTER\.(dimensions|catheter)[a-zA-Z0-9_]+\.v1/}
                    }
                }
            }
        ";
        let root = parse_definition(source).unwrap();
        let items = &root.attributes()[0];
        assert_eq!(items.children().len(), 3);
        assert_eq!(items.children()[0].node_id(), Some("id2"));
        assert_eq!(items.children()[0].occurrences(), None);
        assert_eq!(items.children()[1].occurrences(), Some(&MultiplicityInterval::OPTIONAL));
        let CObject::Slot(slot) = &items.children()[2] else {
            panic!("expected an ARCHETYPE_SLOT");
        };
        assert_eq!(slot.occurrences(), None);
        assert_eq!(slot.includes().len(), 1);
    }
}
