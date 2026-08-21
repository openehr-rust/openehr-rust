//! Ordered and quantified values: `DV_ORDINAL`, `DV_SCALE`, `DV_QUANTITY`,
//! `DV_COUNT`, `DV_PROPORTION`, and the reference ranges that give them
//! clinical meaning.
//!
//! # Comparison refuses more often than it answers
//!
//! openEHR's `DV_ORDERED` declares `is_strictly_comparable_to`, and the reason
//! is not tidiness. Two `DV_QUANTITY` values are comparable only if they share
//! units: `5 mg` and `5 mL` are equal as numbers and are not the same dose of
//! anything. Two `DV_ORDINAL` values are comparable only if their symbols come
//! from the same terminology, because ordinal `2` on a pain scale and ordinal
//! `2` on a sedation scale are unrelated.
//!
//! So [`PartialOrd`] here returns `None` for cross-unit and cross-terminology
//! comparisons. That makes `a < b` false *and* `a >= b` false, which is
//! surprising exactly once and correct every time. The alternative — comparing
//! magnitudes and ignoring units — produces a decision support rule that fires
//! on the wrong drug.
//!
//! This crate does **not** convert units. UCUM conversion needs a UCUM
//! implementation, and one that silently converted `mg` to `g` would let a
//! thousand-fold error look like a successful comparison.

use super::DataValue;
use super::text::{DvCodedText, DvText};
use crate::base::Interval;
use crate::error::ParseError;
use core::cmp::Ordering;
use serde::{Deserialize, Serialize};

/// Whether a measured magnitude is exact, or a bound, or approximate.
///
/// `< 0.5` for an assay below its detection limit is a real and common result,
/// and recording it as `0.5` loses the only part that matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MagnitudeStatus {
    /// `=` — the magnitude is the value.
    Equal,
    /// `<` — the true value is below the magnitude.
    LessThan,
    /// `>` — the true value is above the magnitude.
    GreaterThan,
    /// `<=` — the true value is at or below the magnitude.
    LessOrEqual,
    /// `>=` — the true value is at or above the magnitude.
    GreaterOrEqual,
    /// `~` — the magnitude is approximate.
    Approximate,
}

impl MagnitudeStatus {
    /// The specification's string encoding.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::LessThan => "<",
            Self::GreaterThan => ">",
            Self::LessOrEqual => "<=",
            Self::GreaterOrEqual => ">=",
            Self::Approximate => "~",
        }
    }

    /// Parses the specification's string encoding.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] for anything outside the six permitted values.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        Ok(match s {
            "=" => Self::Equal,
            "<" => Self::LessThan,
            ">" => Self::GreaterThan,
            "<=" => Self::LessOrEqual,
            ">=" => Self::GreaterOrEqual,
            "~" => Self::Approximate,
            _ => {
                return Err(ParseError::invariant(
                    "DV_QUANTIFIED",
                    "Magnitude_status_valid",
                ));
            }
        })
    }

    /// Whether the magnitude is the exact value.
    #[must_use]
    pub fn is_exact(self) -> bool {
        self == Self::Equal
    }
}

impl Serialize for MagnitudeStatus {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MagnitudeStatus {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

/// A named range with a clinical meaning: "normal", "therapeutic", "critical".
///
/// ```
/// use openehr::base::Interval;
/// use openehr::rm::data_types::{DataValue, DvQuantity, DvText, ReferenceRange};
///
/// let low = DataValue::Quantity(DvQuantity::new(3.5, "mmol/l").unwrap());
/// let high = DataValue::Quantity(DvQuantity::new(5.5, "mmol/l").unwrap());
/// let range = ReferenceRange::new(
///     DvText::new("normal").unwrap(),
///     Interval::closed(low, high).unwrap(),
/// );
/// assert_eq!(range.meaning().value(), "normal");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceRange {
    meaning: DvText,
    range: Interval<DataValue>,
}

impl ReferenceRange {
    /// Builds a reference range.
    #[must_use]
    pub fn new(meaning: DvText, range: Interval<DataValue>) -> Self {
        Self { meaning, range }
    }

    /// What the range means.
    #[must_use]
    pub fn meaning(&self) -> &DvText {
        &self.meaning
    }

    /// The range itself.
    #[must_use]
    pub fn range(&self) -> &Interval<DataValue> {
        &self.range
    }

    /// Whether a value falls inside the range.
    ///
    /// Inherits [`Interval`]'s partial-order semantics: a value that is not
    /// comparable with the bounds is not inside.
    #[must_use]
    pub fn contains(&self, value: &DataValue) -> bool {
        self.range.contains(value)
    }
}

/// The attributes every `DV_ORDERED` carries.
///
/// Modelled as one struct and flattened into each concrete class rather than
/// repeated six times: openEHR puts them on the abstract parent, and a reader
/// checking this crate against the specification should find the same grouping.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct OrderedAttrs {
    // Boxed for size, not for ownership: `DataValue` contains `DvQuantity`
    // contains `OrderedAttrs` contains an interval of `DataValue`, and the RM
    // really is that cyclic — a quantity's normal range is itself expressed as
    // quantities. `other_reference_ranges` needs no box because `Vec` already
    // is one.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    normal_range: Option<Box<Interval<DataValue>>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    other_reference_ranges: Vec<ReferenceRange>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    normal_status: Option<super::text::CodePhrase>,
}

impl OrderedAttrs {
    /// The normal range, if recorded.
    #[must_use]
    pub fn normal_range(&self) -> Option<&Interval<DataValue>> {
        self.normal_range.as_deref()
    }

    /// Other named ranges.
    #[must_use]
    pub fn other_reference_ranges(&self) -> &[ReferenceRange] {
        &self.other_reference_ranges
    }

    /// The HL7 abnormal flag, if recorded.
    #[must_use]
    pub fn normal_status(&self) -> Option<&super::text::CodePhrase> {
        self.normal_status.as_ref()
    }
}

/// Read access to the attributes of `DV_ORDERED`.
///
/// A trait rather than inherent methods on each class, so that generic code —
/// a report that prints any ordered value with its reference range — does not
/// have to match on the concrete type.
pub trait DvOrdered {
    /// The `DV_ORDERED` attributes.
    fn ordered_attrs(&self) -> &OrderedAttrs;

    /// Compares two values of the same class, or reports them incomparable.
    ///
    /// This is **not** `PartialOrd::partial_cmp`, and deliberately (`D3.18b`).
    /// Every `DV_ORDERED` derives `PartialEq` over all its fields — including
    /// the `OrderedAttrs` below, and `DV_QUANTITY`'s `precision` and
    /// `units_display_name` — while ordering compares only the magnitude. So
    /// `DvCount::new(5).with_normal_range(r)` and `DvCount::new(5)` are not
    /// equal and order `Equal`, which is precisely what Rust's `PartialOrd`
    /// contract forbids. Rather than break the contract, the crate does not
    /// implement the trait, and comparison is this method.
    ///
    /// `None` is an answer, never a default (`D3.14`): a quantity in `mg` and
    /// one in `ml` are not comparable in either direction.
    fn semantic_cmp(&self, other: &Self) -> Option<Ordering>
    where
        Self: Sized;

    /// The normal range, if recorded.
    fn normal_range(&self) -> Option<&Interval<DataValue>> {
        self.ordered_attrs().normal_range()
    }

    /// Other named ranges.
    fn other_reference_ranges(&self) -> &[ReferenceRange] {
        self.ordered_attrs().other_reference_ranges()
    }

    /// Whether the value is outside its own recorded normal range.
    ///
    /// Returns `None` when there is no normal range, rather than `false`. "We
    /// did not record a range" and "the value is normal" are different facts,
    /// and a dashboard that renders the first as the second is reassuring for
    /// the wrong reason.
    fn is_abnormal(&self) -> Option<bool>;
}

/// Generates the `DV_ORDERED` builders for a concrete descendant.
///
/// The five ordered classes carry the same three reference-range attributes, so
/// they get the same three builders. Writing them out five times is how the
/// fifth one ends up subtly different from the other four.
macro_rules! ordered_builders {
    ($ty:ty, $class:literal) => {
        impl $ty {
            /// Attaches the normal range for this value in its measurement
            /// context.
            #[must_use]
            pub fn with_normal_range(mut self, range: Interval<DataValue>) -> Self {
                self.ordered.normal_range = Some(Box::new(range));
                self
            }

            /// Attaches another named reference range — therapeutic, critical,
            /// age-specific.
            #[must_use]
            pub fn with_other_reference_range(mut self, range: ReferenceRange) -> Self {
                self.ordered.other_reference_ranges.push(range);
                self
            }

            /// Attaches the HL7 abnormal flag.
            ///
            /// Not validated here: openEHR requires it to come from the
            /// `normal statuses` code set **and** to agree with the value's
            /// position in `normal_range`, and the second of those is a
            /// relationship between two attributes rather than a property of
            /// one. Both are checked by [`crate::validation`], which is where a
            /// received document is checked too.
            #[must_use]
            pub fn with_normal_status(mut self, status: super::text::CodePhrase) -> Self {
                self.ordered.normal_status = Some(status);
                self
            }
        }
    };
}

macro_rules! ordered {
    ($ty:ty, $as_value:expr, $cmp:expr) => {
        impl DvOrdered for $ty {
            fn ordered_attrs(&self) -> &OrderedAttrs {
                &self.ordered
            }

            fn semantic_cmp(&self, other: &Self) -> Option<Ordering> {
                #[allow(clippy::redundant_closure_call)]
                ($cmp)(self, other)
            }

            fn is_abnormal(&self) -> Option<bool> {
                let range = self.ordered.normal_range.as_ref()?;
                #[allow(clippy::redundant_closure_call)]
                let value = ($as_value)(self);
                Some(!range.contains(&value))
            }
        }
    };
}

/// A value on a scale of ordered symbols: `+`, `++`, `+++`.
///
/// ```
/// use openehr::rm::data_types::{CodePhrase, DvCodedText, DvOrdered as _, DvOrdinal};
///
/// let mild = DvOrdinal::new(1, DvCodedText::new("mild", CodePhrase::new("local", "at0002").unwrap()).unwrap());
/// let severe = DvOrdinal::new(3, DvCodedText::new("severe", CodePhrase::new("local", "at0004").unwrap()).unwrap());
/// assert_eq!(mild.semantic_cmp(&severe), Some(core::cmp::Ordering::Less));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvOrdinal {
    value: i64,
    symbol: DvCodedText,
    #[serde(flatten)]
    ordered: OrderedAttrs,
}

impl DvOrdinal {
    /// Builds an ordinal.
    #[must_use]
    pub fn new(value: i64, symbol: DvCodedText) -> Self {
        Self {
            value,
            symbol,
            ordered: OrderedAttrs::default(),
        }
    }

    /// The ordinal's position on the scale.
    #[must_use]
    pub fn value(&self) -> i64 {
        self.value
    }

    /// The symbol.
    #[must_use]
    pub fn symbol(&self) -> &DvCodedText {
        &self.symbol
    }

    /// Whether two ordinals sit on the same scale.
    ///
    /// Decided by the symbols' terminology: two ordinals coded against
    /// different terminologies are two different scales, whatever their
    /// numbers say.
    #[must_use]
    pub fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        self.symbol.defining_code().terminology_id()
            == other.symbol.defining_code().terminology_id()
    }
}

ordered!(
    DvOrdinal,
    |s: &DvOrdinal| DataValue::Ordinal(s.clone()),
    |a: &DvOrdinal, b: &DvOrdinal| a
        .is_strictly_comparable_to(b)
        .then(|| a.value.cmp(&b.value))
);
ordered_builders!(DvOrdinal, "DvOrdinal");

/// A value on a numeric scale, where the steps may be fractional.
///
/// `DV_SCALE` is `DV_ORDINAL` with a `Real` value: it exists because scales
/// like the Glasgow Outcome Scale Extended have half-steps, and forcing those
/// into an integer ordinal loses them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvScale {
    value: f64,
    symbol: DvCodedText,
    #[serde(flatten)]
    ordered: OrderedAttrs,
}

impl DvScale {
    /// Builds a scale value.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the value is NaN or infinite. Neither has a
    /// position on a scale, and NaN in particular compares unequal to itself,
    /// which would break every ordering built on top of it.
    pub fn new(value: f64, symbol: DvCodedText) -> Result<Self, ParseError> {
        if !value.is_finite() {
            return Err(ParseError::invariant("DV_SCALE", "Value_finite"));
        }
        Ok(Self {
            value,
            symbol,
            ordered: OrderedAttrs::default(),
        })
    }

    /// The position on the scale.
    #[must_use]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// The symbol.
    #[must_use]
    pub fn symbol(&self) -> &DvCodedText {
        &self.symbol
    }

    /// Whether two scale values sit on the same scale.
    #[must_use]
    pub fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        self.symbol.defining_code().terminology_id()
            == other.symbol.defining_code().terminology_id()
    }
}

ordered!(
    DvScale,
    |s: &DvScale| DataValue::Scale(s.clone()),
    |a: &DvScale, b: &DvScale| {
        if !a.is_strictly_comparable_to(b) {
            return None;
        }
        a.value.partial_cmp(&b.value)
    }
);
ordered_builders!(DvScale, "DvScale");

/// A dimensioned measured quantity: `110 mm[Hg]`, `3.5 mmol/l`.
///
/// ```
/// use openehr::rm::data_types::{DvOrdered as _, DvQuantity, MagnitudeStatus};
///
/// let systolic = DvQuantity::new(184.0, "mm[Hg]").unwrap().with_precision(0).unwrap();
/// assert_eq!(systolic.units(), "mm[Hg]");
///
/// // Below the assay's detection limit — a real result, not 0.5.
/// let trough = DvQuantity::new(0.5, "ng/mL").unwrap()
///     .with_magnitude_status(MagnitudeStatus::LessThan);
/// assert!(!trough.magnitude_status().unwrap().is_exact());
///
/// // Different units are not comparable, in either direction.
/// let mg = DvQuantity::new(5.0, "mg").unwrap();
/// let ml = DvQuantity::new(5.0, "mL").unwrap();
/// assert_eq!(mg.semantic_cmp(&ml), None);
/// assert!(!(mg == ml));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvQuantity {
    magnitude: f64,
    units: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    precision: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    units_system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    units_display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    magnitude_status: Option<MagnitudeStatus>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    accuracy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    accuracy_is_percent: Option<bool>,
    #[serde(flatten)]
    ordered: OrderedAttrs,
}

impl DvQuantity {
    /// The units system openEHR recommends.
    pub const UCUM: &'static str = "UCUM";

    /// The `precision` value meaning "any number of decimal places".
    ///
    /// openEHR spells this `-1`. It is a *stated* precision — "this value is
    /// not limited" — and is distinct from `precision` being absent, which says
    /// nothing about precision at all.
    pub const UNLIMITED_PRECISION: i32 = -1;

    /// The `accuracy` value meaning "accuracy was not recorded".
    ///
    /// openEHR's `DV_AMOUNT.unknown_accuracy_value`, which is `-1.0`. A
    /// `DV_AMOUNT` records absence of accuracy with this sentinel rather than
    /// by omitting the attribute, so a reader that treats `-1` as a measured
    /// error of minus one is wrong by the width of the whole scale.
    pub const UNKNOWN_ACCURACY: f64 = -1.0;

    /// Builds a quantity.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the magnitude is not finite, or if the units
    /// string is empty (`Units_valid`). A quantity with no units is a number,
    /// and openEHR has `DV_COUNT` for numbers.
    pub fn new(magnitude: f64, units: impl Into<String>) -> Result<Self, ParseError> {
        let units = units.into();
        if !magnitude.is_finite() {
            return Err(ParseError::invariant("DV_QUANTITY", "Magnitude_finite"));
        }
        if units.is_empty() {
            return Err(ParseError::invariant("DV_QUANTITY", "Units_valid"));
        }
        Ok(Self {
            magnitude,
            units,
            precision: None,
            units_system: None,
            units_display_name: None,
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            ordered: OrderedAttrs::default(),
        })
    }

    /// Records how many decimal places the value was measured to.
    ///
    /// `0` means an integral quantity. **`-1` means "no limit"** — any number
    /// of decimal places — and is openEHR's way of saying the precision was not
    /// constrained, which is not the same as not recording a precision at all.
    /// Both are representable, and refusing `-1` would reject conformant data.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `precision < -1`
    /// (`DV_QUANTITY.Precision_valid`).
    pub fn with_precision(mut self, precision: i32) -> Result<Self, ParseError> {
        if precision < Self::UNLIMITED_PRECISION {
            return Err(ParseError::invariant("DV_QUANTITY", "Precision_valid"));
        }
        self.precision = Some(precision);
        Ok(self)
    }

    /// Records which units system the units string belongs to.
    #[must_use]
    pub fn with_units_system(mut self, system: impl Into<String>) -> Self {
        self.units_system = Some(system.into());
        self
    }

    /// Records a human-readable rendering of the units.
    #[must_use]
    pub fn with_units_display_name(mut self, name: impl Into<String>) -> Self {
        self.units_display_name = Some(name.into());
        self
    }

    /// Records that the magnitude is a bound or an approximation.
    #[must_use]
    pub fn with_magnitude_status(mut self, status: MagnitudeStatus) -> Self {
        self.magnitude_status = Some(status);
        self
    }

    /// Records the measurement accuracy as a half-range: an accuracy of `x`
    /// means ±`x`.
    ///
    /// Two openEHR rules apply, and they are easy to get backwards:
    ///
    /// - `Accuracy_validity`: a **percentage** accuracy must be in 0–100.
    /// - `Accuracy_is_percent_validity`: an accuracy of **0** must not be
    ///   flagged as a percentage. Zero means 100% accurate — no error at all —
    ///   and "0%" would read as the opposite.
    ///
    /// Pass [`DvQuantity::UNKNOWN_ACCURACY`] to record that accuracy was not
    /// measured. It is not a percentage, so it must be given with
    /// `is_percent = false`.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the accuracy is not finite, or if either rule
    /// above is broken.
    pub fn with_accuracy(mut self, accuracy: f64, is_percent: bool) -> Result<Self, ParseError> {
        if !accuracy.is_finite() {
            return Err(ParseError::invariant("DV_AMOUNT", "Accuracy_finite"));
        }
        // Exact comparison against zero, deliberately: openEHR's rule is about
        // the value 0 and not about a neighbourhood of it, and an accuracy of
        // 1e-15 is a measurement rather than a claim of perfection.
        #[allow(clippy::float_cmp)]
        let is_zero = accuracy == 0.0;
        if is_zero && is_percent {
            return Err(ParseError::invariant(
                "DV_AMOUNT",
                "Accuracy_is_percent_validity",
            ));
        }
        if is_percent && !(0.0..=100.0).contains(&accuracy) {
            return Err(ParseError::invariant("DV_AMOUNT", "Accuracy_validity"));
        }
        self.accuracy = Some(accuracy);
        self.accuracy_is_percent = Some(is_percent);
        Ok(self)
    }

    /// The numeric magnitude.
    #[must_use]
    pub fn magnitude(&self) -> f64 {
        self.magnitude
    }

    /// The units.
    #[must_use]
    pub fn units(&self) -> &str {
        &self.units
    }

    /// The recorded decimal precision, if any.
    #[must_use]
    pub fn precision(&self) -> Option<i32> {
        self.precision
    }

    /// The units system, if recorded.
    #[must_use]
    pub fn units_system(&self) -> Option<&str> {
        self.units_system.as_deref()
    }

    /// The display form of the units, if recorded.
    #[must_use]
    pub fn units_display_name(&self) -> Option<&str> {
        self.units_display_name.as_deref()
    }

    /// Whether the magnitude is exact, bounded, or approximate.
    #[must_use]
    pub fn magnitude_status(&self) -> Option<MagnitudeStatus> {
        self.magnitude_status
    }

    /// The measurement accuracy, if recorded.
    #[must_use]
    pub fn accuracy(&self) -> Option<f64> {
        self.accuracy
    }

    /// Whether [`DvQuantity::accuracy`] is a percentage of the magnitude.
    #[must_use]
    pub fn accuracy_is_percent(&self) -> Option<bool> {
        self.accuracy_is_percent
    }

    /// Whether accuracy was not recorded.
    ///
    /// openEHR's `DV_AMOUNT.accuracy_unknown`. True when the attribute is
    /// absent **or** carries the [`DvQuantity::UNKNOWN_ACCURACY`] sentinel:
    /// both mean the same thing to a reader, and only one of them is obvious.
    #[must_use]
    pub fn accuracy_unknown(&self) -> bool {
        #[allow(clippy::float_cmp)]
        self.accuracy.is_none_or(|a| a == Self::UNKNOWN_ACCURACY)
    }

    /// Whether the quantity is a whole number — openEHR's `is_integral`, which
    /// it defines as `precision = 0`.
    ///
    /// `None` when no precision was recorded: a magnitude that happens to be
    /// whole is not the same as a quantity *declared* integral.
    #[must_use]
    pub fn is_integral(&self) -> Option<bool> {
        self.precision.map(|p| p == 0)
    }

    /// Whether two quantities are in the same units and can be compared.
    ///
    /// String equality on units, not unit conversion — see the module header.
    #[must_use]
    pub fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        self.units == other.units
    }
}

ordered!(
    DvQuantity,
    |s: &DvQuantity| DataValue::Quantity(s.clone()),
    |a: &DvQuantity, b: &DvQuantity| {
        if !a.is_strictly_comparable_to(b) {
            return None;
        }
        a.magnitude.partial_cmp(&b.magnitude)
    }
);
ordered_builders!(DvQuantity, "DvQuantity");

/// A dimensionless count: three tablets, two episodes.
///
/// ```
/// use openehr::rm::data_types::DvCount;
///
/// let tablets = DvCount::new(3);
/// assert_eq!(tablets.magnitude(), 3);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvCount {
    magnitude: i64,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    magnitude_status: Option<MagnitudeStatus>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    accuracy: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    accuracy_is_percent: Option<bool>,
    #[serde(flatten)]
    ordered: OrderedAttrs,
}

impl DvCount {
    /// Builds a count.
    #[must_use]
    pub fn new(magnitude: i64) -> Self {
        Self {
            magnitude,
            magnitude_status: None,
            accuracy: None,
            accuracy_is_percent: None,
            ordered: OrderedAttrs::default(),
        }
    }

    /// The count.
    #[must_use]
    pub fn magnitude(&self) -> i64 {
        self.magnitude
    }

    /// Whether the count is exact, bounded, or approximate.
    #[must_use]
    pub fn magnitude_status(&self) -> Option<MagnitudeStatus> {
        self.magnitude_status
    }

    /// Records that the count is a bound or an approximation.
    #[must_use]
    pub fn with_magnitude_status(mut self, status: MagnitudeStatus) -> Self {
        self.magnitude_status = Some(status);
        self
    }
}

ordered!(
    DvCount,
    |s: &DvCount| DataValue::Count(s.clone()),
    |a: &DvCount, b: &DvCount| Some(a.magnitude.cmp(&b.magnitude))
);
ordered_builders!(DvCount, "DvCount");

/// What kind of proportion a `DV_PROPORTION` expresses.
///
/// The integer values are openEHR's, and they are what appears in instance
/// data — this enum exists so that `2` cannot be written where `pk_percent` was
/// meant and only discovered when a percentage renders as a ratio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ProportionKind {
    /// `0` — a ratio, `1:128`. The denominator is free.
    Ratio = 0,
    /// `1` — a unitary proportion, `x/1`. The denominator must be 1.
    Unitary = 1,
    /// `2` — a percentage, `x/100`. The denominator must be 100.
    Percent = 2,
    /// `3` — a fraction, `1/2`. Both parts must be whole numbers.
    Fraction = 3,
    /// `4` — an integer fraction, `3/4`, rendered as a fraction rather than
    /// reduced. Both parts must be whole numbers.
    IntegerFraction = 4,
}

impl ProportionKind {
    /// The openEHR integer encoding.
    #[must_use]
    pub fn as_i32(self) -> i32 {
        self as i32
    }

    /// Parses the openEHR integer encoding.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] for a value outside 0–4.
    pub fn from_i32(v: i32) -> Result<Self, ParseError> {
        Ok(match v {
            0 => Self::Ratio,
            1 => Self::Unitary,
            2 => Self::Percent,
            3 => Self::Fraction,
            4 => Self::IntegerFraction,
            _ => return Err(ParseError::invariant("DV_PROPORTION", "Type_validity")),
        })
    }

    /// Whether this kind requires both parts to be whole numbers.
    #[must_use]
    pub fn requires_integral(self) -> bool {
        matches!(self, Self::Fraction | Self::IntegerFraction)
    }
}

impl Serialize for ProportionKind {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_i32(self.as_i32())
    }
}

impl<'de> Deserialize<'de> for ProportionKind {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::from_i32(i32::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

/// A ratio, fraction, or percentage.
///
/// ```
/// use openehr::rm::data_types::{DvProportion, ProportionKind};
///
/// let percent = DvProportion::new(37.0, 100.0, ProportionKind::Percent).unwrap();
/// assert!((percent.as_ratio() - 0.37).abs() < 1e-12);
///
/// // A percentage whose denominator is not 100 is not a percentage.
/// assert!(DvProportion::new(37.0, 50.0, ProportionKind::Percent).is_err());
///
/// // A fraction with a fractional part is not a fraction.
/// assert!(DvProportion::new(1.5, 3.0, ProportionKind::Fraction).is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DvProportion {
    numerator: f64,
    denominator: f64,
    #[serde(rename = "type")]
    kind: ProportionKind,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    precision: Option<i32>,
    #[serde(flatten)]
    ordered: OrderedAttrs,
}

impl DvProportion {
    /// Builds a proportion.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if either part is not finite, if the denominator
    /// is zero (`Denominator_valid`), or if the parts contradict the kind:
    ///
    /// | Kind | Rule |
    /// | --- | --- |
    /// | `Unitary` | denominator is 1 |
    /// | `Percent` | denominator is 100 |
    /// | `Fraction`, `IntegerFraction` | both parts are whole numbers |
    pub fn new(numerator: f64, denominator: f64, kind: ProportionKind) -> Result<Self, ParseError> {
        if !numerator.is_finite() || !denominator.is_finite() {
            return Err(ParseError::invariant("DV_PROPORTION", "Parts_finite"));
        }
        if denominator == 0.0 {
            return Err(ParseError::invariant("DV_PROPORTION", "Valid_denominator"));
        }
        // Exact float comparison, deliberately: openEHR's rule is that a
        // unitary proportion's denominator *is* 1 and a percentage's *is* 100,
        // and a denominator of 99.9999999 came from a computation that should
        // not have produced a denominator at all. A tolerance here would accept
        // it and then render it as a percentage.
        #[allow(clippy::float_cmp)]
        match kind {
            ProportionKind::Unitary if denominator != 1.0 => {
                return Err(ParseError::invariant("DV_PROPORTION", "Unitary_validity"));
            }
            ProportionKind::Percent if denominator != 100.0 => {
                return Err(ParseError::invariant("DV_PROPORTION", "Percent_validity"));
            }
            k if k.requires_integral()
                && (numerator.fract() != 0.0 || denominator.fract() != 0.0) =>
            {
                return Err(ParseError::invariant(
                    "DV_PROPORTION",
                    "Is_integral_validity",
                ));
            }
            _ => {}
        }
        Ok(Self {
            numerator,
            denominator,
            kind,
            precision: None,
            ordered: OrderedAttrs::default(),
        })
    }

    /// The numerator.
    #[must_use]
    pub fn numerator(&self) -> f64 {
        self.numerator
    }

    /// The denominator.
    #[must_use]
    pub fn denominator(&self) -> f64 {
        self.denominator
    }

    /// What kind of proportion this is.
    #[must_use]
    pub fn kind(&self) -> ProportionKind {
        self.kind
    }

    /// The recorded decimal precision, if any.
    #[must_use]
    pub fn precision(&self) -> Option<i32> {
        self.precision
    }

    /// Records how many decimal places the numerator and denominator are
    /// expressed to.
    ///
    /// As on [`DvQuantity::with_precision`], `0` means integral and `-1` means
    /// no limit.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `precision < -1`, or if `precision` is `0`
    /// while either part has a fractional component
    /// (`DV_PROPORTION.Precision_validity`: `precision = 0 implies
    /// is_integral`).
    ///
    /// Note the direction of that rule — it is the one an implementation
    /// usually gets backwards. It does **not** say an integral *kind* forbids a
    /// non-zero precision; it says declaring precision `0` asserts that the
    /// numbers really are whole.
    pub fn with_precision(mut self, precision: i32) -> Result<Self, ParseError> {
        if precision < DvQuantity::UNLIMITED_PRECISION {
            return Err(ParseError::invariant("DV_PROPORTION", "Precision_validity"));
        }
        if precision == 0 && !self.is_integral() {
            return Err(ParseError::invariant("DV_PROPORTION", "Precision_validity"));
        }
        self.precision = Some(precision);
        Ok(self)
    }

    /// Whether both parts are whole numbers — openEHR's `is_integral`.
    #[must_use]
    pub fn is_integral(&self) -> bool {
        self.numerator.fract() == 0.0 && self.denominator.fract() == 0.0
    }

    /// The proportion as a plain number.
    #[must_use]
    pub fn as_ratio(&self) -> f64 {
        self.numerator / self.denominator
    }

    /// Whether two proportions can be compared.
    ///
    /// Same kind only. A ratio of `1:128` and a percentage of `0.78%` are close
    /// to the same number and are not the same statement, and openEHR's own
    /// `is_strictly_comparable_to` says so.
    #[must_use]
    pub fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

ordered!(
    DvProportion,
    |s: &DvProportion| DataValue::Proportion(s.clone()),
    |a: &DvProportion, b: &DvProportion| {
        if !a.is_strictly_comparable_to(b) {
            return None;
        }
        a.as_ratio().partial_cmp(&b.as_ratio())
    }
);
ordered_builders!(DvProportion, "DvProportion");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rm::data_types::CodePhrase;

    fn quantity(magnitude: f64, units: &str) -> DvQuantity {
        DvQuantity::new(magnitude, units).unwrap()
    }

    #[test]
    fn different_units_are_not_comparable_in_either_direction() {
        let mg = quantity(5.0, "mg");
        let ml = quantity(5.0, "mL");
        // None, not Some(Equal): the magnitudes agree and the values do not.
        assert_eq!(mg.semantic_cmp(&ml), None);
        assert_eq!(ml.semantic_cmp(&mg), None);
        assert_ne!(mg, ml);
    }

    #[test]
    fn same_units_compare_normally() {
        assert_eq!(
            quantity(90.0, "mm[Hg]").semantic_cmp(&quantity(140.0, "mm[Hg]")),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn ordinals_from_different_terminologies_do_not_compare() {
        let pain = DvOrdinal::new(
            2,
            DvCodedText::new("moderate", CodePhrase::new("local", "at0003").unwrap()).unwrap(),
        );
        let sedation = DvOrdinal::new(
            2,
            DvCodedText::new("moderate", CodePhrase::new("SNOMED-CT", "6736007").unwrap()).unwrap(),
        );
        assert_eq!(pain.semantic_cmp(&sedation), None);
    }

    #[test]
    fn proportion_kind_rules_are_enforced() {
        assert!(DvProportion::new(1.0, 1.0, ProportionKind::Unitary).is_ok());
        assert!(DvProportion::new(1.0, 2.0, ProportionKind::Unitary).is_err());
        assert!(DvProportion::new(37.0, 100.0, ProportionKind::Percent).is_ok());
        assert!(DvProportion::new(37.0, 99.0, ProportionKind::Percent).is_err());
        assert!(DvProportion::new(1.0, 2.0, ProportionKind::Fraction).is_ok());
        assert!(DvProportion::new(1.5, 2.0, ProportionKind::Fraction).is_err());
        assert!(DvProportion::new(1.0, 0.0, ProportionKind::Ratio).is_err());
        assert!(DvProportion::new(1.0, 128.0, ProportionKind::Ratio).is_ok());
    }

    #[test]
    fn is_abnormal_distinguishes_unknown_from_normal() {
        let no_range = quantity(200.0, "mg/dl");
        assert_eq!(no_range.is_abnormal(), None);

        let low = DataValue::Quantity(quantity(3.5, "mmol/l"));
        let high = DataValue::Quantity(quantity(5.5, "mmol/l"));
        let ranged =
            quantity(9.9, "mmol/l").with_normal_range(Interval::closed(low, high).unwrap());
        assert_eq!(ranged.is_abnormal(), Some(true));
    }

    #[test]
    fn precision_accepts_the_unlimited_sentinel() {
        // The defect this pins: `-1` is openEHR's *stated* precision meaning
        // "any number of decimal places" (`DV_QUANTITY.Precision_valid:
        // precision >= -1`). An implementation that requires `precision >= 0`
        // rejects conformant data, which is worse than accepting bad data
        // because the record cannot be read at all.
        assert!(quantity(1.5, "mg").with_precision(-1).is_ok());
        assert!(quantity(1.5, "mg").with_precision(0).is_ok());
        assert!(quantity(1.5, "mg").with_precision(3).is_ok());
        assert!(quantity(1.5, "mg").with_precision(-2).is_err());

        assert_eq!(
            quantity(1.0, "mg").with_precision(0).unwrap().is_integral(),
            Some(true)
        );
        assert_eq!(
            quantity(1.0, "mg").with_precision(2).unwrap().is_integral(),
            Some(false)
        );
        // No precision recorded is not the same as "declared integral".
        assert_eq!(quantity(1.0, "mg").is_integral(), None);
    }

    #[test]
    fn accuracy_zero_may_not_be_a_percentage() {
        // `DV_AMOUNT.Accuracy_is_percent_validity`: accuracy = 0 means 100%
        // accurate — no error at all — and flagging it as a percentage would
        // read as exactly the opposite.
        assert!(quantity(1.0, "mg").with_accuracy(0.0, true).is_err());
        assert!(quantity(1.0, "mg").with_accuracy(0.0, false).is_ok());
    }

    #[test]
    fn a_percentage_accuracy_stays_within_a_hundred() {
        // `DV_AMOUNT.Accuracy_validity` via `valid_percentage`.
        assert!(quantity(1.0, "mg").with_accuracy(2.5, true).is_ok());
        assert!(quantity(1.0, "mg").with_accuracy(100.0, true).is_ok());
        assert!(quantity(1.0, "mg").with_accuracy(100.5, true).is_err());
        // …and an absolute accuracy is not bounded by it.
        assert!(quantity(1.0, "mg").with_accuracy(500.0, false).is_ok());
    }

    #[test]
    fn unknown_accuracy_is_a_value_and_not_an_absence() {
        // openEHR records "not measured" as the sentinel -1.0, not by omitting
        // the attribute. A reader treating it as an error of minus one is wrong
        // by the width of the scale.
        let unknown = quantity(1.0, "mg")
            .with_accuracy(DvQuantity::UNKNOWN_ACCURACY, false)
            .unwrap();
        assert!(unknown.accuracy_unknown());
        assert_eq!(unknown.accuracy(), Some(-1.0));

        // Absent means the same thing to a reader, and answers the same.
        assert!(quantity(1.0, "mg").accuracy_unknown());
        // A real measurement does not.
        assert!(
            !quantity(1.0, "mg")
                .with_accuracy(2.5, true)
                .unwrap()
                .accuracy_unknown()
        );
        // The sentinel is not a percentage, and openEHR's rule refuses it as
        // one without needing a special case.
        assert!(
            quantity(1.0, "mg")
                .with_accuracy(DvQuantity::UNKNOWN_ACCURACY, true)
                .is_err()
        );
    }

    #[test]
    fn proportion_precision_asserts_integrality_rather_than_forbidding_it() {
        // `DV_PROPORTION.Precision_validity: precision = 0 implies is_integral`.
        // The rule runs in the direction an implementation usually gets
        // backwards: declaring precision 0 asserts the numbers are whole; it
        // does not forbid an integral *kind* from carrying a precision.
        let half = DvProportion::new(1.0, 2.0, ProportionKind::Fraction).unwrap();
        assert!(half.is_integral());
        assert!(half.clone().with_precision(0).is_ok());
        assert!(half.with_precision(2).is_ok());

        let fractional = DvProportion::new(1.5, 2.5, ProportionKind::Ratio).unwrap();
        assert!(!fractional.is_integral());
        assert!(fractional.clone().with_precision(0).is_err());
        assert!(fractional.clone().with_precision(2).is_ok());
        assert!(fractional.with_precision(-2).is_err());
    }

    #[test]
    fn non_finite_magnitudes_are_refused() {
        assert!(DvQuantity::new(f64::NAN, "mg").is_err());
        assert!(DvQuantity::new(f64::INFINITY, "mg").is_err());
    }

    /// Every `magnitude_status` marker, in both directions, and through JSON.
    ///
    /// `as_str` could return `""` or one wrong constant for every variant, and
    /// three arms of `parse` could each be deleted, with nothing failing
    /// (`lib:A-09`). This is the marker that says a result is `<0.1` rather
    /// than `0.1` — a below-detection-limit reading and a measured one — and
    /// openEHR constrains it as `DV_QUANTIFIED.Magnitude_status_valid`.
    ///
    /// Confusing `<` with `>` inverts the clinical meaning of a value while
    /// leaving the number correct, which is the kind of defect nothing
    /// downstream can detect.
    #[test]
    fn every_magnitude_status_marker_round_trips_through_its_encoding() {
        let all = [
            (MagnitudeStatus::Equal, "="),
            (MagnitudeStatus::LessThan, "<"),
            (MagnitudeStatus::GreaterThan, ">"),
            (MagnitudeStatus::LessOrEqual, "<="),
            (MagnitudeStatus::GreaterOrEqual, ">="),
            (MagnitudeStatus::Approximate, "~"),
        ];

        let mut encodings = Vec::new();
        for (status, text) in all {
            assert_eq!(status.as_str(), text, "{status:?} encoded wrongly");
            assert_eq!(
                MagnitudeStatus::parse(text).expect(text),
                status,
                "{text} parsed to the wrong marker"
            );
            encodings.push(text);
        }
        // No two markers share an encoding, which a constant `as_str` would
        // make true of all six.
        encodings.sort_unstable();
        encodings.dedup();
        assert_eq!(encodings.len(), all.len(), "two markers encode the same");

        // Only `=` is exact. `<0.1` is not a measurement of 0.1.
        assert!(MagnitudeStatus::Equal.is_exact());
        for (status, text) in all {
            if status != MagnitudeStatus::Equal {
                assert!(!status.is_exact(), "{text} was treated as exact");
            }
        }

        // Anything outside the six is refused, not defaulted to `=`.
        for bad in ["", "==", "=<", "≤", "!=", "less", " <"] {
            assert!(
                MagnitudeStatus::parse(bad).is_err(),
                "{bad:?} was accepted as a magnitude status"
            );
        }

        // And through canonical JSON, which is where the marker actually
        // travels and what the content digest is taken over (`db:M3.16`).
        for (status, text) in all {
            let q = DvQuantity::new(0.1, "mg/L")
                .unwrap()
                .with_magnitude_status(status);
            let json = serde_json::to_string(&q).expect("serialize");
            assert!(
                json.contains(&format!(r#""magnitude_status":"{text}""#)),
                "{status:?} missing from {json}"
            );
            let back: DvQuantity = serde_json::from_str(&json).expect(&json);
            assert_eq!(back.magnitude_status(), Some(status), "{json}");
        }

        // A quantity with no marker says so, rather than reporting `=`:
        // "no status was recorded" and "the value is exact" are different
        // records, and openEHR makes the attribute optional for that reason.
        let plain = DvQuantity::new(0.1, "mg/L").unwrap();
        assert_eq!(plain.magnitude_status(), None);
    }

    /// A reference range decides whether a value is inside it.
    ///
    /// `ReferenceRange::contains` could return a constant `true` or `false`
    /// with nothing failing (`lib:A-09`), and the `DvOrdered` accessors for
    /// `normal_range` and `other_reference_ranges` could return nothing at all.
    ///
    /// This is the machinery that tells a clinician a result is abnormal.
    /// `lib:A-01` already found `Normal_range_and_status_consistency` missing
    /// here; the membership test underneath it was equally unwatched. A
    /// constant `false` reports every result as outside its normal range, and a
    /// constant `true` reports none of them — the second being the dangerous
    /// direction.
    #[test]
    fn a_reference_range_says_which_values_are_inside_it() {
        let mg = |v: f64| DataValue::Quantity(DvQuantity::new(v, "mmol/l").unwrap());
        let normal = Interval::closed(mg(3.5), mg(5.5)).unwrap();
        let range = ReferenceRange::new(DvText::new("normal").unwrap(), normal.clone());

        assert_eq!(range.meaning().value(), "normal");
        // Inside, on both bounds, and outside each end. A constant answer
        // cannot get all five right.
        assert!(range.contains(&mg(4.0)), "a mid-range value is inside");
        assert!(range.contains(&mg(3.5)), "a closed lower bound is inside");
        assert!(range.contains(&mg(5.5)), "a closed upper bound is inside");
        assert!(!range.contains(&mg(3.4)), "below the range is outside");
        assert!(!range.contains(&mg(5.6)), "above the range is outside");

        // A value that is not comparable with the bounds is not inside —
        // different units are not a smaller number (`D3.15`).
        let other_units = DataValue::Quantity(DvQuantity::new(4.0, "mg/dL").unwrap());
        assert!(
            !range.contains(&other_units),
            "a value in different units was reported as within range"
        );

        // Through a DV_ORDERED: the accessors must return what was built, and
        // `is_abnormal` is derived from the normal range.
        let low = DvQuantity::new(2.0, "mmol/l")
            .unwrap()
            .with_normal_range(normal.clone())
            .with_other_reference_range(range.clone());
        assert_eq!(low.normal_range(), Some(&normal));
        assert_eq!(low.other_reference_ranges().len(), 1);
        assert_eq!(low.other_reference_ranges()[0].meaning().value(), "normal");
        assert_eq!(low.is_abnormal(), Some(true), "2.0 is below 3.5");

        let ok = DvQuantity::new(4.0, "mmol/l")
            .unwrap()
            .with_normal_range(normal);
        assert_eq!(ok.is_abnormal(), Some(false), "4.0 is within 3.5–5.5");

        // With no normal range there is no answer, rather than "normal".
        // "Nobody recorded a range" and "the value is fine" are different
        // facts, and only one of them is safe to show.
        let unranged = DvQuantity::new(4.0, "mmol/l").unwrap();
        assert_eq!(unranged.is_abnormal(), None);
        assert_eq!(unranged.normal_range(), None);
        assert!(unranged.other_reference_ranges().is_empty());
    }

    /// Two `DV_SCALE` values compare only when they sit on the same scale.
    ///
    /// `is_strictly_comparable_to` could return a constant, its `==` could be
    /// `!=`, and `partial_cmp`'s `!` could be deleted — five mutants, none
    /// caught (`lib:A-09`). `D3.16` states the rule and its reason: an ordinal
    /// or scale value of 2 on a pain scale and 2 on a sedation scale are
    /// unrelated numbers that happen to be equal.
    ///
    /// Answering `true` unconditionally is the dangerous direction: it makes
    /// two unrelated assessments comparable, and 3-on-one-scale then sorts
    /// above 2-on-another as though the severities were the same measure.
    #[test]
    fn two_scale_values_compare_only_within_one_terminology() {
        let scale = |value: f64, terminology: &str, code: &str, rubric: &str| {
            DvScale::new(
                value,
                DvCodedText::new(rubric, CodePhrase::new(terminology, code).unwrap()).unwrap(),
            )
            .unwrap()
        };

        let pain_2 = scale(2.0, "local", "at0002", "moderate pain");
        let pain_3 = scale(3.0, "local", "at0003", "severe pain");
        let sedation_2 = scale(2.0, "SNOMED-CT", "72641008", "moderately sedated");

        // Same terminology: comparable, and ordered by value.
        assert!(pain_2.is_strictly_comparable_to(&pain_3));
        assert_eq!(pain_2.semantic_cmp(&pain_3), Some(Ordering::Less));
        assert_eq!(pain_2.semantic_cmp(&pain_2), Some(Ordering::Equal));

        // Different terminologies: not comparable, whatever the numbers say.
        assert!(
            !pain_2.is_strictly_comparable_to(&sedation_2),
            "two unrelated scales were called comparable"
        );
        assert_eq!(
            pain_2.semantic_cmp(&sedation_2),
            None,
            "a pain score was ordered against a sedation score"
        );
        assert_eq!(pain_3.semantic_cmp(&sedation_2), None);
        // And symmetrically.
        assert_eq!(sedation_2.semantic_cmp(&pain_2), None);
    }

    /// A quantity reports the units annotations it was given.
    ///
    /// `units_system` and `units_display_name` could each answer `None`, `""`
    /// or `"xyzzy"` for every quantity. They are what say *which* `mg` — the
    /// UCUM system and the label a clinician reads — and `D3.15` makes the
    /// units string the whole basis of comparability.
    #[test]
    fn a_quantity_reports_the_units_annotations_it_carries() {
        let plain = DvQuantity::new(5.0, "mg").unwrap();
        assert_eq!(plain.units(), "mg");
        // Absent, not empty: nothing was recorded.
        assert_eq!(plain.units_system(), None);
        assert_eq!(plain.units_display_name(), None);

        let annotated = DvQuantity::new(5.0, "mg")
            .unwrap()
            .with_units_system("http://unitsofmeasure.org")
            .with_units_display_name("milligram");
        assert_eq!(annotated.units(), "mg");
        assert_eq!(annotated.units_system(), Some("http://unitsofmeasure.org"));
        assert_eq!(annotated.units_display_name(), Some("milligram"));

        // The annotations do not change what the value *is*, so a quantity
        // with them still compares with one without (`D3.15` is about `units`).
        assert!(plain.is_strictly_comparable_to(&annotated));
    }

    /// A quantity's accuracy, and whether it is a percentage.
    ///
    /// `accuracy` could answer `Some(-1.0)` for every quantity and
    /// `accuracy_is_percent` could answer a constant (`lib:A-09`). The second
    /// is the one that matters: `±5` and `±5%` are different claims about the
    /// same measurement, and on a magnitude of 200 they differ by an order of
    /// magnitude. `lib:A-01` already found two of the accuracy *rules* wrong;
    /// reading the values back was equally unwatched.
    #[test]
    fn accuracy_says_how_much_and_whether_it_is_a_percentage() {
        let plain = DvQuantity::new(200.0, "mg").unwrap();
        // Absent, not zero: no accuracy was recorded, which is not a claim of
        // perfect accuracy.
        assert_eq!(plain.accuracy(), None);
        assert_eq!(plain.accuracy_is_percent(), None);

        let absolute = DvQuantity::new(200.0, "mg")
            .unwrap()
            .with_accuracy(5.0, false)
            .unwrap();
        assert_eq!(absolute.accuracy(), Some(5.0));
        assert_eq!(absolute.accuracy_is_percent(), Some(false));

        let percent = DvQuantity::new(200.0, "mg")
            .unwrap()
            .with_accuracy(5.0, true)
            .unwrap();
        assert_eq!(percent.accuracy(), Some(5.0));
        assert_eq!(
            percent.accuracy_is_percent(),
            Some(true),
            "±5% was reported as ±5"
        );

        // The two are different records even though the number matches, which
        // is the whole reason the flag exists.
        assert_ne!(absolute, percent);

        // Zero accuracy is allowed absolutely and refused as a percentage
        // (`Accuracy_is_percent_validity`), and a percentage outside 0–100 is
        // refused (`Accuracy_validity`).
        assert!(DvQuantity::new(1.0, "mg").unwrap().with_accuracy(0.0, false).is_ok());
        assert!(DvQuantity::new(1.0, "mg").unwrap().with_accuracy(0.0, true).is_err());
        assert!(DvQuantity::new(1.0, "mg").unwrap().with_accuracy(101.0, true).is_err());
        assert!(DvQuantity::new(1.0, "mg").unwrap().with_accuracy(-1.0, true).is_err());
        // Absolute accuracy may legitimately exceed 100 — it is not a percent.
        assert!(DvQuantity::new(1.0, "mg").unwrap().with_accuracy(101.0, false).is_ok());
    }

    /// A `DV_COUNT` orders by magnitude and carries its own status marker.
    ///
    /// `partial_cmp` could return `None` for every pair — making counts
    /// unorderable — and `magnitude_status` could answer `None` always. A count
    /// of `>50` is not a count of 50; that marker is how "at least fifty" is
    /// recorded (`db:P6.16` calls range a search kind, and this is the value
    /// such a search compares).
    #[test]
    fn counts_order_by_magnitude_and_keep_their_status_marker() {
        assert_eq!(DvCount::new(1).semantic_cmp(&DvCount::new(2)), Some(Ordering::Less));
        assert_eq!(DvCount::new(-1).semantic_cmp(&DvCount::new(0)), Some(Ordering::Less));
        assert_eq!(
            DvCount::new(5).semantic_cmp(&DvCount::new(5)),
            Some(Ordering::Equal)
        );

        assert_eq!(DvCount::new(50).magnitude(), 50);
        assert_eq!(DvCount::new(50).magnitude_status(), None);
        let at_least = DvCount::new(50).with_magnitude_status(MagnitudeStatus::GreaterThan);
        assert_eq!(at_least.magnitude_status(), Some(MagnitudeStatus::GreaterThan));
        assert_eq!(at_least.magnitude(), 50);
        assert_ne!(at_least, DvCount::new(50), "`>50` is not `50`");
    }

    /// Every `DV_PROPORTION` kind, and the parts and precision it reports.
    ///
    /// Two arms of `from_i32` could be deleted, `numerator` could answer `1.0`
    /// for every proportion, `precision` could answer `Some(0)`, and the
    /// precision guard's `<` could be `<=` (`lib:A-09`).
    ///
    /// The kind is the difference between `1/2` meaning a half, fifty percent,
    /// and one-in-two — openEHR encodes it as a bare integer, so a deleted arm
    /// silently reinterprets a stored value. `A-01` already found rules in this
    /// class taken from secondary sources and wrong.
    #[test]
    fn every_proportion_kind_and_part_is_reported_as_recorded() {
        let all = [
            (0, ProportionKind::Ratio, false),
            (1, ProportionKind::Unitary, false),
            (2, ProportionKind::Percent, false),
            (3, ProportionKind::Fraction, true),
            (4, ProportionKind::IntegerFraction, true),
        ];

        let mut codes = Vec::new();
        for (code, kind, integral) in all {
            assert_eq!(kind.as_i32(), code, "{kind:?} encoded wrongly");
            assert_eq!(
                ProportionKind::from_i32(code).expect("in range"),
                kind,
                "{code} decoded to the wrong kind"
            );
            assert_eq!(
                kind.requires_integral(),
                integral,
                "{kind:?} disagrees about whole numbers"
            );
            codes.push(code);
        }
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len(), "two kinds share an encoding");

        // Outside 0–4 is refused, not defaulted to `Ratio` (`Type_validity`).
        for bad in [-1, 5, 100] {
            assert!(ProportionKind::from_i32(bad).is_err(), "{bad} was accepted");
        }

        // The parts are reported as given, and the two must be told apart —
        // `numerator` answering a constant makes 1/4 and 3/4 the same record.
        let quarter = DvProportion::new(1.0, 4.0, ProportionKind::Ratio).unwrap();
        let three_quarters = DvProportion::new(3.0, 4.0, ProportionKind::Ratio).unwrap();
        assert!((quarter.numerator() - 1.0).abs() < f64::EPSILON);
        assert!((three_quarters.numerator() - 3.0).abs() < f64::EPSILON);
        assert!((quarter.denominator() - 4.0).abs() < f64::EPSILON);
        assert_eq!(quarter.kind(), ProportionKind::Ratio);
        assert_ne!(quarter, three_quarters);

        // Precision: absent unless recorded, so a constant `Some(0)` would
        // claim every proportion was recorded as whole numbers.
        assert_eq!(quarter.precision(), None);
        let two_dp = DvProportion::new(1.0, 3.0, ProportionKind::Ratio)
            .unwrap()
            .with_precision(2)
            .unwrap();
        assert_eq!(two_dp.precision(), Some(2));

        // `-1` is unlimited precision and is the lowest legal value; the guard
        // is `< -1`, so `-1` must be accepted and `-2` refused. Widening it to
        // `<=` rejects the one value openEHR uses to mean "as written".
        let unlimited = DvProportion::new(1.0, 3.0, ProportionKind::Ratio)
            .unwrap()
            .with_precision(DvQuantity::UNLIMITED_PRECISION)
            .unwrap();
        assert_eq!(unlimited.precision(), Some(-1));
        assert!(
            DvProportion::new(1.0, 3.0, ProportionKind::Ratio)
                .unwrap()
                .with_precision(-2)
                .is_err()
        );

        // Precision `0` claims whole numbers, so it is refused where the parts
        // are not integral (`Precision_validity`).
        assert!(
            DvProportion::new(1.5, 3.0, ProportionKind::Ratio)
                .unwrap()
                .with_precision(0)
                .is_err()
        );
        assert!(
            DvProportion::new(1.0, 3.0, ProportionKind::Ratio)
                .unwrap()
                .with_precision(0)
                .is_ok()
        );
    }
}
