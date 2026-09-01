//! `ARCHETYPE_HRID`: the multi-axial, human-readable identifier ADL 1.4 and
//! ADL 2 archetype and template headers actually use.
//!
//! # Not `ArchetypeId`
//!
//! [`crate::base::ArchetypeId`] models the classic
//! `rm_originator-rm_name-rm_entity.domain_concept.version` form and is what
//! `ARCHETYPED.archetype_id` carries on Reference Model data (`I2.14`). This
//! module models a different AOM2 class,
//! `org.openehr.am.aom2.archetype_hrid.adoc` — confirmed as the token both
//! ADL grammars actually cite for their header line, not `ARCHETYPE_ID`
//! (`openEHR/adl-antlr`, `adl14.g4`: `archetype: SYM_ARCHETYPE meta_data?
//! ARCHETYPE_HRID ...`; `adl2.g4`: `authored_archetype: SYM_ARCHETYPE
//! meta_data? archetypeHrid ...` where `archetypeHrid: ARCHETYPE_HRID;`).
//! The lexer rule itself, from `base_lexer.g4`:
//!
//! ```text
//! ARCHETYPE_HRID       : ARCHETYPE_HRID_ROOT '.v' ARCHETYPE_VERSION_ID ;
//! fragment ARCHETYPE_HRID_ROOT  : (NAMESPACE '::')? IDENTIFIER '-' IDENTIFIER '-' IDENTIFIER '.' LABEL ;
//! fragment ARCHETYPE_VERSION_ID : DIGIT+ ('.' DIGIT+ ('.' DIGIT+ (('-rc'|'-alpha'|'-beta') ('.' DIGIT+)?)?)?)? ;
//! ```
//!
//! `ArchetypeId::from_str` rejects two things this grammar allows: a
//! `namespace::` prefix, and a prerelease suffix (`-rc.4`, `-alpha`, `-beta`)
//! on the version. [`crate::am::adl14::parse_header`] and
//! [`crate::am::adl2::parse_header`] used `ArchetypeId` for the header's own
//! identifier before this type existed — narrower than the grammar their own
//! error messages already named — and have been corrected to use
//! [`ArchetypeHrid`] here instead (`spec/audit.md` **A-49**).
//!
//! # Not touched by this: the `specialize` reference
//!
//! ADL 1.4's `specialization_section` names its parent with `ARCHETYPE_REF`,
//! a different, narrower lexer token — no namespace prefix, no prerelease
//! suffix, but (unlike `ARCHETYPE_HRID`) an unbounded chain of `.DIGIT+`
//! version segments. ADL 2's own `specialize_section` allows *either*
//! `ARCHETYPE_HRID` or `ARCHETYPE_REF` (`cadl2.g4`: `archetype_ref:
//! ARCHETYPE_HRID | ARCHETYPE_REF`). Reconciling `ArchetypeId` — which
//! remains what `Adl14Header.specializes`/`Adl2Header.specializes` carry —
//! against either of those is a separate piece of work, not attempted here;
//! see `spec/audit.md` **A-49**'s residual for exactly what is left open and
//! why it was not folded into this pass.
//!
//! # A departure this type shares with `ArchetypeId`
//!
//! AOM2's `ARCHETYPE_HRID` class declares `release_version` a 3-part number
//! and states its own invariant, `Inv_release_version_validity`, to require
//! it. The ADL lexical grammar above does not: `ARCHETYPE_VERSION_ID`
//! accepts one, two, or three numeric parts, each subsequent one optional.
//! This type follows the grammar actually used to write archetype text, the
//! same choice `I2.15` already made for `ArchetypeId` and for the same
//! reason — real archetype identifiers with fewer parts exist, and refusing
//! them is the worse failure. [`ArchetypeHrid::minor_version`] and
//! [`ArchetypeHrid::patch_version`] are therefore `Option<u32>`, not the
//! always-present values AOM2's class model assumes.
//!
//! # A maturity state text cannot express
//!
//! [`VersionStatus`] has five members, matching
//! `org.openehr.base.base_types.version_status.adoc`. Only four are
//! reachable by [`ArchetypeHrid::from_str`]: `Build`'s own rendering is
//! `N.M.P+B`, and `ARCHETYPE_VERSION_ID`'s grammar has no `+` branch at all —
//! a `+`-suffixed version is not `ARCHETYPE_HRID`-shaped text, by any
//! spelling, in this grammar. `Build` exists on the enum for fidelity to the
//! five-member class it models; the parser will never produce it.
//!
//! # Also not modelled: percent-encoded label characters
//!
//! `LABEL`'s own grammar admits `URI_PCT_ENCODED` (`'%' HEX_DIGIT HEX_DIGIT`)
//! alongside letters, digits, `_`, and `-`. This parser does not — a
//! percent-encoded `concept_id` or `namespace` label is refused, not
//! accepted. That is a narrower acceptance than the grammar allows, never a
//! wider one: nothing this parser accepts is ill-formed, and the gap is
//! declared here rather than silent.

use crate::error::ParseError;
use core::fmt;
use core::str::FromStr;
use serde::{Deserialize, Serialize};

/// `VERSION_STATUS`: the maturity of one version of a resource
/// (`org.openehr.base.base_types.version_status.adoc`).
///
/// See the module documentation for why [`Self::Build`] can never come from
/// [`ArchetypeHrid::from_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionStatus {
    /// Unstable: an unknown size of change since the base version. Rendered
    /// `N.M.P-alpha.B`.
    Alpha,
    /// Beta: a reducing but still unknown size of change. Rendered
    /// `N.M.P-beta.B`.
    Beta,
    /// Release candidate: patch-level changes only. Rendered `N.M.P-rc.B`.
    ReleaseCandidate,
    /// Released: the definitive base version. Rendered `N.M.P`.
    Released,
    /// A build of the current base release. Rendered `N.M.P+B`. Never
    /// produced by [`ArchetypeHrid::from_str`] — see the module
    /// documentation.
    Build,
}

/// AOM2's `ARCHETYPE_HRID`: the multi-axial, human-readable identifier for
/// an archetype or template. See the module documentation for how this
/// differs from [`crate::base::ArchetypeId`].
///
/// ```
/// use openehr::am::ArchetypeHrid;
///
/// let id: ArchetypeHrid = "openEHR-EHR-OBSERVATION.blood_pressure.v1.8.2-rc.4"
///     .parse()
///     .unwrap();
/// assert_eq!(id.rm_class(), "OBSERVATION");
/// assert_eq!(id.major_version(), 1);
/// assert_eq!(id.version_id(), "1.8.2-rc.4");
/// assert_eq!(id.semantic_id(), "openEHR-EHR-OBSERVATION.blood_pressure.v1");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchetypeHrid {
    namespace: Option<String>,
    rm_publisher: String,
    rm_package: String,
    rm_class: String,
    concept_id: String,
    release_version: String,
    version_status: VersionStatus,
    build_count: String,
}

impl ArchetypeHrid {
    /// The reverse-domain-name namespace, if the identifier carries one.
    #[must_use]
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// The Reference Model publisher, usually `openEHR`.
    #[must_use]
    pub fn rm_publisher(&self) -> &str {
        &self.rm_publisher
    }

    /// The Reference Model package, such as `EHR`.
    #[must_use]
    pub fn rm_package(&self) -> &str {
        &self.rm_package
    }

    /// The RM class this archetype constrains, such as `OBSERVATION`.
    #[must_use]
    pub fn rm_class(&self) -> &str {
        &self.rm_class
    }

    /// The concept name, including any specialisation segments.
    #[must_use]
    pub fn concept_id(&self) -> &str {
        &self.concept_id
    }

    /// The numeric version text as written — one, two, or three
    /// dot-separated parts. See the module documentation's departure note.
    #[must_use]
    pub fn release_version(&self) -> &str {
        &self.release_version
    }

    /// The version's maturity.
    #[must_use]
    pub const fn version_status(&self) -> VersionStatus {
        self.version_status
    }

    /// The build count, empty when [`Self::version_status`] is
    /// [`VersionStatus::Released`] with no suffix in the source text.
    #[must_use]
    pub fn build_count(&self) -> &str {
        &self.build_count
    }

    /// The major version number.
    ///
    /// # Panics
    ///
    /// Never: the parser guarantees the first part of `release_version` is
    /// numeric.
    #[must_use]
    pub fn major_version(&self) -> u32 {
        self.release_version
            .split('.')
            .next()
            .and_then(|n| n.parse().ok())
            .expect("parser guarantees a numeric major version")
    }

    /// The minor version number, if the source text gave one. See the module
    /// documentation's departure note on why this is not always present.
    #[must_use]
    pub fn minor_version(&self) -> Option<u32> {
        self.release_version
            .split('.')
            .nth(1)
            .and_then(|n| n.parse().ok())
    }

    /// The patch version number, if the source text gave one. See the module
    /// documentation's departure note on why this is not always present.
    #[must_use]
    pub fn patch_version(&self) -> Option<u32> {
        self.release_version
            .split('.')
            .nth(2)
            .and_then(|n| n.parse().ok())
    }

    /// The full version identifier: [`Self::release_version`] plus the
    /// maturity suffix and build count where the source distinguished one,
    /// e.g. `"1.8.2-rc.4"`, or plainly `"1.8.2"` for a released version with
    /// no suffix.
    #[must_use]
    pub fn version_id(&self) -> String {
        let suffix = match self.version_status {
            VersionStatus::Released => return self.release_version.clone(),
            VersionStatus::Alpha => "alpha",
            VersionStatus::Beta => "beta",
            VersionStatus::ReleaseCandidate => "rc",
            VersionStatus::Build => {
                return if self.build_count.is_empty() {
                    self.release_version.clone()
                } else {
                    format!("{}+{}", self.release_version, self.build_count)
                };
            }
        };
        if self.build_count.is_empty() {
            format!("{}-{suffix}", self.release_version)
        } else {
            format!("{}-{suffix}.{}", self.release_version, self.build_count)
        }
    }

    /// The "interface" form of this identifier, down to the major version
    /// only: `[namespace::]rm_publisher-rm_package-rm_class.concept_id.vN`.
    #[must_use]
    pub fn semantic_id(&self) -> String {
        format!(
            "{}{}-{}-{}.{}.v{}",
            self.namespace_prefix(),
            self.rm_publisher,
            self.rm_package,
            self.rm_class,
            self.concept_id,
            self.major_version()
        )
    }

    /// The "physical" form of this identifier, with complete version
    /// information: [`Self::semantic_id`] with [`Self::version_id`] in place
    /// of the bare major version.
    #[must_use]
    pub fn physical_id(&self) -> String {
        format!(
            "{}{}-{}-{}.{}.v{}",
            self.namespace_prefix(),
            self.rm_publisher,
            self.rm_package,
            self.rm_class,
            self.concept_id,
            self.version_id()
        )
    }

    fn namespace_prefix(&self) -> String {
        self.namespace
            .as_deref()
            .map_or_else(String::new, |ns| format!("{ns}::"))
    }
}

impl fmt::Display for ArchetypeHrid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.physical_id())
    }
}

/// An `IDENTIFIER`: starts with a letter, then letters, digits, or `_`.
fn is_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// A `LABEL`: starts with a letter, then letters, digits, `_`, or `-`
/// (percent-encoding not accepted — see the module documentation).
fn is_label(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn parse_version(text: &str) -> Result<(String, VersionStatus, String), ParseError> {
    let Some(digits) = text.strip_prefix('v') else {
        return Err(ParseError::new(
            "ARCHETYPE_HRID",
            "the version section must start with 'v'",
            text,
        ));
    };
    let (release_version, rest) = match digits.split_once('-') {
        Some((rv, rest)) => (rv, Some(rest)),
        None => (digits, None),
    };
    let parts: Vec<&str> = release_version.split('.').collect();
    if parts.is_empty()
        || parts.len() > 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(ParseError::new(
            "ARCHETYPE_HRID",
            "release_version must be one to three dot-separated numbers",
            text,
        ));
    }

    let (version_status, build_count) = match rest {
        None => (VersionStatus::Released, String::new()),
        Some(rest) => {
            let (word, build) = rest.split_once('.').unwrap_or((rest, ""));
            let version_status = match word {
                "rc" => VersionStatus::ReleaseCandidate,
                "alpha" => VersionStatus::Alpha,
                "beta" => VersionStatus::Beta,
                _ => {
                    return Err(ParseError::new(
                        "ARCHETYPE_HRID",
                        "expected a '-rc', '-alpha', or '-beta' maturity suffix",
                        text,
                    ));
                }
            };
            if !build.is_empty() && !build.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ParseError::new(
                    "ARCHETYPE_HRID",
                    "the build count after a maturity suffix must be digits",
                    text,
                ));
            }
            (version_status, build.to_owned())
        }
    };
    Ok((release_version.to_owned(), version_status, build_count))
}

impl FromStr for ArchetypeHrid {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the text does not match `[namespace::]
    /// rm_publisher-rm_package-rm_class.concept_id.v<release_version>
    /// [-{rc|alpha|beta}[.build_count]]`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (namespace, rest) = match s.split_once("::") {
            Some((ns, rest)) => {
                if ns.is_empty() || !ns.split('.').all(is_label) {
                    return Err(ParseError::new(
                        "ARCHETYPE_HRID",
                        "the namespace before '::' is empty or not dot-separated labels",
                        s,
                    ));
                }
                (Some(ns.to_owned()), rest)
            }
            None => (None, s),
        };

        let mut sections = rest.splitn(3, '.');
        let (Some(qualified), Some(concept_id), Some(version_text)) =
            (sections.next(), sections.next(), sections.next())
        else {
            return Err(ParseError::new(
                "ARCHETYPE_HRID",
                "expected rm_publisher-rm_package-rm_class.concept_id.version",
                s,
            ));
        };

        let rm: Vec<&str> = qualified.split('-').collect();
        let [rm_publisher, rm_package, rm_class] = rm.as_slice() else {
            return Err(ParseError::new(
                "ARCHETYPE_HRID",
                "expected rm_publisher-rm_package-rm_class",
                s,
            ));
        };
        for part in [rm_publisher, rm_package, rm_class] {
            if !is_identifier(part) {
                return Err(ParseError::new(
                    "ARCHETYPE_HRID",
                    "rm_publisher, rm_package, and rm_class must start with a letter and hold \
                     only letters, digits, or underscores",
                    s,
                ));
            }
        }

        if !is_label(concept_id) {
            return Err(ParseError::new(
                "ARCHETYPE_HRID",
                "concept_id must start with a letter and hold only letters, digits, \
                 underscores, or hyphens",
                s,
            ));
        }

        let (release_version, version_status, build_count) = parse_version(version_text)?;

        Ok(Self {
            namespace,
            rm_publisher: (*rm_publisher).to_owned(),
            rm_package: (*rm_package).to_owned(),
            rm_class: (*rm_class).to_owned(),
            concept_id: concept_id.to_owned(),
            release_version,
            version_status,
            build_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ArchetypeHrid, VersionStatus};

    #[test]
    fn a_plain_released_hrid_round_trips_through_its_own_display() {
        let id: ArchetypeHrid = "openEHR-EHR-OBSERVATION.blood_pressure.v1".parse().unwrap();
        assert_eq!(id.rm_publisher(), "openEHR");
        assert_eq!(id.rm_package(), "EHR");
        assert_eq!(id.rm_class(), "OBSERVATION");
        assert_eq!(id.concept_id(), "blood_pressure");
        assert_eq!(id.release_version(), "1");
        assert_eq!(id.version_status(), VersionStatus::Released);
        assert_eq!(id.build_count(), "");
        assert_eq!(id.major_version(), 1);
        assert_eq!(id.minor_version(), None);
        assert_eq!(id.patch_version(), None);
        assert_eq!(id.version_id(), "1");
        assert_eq!(id.to_string(), "openEHR-EHR-OBSERVATION.blood_pressure.v1");
    }

    #[test]
    fn a_namespace_prefix_is_read_and_rendered_back() {
        let id: ArchetypeHrid = "acme.health::openEHR-EHR-OBSERVATION.blood_pressure.v1"
            .parse()
            .unwrap();
        assert_eq!(id.namespace(), Some("acme.health"));
        assert_eq!(
            id.to_string(),
            "acme.health::openEHR-EHR-OBSERVATION.blood_pressure.v1"
        );
    }

    #[test]
    fn a_prerelease_suffix_with_a_build_count_is_read_and_recomposed() {
        let id: ArchetypeHrid = "openEHR-EHR-OBSERVATION.blood_pressure.v1.8.2-rc.4"
            .parse()
            .unwrap();
        assert_eq!(id.release_version(), "1.8.2");
        assert_eq!(id.version_status(), VersionStatus::ReleaseCandidate);
        assert_eq!(id.build_count(), "4");
        assert_eq!(id.major_version(), 1);
        assert_eq!(id.minor_version(), Some(8));
        assert_eq!(id.patch_version(), Some(2));
        assert_eq!(id.version_id(), "1.8.2-rc.4");
        assert_eq!(
            id.semantic_id(),
            "openEHR-EHR-OBSERVATION.blood_pressure.v1"
        );
        assert_eq!(
            id.physical_id(),
            "openEHR-EHR-OBSERVATION.blood_pressure.v1.8.2-rc.4"
        );
    }

    #[test]
    fn alpha_and_beta_suffixes_with_no_build_count_are_accepted() {
        for (suffix, status) in [
            ("alpha", VersionStatus::Alpha),
            ("beta", VersionStatus::Beta),
        ] {
            let text = format!("openEHR-EHR-OBSERVATION.blood_pressure.v1.0.0-{suffix}");
            let id: ArchetypeHrid = text.parse().unwrap();
            assert_eq!(id.version_status(), status);
            assert_eq!(id.build_count(), "");
            assert_eq!(id.version_id(), format!("1.0.0-{suffix}"));
        }
    }

    #[test]
    fn a_two_part_release_version_parses_per_the_grammars_own_laxity() {
        // The departure this module's own documentation declares: AOM2's
        // class model requires three parts, the ADL lexical grammar does
        // not, and this parser follows the grammar.
        let id: ArchetypeHrid = "openEHR-EHR-OBSERVATION.blood_pressure.v1.2"
            .parse()
            .unwrap();
        assert_eq!(id.minor_version(), Some(2));
        assert_eq!(id.patch_version(), None);
    }

    #[test]
    fn a_four_part_release_version_is_refused() {
        assert!(
            "openEHR-EHR-OBSERVATION.blood_pressure.v1.2.3.4"
                .parse::<ArchetypeHrid>()
                .is_err()
        );
    }

    #[test]
    fn an_empty_namespace_before_the_separator_is_refused_not_treated_as_absent() {
        assert!(
            "::openEHR-EHR-OBSERVATION.blood_pressure.v1"
                .parse::<ArchetypeHrid>()
                .is_err()
        );
    }

    #[test]
    fn a_build_status_suffix_is_not_producible_from_text_because_the_grammar_has_no_plus_branch() {
        assert!(
            "openEHR-EHR-OBSERVATION.blood_pressure.v1.0.0+4"
                .parse::<ArchetypeHrid>()
                .is_err()
        );
    }

    #[test]
    fn an_unknown_maturity_word_is_refused_naming_the_expectation() {
        let err = "openEHR-EHR-OBSERVATION.blood_pressure.v1.0.0-nightly"
            .parse::<ArchetypeHrid>()
            .unwrap_err();
        assert!(err.reason.contains("rc"), "{err}");
    }

    #[test]
    fn an_identifier_segment_starting_with_a_digit_is_refused() {
        // `IDENTIFIER: ALPHA_CHAR WORD_CHAR*` requires a leading letter.
        assert!(
            "9penEHR-EHR-OBSERVATION.blood_pressure.v1"
                .parse::<ArchetypeHrid>()
                .is_err()
        );
    }

    #[test]
    fn a_qualified_section_with_the_wrong_hyphen_count_is_refused() {
        // `IDENTIFIER` has no hyphen in its own grammar, so a fourth
        // hyphen-separated segment before the first `.` is refused as the
        // wrong shape rather than validated as a fourth identifier.
        assert!(
            "openEHR-EHR-OBSER-VATION.blood_pressure.v1"
                .parse::<ArchetypeHrid>()
                .is_err()
        );
    }
}
