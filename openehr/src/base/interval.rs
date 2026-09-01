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
use core::cmp::Ordering;
use serde::{Deserialize, Serialize};

/// A partial order that is **not** required to agree with `PartialEq`.
///
/// `INTERVAL<T>` is bounded on this rather than on `PartialOrd` (`D3.18c`),
/// and the reason is `D3.18a`/`D3.18b`: for an openEHR value, equality and
/// order answer different questions and cannot both be trait impls.
///
/// Equality is **record identity**. A `DV_QUANTITY` that records
/// `precision: 1` is not the same stored value as one that records `2`, a
/// `DV_DATE_TIME` written `12:00:00+01:00` is not the one written `11:00:00Z`
/// (`D3.10`), and a content digest is taken over exactly those bytes
/// (`db:M3.43`). Order is what a reference range and a query need, and it
/// compares only the magnitude — so the two disagree, by design, on values
/// that denote the same point.
///
/// Rust requires `a == b` if and only if `partial_cmp` reports `Some(Equal)`.
/// Implementing both would break that contract in every one of those cases:
/// `a != b` while `a <= b` and `a >= b` are both true, which is invisible here
/// — every comparison in this crate goes through the ordering consistently —
/// and surfaces in a caller's `binary_search`, `dedup_by`, or `sort_by`. That
/// was the finding `A-35`.
///
/// # Why there is no blanket impl
///
/// `impl<T: PartialOrd> SemanticOrd for T` would collide with the explicit
/// impls under Rust's coherence rules. It is also the wrong thing to want: the
/// explicit list is what stops a type with the `D3.18b` defect from reaching
/// `INTERVAL<T>` again without anyone deciding that it should.
pub trait SemanticOrd {
    /// Compares two values, or reports that they are not comparable.
    ///
    /// `None` is a real answer and never a default: a `DV_QUANTITY` in `mg`
    /// and one in `ml` are not "not less than" each other (`D3.14`).
    fn semantic_cmp(&self, other: &Self) -> Option<Ordering>;
}

/// The primitives an interval is used over here, plus the ones a caller
/// reasonably would. For these, equality and order already agree, so
/// delegating to `PartialOrd` is exactly right.
macro_rules! semantic_ord_via_partial_ord {
    ($($ty:ty),+ $(,)?) => {$(
        impl SemanticOrd for $ty {
            fn semantic_cmp(&self, other: &Self) -> Option<Ordering> {
                self.partial_cmp(other)
            }
        }
    )+};
}

semantic_ord_via_partial_ord!(i8, i16, i32, i64, i128, isize);
semantic_ord_via_partial_ord!(u8, u16, u32, u64, u128, usize);
semantic_ord_via_partial_ord!(f32, f64);
semantic_ord_via_partial_ord!(char, &str, String);

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
    bound = "T: Clone + SemanticOrd + Serialize + serde::de::DeserializeOwned"
)]
pub struct Interval<T> {
    lower: Option<T>,
    upper: Option<T>,
    lower_included: Option<bool>,
    upper_included: Option<bool>,
}

impl<T: SemanticOrd> Interval<T> {
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
                lo.semantic_cmp(hi),
                Some(Ordering::Less | Ordering::Equal)
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
        // Written against `semantic_cmp` rather than `>=`/`<=` because `T` is
        // not `PartialOrd` (`D3.18c`) — and because the operators quietly read
        // "not comparable" as "not greater, therefore below", which for an
        // openEHR value is a wrong answer rather than a missing one.
        let above_lower = match &self.lower {
            None => true,
            Some(lo) => match value.semantic_cmp(lo) {
                Some(Ordering::Greater) => true,
                Some(Ordering::Equal) => self.lower_included.unwrap_or(true),
                Some(Ordering::Less) | None => false,
            },
        };
        let below_upper = match &self.upper {
            None => true,
            Some(hi) => match value.semantic_cmp(hi) {
                Some(Ordering::Less) => true,
                Some(Ordering::Equal) => self.upper_included.unwrap_or(true),
                Some(Ordering::Greater) | None => false,
            },
        };
        above_lower && below_upper
    }

    /// Whether every point of `other` is also a point of `self`.
    ///
    /// The BASE foundation type declares this as `INTERVAL.contains(other:
    /// INTERVAL)`, distinct from `has(e: T)` — the element test this crate
    /// calls [`Interval::contains`], for a name already public before this
    /// method existed. `has`/`element_contains` was not renamed to make room;
    /// this is `contains_interval` instead, named for what it takes rather
    /// than to match a name this crate cannot also use for something else.
    ///
    /// Boundary alignment matters and is checked exactly, not approximated by
    /// testing `other`'s raw bound values against `self.contains`: `self =
    /// (0, 10)`, `other = (0, 5)` share the excluded point `0`, and neither
    /// interval contains it — testing `self.contains(0)` alone would answer
    /// `false` and wrongly conclude `other` is not contained, when every
    /// actual point of `other` (everything strictly between `0` and `5`) is a
    /// point of `self`.
    #[must_use]
    pub fn contains_interval(&self, other: &Self) -> bool {
        let lower_ok = match (&self.lower, &other.lower) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(sl), Some(ol)) => match sl.semantic_cmp(ol) {
                Some(Ordering::Less) => true,
                Some(Ordering::Equal) => {
                    self.lower_included.unwrap_or(true) || !other.lower_included.unwrap_or(true)
                }
                Some(Ordering::Greater) | None => false,
            },
        };
        let upper_ok = match (&self.upper, &other.upper) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(su), Some(ou)) => match su.semantic_cmp(ou) {
                Some(Ordering::Greater) => true,
                Some(Ordering::Equal) => {
                    self.upper_included.unwrap_or(true) || !other.upper_included.unwrap_or(true)
                }
                Some(Ordering::Less) | None => false,
            },
        };
        lower_ok && upper_ok
    }

    /// Whether at least one limit of `other` falls **strictly** inside
    /// `self` — the BASE foundation type's `INTERVAL.intersects(other:
    /// INTERVAL)`.
    ///
    /// "Strictly inside" means beyond both of `self`'s own limits, regardless
    /// of whether `self` includes them: two intervals that only touch at a
    /// shared boundary point, open on at least one side so neither actually
    /// contains that point (`self = (0, 10)`, `other = (10, 20)`), do not
    /// intersect — there is no point either interval actually contains that
    /// the other also contains.
    #[must_use]
    pub fn intersects(&self, other: &Self) -> bool {
        let strictly_within = |value: &T| {
            let above = match &self.lower {
                None => true,
                Some(lo) => matches!(value.semantic_cmp(lo), Some(Ordering::Greater)),
            };
            let below = match &self.upper {
                None => true,
                Some(hi) => matches!(value.semantic_cmp(hi), Some(Ordering::Less)),
            };
            above && below
        };
        other.lower.as_ref().is_some_and(strictly_within)
            || other.upper.as_ref().is_some_and(strictly_within)
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

    /// `contains` uses the strict comparison on an open bound, not just at the
    /// boundary.
    ///
    /// `open_and_closed_ends_differ_at_the_boundary` above only checks the
    /// value *equal to* the excluded bound, which cannot tell `value < hi` from
    /// `value > hi`: both exclude 100 from `open(60, 100)`. A value strictly
    /// between the bounds is what tells the two comparisons apart, and it was
    /// never checked (`lib:A-09`). This is the membership test
    /// `ReferenceRange::contains` delegates to, so a flipped comparison here
    /// silently inverts which results read as abnormal.
    #[test]
    fn contains_uses_the_right_comparison_on_both_sides_of_an_open_bound() {
        let open = Interval::open(60_i32, 100).unwrap();
        assert!(open.contains(&61));
        assert!(open.contains(&99));
        assert!(!open.contains(&60), "the excluded lower bound was included");
        assert!(!open.contains(&100), "the excluded upper bound was included");
        assert!(!open.contains(&30), "below the range was reported inside it");
        assert!(!open.contains(&130), "above the range was reported inside it");

        let closed = Interval::closed(60_i32, 100).unwrap();
        assert!(closed.contains(&60));
        assert!(closed.contains(&100));
        assert!(closed.contains(&80));
        assert!(!closed.contains(&59));
        assert!(!closed.contains(&101));

        // Unbounded on one side: only the bounded side constrains.
        let at_least = Interval::at_least(10_i32).unwrap();
        assert!(at_least.contains(&10));
        assert!(at_least.contains(&1_000_000));
        assert!(!at_least.contains(&9));

        let at_most = Interval::at_most(10_i32).unwrap();
        assert!(at_most.contains(&10));
        assert!(at_most.contains(&-1_000_000));
        assert!(!at_most.contains(&11));
    }

    /// `contains_interval` on the exact case a naive "test both raw bound
    /// values against `self.contains`" approach gets wrong: `self = (0, 10)`
    /// and `other = (0, 5)` share the excluded bound `0`. Neither interval
    /// contains the point `0`, so `self.contains(0)` alone answers `false`
    /// and a check built on it would wrongly conclude `other` is not
    /// contained — when every point `other` actually has (everything
    /// strictly between `0` and `5`) is also a point of `self`.
    #[test]
    fn contains_interval_agrees_at_a_shared_excluded_boundary() {
        let outer = Interval::open(0_i32, 10).unwrap();
        let inner = Interval::open(0_i32, 5).unwrap();
        assert!(
            outer.contains_interval(&inner),
            "every point of (0, 5) is a point of (0, 10)"
        );
        // And the other direction is false: (0, 10) has points (0, 5) does not.
        assert!(!inner.contains_interval(&outer));
    }

    /// `contains_interval` where the two intervals' bounds coincide but one
    /// is open where the other is closed at that exact point.
    #[test]
    fn contains_interval_distinguishes_open_from_closed_at_a_shared_bound() {
        let closed = Interval::closed(0_i32, 10).unwrap();
        let open = Interval::open(0_i32, 10).unwrap();

        // The closed interval's every point includes the open one's every
        // point, plus the two endpoints the open interval excludes.
        assert!(closed.contains_interval(&open));
        // The open interval is missing 0 and 10, both of which the closed
        // interval has, so the reverse does not hold.
        assert!(!open.contains_interval(&closed));

        // Equal intervals contain each other.
        assert!(closed.contains_interval(&closed));
        assert!(open.contains_interval(&open));
    }

    /// `contains_interval` where one side is unbounded. `INTERVAL` here
    /// cannot have *both* bounds absent (`Interval::new` refuses it — a
    /// meaningless interval, not a representable "everything"), so this
    /// covers the one-sided-unbounded cases that are constructible.
    #[test]
    fn contains_interval_handles_unbounded_sides() {
        let at_least_0 = Interval::at_least(0_i32).unwrap();
        let at_least_5 = Interval::at_least(5_i32).unwrap();
        assert!(
            at_least_0.contains_interval(&at_least_5),
            "[0, ∞) contains [5, ∞)"
        );
        assert!(
            !at_least_5.contains_interval(&at_least_0),
            "[5, ∞) does not contain [0, ∞)"
        );
    }

    /// `intersects`: "at least one limit of `other` falls strictly inside
    /// `self`". Two intervals that only touch at a shared boundary, open on
    /// at least one side so neither contains that point, do not intersect —
    /// there is no point either actually holds that the other also holds.
    #[test]
    fn intersects_requires_a_limit_strictly_inside_not_merely_touching() {
        let left = Interval::open(0_i32, 10).unwrap();
        let right = Interval::open(10_i32, 20).unwrap();
        assert!(
            !left.intersects(&right),
            "touching at an excluded 10 is not an intersection"
        );
        assert!(!right.intersects(&left), "symmetric");

        let overlapping = Interval::open(5_i32, 15).unwrap();
        assert!(
            left.intersects(&overlapping),
            "5 is strictly inside (0, 10)"
        );
        assert!(
            overlapping.intersects(&left),
            "the check is symmetric in effect, even though each call tests \
             the other side's limits against its own"
        );

        let disjoint = Interval::open(20_i32, 30).unwrap();
        assert!(!left.intersects(&disjoint));
    }

    /// The `*_unbounded` and `*_included` accessors report what the interval
    /// actually holds.
    ///
    /// `lower_unbounded`/`upper_unbounded` could each be a constant, and
    /// `lower_included`/`upper_included` could too — five mutants
    /// (`lib:A-09`). These are the attributes `Q12.7a` requires a path to
    /// reach, and `Q12.7b` requires them navigable even though they are
    /// derived: a reader asking whether a range is open at one end must get a
    /// real answer.
    #[test]
    fn unbounded_and_included_are_reported_for_each_shape() {
        let closed = Interval::closed(60_i32, 100).unwrap();
        assert!(!closed.lower_unbounded());
        assert!(!closed.upper_unbounded());
        assert_eq!(closed.lower_included(), Some(true));
        assert_eq!(closed.upper_included(), Some(true));

        let open = Interval::open(60_i32, 100).unwrap();
        assert_eq!(open.lower_included(), Some(false));
        assert_eq!(open.upper_included(), Some(false));

        let at_least = Interval::at_least(10_i32).unwrap();
        assert!(!at_least.lower_unbounded());
        assert!(at_least.upper_unbounded(), "an open-ended range was reported bounded");
        assert_eq!(at_least.upper(), None);

        let at_most = Interval::at_most(10_i32).unwrap();
        assert!(at_most.lower_unbounded(), "an open-started range was reported bounded");
        assert!(!at_most.upper_unbounded());
        assert_eq!(at_most.lower(), None);
    }
}
