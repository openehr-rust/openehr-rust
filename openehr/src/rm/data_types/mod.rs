//! The openEHR **Data Types Information Model**: every leaf value a clinical
//! record can hold.
//!
//! ```text
//! DATA_VALUE
//! ├── DV_BOOLEAN            ├── DV_ORDERED
//! ├── DV_STATE              │   ├── DV_ORDINAL
//! ├── DV_IDENTIFIER         │   ├── DV_SCALE
//! ├── DV_TEXT               │   └── DV_QUANTIFIED
//! │   └── DV_CODED_TEXT     │       └── DV_AMOUNT
//! ├── DV_PARAGRAPH          │           ├── DV_QUANTITY
//! ├── DV_INTERVAL<T>        │           ├── DV_COUNT
//! ├── DV_URI                │           └── DV_PROPORTION
//! │   └── DV_EHR_URI        ├── DV_TEMPORAL
//! ├── DV_ENCAPSULATED       │   ├── DV_DATE / DV_TIME
//! │   ├── DV_MULTIMEDIA     │   ├── DV_DATE_TIME
//! │   └── DV_PARSABLE       │   └── DV_DURATION
//! └── DV_TIME_SPECIFICATION
//!     ├── DV_PERIODIC_TIME_SPECIFICATION
//!     └── DV_GENERAL_TIME_SPECIFICATION
//! ```
//!
//! # `_type` is required here and nowhere else is it more important
//!
//! [`DataValue`] serializes with an internal `_type` tag and **requires one on
//! input**. openEHR's rule is that `_type` is mandatory wherever the declared
//! type is abstract, and `ELEMENT.value` is declared `DATA_VALUE`, which is as
//! abstract as it gets.
//!
//! Guessing from shape is not available as a fallback the way it is for
//! [`Text`]: `{"value": "5"}` is a syntactically valid `DV_TEXT`, `DV_URI`,
//! `DV_DATE`, and `DV_DURATION`, and picking one would turn a systolic pressure
//! into a string in a way nothing downstream could detect.
//!
//! ```
//! use openehr::rm::data_types::DataValue;
//!
//! let json = r#"{"_type":"DV_QUANTITY","magnitude":184.0,"units":"mm[Hg]"}"#;
//! let v: DataValue = serde_json::from_str(json).unwrap();
//! assert_eq!(v.type_name(), "DV_QUANTITY");
//!
//! // Without `_type`, the value is refused rather than guessed at.
//! assert!(serde_json::from_str::<DataValue>(r#"{"magnitude":184.0,"units":"mm[Hg]"}"#).is_err());
//! ```

pub mod basic;
pub mod date_time;
pub mod encapsulated;
pub mod quantity;
pub mod text;
pub mod time_specification;
pub mod uri;

pub use basic::{DvBoolean, DvIdentifier, DvState};
pub use date_time::{DvDate, DvDateTime, DvDuration, DvTime};
pub use encapsulated::{DvMultimedia, DvParsable, EncapsulatedAttrs, IntegrityCheck};
pub use quantity::{
    DvCount, DvOrdered, DvOrdinal, DvProportion, DvQuantity, DvScale, MagnitudeStatus,
    OrderedAttrs, ProportionKind, ReferenceRange,
};
pub use text::{
    CodePhrase, DvCodedText, DvParagraph, DvText, MappingMatch, TermMapping, Text, formatting,
};
pub use time_specification::{DvGeneralTimeSpecification, DvPeriodicTimeSpecification};
pub use uri::{DvEhrUri, DvUri};

use crate::base::Interval;
use core::cmp::Ordering;
use serde::{Deserialize, Serialize};

/// A range of data values — openEHR's `DV_INTERVAL<T>`.
///
/// The reference model makes `DV_INTERVAL` both an `INTERVAL` and a
/// `DATA_VALUE`, so it can be an `ELEMENT.value` in its own right: a blood
/// pressure target of "130–140 `mm[Hg]`" is one value, not two.
pub type DvInterval = Interval<DataValue>;

/// Any openEHR data value.
///
/// Comparison follows the reference model's `is_strictly_comparable_to`:
/// values of different classes never compare, and values of the same class
/// compare by that class's rule. See [`quantity`] for why that is a partial
/// order rather than a total one.
///
/// ```
/// use openehr::rm::data_types::{DataValue, DvQuantity, DvCount};
///
/// let q = DataValue::Quantity(DvQuantity::new(5.0, "mg").unwrap());
/// let c = DataValue::Count(DvCount::new(5));
/// assert_eq!(q.semantic_cmp(&c), None); // 5 mg is not 5 of anything
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
#[non_exhaustive]
pub enum DataValue {
    /// `DV_BOOLEAN`
    #[serde(rename = "DV_BOOLEAN")]
    Boolean(DvBoolean),
    /// `DV_STATE`
    #[serde(rename = "DV_STATE")]
    State(DvState),
    /// `DV_IDENTIFIER`
    #[serde(rename = "DV_IDENTIFIER")]
    Identifier(DvIdentifier),
    /// `DV_TEXT`
    #[serde(rename = "DV_TEXT")]
    Text(DvText),
    /// `DV_CODED_TEXT`
    #[serde(rename = "DV_CODED_TEXT")]
    CodedText(DvCodedText),
    /// `DV_PARAGRAPH` (deprecated by openEHR; readable for round-tripping)
    #[serde(rename = "DV_PARAGRAPH")]
    Paragraph(DvParagraph),
    /// `DV_ORDINAL`
    #[serde(rename = "DV_ORDINAL")]
    Ordinal(DvOrdinal),
    /// `DV_SCALE`
    #[serde(rename = "DV_SCALE")]
    Scale(DvScale),
    /// `DV_QUANTITY`
    #[serde(rename = "DV_QUANTITY")]
    Quantity(DvQuantity),
    /// `DV_COUNT`
    #[serde(rename = "DV_COUNT")]
    Count(DvCount),
    /// `DV_PROPORTION`
    #[serde(rename = "DV_PROPORTION")]
    Proportion(DvProportion),
    /// `DV_DATE`
    #[serde(rename = "DV_DATE")]
    Date(DvDate),
    /// `DV_TIME`
    #[serde(rename = "DV_TIME")]
    Time(DvTime),
    /// `DV_DATE_TIME`
    #[serde(rename = "DV_DATE_TIME")]
    DateTime(DvDateTime),
    /// `DV_DURATION`
    #[serde(rename = "DV_DURATION")]
    Duration(DvDuration),
    /// `DV_MULTIMEDIA`
    #[serde(rename = "DV_MULTIMEDIA")]
    Multimedia(DvMultimedia),
    /// `DV_PARSABLE`
    #[serde(rename = "DV_PARSABLE")]
    Parsable(DvParsable),
    /// `DV_URI`
    #[serde(rename = "DV_URI")]
    Uri(DvUri),
    /// `DV_EHR_URI`
    #[serde(rename = "DV_EHR_URI")]
    EhrUri(DvEhrUri),
    /// `DV_INTERVAL`
    #[serde(rename = "DV_INTERVAL")]
    Interval(Box<DvInterval>),
    /// `DV_PERIODIC_TIME_SPECIFICATION`
    #[serde(rename = "DV_PERIODIC_TIME_SPECIFICATION")]
    PeriodicTimeSpecification(DvPeriodicTimeSpecification),
    /// `DV_GENERAL_TIME_SPECIFICATION`
    #[serde(rename = "DV_GENERAL_TIME_SPECIFICATION")]
    GeneralTimeSpecification(DvGeneralTimeSpecification),
}

impl DataValue {
    /// The openEHR class name, as it appears in `_type`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "DV_BOOLEAN",
            Self::State(_) => "DV_STATE",
            Self::Identifier(_) => "DV_IDENTIFIER",
            Self::Text(_) => "DV_TEXT",
            Self::CodedText(_) => "DV_CODED_TEXT",
            Self::Paragraph(_) => "DV_PARAGRAPH",
            Self::Ordinal(_) => "DV_ORDINAL",
            Self::Scale(_) => "DV_SCALE",
            Self::Quantity(_) => "DV_QUANTITY",
            Self::Count(_) => "DV_COUNT",
            Self::Proportion(_) => "DV_PROPORTION",
            Self::Date(_) => "DV_DATE",
            Self::Time(_) => "DV_TIME",
            Self::DateTime(_) => "DV_DATE_TIME",
            Self::Duration(_) => "DV_DURATION",
            Self::Multimedia(_) => "DV_MULTIMEDIA",
            Self::Parsable(_) => "DV_PARSABLE",
            Self::Uri(_) => "DV_URI",
            Self::EhrUri(_) => "DV_EHR_URI",
            Self::Interval(_) => "DV_INTERVAL",
            Self::PeriodicTimeSpecification(_) => "DV_PERIODIC_TIME_SPECIFICATION",
            Self::GeneralTimeSpecification(_) => "DV_GENERAL_TIME_SPECIFICATION",
        }
    }

    /// The value rendered for a human, where the class has a sensible one.
    ///
    /// Returns `None` for the classes whose content is not text —
    /// `DV_MULTIMEDIA` above all. There is deliberately no fallback that
    /// stringifies bytes.
    ///
    /// ```
    /// use openehr::rm::data_types::{DataValue, DvQuantity, DvText};
    ///
    /// let q = DataValue::Quantity(DvQuantity::new(184.0, "mm[Hg]").unwrap());
    /// assert_eq!(q.display_value().as_deref(), Some("184 mm[Hg]"));
    ///
    /// let t = DataValue::Text(DvText::new("no chest pain").unwrap());
    /// assert_eq!(t.display_value().as_deref(), Some("no chest pain"));
    /// ```
    #[must_use]
    pub fn display_value(&self) -> Option<String> {
        Some(match self {
            Self::Boolean(v) => v.value().to_string(),
            Self::State(v) => v.value().value().to_owned(),
            Self::Text(v) => v.value().to_owned(),
            Self::CodedText(v) => v.value().to_owned(),
            Self::Ordinal(v) => v.symbol().value().to_owned(),
            Self::Scale(v) => v.symbol().value().to_owned(),
            Self::Quantity(v) => format!("{} {}", trim_float(v.magnitude()), v.units()),
            Self::Count(v) => v.magnitude().to_string(),
            Self::Proportion(v) => format!(
                "{}/{}",
                trim_float(v.numerator()),
                trim_float(v.denominator())
            ),
            Self::Date(v) => v.as_str().to_owned(),
            Self::Time(v) => v.as_str().to_owned(),
            Self::DateTime(v) => v.as_str().to_owned(),
            Self::Duration(v) => v.as_str().to_owned(),
            Self::Uri(v) => v.value().to_owned(),
            Self::EhrUri(v) => v.value().to_owned(),
            // Deliberately no rendering: an identifier is PHI and has its own
            // Display contract; multimedia is bytes; the rest are structures.
            Self::Identifier(_)
            | Self::Paragraph(_)
            | Self::Multimedia(_)
            | Self::Parsable(_)
            | Self::Interval(_)
            | Self::PeriodicTimeSpecification(_)
            | Self::GeneralTimeSpecification(_) => return None,
        })
    }

    /// Whether two values are of a class and shape that admits comparison.
    #[must_use]
    pub fn is_strictly_comparable_to(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Ordinal(a), Self::Ordinal(b)) => a.is_strictly_comparable_to(b),
            (Self::Scale(a), Self::Scale(b)) => a.is_strictly_comparable_to(b),
            (Self::Quantity(a), Self::Quantity(b)) => a.is_strictly_comparable_to(b),
            (Self::Proportion(a), Self::Proportion(b)) => a.is_strictly_comparable_to(b),
            (Self::Count(_), Self::Count(_))
            | (Self::Date(_), Self::Date(_))
            | (Self::Time(_), Self::Time(_))
            | (Self::DateTime(_), Self::DateTime(_))
            | (Self::Duration(_), Self::Duration(_)) => true,
            _ => false,
        }
    }
}

/// Formats a float without a trailing `.0`, so `184.0 mm[Hg]` reads as
/// `184 mm[Hg]` — which is how it was measured and how it is charted.
///
/// `Display` for `f64` already does exactly this, and that is the whole
/// function. It used to read:
///
/// ```text
/// if v.fract() == 0.0 && v.abs() < 1e15 { format!("{v:.0}") } else { format!("{v}") }
/// ```
///
/// Mutation testing survived every change to that comparison — `<` replaced by
/// `==`, `>`, and `<=` — which looked like an untested bound and was not. The
/// two branches produce the **same string for every finite `f64`**, checked
/// across 3,952 values spanning `1e-20` to `f64::MAX` in both signs (`W-18`).
///
/// Both halves of the guard were load-bearing against a version of the problem
/// that `Display` had already solved. `{:.0}` was there to drop the `.0`, which
/// `Display` does; and `v.abs() < 1e15` was there to stop `{:.0}` printing a
/// large float's *exact binary value* — `9876543209999998976` where `Display`
/// writes `9876543210000000000` — which is sixteen digits of noise implying
/// precision the value does not have.
///
/// So the guard protected a branch that was never needed. It is gone, and the
/// property it was aiming at is pinned by
/// `a_whole_number_loses_its_decimal_point_and_a_huge_one_gains_no_digits`.
fn trim_float(v: f64) -> String {
    format!("{v}")
}

/// `SemanticOrd` for every type an `INTERVAL<T>` is used over here (`D3.18c`).
///
/// Written out rather than blanket-implemented: a blanket
/// `impl<T: PartialOrd> SemanticOrd for T` collides with these under coherence,
/// and the explicit list is what stops a type with the `D3.18b` defect from
/// reaching `INTERVAL<T>` again without anyone deciding that it should.
macro_rules! semantic_ord_via_dv_ordered {
    ($($ty:ty),+ $(,)?) => {$(
        impl crate::base::SemanticOrd for $ty {
            fn semantic_cmp(&self, other: &Self) -> Option<Ordering> {
                <$ty as DvOrdered>::semantic_cmp(self, other)
            }
        }
    )+};
}

semantic_ord_via_dv_ordered!(
    DvDate,
    DvTime,
    DvDateTime,
    DvDuration,
    DvOrdinal,
    DvScale,
    DvQuantity,
    DvCount,
    DvProportion,
);

impl crate::base::SemanticOrd for DataValue {
    fn semantic_cmp(&self, other: &Self) -> Option<Ordering> {
        Self::semantic_cmp(self, other)
    }
}

impl DataValue {
    /// Compares two data values, or reports them incomparable.
    ///
    /// Not `PartialOrd::partial_cmp`, and deliberately (`D3.18b`): the derived
    /// `PartialEq` compares every field of the wrapped value, and this compares
    /// only what is ordered, so the two disagree on values that denote the same
    /// point. Values of **different** classes are never comparable (`D3.14`) —
    /// `5 mg` and `5` are not the same kind of thing, and neither is greater.
    #[must_use]
    pub fn semantic_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Self::Ordinal(a), Self::Ordinal(b)) => a.semantic_cmp(b),
            (Self::Scale(a), Self::Scale(b)) => a.semantic_cmp(b),
            (Self::Quantity(a), Self::Quantity(b)) => a.semantic_cmp(b),
            (Self::Count(a), Self::Count(b)) => a.semantic_cmp(b),
            (Self::Proportion(a), Self::Proportion(b)) => a.semantic_cmp(b),
            (Self::Date(a), Self::Date(b)) => a.semantic_cmp(b),
            (Self::Time(a), Self::Time(b)) => a.semantic_cmp(b),
            (Self::DateTime(a), Self::DateTime(b)) => a.semantic_cmp(b),
            (Self::Duration(a), Self::Duration(b)) => a.semantic_cmp(b),
            _ => None,
        }
    }
}

macro_rules! from_data_value {
    ($($ty:ty => $variant:ident),* $(,)?) => {
        $(
            impl From<$ty> for DataValue {
                fn from(v: $ty) -> Self {
                    Self::$variant(v)
                }
            }
        )*
    };
}

from_data_value! {
    DvBoolean => Boolean,
    DvState => State,
    DvIdentifier => Identifier,
    DvText => Text,
    DvCodedText => CodedText,
    DvParagraph => Paragraph,
    DvOrdinal => Ordinal,
    DvScale => Scale,
    DvQuantity => Quantity,
    DvCount => Count,
    DvProportion => Proportion,
    DvDate => Date,
    DvTime => Time,
    DvDateTime => DateTime,
    DvDuration => Duration,
    DvMultimedia => Multimedia,
    DvParsable => Parsable,
    DvUri => Uri,
    DvEhrUri => EhrUri,
    DvPeriodicTimeSpecification => PeriodicTimeSpecification,
    DvGeneralTimeSpecification => GeneralTimeSpecification,
}

impl From<Text> for DataValue {
    fn from(v: Text) -> Self {
        match v {
            Text::Plain(t) => Self::Text(t),
            Text::Coded(t) => Self::CodedText(t),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every arm of `DataValue::semantic_cmp` actually compares.
    ///
    /// **Failure mode, and it is the one `lib:A-28` records about a different
    /// match.** Deleting an arm here is **silent**: the pair falls to
    /// `_ => None`, which means "not comparable" — a legitimate answer for two
    /// values of different classes, and a *wrong* one for two dates. Nothing
    /// errors. Mutation testing deleted six of these nine arms — `Ordinal`,
    /// `Scale`, `Proportion`, `Date`, `Time`, `Duration` — and the whole suite
    /// stayed green (`W-18`).
    ///
    /// The consequence is not abstract. `DvOrdered::is_abnormal` asks
    /// `normal_range.contains(&DataValue::…)`, and `Interval::contains` reads
    /// "not comparable" as "not inside". So a date outside its recorded normal
    /// range would report as **not abnormal**, and a dashboard would render
    /// that as reassurance.
    ///
    /// **Adding a variant to `semantic_cmp` means adding a row here**, which is
    /// the same instruction `CLAUDE.md` gives for `path.rs`'s navigation table.
    #[test]
    fn every_comparable_variant_of_a_data_value_compares() {
        fn symbol(code: &str, rubric: &str) -> DvCodedText {
            DvCodedText::new(rubric, CodePhrase::new("local", code).unwrap()).unwrap()
        }

        // One row per arm. `lower` must order strictly below `upper`, and both
        // must be the same `DataValue` variant.
        let rows: Vec<(&str, DataValue, DataValue)> = vec![
            (
                "Ordinal",
                DataValue::Ordinal(DvOrdinal::new(1, symbol("at0002", "mild"))),
                DataValue::Ordinal(DvOrdinal::new(3, symbol("at0004", "severe"))),
            ),
            (
                "Scale",
                DataValue::Scale(DvScale::new(1.5, symbol("at0002", "mild")).unwrap()),
                DataValue::Scale(DvScale::new(3.5, symbol("at0004", "severe")).unwrap()),
            ),
            (
                "Quantity",
                DataValue::Quantity(DvQuantity::new(90.0, "mm[Hg]").unwrap()),
                DataValue::Quantity(DvQuantity::new(140.0, "mm[Hg]").unwrap()),
            ),
            (
                "Count",
                DataValue::Count(DvCount::new(1)),
                DataValue::Count(DvCount::new(2)),
            ),
            (
                "Proportion",
                DataValue::Proportion(DvProportion::new(1.0, 4.0, ProportionKind::Ratio).unwrap()),
                DataValue::Proportion(DvProportion::new(1.0, 2.0, ProportionKind::Ratio).unwrap()),
            ),
            (
                "Date",
                DataValue::Date(DvDate::new("2026-08-01").unwrap()),
                DataValue::Date(DvDate::new("2026-08-02").unwrap()),
            ),
            (
                "Time",
                DataValue::Time(DvTime::new("09:00:00").unwrap()),
                DataValue::Time(DvTime::new("10:00:00").unwrap()),
            ),
            (
                "DateTime",
                DataValue::DateTime(DvDateTime::new("2026-08-01T09:00:00Z").unwrap()),
                DataValue::DateTime(DvDateTime::new("2026-08-01T10:00:00Z").unwrap()),
            ),
            (
                "Duration",
                DataValue::Duration(DvDuration::new("PT1H").unwrap()),
                DataValue::Duration(DvDuration::new("PT2H").unwrap()),
            ),
        ];

        assert_eq!(
            rows.len(),
            9,
            "semantic_cmp has nine arms; a row was added or removed without the other"
        );

        for (what, lower, upper) in &rows {
            assert_eq!(
                lower.semantic_cmp(upper),
                Some(Ordering::Less),
                "{what}: the lower value did not order below the upper — the arm \
                 may have been deleted, which falls through to `None`"
            );
            assert_eq!(
                upper.semantic_cmp(lower),
                Some(Ordering::Greater),
                "{what}: comparison is not symmetric"
            );
            assert_eq!(
                lower.semantic_cmp(lower),
                Some(Ordering::Equal),
                "{what}: a value did not compare equal to itself"
            );
        }

        // And the `_ => None` arm is doing its own job: two different classes
        // are not comparable in either direction (`D3.14`), which is why a
        // deleted arm is invisible without this test.
        let (_, count, _) = &rows[3];
        let (_, date, _) = &rows[5];
        assert_eq!(count.semantic_cmp(date), None, "5 is not a date");
        assert_eq!(date.semantic_cmp(count), None, "nor the other way round");

        // `is_strictly_comparable_to` is a second match over the same nine
        // pairs and had the identical hole: every arm deletable in silence, and
        // the whole function replaceable with `false`. It is openEHR's own
        // predicate, and a caller that asks it before comparing would be told
        // two dates cannot be compared and would refuse to compare them.
        for (what, lower, upper) in &rows {
            assert!(
                lower.is_strictly_comparable_to(upper),
                "{what}: two values of one class must be strictly comparable"
            );
        }
        assert!(
            !count.is_strictly_comparable_to(date),
            "different classes are not strictly comparable"
        );
    }

    /// A whole number renders without `.0`, and a huge one gains no false digits.
    ///
    /// **This is the property the old implementation was aiming at**, asserted
    /// on the output rather than on the branch that used to produce it. The
    /// branch was dead — `Display` already does both of these — and mutation
    /// testing found it by surviving every change to its comparison (`W-18`).
    ///
    /// It stays a test because the property is what matters and is not this
    /// crate's to guarantee: it rests on `Display for f64` writing `184` rather
    /// than `184.0`, and on it choosing the shortest round-tripping form rather
    /// than the exact binary expansion. If either ever changed, a chart would
    /// start reading `184.0 mm[Hg]`, or `9876543209999998976`.
    #[test]
    fn a_whole_number_loses_its_decimal_point_and_a_huge_one_gains_no_digits() {
        assert_eq!(trim_float(184.0), "184");
        assert_eq!(trim_float(0.0), "0");
        assert_eq!(trim_float(-3.0), "-3");
        assert_eq!(trim_float(2.5), "2.5", "a real keeps its fraction");

        // The case the deleted `v.abs() < 1e15` guard existed for. `{:.0}`
        // renders this float's exact binary value, `9876543209999998976`;
        // shortest-round-trip renders what was meant.
        assert_eq!(trim_float(9.876_543_21e18), "9876543210000000000");
        assert_ne!(
            trim_float(9.876_543_21e18),
            format!("{:.0}", 9.876_543_21e18),
            "the exact-binary form leaked into a rendered value"
        );
    }

    #[test]
    fn every_variant_round_trips_through_its_type_tag() {
        let values = vec![
            DataValue::Boolean(DvBoolean::new(true)),
            DataValue::Text(DvText::new("x").unwrap()),
            DataValue::CodedText(
                DvCodedText::new("masked", CodePhrase::openehr("272").unwrap()).unwrap(),
            ),
            DataValue::Quantity(DvQuantity::new(1.5, "mg").unwrap()),
            DataValue::Count(DvCount::new(3)),
            DataValue::Proportion(DvProportion::new(1.0, 2.0, ProportionKind::Fraction).unwrap()),
            DataValue::Date(DvDate::new("2026-07-31").unwrap()),
            DataValue::Time(DvTime::new("09:15:00Z").unwrap()),
            DataValue::DateTime(DvDateTime::new("2026-07-31T09:15:00Z").unwrap()),
            DataValue::Duration(DvDuration::new("PT8H").unwrap()),
            DataValue::Uri(DvUri::new("https://example.org").unwrap()),
            DataValue::EhrUri(DvEhrUri::new("ehr://1/2").unwrap()),
            DataValue::Identifier(DvIdentifier::new("MRN-1").unwrap()),
        ];
        for v in values {
            let json = serde_json::to_string(&v).unwrap();
            assert!(
                json.contains(&format!(r#""_type":"{}""#, v.type_name())),
                "{json}"
            );
            let back: DataValue = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v, "round trip failed for {}", v.type_name());
        }
    }

    #[test]
    fn a_value_without_a_type_tag_is_refused() {
        // The specific failure guarded against: `{"value":"P1D"}` is a valid
        // DV_TEXT and a valid DV_DURATION, and a crate that picked one would
        // do so silently.
        assert!(serde_json::from_str::<DataValue>(r#"{"value":"P1D"}"#).is_err());
    }

    #[test]
    fn display_value_declines_where_there_is_nothing_safe_to_show() {
        let id = DataValue::Identifier(DvIdentifier::new("943-476-5919").unwrap());
        assert_eq!(id.display_value(), None);

        let m = DataValue::Multimedia(DvMultimedia::inline(
            CodePhrase::new("IANA_media-types", "image/png").unwrap(),
            vec![1, 2, 3],
        ));
        assert_eq!(m.display_value(), None);
    }

    #[test]
    fn quantities_do_not_gain_a_spurious_decimal_point() {
        let q = DataValue::Quantity(DvQuantity::new(184.0, "mm[Hg]").unwrap());
        assert_eq!(q.display_value().as_deref(), Some("184 mm[Hg]"));
        let f = DataValue::Quantity(DvQuantity::new(36.6, "Cel").unwrap());
        assert_eq!(f.display_value().as_deref(), Some("36.6 Cel"));
    }
}
