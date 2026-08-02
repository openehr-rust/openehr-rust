//! `INTERVAL<T>`: a range with independently open or closed ends.
//!
//! # Why `lower_unbounded` is derived and not stored
//!
//! openEHR models an interval with six attributes — two bounds, two
//! "unbounded" flags, two "included" flags — and states invariants tying the
//! flags to the bounds: `lower_unbounded = (lower = Void)`. Storing the flags
//! makes those invariants *checkable*, which means it also makes them
//! *violable*: `{lower: 5, lower_unbounded: true}` is representable and
//! meaningless.
//!
//! This type stores the bounds and derives the flags, so the invariant holds by
//! construction and cannot be broken by a caller, a deserializer, or a future
//! edit. The flags are still written on serialization, because a reader
//! expecting openEHR canonical JSON expects to find them.
//!
//! The `included` flags are *not* derivable, so they are stored — and they are
//! `Option`, because openEHR says inclusion is undefined for an unbounded end.

use crate::error::ParseError;
use serde::{Deserialize, Serialize};

/// A range over any ordered type.
///
/// ```
/// use openehr::base::Interval;
///
/// let normal = Interval::closed(60, 100).unwrap();
/// assert!(normal.contains(&72));
/// assert!(normal.contains(&60));
/// assert!(!normal.contains(&101));
///
/// let over = Interval::greater_than(100).unwrap();
/// assert!(over.contains(&101));
/// assert!(!over.contains(&100));
/// assert!(over.upper_unbounded());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    from = "IntervalWire<T>",
    into = "IntervalWire<T>",
    bound = "T: Clone + PartialOrd + Serialize + serde::de::DeserializeOwned"
)]
pub struct Interval<T> {
    lower: Option<T>,
    upper: Option<T>,
    lower_included: Option<bool>,
    upper_included: Option<bool>,
}

impl<T: PartialOrd> Interval<T> {
    /// Builds an interval from its parts.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if both bounds are absent (an interval bounded at
    /// neither end constrains nothing and is almost always a construction bug),
    /// if `lower > upper`, or if an `included` flag is given for an absent
    /// bound.
    pub fn new(
        lower: Option<T>,
        upper: Option<T>,
        lower_included: Option<bool>,
        upper_included: Option<bool>,
    ) -> Result<Self, ParseError> {
        if lower.is_none() && upper.is_none() {
            return Err(ParseError::new("INTERVAL", "both bounds are absent", ""));
        }
        if lower.is_none() && lower_included.is_some() {
            return Err(ParseError::new(
                "INTERVAL",
                "lower_included given for an unbounded lower end",
                "",
            ));
        }
        if upper.is_none() && upper_included.is_some() {
            return Err(ParseError::new(
                "INTERVAL",
                "upper_included given for an unbounded upper end",
                "",
            ));
        }
        if let (Some(lo), Some(hi)) = (&lower, &upper) {
            // Matching on `partial_cmp` rather than writing `lo > hi`: `T` is
            // only partially ordered, and two openEHR values genuinely can be
            // incomparable — a `DV_DATE_TIME` of `2024-05` against one of
            // `2024-05-17`. `lo > hi` reads that as "not greater, therefore
            // fine" and builds an interval whose ends have no established
            // order.
            if !matches!(
                lo.partial_cmp(hi),
                Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal)
            ) {
                // `DV_INTERVAL.Limits_consistent`, reported by openEHR's own
                // name. It said `INTERVAL` with prose until `lib:A-24`, which
                // meant the rule was enforced and a reader could not find it in
                // any class definition — the defect `L10.4` exists to prevent,
                // and the one `A-20` fixed fifteen times. It survived because
                // that audit greps for names the crate *uses*, and this rule
                // was the one that used none.
                return Err(ParseError::invariant("DV_INTERVAL", "Limits_consistent"));
            }
        }
        Ok(Self {
            lower,
            upper,
            lower_included,
            upper_included,
        })
    }

    /// `[lower, upper]` — both ends included.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `lower > upper`.
    pub fn closed(lower: T, upper: T) -> Result<Self, ParseError> {
        Self::new(Some(lower), Some(upper), Some(true), Some(true))
    }

    /// `(lower, upper)` — both ends excluded.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `lower > upper`.
    pub fn open(lower: T, upper: T) -> Result<Self, ParseError> {
        Self::new(Some(lower), Some(upper), Some(false), Some(false))
    }

    /// `[lower, ∞)`.
    ///
    /// # Errors
    ///
    /// Never in practice; the signature is fallible so that all constructors
    /// compose the same way.
    pub fn at_least(lower: T) -> Result<Self, ParseError> {
        Self::new(Some(lower), None, Some(true), None)
    }

    /// `(lower, ∞)`.
    ///
    /// # Errors
    ///
    /// Never in practice; see [`Interval::at_least`].
    pub fn greater_than(lower: T) -> Result<Self, ParseError> {
        Self::new(Some(lower), None, Some(false), None)
    }

    /// `(-∞, upper]`.
    ///
    /// # Errors
    ///
    /// Never in practice; see [`Interval::at_least`].
    pub fn at_most(upper: T) -> Result<Self, ParseError> {
        Self::new(None, Some(upper), None, Some(true))
    }

    /// `(-∞, upper)`.
    ///
    /// # Errors
    ///
    /// Never in practice; see [`Interval::at_least`].
    pub fn less_than(upper: T) -> Result<Self, ParseError> {
        Self::new(None, Some(upper), None, Some(false))
    }

    /// A degenerate interval containing exactly one value.
    ///
    /// # Errors
    ///
    /// Never in practice; see [`Interval::at_least`].
    pub fn point(value: T) -> Result<Self, ParseError>
    where
        T: Clone,
    {
        Self::closed(value.clone(), value)
    }

    /// Whether a value falls inside the interval.
    ///
    /// An absent `included` flag on a present bound is read as *included*,
    /// matching openEHR's default and the reading a clinician expects of "the
    /// normal range is 60 to 100".
    #[must_use]
    pub fn contains(&self, value: &T) -> bool {
        let above_lower = match &self.lower {
            None => true,
            Some(lo) if self.lower_included.unwrap_or(true) => value >= lo,
            Some(lo) => value > lo,
        };
        let below_upper = match &self.upper {
            None => true,
            Some(hi) if self.upper_included.unwrap_or(true) => value <= hi,
            Some(hi) => value < hi,
        };
        above_lower && below_upper
    }
}

impl<T> Interval<T> {
    /// The lower bound, if any.
    #[must_use]
    pub fn lower(&self) -> Option<&T> {
        self.lower.as_ref()
    }

    /// The upper bound, if any.
    #[must_use]
    pub fn upper(&self) -> Option<&T> {
        self.upper.as_ref()
    }

    /// Whether the lower end is open to infinity. Derived, never stored.
    #[must_use]
    pub fn lower_unbounded(&self) -> bool {
        self.lower.is_none()
    }

    /// Whether the upper end is open to infinity. Derived, never stored.
    #[must_use]
    pub fn upper_unbounded(&self) -> bool {
        self.upper.is_none()
    }

    /// Whether the lower bound is part of the interval.
    #[must_use]
    pub fn lower_included(&self) -> Option<bool> {
        self.lower_included
    }

    /// Whether the upper bound is part of the interval.
    #[must_use]
    pub fn upper_included(&self) -> Option<bool> {
        self.upper_included
    }
}

/// The six-attribute JSON shape openEHR specifies.
///
/// Deserialization ignores the incoming `*_unbounded` flags entirely rather than
/// checking them against the bounds. A sender that writes
/// `{"lower": 5, "lower_unbounded": true}` has contradicted itself, and the
/// bound is the attribute carrying information — the flag is redundant by
/// openEHR's own invariant. Rejecting the payload would lose a usable interval
/// over a derived field.
#[derive(Serialize, Deserialize)]
struct IntervalWire<T> {
    #[serde(skip_serializing_if = "Option::is_none", default = "none")]
    lower: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none", default = "none")]
    upper: Option<T>,
    lower_unbounded: bool,
    upper_unbounded: bool,
    #[serde(skip_serializing_if = "Option::is_none", default = "none")]
    lower_included: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default = "none")]
    upper_included: Option<bool>,
}

fn none<T>() -> Option<T> {
    None
}

impl<T> From<Interval<T>> for IntervalWire<T> {
    fn from(v: Interval<T>) -> Self {
        Self {
            lower_unbounded: v.lower.is_none(),
            upper_unbounded: v.upper.is_none(),
            lower: v.lower,
            upper: v.upper,
            lower_included: v.lower_included,
            upper_included: v.upper_included,
        }
    }
}

impl<T> From<IntervalWire<T>> for Interval<T> {
    fn from(v: IntervalWire<T>) -> Self {
        Self {
            lower: v.lower,
            upper: v.upper,
            lower_included: v.lower_included,
            upper_included: v.upper_included,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unbounded_flags_are_derived_from_the_bounds() {
        let i = Interval::at_least(1_i32).unwrap();
        assert!(!i.lower_unbounded());
        assert!(i.upper_unbounded());
    }

    #[test]
    fn a_self_contradictory_payload_trusts_the_bound() {
        let json = r#"{"lower":5,"lower_unbounded":true,"upper_unbounded":true}"#;
        let i: Interval<i32> = serde_json::from_str(json).unwrap();
        assert_eq!(i.lower(), Some(&5));
        assert!(!i.lower_unbounded());
        // ...and serialization emits the flag that agrees with the bound.
        let out = serde_json::to_string(&i).unwrap();
        assert!(out.contains(r#""lower_unbounded":false"#), "{out}");
    }

    #[test]
    fn inverted_bounds_are_refused() {
        assert!(Interval::closed(100_i32, 60).is_err());
    }

    #[test]
    fn open_and_closed_ends_differ_at_the_boundary() {
        assert!(Interval::closed(60_i32, 100).unwrap().contains(&100));
        assert!(!Interval::open(60_i32, 100).unwrap().contains(&100));
    }
}
