//! `RM_OVERLAY`: visibility and aliasing for reference-model attributes
//! outside the archetyped structure.
//!
//! An archetype's `definition` constrains some of a class's attributes and
//! says nothing about the rest, which remain simply part of the underlying
//! Reference Model. `RM_OVERLAY` is where an archetype author can say
//! something about those *other* attributes anyway — hide one from an
//! authoring tool, or give it an alias — without adding a constraint on it.
//! `org.openehr.am.aom2.rm_overlay.adoc`'s own description: "Container object
//! for archetype statements relating to RM attributes, which may be directly
//! on objects constrained within the archetype, or at deeper non-constrained
//! RM paths from an object or the root."
//!
//! # Not read by `am::validate`
//!
//! Hiding an attribute from a tool, or aliasing it, does not change whether
//! an instance conforms — `org.openehr.am.aom2.rm_overlay.adoc` names no
//! invariant that would connect the two, and `am::validate::validate_against_archetype`
//! does not read [`Archetype::rm_overlay`] at all. This is authoring-tool
//! metadata, carried on [`Archetype`](crate::am::Archetype) so it survives a
//! round trip (`K15.3`), and nothing more.

use crate::base::TerminologyCode;
use crate::error::ParseError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Whether a tool should show or hide a model element: `VISIBILITY_TYPE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisibilityType {
    /// Show the element to which this marker is attached.
    Show,
    /// Hide the element to which this marker is attached.
    Hide,
}

/// Visibility and aliasing for one RM attribute: `RM_ATTRIBUTE_VISIBILITY`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmAttributeVisibility {
    visibility: Option<VisibilityType>,
    alias: Option<TerminologyCode>,
}

impl RmAttributeVisibility {
    /// Builds a visibility statement for one RM attribute.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `alias` is present with no `visibility`
    /// (AOM2's `Inv_alias_validity`: `alias /= Void implies visibility /=
    /// Void`) — an alias for an attribute the statement does not also say
    /// whether to show or hide names something without saying anything a
    /// tool can act on.
    pub fn new(
        visibility: Option<VisibilityType>,
        alias: Option<TerminologyCode>,
    ) -> Result<Self, ParseError> {
        if alias.is_some() && visibility.is_none() {
            return Err(ParseError::invariant(
                "RM_ATTRIBUTE_VISIBILITY",
                "Inv_alias_validity",
            ));
        }
        Ok(Self { visibility, alias })
    }

    /// Whether a tool should show or hide the attribute, if stated.
    #[must_use]
    pub const fn visibility(&self) -> Option<VisibilityType> {
        self.visibility
    }

    /// The attribute's alias, if it has one.
    #[must_use]
    pub const fn alias(&self) -> Option<&TerminologyCode> {
        self.alias.as_ref()
    }
}

/// Container for archetype statements about RM attributes: `RM_OVERLAY`.
///
/// [`Archetype::with_rm_overlay`](crate::am::Archetype::with_rm_overlay)
/// attaches one; `Default` is the empty overlay `with_visibility` builds up
/// from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RmOverlay {
    /// Path to an RM attribute → its visibility statement. AOM2's own key
    /// description: "typically formed from a path to an archetyped node
    /// concatenated with a further pure RM attribute path; may also refer to
    /// a non-archetyped attribute" — carried as written, and not checked
    /// against [`crate::path::Node`], since resolving it needs a real
    /// instance tree an overlay is not attached to.
    rm_visibility: BTreeMap<String, RmAttributeVisibility>,
}

impl RmOverlay {
    /// Records a visibility statement for the RM attribute at `path`,
    /// replacing any earlier statement for the same path.
    #[must_use]
    pub fn with_visibility(
        mut self,
        path: impl Into<String>,
        visibility: RmAttributeVisibility,
    ) -> Self {
        self.rm_visibility.insert(path.into(), visibility);
        self
    }

    /// The visibility statement for `path`, if one is recorded.
    #[must_use]
    pub fn visibility(&self, path: &str) -> Option<&RmAttributeVisibility> {
        self.rm_visibility.get(path)
    }

    /// Every path this overlay carries a statement for.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.rm_visibility.keys().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_alias_with_no_stated_visibility_is_refused() {
        let err = RmAttributeVisibility::new(
            None,
            Some(TerminologyCode::new("local", "at0099")),
        )
        .unwrap_err();
        assert_eq!(err.reason, "Inv_alias_validity");

        // The same alias is fine once a visibility is stated alongside it.
        assert!(
            RmAttributeVisibility::new(
                Some(VisibilityType::Show),
                Some(TerminologyCode::new("local", "at0099")),
            )
            .is_ok()
        );
    }

    #[test]
    fn a_visibility_with_no_alias_needs_no_alias() {
        assert!(RmAttributeVisibility::new(Some(VisibilityType::Hide), None).is_ok());
    }

    #[test]
    fn an_overlay_records_one_statement_per_path() {
        let overlay = RmOverlay::default()
            .with_visibility(
                "data/events/data/items[at0099]",
                RmAttributeVisibility::new(Some(VisibilityType::Hide), None).unwrap(),
            )
            .with_visibility(
                "protocol",
                RmAttributeVisibility::new(Some(VisibilityType::Show), None).unwrap(),
            );
        assert_eq!(
            overlay
                .visibility("data/events/data/items[at0099]")
                .unwrap()
                .visibility(),
            Some(VisibilityType::Hide)
        );
        assert_eq!(overlay.paths().count(), 2);
        assert!(overlay.visibility("no/such/path").is_none());
    }

    #[test]
    fn an_overlay_round_trips_through_canonical_json() {
        let overlay = RmOverlay::default().with_visibility(
            "protocol",
            RmAttributeVisibility::new(
                Some(VisibilityType::Hide),
                Some(TerminologyCode::new("local", "at0099")),
            )
            .unwrap(),
        );
        let json = serde_json::to_value(&overlay).unwrap();
        let back: RmOverlay = serde_json::from_value(json).unwrap();
        assert_eq!(back, overlay);
    }
}
