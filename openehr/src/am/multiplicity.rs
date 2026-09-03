//! `MULTIPLICITY_INTERVAL`: how many times something may appear.
//!
//! AOM2 uses one interval type for three different questions — how many times
//! an object may occur under its parent (`occurrences`), whether an attribute
//! is present at all (`existence`), and how many members a container attribute
//! may hold (`cardinality`) — and they are the same shape: a lower bound, and
//! an upper bound that may be open.
//!
//! # Why this is not [`crate::base::Interval`]
//!
//! `INTERVAL<T>` carries independently open or closed ends, because a reference
//! range legitimately excludes its bound. A multiplicity cannot: "more than 1
//! and fewer than 3 occurrences" is 2, and no archetype means anything else by
//! it. Modelling multiplicity with the general interval would make
//! `lower_included: false` representable and meaningless, which is the defect
//! [`crate::base::Interval`]'s own module header explains it was shaped to
//! avoid.

use crate::error::ParseError;
use core::fmt;
use serde::{Deserialize, Serialize};

/// An inclusive `lower..upper` bound on a count, where `upper` may be open.
///
/// ```
/// use openehr::am::MultiplicityInterval;
///
/// let optional = MultiplicityInterval::OPTIONAL;
/// assert!(optional.contains(0));
/// assert!(optional.contains(1));
/// assert!(!optional.contains(2));
///
/// let any = MultiplicityInterval::new(0, None).unwrap();
/// assert!(any.is_open());
/// assert_eq!(any.to_string(), "0..*");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MultiplicityInterval {
    lower: u32,
    /// `None` is openEHR's `*`: unbounded above.
    upper: Option<u32>,
}

impl MultiplicityInterval {
    /// Exactly one: `1..1`.
    pub const MANDATORY: Self = Self {
        lower: 1,
        upper: Some(1),
    };

    /// Zero or one: `0..1`.
    pub const OPTIONAL: Self = Self {
        lower: 0,
        upper: Some(1),
    };

    /// Exactly zero: `0..0`. A specialisation uses this to remove something its
    /// parent allowed, which is why it is a value rather than an absence.
    pub const PROHIBITED: Self = Self {
        lower: 0,
        upper: Some(0),
    };

    /// Builds an interval.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the upper bound is below the lower one. A
    /// `2..1` occurrences constraint is satisfiable by nothing, and an
    /// archetype that carries one has a defect a runtime cannot repair —
    /// refusing at construction is the only place it can be reported to the
    /// person who can fix it.
    pub fn new(lower: u32, upper: Option<u32>) -> Result<Self, ParseError> {
        if let Some(upper) = upper
            && upper < lower
        {
            return Err(ParseError::invariant(
                "MULTIPLICITY_INTERVAL",
                "upper bound below lower bound",
            ));
        }
        Ok(Self { lower, upper })
    }

    /// `lower..*`, unbounded above.
    ///
    /// # Errors
    ///
    /// Never; the signature matches [`MultiplicityInterval::new`] so the two
    /// compose in the same `?` chain.
    pub fn at_least(lower: u32) -> Result<Self, ParseError> {
        Self::new(lower, None)
    }

    /// `0..upper`, or `0..*` when `upper` is `None` — the shape AOM2's
    /// `C_OBJECT.effective_occurrences()` infers for a node that states no
    /// occurrences: "always assume 0 as the lower bound", with the upper
    /// taken from the owning attribute (`K15.32`). Infallible by
    /// construction, since `0` is below every upper bound, which is why this
    /// is not [`Self::new`].
    #[must_use]
    pub const fn from_zero_to(upper: Option<u32>) -> Self {
        Self { lower: 0, upper }
    }

    /// The lower bound, inclusive.
    #[must_use]
    pub const fn lower(&self) -> u32 {
        self.lower
    }

    /// The upper bound, inclusive, or `None` for open.
    #[must_use]
    pub const fn upper(&self) -> Option<u32> {
        self.upper
    }

    /// Whether the interval is unbounded above.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.upper.is_none()
    }

    /// Whether at least one occurrence is required.
    #[must_use]
    pub const fn is_mandatory(&self) -> bool {
        self.lower >= 1
    }

    /// Whether nothing at all is permitted.
    #[must_use]
    pub const fn is_prohibited(&self) -> bool {
        matches!(self.upper, Some(0))
    }

    /// Whether a count satisfies the interval.
    #[must_use]
    pub const fn contains(&self, count: u32) -> bool {
        count >= self.lower
            && match self.upper {
                Some(upper) => count <= upper,
                None => true,
            }
    }

    /// Whether every count this interval permits, the other permits too.
    ///
    /// This is the narrowing test a specialisation and a template have to pass
    /// (`K15.13`, `K15.17`): a child may tighten what its parent allowed and
    /// may never widen it. It is defined here, with the type, rather than in
    /// the code that will eventually call it, because "narrows" is a property
    /// of the interval and every caller must mean the same thing by it.
    #[must_use]
    pub const fn narrows(&self, parent: &Self) -> bool {
        if self.lower < parent.lower {
            return false;
        }
        match (self.upper, parent.upper) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(mine), Some(theirs)) => mine <= theirs,
        }
    }
}

impl fmt::Display for MultiplicityInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.upper {
            Some(upper) => write!(f, "{}..{upper}", self.lower),
            None => write!(f, "{}..*", self.lower),
        }
    }
}

/// `CARDINALITY`: how many members a container attribute may hold, and whether
/// their order and uniqueness are part of the constraint.
///
/// ```
/// use openehr::am::{Cardinality, MultiplicityInterval};
///
/// let list = Cardinality::new(MultiplicityInterval::at_least(1).unwrap())
///     .ordered()
///     .unique();
/// assert!(list.is_ordered() && list.is_unique());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cardinality {
    interval: MultiplicityInterval,
    is_ordered: bool,
    is_unique: bool,
}

impl Cardinality {
    /// Builds an unordered, non-unique cardinality over `interval`.
    #[must_use]
    pub const fn new(interval: MultiplicityInterval) -> Self {
        Self {
            interval,
            is_ordered: false,
            is_unique: false,
        }
    }

    /// Marks the container's order as significant.
    #[must_use]
    pub const fn ordered(mut self) -> Self {
        self.is_ordered = true;
        self
    }

    /// Marks the container's members as required to be distinct.
    #[must_use]
    pub const fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }

    /// The permitted member count.
    #[must_use]
    pub const fn interval(&self) -> &MultiplicityInterval {
        &self.interval
    }

    /// Whether member order is significant.
    #[must_use]
    pub const fn is_ordered(&self) -> bool {
        self.is_ordered
    }

    /// Whether members must be distinct.
    #[must_use]
    pub const fn is_unique(&self) -> bool {
        self.is_unique
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_upper_bound_below_the_lower_one_is_refused() {
        assert!(MultiplicityInterval::new(2, Some(1)).is_err());
        assert!(MultiplicityInterval::new(2, Some(2)).is_ok());
    }

    #[test]
    fn prohibited_is_distinguishable_from_optional() {
        assert!(MultiplicityInterval::PROHIBITED.is_prohibited());
        assert!(!MultiplicityInterval::OPTIONAL.is_prohibited());
        assert!(!MultiplicityInterval::PROHIBITED.contains(1));
    }

    #[test]
    fn narrowing_is_directional() {
        let parent = MultiplicityInterval::new(0, Some(4)).unwrap();
        let child = MultiplicityInterval::new(1, Some(2)).unwrap();
        assert!(child.narrows(&parent));
        assert!(!parent.narrows(&child));

        // An open child cannot narrow a bounded parent, and a bounded child
        // always narrows an open parent.
        let open = MultiplicityInterval::at_least(1).unwrap();
        assert!(!open.narrows(&parent));
        assert!(parent.narrows(&MultiplicityInterval::new(0, None).unwrap()));
    }

    #[test]
    fn display_writes_openehr_multiplicity_syntax() {
        assert_eq!(MultiplicityInterval::MANDATORY.to_string(), "1..1");
        assert_eq!(
            MultiplicityInterval::at_least(2).unwrap().to_string(),
            "2..*"
        );
    }
}
