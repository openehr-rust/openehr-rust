//! `ARCHETYPE_TERMINOLOGY`: what the codes in an archetype mean.
//!
//! An archetype's constraint tree is written in codes — `at0004`, `id1.1`,
//! `ac0001` — and this is where each one acquires a rubric, a description, and
//! a language. It is also where AOM2 puts value sets and external bindings, so
//! it is the object that decides whether a `C_TERMINOLOGY_CODE` constraint
//! means anything at all.
//!
//! # Internal codes are checked here; external ones are not
//!
//! `at`- and `ac`-codes belong to the archetype, so this crate has everything
//! it needs to check them, and it does (`VATDF`, `VACDF`). A binding to SNOMED
//! CT or LOINC names a terminology this crate cannot reach, and `S1.10` still
//! governs: those are carried, reported as **unchecked**, and never reported as
//! satisfied (`K15.22`).

use crate::error::ParseError;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// What one code means in one language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermDefinition {
    text: String,
    description: Option<String>,
}

impl TermDefinition {
    /// Builds a term definition.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the rubric is empty. A code whose text is
    /// blank is a node a clinician cannot be shown and a form cannot label.
    pub fn new(
        text: impl Into<String>,
        description: Option<String>,
    ) -> Result<Self, ParseError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(ParseError::invariant("ARCHETYPE_TERM", "empty text"));
        }
        Ok(Self { text, description })
    }

    /// The rubric.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The longer description, if the author wrote one.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// The terminology section of an archetype.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchetypeTerminology {
    original_language: String,
    /// language → code → definition.
    term_definitions: BTreeMap<String, BTreeMap<String, TermDefinition>>,
    /// `ac`-code → the `at`-codes it admits.
    value_sets: BTreeMap<String, BTreeSet<String>>,
    /// terminology id → code → external code string.
    term_bindings: BTreeMap<String, BTreeMap<String, String>>,
}

impl ArchetypeTerminology {
    /// Builds a terminology with one language and its definitions.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the language tag is empty, or if there are no
    /// definitions for it. An archetype whose original language defines nothing
    /// cannot name a single one of its own nodes.
    pub fn new(
        original_language: impl Into<String>,
        definitions: BTreeMap<String, TermDefinition>,
    ) -> Result<Self, ParseError> {
        let original_language = original_language.into();
        if original_language.trim().is_empty() {
            return Err(ParseError::invariant(
                "ARCHETYPE_TERMINOLOGY",
                "empty original_language",
            ));
        }
        if definitions.is_empty() {
            return Err(ParseError::invariant(
                "ARCHETYPE_TERMINOLOGY",
                "no term definitions for the original language",
            ));
        }
        let mut term_definitions = BTreeMap::new();
        term_definitions.insert(original_language.clone(), definitions);
        Ok(Self {
            original_language,
            term_definitions,
            value_sets: BTreeMap::new(),
            term_bindings: BTreeMap::new(),
        })
    }

    /// Adds a value set: the `at`-codes an `ac`-code admits.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the value set is empty, or if any member is
    /// undefined in the original language (`VACDF`). A value set naming a code
    /// nobody defined offers a chooser with a blank option.
    pub fn with_value_set(
        mut self,
        ac_code: impl Into<String>,
        members: BTreeSet<String>,
    ) -> Result<Self, ParseError> {
        let ac_code = ac_code.into();
        if members.is_empty() {
            return Err(ParseError::invariant("ARCHETYPE_TERMINOLOGY", "VACDF"));
        }
        for member in &members {
            if !self.defines(member) {
                return Err(ParseError::new("ARCHETYPE_TERMINOLOGY", "VATDF", member));
            }
        }
        self.value_sets.insert(ac_code, members);
        Ok(self)
    }

    /// Adds an external binding, which this crate carries and never checks
    /// (`S1.10`, `K15.22`).
    #[must_use]
    pub fn with_binding(
        mut self,
        terminology: impl Into<String>,
        code: impl Into<String>,
        external: impl Into<String>,
    ) -> Self {
        self.term_bindings
            .entry(terminology.into())
            .or_default()
            .insert(code.into(), external.into());
        self
    }

    /// The language the archetype was authored in.
    #[must_use]
    pub fn original_language(&self) -> &str {
        &self.original_language
    }

    /// Whether a code is defined in the original language.
    #[must_use]
    pub fn defines(&self, code: &str) -> bool {
        self.term_definitions
            .get(&self.original_language)
            .is_some_and(|definitions| definitions.contains_key(code))
    }

    /// The definition of a code in the original language.
    #[must_use]
    pub fn definition(&self, code: &str) -> Option<&TermDefinition> {
        self.term_definitions
            .get(&self.original_language)?
            .get(code)
    }

    /// The `at`-codes an `ac`-code admits, if it names a value set here.
    #[must_use]
    pub fn value_set(&self, ac_code: &str) -> Option<&BTreeSet<String>> {
        self.value_sets.get(ac_code)
    }

    /// Every code defined in the original language.
    pub fn codes(&self) -> impl Iterator<Item = &str> {
        self.term_definitions
            .get(&self.original_language)
            .into_iter()
            .flat_map(|definitions| definitions.keys().map(String::as_str))
    }

    /// The external bindings, which are carried and unchecked.
    #[must_use]
    pub const fn bindings(&self) -> &BTreeMap<String, BTreeMap<String, String>> {
        &self.term_bindings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definitions(codes: &[(&str, &str)]) -> BTreeMap<String, TermDefinition> {
        codes
            .iter()
            .map(|(code, text)| {
                (
                    (*code).to_owned(),
                    TermDefinition::new(*text, None).unwrap(),
                )
            })
            .collect()
    }

    #[test]
    fn a_value_set_member_nobody_defined_is_refused() {
        let terminology =
            ArchetypeTerminology::new("en", definitions(&[("at0001", "Systolic")])).unwrap();
        let err = terminology
            .clone()
            .with_value_set("ac0001", BTreeSet::from(["at0009".to_owned()]))
            .unwrap_err();
        assert_eq!(err.reason, "VATDF");

        assert!(
            terminology
                .with_value_set("ac0001", BTreeSet::from(["at0001".to_owned()]))
                .is_ok()
        );
    }

    #[test]
    fn an_empty_rubric_is_refused() {
        assert!(TermDefinition::new("   ", None).is_err());
    }

    #[test]
    fn external_bindings_are_carried_and_not_checked() {
        let terminology = ArchetypeTerminology::new("en", definitions(&[("at0001", "Systolic")]))
            .unwrap()
            .with_binding("SNOMED-CT", "at0001", "271649006");
        // Carried…
        assert_eq!(terminology.bindings()["SNOMED-CT"]["at0001"], "271649006");
        // …and no accessor reports it as validated, because nothing validated it.
        assert!(terminology.defines("at0001"));
    }
}
