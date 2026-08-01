//! Time specifications: `DV_PERIODIC_TIME_SPECIFICATION` and
//! `DV_GENERAL_TIME_SPECIFICATION`.
//!
//! These are the "twice daily with meals" of the reference model. openEHR does
//! not define their syntax — it defers to HL7's `PIVL` and `GTS`, carried
//! inside a [`DvParsable`].
//!
//! # This crate does not interpret them
//!
//! It validates the wrapper — that the value is a `DV_PARSABLE` and that its
//! formalism is one of the two openEHR names — and stops. Computing the next
//! administration time from a `GTS` expression requires an HL7 `GTS` engine,
//! and a partial one would produce a dosing schedule that is right most of the
//! time. [`DvGeneralTimeSpecification::calendar_alignment`] and its siblings
//! therefore return `Err(Error::Unsupported)` rather than a guess.

use super::encapsulated::DvParsable;
use crate::error::{Error, ParseError};
use serde::{Deserialize, Serialize};

/// The formalism name openEHR gives for periodic specifications.
pub const FORMALISM_PIVL: &str = "HL7:PIVL";
/// The formalism name openEHR gives for general specifications.
pub const FORMALISM_GTS: &str = "HL7:GTS";
/// The formalism name openEHR gives for event-related specifications.
pub const FORMALISM_EIVL: &str = "HL7:EIVL";

/// A repeating schedule, expressed as an HL7 `PIVL`.
///
/// ```
/// use openehr::rm::data_types::{DvParsable, DvPeriodicTimeSpecification};
///
/// let twice_daily = DvPeriodicTimeSpecification::new(
///     DvParsable::new("[200001010800;200001010900]/(12 h)", "HL7:PIVL").unwrap(),
/// ).unwrap();
/// assert_eq!(twice_daily.value().formalism(), "HL7:PIVL");
///
/// // A GTS expression is not a PIVL, and mislabelling it means whatever reads
/// // it next applies the wrong grammar.
/// let gts = DvParsable::new("[200001010800]", "HL7:GTS").unwrap();
/// assert!(DvPeriodicTimeSpecification::new(gts).is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvPeriodicTimeSpecification {
    value: DvParsable,
}

impl DvPeriodicTimeSpecification {
    /// Builds a periodic time specification.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the formalism is not [`FORMALISM_PIVL`]
    /// (`Value_valid`).
    pub fn new(value: DvParsable) -> Result<Self, ParseError> {
        if value.formalism() != FORMALISM_PIVL {
            return Err(ParseError::invariant(
                "DV_PERIODIC_TIME_SPECIFICATION",
                "Value_valid",
            ));
        }
        Ok(Self { value })
    }

    /// The underlying expression.
    #[must_use]
    pub fn value(&self) -> &DvParsable {
        &self.value
    }

    /// The repetition period.
    ///
    /// # Errors
    ///
    /// Always [`Error::Unsupported`]. Extracting the period means parsing
    /// `PIVL`, which this crate does not do; see the module header.
    pub fn period(&self) -> Result<super::DvDuration, Error> {
        Err(Error::Unsupported {
            what: "DV_PERIODIC_TIME_SPECIFICATION.period (HL7 PIVL parsing)",
            spec_ref: "spec/01-scope.md S1.8",
        })
    }
}

/// A schedule expressed as an HL7 `GTS` or `EIVL`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvGeneralTimeSpecification {
    value: DvParsable,
}

impl DvGeneralTimeSpecification {
    /// Builds a general time specification.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the formalism is neither [`FORMALISM_GTS`] nor
    /// [`FORMALISM_EIVL`].
    pub fn new(value: DvParsable) -> Result<Self, ParseError> {
        if value.formalism() != FORMALISM_GTS && value.formalism() != FORMALISM_EIVL {
            return Err(ParseError::invariant(
                "DV_GENERAL_TIME_SPECIFICATION",
                "Value_valid",
            ));
        }
        Ok(Self { value })
    }

    /// The underlying expression.
    #[must_use]
    pub fn value(&self) -> &DvParsable {
        &self.value
    }

    /// Which calendar unit the schedule aligns to.
    ///
    /// # Errors
    ///
    /// Always [`Error::Unsupported`]; see the module header.
    pub fn calendar_alignment(&self) -> Result<String, Error> {
        Err(Error::Unsupported {
            what: "DV_GENERAL_TIME_SPECIFICATION.calendar_alignment (HL7 GTS parsing)",
            spec_ref: "spec/01-scope.md S1.8",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formalism_is_checked_against_the_class() {
        let pivl = DvParsable::new("[x]/(12 h)", FORMALISM_PIVL).unwrap();
        let gts = DvParsable::new("[x]", FORMALISM_GTS).unwrap();
        assert!(DvPeriodicTimeSpecification::new(pivl.clone()).is_ok());
        assert!(DvPeriodicTimeSpecification::new(gts.clone()).is_err());
        assert!(DvGeneralTimeSpecification::new(gts).is_ok());
        assert!(DvGeneralTimeSpecification::new(pivl).is_err());
    }

    #[test]
    fn unimplemented_accessors_refuse_rather_than_guess() {
        let spec = DvPeriodicTimeSpecification::new(
            DvParsable::new("[x]/(12 h)", FORMALISM_PIVL).unwrap(),
        )
        .unwrap();
        let err = spec.period().unwrap_err();
        assert!(matches!(err, Error::Unsupported { .. }));
        // And the refusal names where the exclusion is recorded, so it is
        // traceable rather than merely unimplemented.
        assert!(err.to_string().contains("spec/01-scope.md"));
    }
}
