//! Temporal data values: `DV_DATE`, `DV_TIME`, `DV_DATE_TIME`, `DV_DURATION`.
//!
//! Each wraps the corresponding parsed type from [`crate::base::iso8601`], so
//! the partial-precision and partial-order semantics described there apply
//! unchanged. What this module adds is the reference model's framing: these are
//! `DV_ORDERED` descendants, so they carry reference ranges, and they
//! serialize as `{"value": "…"}` rather than as bare strings.

use super::DataValue;
use super::quantity::{DvOrdered, MagnitudeStatus, OrderedAttrs};
use crate::base::iso8601;
use crate::error::ParseError;
use core::cmp::Ordering;
use core::fmt;
use serde::{Deserialize, Serialize};

macro_rules! temporal {
    (
        $(#[$attr:meta])*
        $ty:ident, $inner:ty, $class:literal, $variant:ident
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
        pub struct $ty {
            value: $inner,
            #[serde(skip_serializing_if = "Option::is_none", default)]
            magnitude_status: Option<MagnitudeStatus>,
            #[serde(flatten)]
            ordered: OrderedAttrs,
        }

        impl $ty {
            /// Builds the value from its ISO 8601 text.
            ///
            /// # Errors
            ///
            /// Returns [`ParseError`] if the text is not a valid value of this
            /// type. See [`crate::base::iso8601`] for the accepted forms.
            pub fn new(value: &str) -> Result<Self, ParseError> {
                Ok(Self {
                    value: value.parse()?,
                    magnitude_status: None,
                    ordered: OrderedAttrs::default(),
                })
            }

            /// The parsed value.
            #[must_use]
            pub fn value(&self) -> &$inner {
                &self.value
            }

            /// The value's original text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                self.value.as_str()
            }

            /// Whether the value is exact, bounded, or approximate.
            ///
            /// `~` on a date is how openEHR records "around 2019" without
            /// pretending to a precision the record does not have.
            #[must_use]
            pub fn magnitude_status(&self) -> Option<MagnitudeStatus> {
                self.magnitude_status
            }

            /// Records that the value is a bound or an approximation.
            #[must_use]
            pub fn with_magnitude_status(mut self, status: MagnitudeStatus) -> Self {
                self.magnitude_status = Some(status);
                self
            }
        }

        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.value, f)
            }
        }

        impl core::str::FromStr for $ty {
            type Err = ParseError;

            /// # Errors
            ///
            /// See the type's `new`.
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s)
            }
        }

        impl DvOrdered for $ty {
            fn ordered_attrs(&self) -> &OrderedAttrs {
                &self.ordered
            }

            /// Inherits the partial order of the underlying ISO 8601 type: two
            /// values of different precision whose known components agree are
            /// **not** ordered.
            ///
            /// This is a named method and not `PartialOrd`, for the reason
            /// `D3.18a` gives about the inner type and `D3.18b` extends to
            /// this one: the derived `PartialEq` is lexical *and* covers the
            /// `OrderedAttrs`, while this compares only the instant. The two
            /// disagree by design, and Rust's `PartialOrd` contract does not
            /// permit that.
            fn semantic_cmp(&self, other: &Self) -> Option<Ordering> {
                self.value.semantic_cmp(&other.value)
            }

            fn is_abnormal(&self) -> Option<bool> {
                let range = self.ordered.normal_range()?;
                Some(!range.contains(&DataValue::$variant(self.clone())))
            }
        }
    };
}

temporal! {
    /// A date, to year, month, or day precision.
    ///
    /// ```
    /// use openehr::rm::data_types::{DvDate, DvOrdered as _};
    ///
    /// let dob = DvDate::new("1953-11-02").unwrap();
    /// assert_eq!(dob.as_str(), "1953-11-02");
    ///
    /// // A date known only to the month stays that way.
    /// let partial = DvDate::new("1953-11").unwrap();
    /// assert_eq!(partial.as_str(), "1953-11");
    /// assert_eq!(partial.semantic_cmp(&dob), None);
    /// ```
    DvDate, iso8601::Date, "DV_DATE", Date
}

temporal! {
    /// A time of day, with optional UTC offset.
    ///
    /// ```
    /// use openehr::rm::data_types::DvTime;
    ///
    /// let t = DvTime::new("09:15:00Z").unwrap();
    /// assert_eq!(t.value().hour(), 9);
    /// ```
    DvTime, iso8601::Time, "DV_TIME", Time
}

temporal! {
    /// A date and time.
    ///
    /// ```
    /// use openehr::rm::data_types::DvDateTime;
    ///
    /// let t = DvDateTime::new("2026-07-31T09:15:00Z").unwrap();
    /// assert_eq!(t.value().date().year(), 2026);
    /// assert!(t.value().offset().is_some());
    /// ```
    DvDateTime, iso8601::DateTime, "DV_DATE_TIME", DateTime
}

temporal! {
    /// A length of time.
    ///
    /// ```
    /// use openehr::rm::data_types::DvDuration;
    ///
    /// let d = DvDuration::new("PT8H").unwrap();
    /// assert_eq!(d.value().hours(), 8);
    ///
    /// // openEHR permits negative durations; plain ISO 8601 does not.
    /// assert!(DvDuration::new("-PT30M").unwrap().value().is_negative());
    /// ```
    DvDuration, iso8601::Duration, "DV_DURATION", Duration
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporals_serialize_as_a_value_object_not_a_bare_string() {
        let d = DvDate::new("2026-07-31").unwrap();
        assert_eq!(
            serde_json::to_string(&d).unwrap(),
            r#"{"value":"2026-07-31"}"#
        );
        let back: DvDate = serde_json::from_str(r#"{"value":"2026-07-31"}"#).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn precision_survives_a_round_trip() {
        for text in ["2026", "2026-07", "2026-07-31"] {
            let d = DvDate::new(text).unwrap();
            let json = serde_json::to_string(&d).unwrap();
            let back: DvDate = serde_json::from_str(&json).unwrap();
            assert_eq!(back.as_str(), text);
        }
    }

    #[test]
    fn an_invalid_date_is_refused_at_construction() {
        assert!(DvDate::new("2026-02-30").is_err());
        assert!(DvDateTime::new("2026-07-31T25:00:00").is_err());
        assert!(DvDuration::new("8 hours").is_err());
    }
}
