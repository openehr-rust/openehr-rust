//! `Real`: a number that remembers how it was written.
//!
//! # Why a type at all
//!
//! `1.50 mg` and `1.5 mg` are different measurements. The first was measured to
//! two decimal places; the second to one. openEHR carries that distinction in
//! `DV_QUANTITY.precision` as a *declaration*, and in the magnitude's own text
//! as a *fact* — and an `f64` keeps neither: both parse to the same bits, and
//! by the time storage sees the value the difference is gone.
//!
//! `db:D-08` is this failure one layer out. MySQL rewrote a stored magnitude of
//! `1.10` as `1.1`, which changed the bytes a content digest had been taken
//! over, and `db:M3.43` moved canonical JSON onto a byte-preserving column
//! because of it. That fixed storage while the crate was still losing the same
//! digits at parse time (`D3.18d`).
//!
//! # The shape is `iso8601`'s
//!
//! Text authoritative, derived value alongside (`D3.18e`) — the same split as
//! `Date`, `Time` and `DateTime`, and the same as the `…_text`/`…_utc` column
//! pair in the schema (`db:M3.31`). It produces the same consequence:
//!
//! * **Equality is lexical.** `1.50 != 1.5`, because they are different records
//!   and a digest is taken over the text.
//! * **Ordering is numeric.** A reference range asks which is larger, and
//!   `1.50` is not larger than `1.5`.
//!
//! Those disagree, so `Real` does **not** implement `PartialOrd` — Rust
//! requires `a == b` exactly where `partial_cmp` says `Equal`, and this is
//! `D3.18b` again. [`Real::semantic_cmp`] carries the ordering.

use core::cmp::Ordering;
use core::fmt;
use serde::{Deserialize, Serialize};

/// A real number, holding the text it was written as and the value it denotes.
///
/// ```
/// use openehr::base::Real;
///
/// let coarse: Real = "1.5".parse().unwrap();
/// let fine: Real = "1.50".parse().unwrap();
///
/// // Different records…
/// assert_ne!(coarse, fine);
/// assert_eq!(fine.as_str(), "1.50");
/// // …denoting the same number.
/// assert_eq!(coarse.semantic_cmp(&fine), Some(core::cmp::Ordering::Equal));
/// assert_eq!(fine.as_f64(), 1.5);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "serde_json::Number", into = "serde_json::Number")]
pub struct Real {
    /// The number exactly as written.
    text: String,
    /// What it denotes. Derived, and never authoritative.
    value: f64,
}

impl Real {
    /// Builds a real from a value the program computed.
    ///
    /// The text is the shortest form that round-trips, which is what
    /// `Display for f64` produces — a program that computed `1.5` did not
    /// measure `1.50`, and inventing digits would be a claim about precision
    /// nobody made.
    #[must_use]
    pub fn from_f64(value: f64) -> Self {
        Self {
            text: format!("{value}"),
            value,
        }
    }

    /// The number as written. Authoritative (`D3.18e`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// What the number denotes. Derived.
    #[must_use]
    pub fn as_f64(&self) -> f64 {
        self.value
    }

    /// Whether the value is finite — neither infinite nor `NaN`.
    ///
    /// openEHR is written against mathematical reals and does not anticipate
    /// either. A `NaN` that reaches a clinical comparison is false against
    /// every bound including itself (`D3.19`, `D3.21`).
    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.value.is_finite()
    }

    /// Compares two reals by what they denote, not by how they were written.
    ///
    /// Not `PartialOrd::partial_cmp`, and deliberately (`D3.18e`): `1.50` and
    /// `1.5` order `Equal` and are **not** equal, which Rust's contract forbids
    /// a type to say through both traits at once.
    #[must_use]
    pub fn semantic_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

/// Lexical, because the text is the record (`D3.18e`).
impl PartialEq for Real {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl fmt::Display for Real {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl From<f64> for Real {
    fn from(value: f64) -> Self {
        Self::from_f64(value)
    }
}

/// The JSON bridge.
///
/// `serde_json::Number` under the `arbitrary_precision` feature holds the
/// literal text of the number it parsed, which is the whole reason this type
/// can exist. Without that feature a `Number` is an `f64` and `1.50` is gone
/// before this code runs — see
/// `spec/serde-json-float-roundtrip-arbitrary-precision/` `SJ2`.
impl From<serde_json::Number> for Real {
    fn from(n: serde_json::Number) -> Self {
        // `as_f64` is `None` only for a value outside `f64`'s range, which JSON
        // permits to be written and this crate refuses on validation rather
        // than silently. Infinity is the honest reading of "too large", and
        // `is_finite` is what reports it (`D3.19`).
        let value = n.as_f64().unwrap_or(f64::INFINITY);
        Self {
            text: n.to_string(),
            value,
        }
    }
}

impl From<Real> for serde_json::Number {
    fn from(r: Real) -> Self {
        // Reconstructed from the text, so the bytes out are the bytes in.
        // Parsing cannot fail: `text` came from a `Number` or from `{f64}`.
        serde_json::from_str(&r.text).unwrap_or_else(|_| {
            serde_json::Number::from_f64(r.value)
                .unwrap_or_else(|| serde_json::Number::from(0))
        })
    }
}

impl core::str::FromStr for Real {
    type Err = crate::error::ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`](crate::error::ParseError) if the text is not a
    /// JSON number.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let n: serde_json::Number = serde_json::from_str(s)
            .map_err(|_| crate::error::ParseError::new("REAL", "not a number", s))?;
        Ok(Self::from(n))
    }
}

impl crate::base::SemanticOrd for Real {
    fn semantic_cmp(&self, other: &Self) -> Option<Ordering> {
        Self::semantic_cmp(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The distinction this type exists for.
    ///
    /// **Failure mode.** `DV_QUANTITY.magnitude` was an `f64`, so `1.50` and
    /// `1.5` were the same value at parse time and no storage rule could
    /// recover the difference. `security::canonical` documented that as the
    /// limit of the guarantee; `db:D-08` is the same loss one layer out.
    #[test]
    fn a_measured_precision_survives_a_round_trip() {
        let fine: Real = "1.50".parse().unwrap();
        assert_eq!(fine.as_str(), "1.50");
        // Exact by construction: `1.50` and `1.5` parse to the same bits.
        #[allow(clippy::float_cmp, reason = "the whole point is that these are the same f64")]
        {
            assert_eq!(fine.as_f64(), 1.5);
        }

        let json = serde_json::to_string(&fine).unwrap();
        assert_eq!(json, "1.50", "the digits were normalised away");
        let back: Real = serde_json::from_str(&json).unwrap();
        assert_eq!(back, fine);
        assert_eq!(back.as_str(), "1.50");
    }

    /// Equality is lexical and ordering is numeric, and they disagree.
    #[test]
    fn equality_and_order_answer_different_questions() {
        let coarse: Real = "1.5".parse().unwrap();
        let fine: Real = "1.50".parse().unwrap();
        assert_ne!(coarse, fine, "different records");
        assert_eq!(
            coarse.semantic_cmp(&fine),
            Some(Ordering::Equal),
            "the same number"
        );

        let bigger: Real = "2".parse().unwrap();
        assert_eq!(coarse.semantic_cmp(&bigger), Some(Ordering::Less));
        assert_eq!(bigger.semantic_cmp(&coarse), Some(Ordering::Greater));
    }

    /// A computed value is written as the shortest form that round-trips.
    #[test]
    fn a_computed_value_does_not_invent_digits() {
        assert_eq!(Real::from_f64(1.5).as_str(), "1.5");
        assert_eq!(Real::from_f64(184.0).as_str(), "184");
        assert_eq!(Real::from_f64(-0.25).as_str(), "-0.25");
    }

    /// Reading is total on a value that passed no constructor (`D3.18f`).
    #[test]
    fn a_real_that_never_saw_a_constructor_is_still_readable() {
        let r: Real = serde_json::from_str("1e400").expect("JSON permits it");
        assert!(!r.is_finite(), "out of range reads as infinite, not a panic");
        assert_eq!(r.as_str(), "1e+400", "the digits are still what arrived");
    }

    /// Exactly what survives, and the one thing that does not.
    ///
    /// Measured rather than assumed, because `D3.18d` says "preserved exactly"
    /// and that is very nearly true. Every digit survives — trailing zeros,
    /// and significant digits past what an `f64` can hold. **Exponent notation
    /// is normalised**: `1e5` and `1E5` both become `1e+5`. The value is
    /// unchanged and no digit is lost; what changes is the case of the `e` and
    /// the explicit `+`.
    ///
    /// That limit is stated rather than papered over. It is also not reachable
    /// by a clinical magnitude in practice — nobody charts a blood pressure in
    /// scientific notation — but "in practice" is not a guarantee, and this is.
    #[test]
    fn every_digit_survives_and_only_the_exponent_form_is_normalised() {
        for text in [
            "1.50",   // the trailing zero this type exists for
            "0.10",
            "184.0",
            "0.000",
            "-0.0",
            "184",
            "1e-5",
            "1e+5",
            "12345678901234567890",     // more digits than an f64 holds
            "3.14159265358979323846",   // and more than it can distinguish
        ] {
            let r: Real = text.parse().expect(text);
            assert_eq!(r.as_str(), text, "{text} was normalised");
        }

        // The exception, asserted so that a change to it is a test failure
        // rather than a surprise.
        for (written, stored) in [("1e5", "1e+5"), ("1E5", "1e+5")] {
            let r: Real = written.parse().expect(written);
            assert_eq!(r.as_str(), stored);
            #[allow(clippy::float_cmp, reason = "an exponent form denotes an exact integer")]
            {
                assert_eq!(r.as_f64(), 100_000.0, "the value is untouched");
            }
        }
    }

    /// `is_finite` answers **both** ways, and `Display` writes the number.
    ///
    /// **Failure mode, and it was live.** Mutation testing replaced
    /// `is_finite` with `false` and nothing failed: the only assertion about it
    /// was `assert!(!r.is_finite())` for `1e400`, which `false` satisfies. A
    /// test that agrees with the mutant is not a test, and this is the third
    /// time that exact shape has appeared here — `DvUri::rest` asserted `""`
    /// where `""` was the right answer, and `check_projection` asserted only
    /// its `true` case (`W-18`, `db:D-10`).
    ///
    /// `Display` was replaceable with `Ok(())` — writing nothing — for the same
    /// reason as `DvUri`'s: nothing asserted what a `Real` renders as. It is
    /// what an error message embeds and what a `DV_QUANTITY`'s own `Display`
    /// is built from, so a `Real` that rendered as nothing would silently empty
    /// a charted value.
    #[test]
    fn a_real_reports_finiteness_both_ways_and_displays_its_digits() {
        let ordinary: Real = "1.50".parse().unwrap();
        assert!(ordinary.is_finite(), "a plain number is finite");
        assert_eq!(ordinary.to_string(), "1.50", "Display must write the digits");

        let huge: Real = "1e400".parse().unwrap();
        assert!(!huge.is_finite(), "out of `f64` range is not finite");
        assert_eq!(huge.to_string(), "1e+400");
    }

    /// The `SemanticOrd` impl is reached, and an interval over reals works.
    ///
    /// **Failure mode.** `<Real as SemanticOrd>::semantic_cmp` was replaceable
    /// with `None` — every test called the *inherent* method instead, so the
    /// trait impl had no caller. `None` means "not comparable", and
    /// `Interval::contains` reads that as not contained (`D3.14a`), so an
    /// `INTERVAL<Real>` would have contained nothing at all and reported it as
    /// an ordinary negative answer.
    #[test]
    fn an_interval_over_reals_contains_what_it_should() {
        use crate::base::{Interval, SemanticOrd};

        let low: Real = "1.0".parse().unwrap();
        let high: Real = "10.00".parse().unwrap();
        let range = Interval::closed(low.clone(), high.clone()).unwrap();

        assert!(range.contains(&"5".parse::<Real>().unwrap()));
        assert!(range.contains(&low), "a closed bound is included");
        assert!(range.contains(&high));
        // Written differently, denoting a bound: still inside.
        assert!(range.contains(&"10.0".parse::<Real>().unwrap()));
        assert!(!range.contains(&"10.5".parse::<Real>().unwrap()));

        // And through the trait by name, so the impl itself has a caller.
        assert_eq!(
            SemanticOrd::semantic_cmp(&low, &high),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn text_that_is_not_a_number_is_refused() {
        assert!("".parse::<Real>().is_err());
        assert!("1.2.3".parse::<Real>().is_err());
        assert!("NaN".parse::<Real>().is_err());
    }
}
