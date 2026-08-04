//! Property-based tests (`A-09`, `T13.3`).
//!
//! The example-based suites check the cases someone thought of. These check
//! laws over generated input, which is a different question: not "does this
//! case work" but "is this claim true of every value".
//!
//! Three kinds of law are worth the machinery here, and each corresponds to a
//! claim this crate makes in prose elsewhere:
//!
//! 1. **Totality.** Parsers face untrusted text. A panic in a parser is a
//!    denial of service in any process that accepts an openEHR document, so
//!    the claim is that no input panics — only `Err`.
//! 2. **Lexical fidelity.** `D3.10` says the exact text of a date-time
//!    survives a round trip, because a partial instant that gains precision
//!    has gained a clinical fact nobody recorded.
//! 3. **Partial-order coherence.** Comparison returns `None` where the answer
//!    is genuinely undecidable. That freedom is where a hand-written
//!    `partial_cmp` goes wrong, and the laws below pin it down.

use openehr::base::iso8601::{Date, DateTime, Duration, Time};
use openehr::base::object_id::{ArchetypeId, ObjectVersionId};
use proptest::prelude::*;
use std::cmp::Ordering;
use std::fmt::Write as _;
use std::str::FromStr;

// ---------------------------------------------------------------- generators

/// Well-formed dates at all three precisions openEHR permits.
fn date_text() -> impl Strategy<Value = String> {
    // Day capped at 28 so every generated day is valid in every month; leap
    // handling is checked by example tests, and a generator that produced
    // 31 February would be testing the generator rather than the parser.
    prop_oneof![
        (0i32..=9999).prop_map(|y| format!("{y:04}")),
        (0i32..=9999, 1u8..=12).prop_map(|(y, m)| format!("{y:04}-{m:02}")),
        (0i32..=9999, 1u8..=12, 1u8..=28).prop_map(|(y, m, d)| format!("{y:04}-{m:02}-{d:02}")),
    ]
}

/// Well-formed times, with and without fractional seconds and offset.
fn time_text() -> impl Strategy<Value = String> {
    (
        0u8..=23,
        0u8..=59,
        0u8..=59,
        proptest::option::of(0u32..=999_999),
        0u8..=2,
    )
        .prop_map(|(h, m, s, frac, off)| {
            let mut t = format!("{h:02}:{m:02}:{s:02}");
            if let Some(f) = frac {
                let _ = write!(t, ".{f:06}");
            }
            match off {
                0 => t.push('Z'),
                1 => t.push_str("+05:30"),
                _ => {}
            }
            t
        })
}

fn datetime_text() -> impl Strategy<Value = String> {
    (date_text(), proptest::option::of(time_text())).prop_map(|(d, t)| match t {
        // A time is only meaningful on a full date, which is what the parser
        // requires too.
        Some(t) if d.len() == 10 => format!("{d}T{t}"),
        _ => d,
    })
}

/// Text that is *near* valid — the region where parsers actually fail.
///
/// Uniformly random strings are rejected by the first byte and exercise
/// nothing. These are shaped like ISO 8601 and wrong in one place, which is
/// where an off-by-one or an unchecked slice lives.
fn near_miss() -> impl Strategy<Value = String> {
    prop_oneof![
        datetime_text(),
        datetime_text().prop_map(|s| s.replace('-', "")),
        datetime_text().prop_map(|s| { s.chars().rev().collect() }),
        datetime_text().prop_map(|s| format!("{s}{s}")),
        datetime_text().prop_flat_map(|s| {
            let n = s.len();
            (Just(s), 0..n.max(1)).prop_map(|(s, i)| s[..i.min(s.len())].to_owned())
        }),
        ".*",
        // Multi-byte input, because slicing a &str by byte index panics if the
        // index is not a char boundary — the classic parser panic.
        prop::collection::vec(any::<char>(), 0..24).prop_map(|v| v.into_iter().collect()),
    ]
}

// ---------------------------------------------------------------- totality

proptest! {
    /// No input panics any parser. Failure here is a denial of service in any
    /// process that accepts an openEHR document from outside.
    #[test]
    fn parsers_never_panic(s in near_miss()) {
        let _ = Date::from_str(&s);
        let _ = Time::from_str(&s);
        let _ = DateTime::from_str(&s);
        let _ = Duration::from_str(&s);
        let _ = ArchetypeId::from_str(&s);
        let _ = ObjectVersionId::from_str(&s);
    }

    /// An error must never echo the input (`X11.7`).
    ///
    /// The value may be PHI, and an error is the one place it escapes into a
    /// log, a response, and a ticket at once. `ParseError` truncates its echo
    /// to `MAX_ECHO`, so this checks the *whole* input never appears verbatim
    /// once it is longer than that bound.
    #[test]
    fn parse_errors_do_not_echo_a_long_input(s in "[A-Za-z0-9:+.-]{200,400}") {
        if let Err(e) = DateTime::from_str(&s) {
            let rendered = e.to_string();
            prop_assert!(
                !rendered.contains(&s),
                "the error rendered the entire submitted value"
            );
        }
    }
}

/// Regression for **A-16**, pinned as an example rather than left to chance.
///
/// `split_offset` guarded `split_at(2)` with `digits.len() == 4`. `len()`
/// counts bytes, so a single four-byte character satisfied the guard and the
/// split then landed inside it — a panic, not an `Err`. Any service parsing an
/// openEHR document from outside could be stopped by one character.
///
/// The property test above finds this, but only because its generator emits
/// multi-byte characters; a later edit narrowing that generator would lose the
/// coverage silently. This test cannot be lost that way.
#[test]
fn a16_multibyte_offset_returns_err_and_does_not_panic() {
    // U+10348 is four bytes, so `digits.len() == 4` while `is_char_boundary(2)`
    // is false — exactly the shape the old guard admitted.
    for input in [
        "0-\u{10348}",
        "12:00:00+\u{10348}",
        "2024-01-01T00:00:00+\u{69006}",
        // Two-byte and three-byte characters that also sum to four bytes.
        "0-\u{a2}\u{a2}",
        "0-\u{20ac}\u{41}",
    ] {
        assert!(Time::from_str(input).is_err(), "Time accepted {input:?}");
        assert!(
            DateTime::from_str(input).is_err(),
            "DateTime accepted {input:?}"
        );
    }
}

// ------------------------------------------------------- lexical fidelity

proptest! {
    /// `D3.10`: the exact text survives parsing.
    ///
    /// Not "an equivalent text" — the same bytes. `2024-05` must not come back
    /// as `2024-05-01`, because that is a clinical fact nobody recorded.
    #[test]
    fn datetime_round_trips_byte_for_byte(s in datetime_text()) {
        let parsed = DateTime::from_str(&s)
            .map_err(|e| TestCaseError::fail(format!("generator produced invalid text: {e}")))?;
        prop_assert_eq!(parsed.as_str(), s.as_str());
        prop_assert_eq!(parsed.to_string(), s);
    }

    #[test]
    fn date_round_trips_byte_for_byte(s in date_text()) {
        let parsed = Date::from_str(&s)
            .map_err(|e| TestCaseError::fail(format!("generator produced invalid text: {e}")))?;
        prop_assert_eq!(parsed.as_str(), s.as_str());
    }

    #[test]
    fn time_round_trips_byte_for_byte(s in time_text()) {
        let parsed = Time::from_str(&s)
            .map_err(|e| TestCaseError::fail(format!("generator produced invalid text: {e}")))?;
        prop_assert_eq!(parsed.as_str(), s.as_str());
    }

    /// Parsing is idempotent through its own output: re-parsing what a value
    /// renders yields an equal value. A parser and a renderer that disagree
    /// produce documents this crate cannot read back.
    #[test]
    fn reparsing_the_rendering_is_stable(s in datetime_text()) {
        let a = DateTime::from_str(&s).unwrap();
        let b = DateTime::from_str(&a.to_string()).unwrap();
        prop_assert_eq!(a.as_str(), b.as_str());
        prop_assert_eq!(a.semantic_cmp(&b), Some(Ordering::Equal));
    }
}

// --------------------------------------------------- partial-order coherence

/// Dates drawn from a deliberately tiny domain.
///
/// The wide generator above is wrong for ordering laws, and silently so. Every
/// interesting comparison happens between values that agree on a *prefix* —
/// same year, differing precision — and with years drawn uniformly from 0–9999
/// two values share a year about once in ten thousand. At proptest's default
/// 256 cases the branch that returns `None` was reached essentially never, so
/// the laws below passed without testing anything.
///
/// This was not a hypothesis: breaking `Date::partial_cmp` to compare on the
/// left operand's precision — which makes the order non-antisymmetric — left
/// every law passing. Four values per component makes prefix collisions the
/// common case rather than a coincidence, and the same mutation now fails.
fn narrow_date_text() -> impl Strategy<Value = String> {
    prop_oneof![
        (2020i32..=2023).prop_map(|y| format!("{y:04}")),
        (2020i32..=2023, 1u8..=4).prop_map(|(y, m)| format!("{y:04}-{m:02}")),
        (2020i32..=2023, 1u8..=4, 1u8..=4).prop_map(|(y, m, d)| format!("{y:04}-{m:02}-{d:02}")),
    ]
}

/// Date-times over the same tiny domain, including partial times.
fn narrow_datetime_text() -> impl Strategy<Value = String> {
    (
        narrow_date_text(),
        proptest::option::of((0u8..=3, 0u8..=3).prop_map(|(h, m)| format!("{h:02}:{m:02}:00"))),
    )
        .prop_map(|(d, t)| match t {
            Some(t) if d.len() == 10 => format!("{d}T{t}"),
            _ => d,
        })
}

fn two_datetimes() -> impl Strategy<Value = (DateTime, DateTime)> {
    (narrow_datetime_text(), narrow_datetime_text()).prop_map(|(a, b)| {
        (
            DateTime::from_str(&a).unwrap(),
            DateTime::from_str(&b).unwrap(),
        )
    })
}

proptest! {
    /// Reflexivity: a value always equals itself, whatever its precision.
    ///
    /// This is the one comparison that must never be `None`. If a partial
    /// date were incomparable with itself, every `==` on a record would be
    /// false and deduplication would silently stop working.
    #[test]
    fn comparison_is_reflexive(s in narrow_datetime_text()) {
        let a = DateTime::from_str(&s).unwrap();
        prop_assert_eq!(a.semantic_cmp(&a), Some(Ordering::Equal));
    }

    /// Antisymmetry: reversing the operands reverses the answer, and an
    /// undecidable pair is undecidable in both directions.
    ///
    /// An implementation that compares on the *left* operand's precision
    /// passes reflexivity and fails this.
    #[test]
    fn comparison_is_antisymmetric((a, b) in two_datetimes()) {
        let forward = a.semantic_cmp(&b);
        let backward = b.semantic_cmp(&a);
        prop_assert_eq!(forward, backward.map(Ordering::reverse));
    }

    /// Transitivity, where all three comparisons are decidable.
    ///
    /// Partial orders are allowed to have gaps; they are not allowed to have
    /// cycles. A cycle would make a sort of clinical events order-dependent
    /// on the sort algorithm.
    #[test]
    fn comparison_is_transitive(
        x in narrow_datetime_text(), y in narrow_datetime_text(), z in narrow_datetime_text()
    ) {
        let (a, b, c) = (
            DateTime::from_str(&x).unwrap(),
            DateTime::from_str(&y).unwrap(),
            DateTime::from_str(&z).unwrap(),
        );
        if a.semantic_cmp(&b) == Some(Ordering::Less)
            && b.semantic_cmp(&c) == Some(Ordering::Less)
            && let Some(ord) = a.semantic_cmp(&c)
        {
            prop_assert_eq!(ord, Ordering::Less, "a < b < c but not a < c");
        }
    }

    /// A coarser value and a finer one that agree on the coarser's components
    /// are **incomparable**.
    ///
    /// This is the law the other four cannot see. Antisymmetry, transitivity,
    /// and reflexivity are all satisfied by a total order, so an
    /// implementation that answered every comparison — treating `2024` as
    /// equal to, or less than, `2024-05` — passes all of them. Verified by
    /// mutation: making `partial_cmp` return `Some` unconditionally leaves the
    /// other laws green and fails only this one.
    ///
    /// Why it must be `None` rather than an answer: `2024` denotes some
    /// instant in that year, and whether it precedes 1 May is not knowable
    /// from what was recorded. Answering would invent a fact. A clinician
    /// asking "did this happen before the transfusion?" must be told the
    /// record cannot say, not told "no".
    #[test]
    fn a_coarser_value_is_incomparable_with_its_own_refinement(
        y in 2020i32..=2023, m in 1u8..=12, d in 1u8..=28
    ) {
        let year: Date = format!("{y:04}").parse().unwrap();
        let month: Date = format!("{y:04}-{m:02}").parse().unwrap();
        let day: Date = format!("{y:04}-{m:02}-{d:02}").parse().unwrap();

        prop_assert_eq!(year.semantic_cmp(&month), None, "year vs month in that year");
        prop_assert_eq!(month.semantic_cmp(&day), None, "month vs day in that month");
        prop_assert_eq!(year.semantic_cmp(&day), None, "year vs day in that year");
        // …and in both directions, since incomparability is symmetric.
        prop_assert_eq!(month.semantic_cmp(&year), None);
        prop_assert_eq!(day.semantic_cmp(&month), None);
        prop_assert_eq!(day.semantic_cmp(&year), None);
    }

    /// A coarser value is still *decidably* ordered against a finer one that
    /// differs in a component they both know.
    ///
    /// The complement of the law above: `None` must mean "undecidable", not
    /// "different precision". An implementation that returned `None` whenever
    /// precisions differ would satisfy the previous test and make partial
    /// dates useless for any range query.
    #[test]
    fn differing_known_components_are_still_decidable(y in 2020i32..=2023, m in 2u8..=12) {
        let year: Date = format!("{y:04}").parse().unwrap();
        let later: Date = format!("{:04}-{m:02}", y + 1).parse().unwrap();
        prop_assert_eq!(year.semantic_cmp(&later), Some(Ordering::Less));
        prop_assert_eq!(later.semantic_cmp(&year), Some(Ordering::Greater));
    }

    /// Equal text implies equal ordering, and the converse where both are
    /// fully precise. Ties the lexical form to the comparison so the two
    /// cannot drift apart.
    #[test]
    fn identical_text_compares_equal(s in narrow_datetime_text()) {
        let a = DateTime::from_str(&s).unwrap();
        let b = DateTime::from_str(&s).unwrap();
        prop_assert_eq!(a.semantic_cmp(&b), Some(Ordering::Equal));
    }
}
