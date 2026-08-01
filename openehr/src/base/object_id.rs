//! `OBJECT_ID` and its descendants.
//!
//! These are the identifiers openEHR uses to name things rather than to name
//! *values*: the record, the version, the archetype, the terminology. Each one
//! has a grammar, and each grammar is load-bearing — an `OBJECT_VERSION_ID` is
//! parsed into three parts by every server that implements the REST API,
//! because the middle part says which system created the version and the last
//! part says where it sits in the version tree.
//!
//! # Why these are parsed and not just held as strings
//!
//! It is tempting to keep `"87284370-…::ehr1.nhs.uk::2"` as a `String` and be
//! done. The cost shows up later and elsewhere: an unparsed version id makes
//! `preceding_version_uid` unverifiable, so a client can commit a version whose
//! parent belongs to a different versioned object and no layer notices until
//! the history is read back and does not connect. Parsing at the boundary is
//! what makes [`crate::rm::common::VersionedObject`] able to refuse that.

use crate::base::uid::Uid;
use crate::error::ParseError;
use core::fmt;
use core::str::FromStr;

/// A globally unique identifier for a versioned container, optionally with a
/// local extension.
///
/// Lexical form: `uid [ '::' extension ]`.
///
/// The extension exists for identifiers minted by a system that has its own
/// numbering inside a namespace — a hospital's MRN inside an issuing
/// authority's OID. It is an opaque string; openEHR places no grammar on it.
///
/// ```
/// use openehr::base::HierObjectId;
///
/// let plain: HierObjectId = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B".parse().unwrap();
/// assert!(plain.extension().is_none());
///
/// let extended: HierObjectId = "2.16.840.1.113883.2.1.4.3::M123456".parse().unwrap();
/// assert_eq!(extended.extension(), Some("M123456"));
/// assert_eq!(extended.to_string(), "2.16.840.1.113883.2.1.4.3::M123456");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HierObjectId {
    root: Uid,
    extension: Option<String>,
}

impl HierObjectId {
    /// Builds an identifier from a root and an optional extension.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the extension is empty (an empty extension is
    /// not the same as no extension: `x::` and `x` would print differently and
    /// compare unequal, which is a distinction with no meaning) or contains
    /// `::` (which would make the printed form re-parse into different parts).
    pub fn new(root: Uid, extension: Option<String>) -> Result<Self, ParseError> {
        if let Some(ext) = &extension {
            if ext.is_empty() {
                return Err(ParseError::new("HIER_OBJECT_ID", "empty extension", ""));
            }
            if ext.contains("::") {
                return Err(ParseError::new(
                    "HIER_OBJECT_ID",
                    "extension contains the `::` separator",
                    ext,
                ));
            }
        }
        Ok(Self { root, extension })
    }

    /// Convenience constructor for the common case of a bare UUID.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the text is not a valid `UID`.
    pub fn from_uid_str(uid: &str) -> Result<Self, ParseError> {
        Ok(Self {
            root: uid.parse()?,
            extension: None,
        })
    }

    /// The identifying part.
    #[must_use]
    pub fn root(&self) -> &Uid {
        &self.root
    }

    /// The local extension, if any.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }
}

impl fmt::Display for HierObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.extension {
            Some(ext) => write!(f, "{}::{ext}", self.root),
            None => write!(f, "{}", self.root),
        }
    }
}

impl FromStr for HierObjectId {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the root is not a valid `UID`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // split_once, not splitn(3): an extension containing `::` would be
        // silently truncated by a three-way split, and `new` rejects it.
        match s.split_once("::") {
            Some((root, ext)) => Self::new(root.parse()?, Some(ext.to_owned())),
            None => Self::new(s.parse()?, None),
        }
    }
}

crate::impl_valued_serde!(HierObjectId, "HIER_OBJECT_ID");

/// The identifier of one version of one versioned object.
///
/// Lexical form: `object_id '::' creating_system_id '::' version_tree_id`.
///
/// ```
/// use openehr::base::ObjectVersionId;
///
/// let id: ObjectVersionId =
///     "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.nhs.uk::2".parse().unwrap();
/// assert_eq!(id.object_id().to_string(), "87284370-2D4B-4E3D-A3F3-F303D2F4F34B");
/// assert_eq!(id.creating_system_id().to_string(), "ehr1.nhs.uk");
/// assert_eq!(id.version_tree_id().trunk_version(), 2);
/// assert!(!id.is_branch());
/// ```
///
/// A branch version carries three numbers in the last part:
///
/// ```
/// use openehr::base::ObjectVersionId;
///
/// let branch: ObjectVersionId =
///     "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.nhs.uk::2.1.4".parse().unwrap();
/// assert!(branch.is_branch());
/// assert_eq!(branch.version_tree_id().branch_number(), Some(1));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
// The `_id` suffix on all three fields is openEHR's naming, not redundancy this
// crate introduced. Renaming them to `object`, `creating_system`, `version_tree`
// would break the correspondence between this struct and the class definition a
// reader is holding open next to it, which is the one thing that makes an RM
// port checkable.
#[allow(clippy::struct_field_names)]
pub struct ObjectVersionId {
    object_id: Uid,
    creating_system_id: Uid,
    version_tree_id: VersionTreeId,
}

impl ObjectVersionId {
    /// Builds a version identifier from its three parts.
    #[must_use]
    pub fn new(object_id: Uid, creating_system_id: Uid, version_tree_id: VersionTreeId) -> Self {
        Self {
            object_id,
            creating_system_id,
            version_tree_id,
        }
    }

    /// The identifier of the versioned object this version belongs to.
    ///
    /// This is the part that must match across every version in one history,
    /// and the part `VERSIONED_OBJECT.uid` repeats.
    #[must_use]
    pub fn object_id(&self) -> &Uid {
        &self.object_id
    }

    /// The system that created this version.
    ///
    /// Two systems editing the same versioned object offline both produce
    /// `…::2`; the creating system id is what keeps those distinct, which is
    /// why openEHR puts it in the identifier rather than in an attribute.
    #[must_use]
    pub fn creating_system_id(&self) -> &Uid {
        &self.creating_system_id
    }

    /// Where this version sits in the version tree.
    #[must_use]
    pub fn version_tree_id(&self) -> &VersionTreeId {
        &self.version_tree_id
    }

    /// Whether this version is on a branch rather than the trunk.
    #[must_use]
    pub fn is_branch(&self) -> bool {
        self.version_tree_id.is_branch()
    }

    /// Whether two version ids name versions of the same versioned object.
    ///
    /// ```
    /// use openehr::base::ObjectVersionId;
    ///
    /// let v1: ObjectVersionId = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::a.example::1".parse().unwrap();
    /// let v2: ObjectVersionId = "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::b.example::2".parse().unwrap();
    /// assert!(v1.same_object_as(&v2)); // different systems, same record
    /// ```
    #[must_use]
    pub fn same_object_as(&self, other: &Self) -> bool {
        self.object_id == other.object_id
    }
}

impl fmt::Display for ObjectVersionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}::{}::{}",
            self.object_id, self.creating_system_id, self.version_tree_id
        )
    }
}

impl FromStr for ObjectVersionId {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] unless the text has exactly three `::`-separated
    /// parts, the first two are valid `UID`s, and the third is a valid
    /// [`VersionTreeId`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split("::").collect();
        let [object_id, creating_system_id, version_tree_id] = parts.as_slice() else {
            return Err(ParseError::new(
                "OBJECT_VERSION_ID",
                "expected object_id::creating_system_id::version_tree_id",
                s,
            ));
        };
        Ok(Self {
            object_id: object_id.parse()?,
            creating_system_id: creating_system_id.parse()?,
            version_tree_id: version_tree_id.parse()?,
        })
    }
}

crate::impl_valued_serde!(ObjectVersionId, "OBJECT_VERSION_ID");

/// A position in a version tree: `trunk [ '.' branch_number '.' branch_version ]`.
///
/// # Why leading zeros are refused
///
/// `01` and `1` denote the same trunk version, so accepting both would make two
/// distinct strings name one version. Every REST path, `preceding_version_uid`,
/// and `LOCATABLE_REF` in the system carries the identifier as text and
/// compares it as text. This type therefore stores the numbers and prints the
/// canonical form, and refuses input that would not survive that round trip.
///
/// ```
/// use openehr::base::VersionTreeId;
///
/// assert!("1".parse::<VersionTreeId>().is_ok());
/// assert!("2.1.4".parse::<VersionTreeId>().is_ok());
/// assert!("01".parse::<VersionTreeId>().is_err());  // would not round-trip
/// assert!("0".parse::<VersionTreeId>().is_err());   // trunk starts at 1
/// assert!("2.1".parse::<VersionTreeId>().is_err()); // branch parts come in pairs
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VersionTreeId {
    trunk_version: u32,
    branch: Option<(u32, u32)>,
}

impl VersionTreeId {
    /// The first version of a new versioned object: `1`.
    ///
    /// ```
    /// use openehr::base::VersionTreeId;
    /// assert_eq!(VersionTreeId::FIRST.to_string(), "1");
    /// ```
    pub const FIRST: Self = Self {
        trunk_version: 1,
        branch: None,
    };

    /// Builds a trunk version.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if `trunk_version` is zero.
    pub fn trunk(trunk_version: u32) -> Result<Self, ParseError> {
        if trunk_version == 0 {
            return Err(ParseError::new(
                "VERSION_TREE_ID",
                "trunk_version is 0",
                "0",
            ));
        }
        Ok(Self {
            trunk_version,
            branch: None,
        })
    }

    /// Builds a branch version.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if any of the three numbers is zero.
    pub fn branch(
        trunk_version: u32,
        branch_number: u32,
        branch_version: u32,
    ) -> Result<Self, ParseError> {
        if trunk_version == 0 || branch_number == 0 || branch_version == 0 {
            return Err(ParseError::new(
                "VERSION_TREE_ID",
                "version numbers start at 1",
                "",
            ));
        }
        Ok(Self {
            trunk_version,
            branch: Some((branch_number, branch_version)),
        })
    }

    /// The trunk version number.
    #[must_use]
    pub fn trunk_version(&self) -> u32 {
        self.trunk_version
    }

    /// The branch number, if this is a branch version.
    #[must_use]
    pub fn branch_number(&self) -> Option<u32> {
        self.branch.map(|(n, _)| n)
    }

    /// The version within the branch, if this is a branch version.
    #[must_use]
    pub fn branch_version(&self) -> Option<u32> {
        self.branch.map(|(_, v)| v)
    }

    /// Whether this is a branch version.
    #[must_use]
    pub fn is_branch(&self) -> bool {
        self.branch.is_some()
    }

    /// Whether this is the very first version, `1`.
    #[must_use]
    pub fn is_first(&self) -> bool {
        *self == Self::FIRST
    }

    /// The next version along the same line.
    ///
    /// Trunk `2` follows trunk `1`; branch `2.1.4` is followed by `2.1.5`. The
    /// trunk number of a branch never advances — that is what makes it a
    /// branch.
    ///
    /// ```
    /// use openehr::base::VersionTreeId;
    ///
    /// assert_eq!(VersionTreeId::FIRST.next().to_string(), "2");
    /// assert_eq!("2.1.4".parse::<VersionTreeId>().unwrap().next().to_string(), "2.1.5");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the relevant counter is `u32::MAX`. A versioned object with
    /// four billion versions is a runaway writer, and silently wrapping to
    /// version `0` would corrupt the history rather than surface the bug.
    #[must_use]
    pub fn next(&self) -> Self {
        match self.branch {
            None => Self {
                trunk_version: self
                    .trunk_version
                    .checked_add(1)
                    .expect("version tree trunk overflowed u32"),
                branch: None,
            },
            Some((n, v)) => Self {
                trunk_version: self.trunk_version,
                branch: Some((
                    n,
                    v.checked_add(1)
                        .expect("version tree branch overflowed u32"),
                )),
            },
        }
    }
}

impl fmt::Display for VersionTreeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.branch {
            None => write!(f, "{}", self.trunk_version),
            Some((n, v)) => write!(f, "{}.{n}.{v}", self.trunk_version),
        }
    }
}

impl FromStr for VersionTreeId {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the text is not one or three dot-separated
    /// numbers, if any number is zero, or if any number carries a leading zero.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn num(part: &str, whole: &str) -> Result<u32, ParseError> {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ParseError::new("VERSION_TREE_ID", "not a number", whole));
            }
            if part.len() > 1 && part.starts_with('0') {
                return Err(ParseError::new(
                    "VERSION_TREE_ID",
                    "leading zero would not round-trip",
                    whole,
                ));
            }
            part.parse().map_err(|_| {
                ParseError::new("VERSION_TREE_ID", "number does not fit in u32", whole)
            })
        }

        let parts: Vec<&str> = s.split('.').collect();
        match parts.as_slice() {
            [trunk] => Self::trunk(num(trunk, s)?),
            [trunk, branch_number, branch_version] => Self::branch(
                num(trunk, s)?,
                num(branch_number, s)?,
                num(branch_version, s)?,
            ),
            _ => Err(ParseError::new(
                "VERSION_TREE_ID",
                "expected `trunk` or `trunk.branch_number.branch_version`",
                s,
            )),
        }
    }
}

crate::impl_string_serde!(VersionTreeId, "VERSION_TREE_ID");

/// An archetype identifier such as
/// `openEHR-EHR-OBSERVATION.blood_pressure.v2`.
///
/// Lexical form:
/// `rm_originator '-' rm_name '-' rm_entity '.' concept_name {'-' specialisation}* '.' version`
///
/// # The version part accepts both `v2` and `v2.1.0`
///
/// The BASE specification's grammar gives `'v' number`, which is what ADL 1.4
/// archetypes carry. ADL 2 archetypes carry a full three-part semantic version,
/// and the CKM publishes both. Accepting only the narrower grammar would reject
/// identifiers that appear in real instance data, so this type accepts one to
/// three numeric components and preserves which it was given.
///
/// ```
/// use openehr::base::ArchetypeId;
///
/// let id: ArchetypeId = "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap();
/// assert_eq!(id.rm_originator(), "openEHR");
/// assert_eq!(id.rm_name(), "EHR");
/// assert_eq!(id.rm_entity(), "OBSERVATION");
/// assert_eq!(id.concept_name(), "blood_pressure");
/// assert!(id.specialisations().next().is_none());
///
/// let adl2: ArchetypeId = "openEHR-EHR-OBSERVATION.blood_pressure.v2.1.0".parse().unwrap();
/// assert_eq!(adl2.version_id(), "v2.1.0");
/// assert_eq!(adl2.major_version(), 2);
///
/// let spec: ArchetypeId =
///     "openEHR-EHR-OBSERVATION.progress_note-naturopathy.v2".parse().unwrap();
/// assert_eq!(spec.concept_name(), "progress_note");
/// assert_eq!(spec.specialisations().collect::<Vec<_>>(), ["naturopathy"]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArchetypeId {
    rm_originator: String,
    rm_name: String,
    rm_entity: String,
    domain_concept: String,
    version_id: String,
}

impl ArchetypeId {
    /// The organisation that issued the reference model, usually `openEHR`.
    #[must_use]
    pub fn rm_originator(&self) -> &str {
        &self.rm_originator
    }

    /// The reference model name, such as `EHR` or `DEMOGRAPHIC`.
    #[must_use]
    pub fn rm_name(&self) -> &str {
        &self.rm_name
    }

    /// The RM class the archetype constrains, such as `OBSERVATION`.
    ///
    /// This is the attribute that makes an archetype id checkable against the
    /// object it annotates: an `ARCHETYPED` on a `COMPOSITION` whose archetype
    /// id says `OBSERVATION` is a defect, and
    /// [`crate::validation`] reports it.
    #[must_use]
    pub fn rm_entity(&self) -> &str {
        &self.rm_entity
    }

    /// The domain concept including any specialisations, such as
    /// `progress_note-naturopathy`.
    #[must_use]
    pub fn domain_concept(&self) -> &str {
        &self.domain_concept
    }

    /// The concept name without its specialisations.
    #[must_use]
    pub fn concept_name(&self) -> &str {
        self.domain_concept
            .split_once('-')
            .map_or(self.domain_concept.as_str(), |(head, _)| head)
    }

    /// The specialisation segments, outermost first.
    pub fn specialisations(&self) -> impl Iterator<Item = &str> {
        let mut parts = self.domain_concept.split('-');
        parts.next();
        parts
    }

    /// The version part including its leading `v`, such as `v2` or `v2.1.0`.
    #[must_use]
    pub fn version_id(&self) -> &str {
        &self.version_id
    }

    /// The major version number.
    ///
    /// This is the number that matters for compatibility: openEHR's archetype
    /// versioning rules make a major-version bump the only breaking change, so
    /// data archetyped `v1` is readable against `v1.9.3` and not against `v2`.
    ///
    /// # Panics
    ///
    /// Never: the parser guarantees the major component is numeric.
    #[must_use]
    pub fn major_version(&self) -> u32 {
        self.version_id[1..]
            .split('.')
            .next()
            .and_then(|n| n.parse().ok())
            .expect("parser guarantees a numeric major version")
    }

    /// Whether this archetype constrains the given RM class name.
    ///
    /// ```
    /// use openehr::base::ArchetypeId;
    ///
    /// let id: ArchetypeId = "openEHR-EHR-OBSERVATION.blood_pressure.v2".parse().unwrap();
    /// assert!(id.constrains("OBSERVATION"));
    /// assert!(!id.constrains("EVALUATION"));
    /// ```
    #[must_use]
    pub fn constrains(&self, rm_class: &str) -> bool {
        self.rm_entity == rm_class
    }
}

impl fmt::Display for ArchetypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}-{}-{}.{}.{}",
            self.rm_originator, self.rm_name, self.rm_entity, self.domain_concept, self.version_id
        )
    }
}

impl FromStr for ArchetypeId {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the text does not have the three dot-separated
    /// sections, if the first is not three hyphen-separated names, or if the
    /// version is not `v` followed by one to three numbers.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The domain concept cannot contain a dot, so the section boundaries
        // are unambiguous even though both the qualified entity and the domain
        // concept may contain hyphens.
        let mut sections = s.splitn(3, '.');
        let (Some(qualified), Some(domain_concept), Some(version_id)) =
            (sections.next(), sections.next(), sections.next())
        else {
            return Err(ParseError::new(
                "ARCHETYPE_ID",
                "expected rm_entity.domain_concept.version",
                s,
            ));
        };

        let rm: Vec<&str> = qualified.split('-').collect();
        let [rm_originator, rm_name, rm_entity] = rm.as_slice() else {
            return Err(ParseError::new(
                "ARCHETYPE_ID",
                "expected rm_originator-rm_name-rm_entity",
                s,
            ));
        };
        for part in [rm_originator, rm_name, rm_entity] {
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err(ParseError::new(
                    "ARCHETYPE_ID",
                    "reference model name is empty or has an illegal character",
                    s,
                ));
            }
        }

        if domain_concept.is_empty()
            || !domain_concept
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            return Err(ParseError::new(
                "ARCHETYPE_ID",
                "domain concept is empty or has an illegal character",
                s,
            ));
        }
        if domain_concept.split('-').any(str::is_empty) {
            return Err(ParseError::new(
                "ARCHETYPE_ID",
                "empty specialisation segment",
                s,
            ));
        }

        let Some(numbers) = version_id.strip_prefix('v') else {
            return Err(ParseError::new("ARCHETYPE_ID", "version lacks `v`", s));
        };
        let components: Vec<&str> = numbers.split('.').collect();
        if components.is_empty() || components.len() > 3 {
            return Err(ParseError::new(
                "ARCHETYPE_ID",
                "version has more than three components",
                s,
            ));
        }
        for c in &components {
            if c.is_empty() || !c.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ParseError::new(
                    "ARCHETYPE_ID",
                    "version component is not a number",
                    s,
                ));
            }
            if c.parse::<u32>().is_err() {
                return Err(ParseError::new(
                    "ARCHETYPE_ID",
                    "version component does not fit in u32",
                    s,
                ));
            }
        }

        Ok(Self {
            rm_originator: (*rm_originator).to_owned(),
            rm_name: (*rm_name).to_owned(),
            rm_entity: (*rm_entity).to_owned(),
            domain_concept: domain_concept.to_owned(),
            version_id: version_id.to_owned(),
        })
    }
}

crate::impl_valued_serde!(ArchetypeId, "ARCHETYPE_ID");

/// A template identifier.
///
/// openEHR states the lexical form as "to be determined", so this type stores
/// the text verbatim and requires only that it be non-empty and free of
/// whitespace. Inventing a stricter grammar would reject valid identifiers from
/// conformant tools; accepting anything at all, including an empty string,
/// would let a missing template id look like a present one.
///
/// ```
/// use openehr::base::TemplateId;
///
/// assert!("Blood pressure.v1".parse::<TemplateId>().is_err()); // whitespace
/// assert!("blood_pressure.v1".parse::<TemplateId>().is_ok());
/// assert!("".parse::<TemplateId>().is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TemplateId(String);

impl TemplateId {
    /// The identifier's text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TemplateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for TemplateId {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the text is empty or contains whitespace.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseError::new("TEMPLATE_ID", "empty", s));
        }
        if s.chars().any(char::is_whitespace) {
            return Err(ParseError::new("TEMPLATE_ID", "contains whitespace", s));
        }
        Ok(Self(s.to_owned()))
    }
}

crate::impl_valued_serde!(TemplateId, "TEMPLATE_ID");

/// A terminology identifier such as `SNOMED-CT`, `openehr`, or
/// `ICD10AM(3rd_ed)`.
///
/// Lexical form: `name [ '(' version ')' ]`.
///
/// ```
/// use openehr::base::TerminologyId;
///
/// let plain: TerminologyId = "SNOMED-CT".parse().unwrap();
/// assert_eq!(plain.name(), "SNOMED-CT");
/// assert!(plain.version_id().is_none());
///
/// let versioned: TerminologyId = "ICD10AM(3rd_ed)".parse().unwrap();
/// assert_eq!(versioned.name(), "ICD10AM");
/// assert_eq!(versioned.version_id(), Some("3rd_ed"));
/// assert_eq!(versioned.to_string(), "ICD10AM(3rd_ed)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TerminologyId {
    name: String,
    version_id: Option<String>,
}

impl TerminologyId {
    /// The openEHR support terminology, which supplies null flavours, change
    /// types, lifecycle states, and the other coded vocabularies the reference
    /// model refers to by code.
    ///
    /// ```
    /// use openehr::base::TerminologyId;
    /// assert_eq!(TerminologyId::openehr().name(), "openehr");
    /// ```
    #[must_use]
    #[allow(non_snake_case)]
    pub fn openehr() -> Self {
        Self {
            name: "openehr".to_owned(),
            version_id: None,
        }
    }

    /// The terminology's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The terminology's version, if the identifier carries one.
    #[must_use]
    pub fn version_id(&self) -> Option<&str> {
        self.version_id.as_deref()
    }

    /// Whether this identifies the openEHR support terminology.
    ///
    /// Compared case-insensitively: instances carry `openehr`, `openEHR`, and
    /// `OpenEHR`, and a code-lookup that missed on case would report a valid
    /// null flavour as an unknown code.
    #[must_use]
    pub fn is_openehr(&self) -> bool {
        self.name.eq_ignore_ascii_case("openehr")
    }
}

impl fmt::Display for TerminologyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version_id {
            Some(v) => write!(f, "{}({v})", self.name),
            None => f.write_str(&self.name),
        }
    }
}

impl FromStr for TerminologyId {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the name is empty, if a `(` is unclosed, or if
    /// the parenthesised version is empty.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, version_id) = match s.split_once('(') {
            None => (s, None),
            Some((name, rest)) => {
                let Some(version) = rest.strip_suffix(')') else {
                    return Err(ParseError::new("TERMINOLOGY_ID", "unclosed `(`", s));
                };
                if version.is_empty() {
                    return Err(ParseError::new("TERMINOLOGY_ID", "empty version", s));
                }
                (name, Some(version.to_owned()))
            }
        };
        if name.is_empty() {
            return Err(ParseError::new("TERMINOLOGY_ID", "empty name", s));
        }
        Ok(Self {
            name: name.to_owned(),
            version_id,
        })
    }
}

crate::impl_valued_serde!(TerminologyId, "TERMINOLOGY_ID");

/// An identifier in a scheme openEHR does not define.
///
/// The escape hatch, and the only `OBJECT_ID` whose grammar is not checkable:
/// the scheme names an authority, and this crate has no registry of authorities
/// to check the value against. It requires both parts to be non-empty, so that
/// a `GENERIC_ID` at least says *which* uncontrolled scheme it belongs to.
///
/// ```
/// use openehr::base::GenericId;
///
/// let id = GenericId::new("NHS-UK-1234567890", "uk.nhs.nhs_number").unwrap();
/// assert_eq!(id.scheme(), "uk.nhs.nhs_number");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GenericId {
    #[serde(rename = "_type", default = "generic_id_type")]
    _type: GenericIdType,
    value: String,
    scheme: String,
}

fn generic_id_type() -> GenericIdType {
    GenericIdType::GenericId
}

/// The `_type` discriminator of [`GenericId`], as a type so that a payload
/// declaring some other class fails to deserialize rather than being quietly
/// reinterpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum GenericIdType {
    /// `GENERIC_ID`.
    #[serde(rename = "GENERIC_ID")]
    GenericId,
}

impl GenericId {
    /// Builds a generic identifier.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if either part is empty.
    pub fn new(value: impl Into<String>, scheme: impl Into<String>) -> Result<Self, ParseError> {
        let value = value.into();
        let scheme = scheme.into();
        if value.is_empty() {
            return Err(ParseError::new("GENERIC_ID", "empty value", &value));
        }
        if scheme.is_empty() {
            return Err(ParseError::new("GENERIC_ID", "empty scheme", &scheme));
        }
        Ok(Self {
            _type: GenericIdType::GenericId,
            value,
            scheme,
        })
    }

    /// The identifier text.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The naming scheme.
    #[must_use]
    pub fn scheme(&self) -> &str {
        &self.scheme
    }
}

impl fmt::Display for GenericId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.value)
    }
}

/// Any `OBJECT_ID`.
///
/// # Deserialization without a `_type`
///
/// `_type` is required by openEHR wherever the static type is abstract, and
/// `OBJECT_ID` always is. Real payloads omit it anyway. When it is missing this
/// type infers from the lexical shape — three `::` parts means
/// `OBJECT_VERSION_ID`, otherwise `HIER_OBJECT_ID` — and never guesses
/// `ARCHETYPE_ID`, `TEMPLATE_ID`, or `GENERIC_ID`, because those are not
/// distinguishable from a `HIER_OBJECT_ID` extension by shape alone and a wrong
/// guess is worse than a rejection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ObjectId {
    /// A versioned-container identifier.
    HierObjectId(HierObjectId),
    /// A version identifier.
    ObjectVersionId(ObjectVersionId),
    /// An archetype identifier.
    ArchetypeId(ArchetypeId),
    /// A template identifier.
    TemplateId(TemplateId),
    /// A terminology identifier.
    TerminologyId(TerminologyId),
    /// An identifier in an externally defined scheme.
    GenericId(GenericId),
}

impl ObjectId {
    /// The identifier's lexical form.
    #[must_use]
    pub fn value(&self) -> String {
        self.to_string()
    }

    /// The openEHR class name, as it appears in `_type`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::HierObjectId(_) => "HIER_OBJECT_ID",
            Self::ObjectVersionId(_) => "OBJECT_VERSION_ID",
            Self::ArchetypeId(_) => "ARCHETYPE_ID",
            Self::TemplateId(_) => "TEMPLATE_ID",
            Self::TerminologyId(_) => "TERMINOLOGY_ID",
            Self::GenericId(_) => "GENERIC_ID",
        }
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HierObjectId(v) => v.fmt(f),
            Self::ObjectVersionId(v) => v.fmt(f),
            Self::ArchetypeId(v) => v.fmt(f),
            Self::TemplateId(v) => v.fmt(f),
            Self::TerminologyId(v) => v.fmt(f),
            Self::GenericId(v) => v.fmt(f),
        }
    }
}

/// Either kind of identifier that can appear in `LOCATABLE.uid`.
///
/// `LOCATABLE.uid` is typed `UID_BASED_ID`, which admits exactly two of the six
/// `OBJECT_ID` classes. Modelling that as its own enum rather than reusing
/// [`ObjectId`] means an archetype id cannot be put in a `uid` field by mistake
/// — a class of error that would otherwise only surface at validation time.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UidBasedId {
    /// A versioned-container identifier.
    HierObjectId(HierObjectId),
    /// A version identifier.
    ObjectVersionId(ObjectVersionId),
}

impl UidBasedId {
    /// The identifying root, shared by both forms.
    ///
    /// For a [`HierObjectId`] this is its root; for an [`ObjectVersionId`] it
    /// is the `object_id`. Both answer the same question — *which* record —
    /// which is what makes the abstraction worth having.
    #[must_use]
    pub fn root(&self) -> &Uid {
        match self {
            Self::HierObjectId(id) => id.root(),
            Self::ObjectVersionId(id) => id.object_id(),
        }
    }

    /// The extension, which only a [`HierObjectId`] can carry.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        match self {
            Self::HierObjectId(id) => id.extension(),
            Self::ObjectVersionId(_) => None,
        }
    }

    /// The openEHR class name, as it appears in `_type`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::HierObjectId(_) => "HIER_OBJECT_ID",
            Self::ObjectVersionId(_) => "OBJECT_VERSION_ID",
        }
    }
}

impl fmt::Display for UidBasedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HierObjectId(v) => v.fmt(f),
            Self::ObjectVersionId(v) => v.fmt(f),
        }
    }
}

impl FromStr for UidBasedId {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the text is neither a `HIER_OBJECT_ID` nor an
    /// `OBJECT_VERSION_ID`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.matches("::").count() == 2 {
            return Ok(Self::ObjectVersionId(s.parse()?));
        }
        Ok(Self::HierObjectId(s.parse()?))
    }
}

impl From<HierObjectId> for UidBasedId {
    fn from(v: HierObjectId) -> Self {
        Self::HierObjectId(v)
    }
}

impl From<ObjectVersionId> for UidBasedId {
    fn from(v: ObjectVersionId) -> Self {
        Self::ObjectVersionId(v)
    }
}

mod uid_based_id_serde {
    use super::{HierObjectId, ObjectVersionId, UidBasedId};
    use serde::de::{Error as _, MapAccess, Visitor};
    use serde::ser::SerializeStruct as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for UidBasedId {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let mut st = s.serialize_struct("UID_BASED_ID", 2)?;
            st.serialize_field("_type", self.type_name())?;
            st.serialize_field("value", &self.to_string())?;
            st.end()
        }
    }

    impl<'de> Deserialize<'de> for UidBasedId {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct V;

            impl<'de> Visitor<'de> for V {
                type Value = UidBasedId;

                fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str("a UID_BASED_ID as a string or a typed object")
                }

                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<UidBasedId, E> {
                    v.parse().map_err(E::custom)
                }

                fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<UidBasedId, A::Error> {
                    let mut ty: Option<String> = None;
                    let mut value: Option<String> = None;
                    while let Some(key) = map.next_key::<String>()? {
                        match key.as_str() {
                            "_type" => ty = Some(map.next_value()?),
                            "value" => value = Some(map.next_value()?),
                            _ => {
                                map.next_value::<serde::de::IgnoredAny>()?;
                            }
                        }
                    }
                    let value = value.ok_or_else(|| A::Error::missing_field("value"))?;
                    match ty.as_deref() {
                        Some("HIER_OBJECT_ID") => Ok(UidBasedId::HierObjectId(
                            value.parse::<HierObjectId>().map_err(A::Error::custom)?,
                        )),
                        Some("OBJECT_VERSION_ID") => Ok(UidBasedId::ObjectVersionId(
                            value.parse::<ObjectVersionId>().map_err(A::Error::custom)?,
                        )),
                        Some(other) => Err(A::Error::custom(format!(
                            "_type is {other}, expected HIER_OBJECT_ID or OBJECT_VERSION_ID"
                        ))),
                        None => value.parse().map_err(A::Error::custom),
                    }
                }
            }

            d.deserialize_any(V)
        }
    }
}

mod object_id_serde {
    use super::{
        ArchetypeId, GenericId, HierObjectId, ObjectId, ObjectVersionId, TemplateId, TerminologyId,
        UidBasedId,
    };
    use serde::de::{Error as _, MapAccess, Visitor};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for ObjectId {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            match self {
                ObjectId::HierObjectId(v) => v.serialize(s),
                ObjectId::ObjectVersionId(v) => v.serialize(s),
                ObjectId::ArchetypeId(v) => v.serialize(s),
                ObjectId::TemplateId(v) => v.serialize(s),
                ObjectId::TerminologyId(v) => v.serialize(s),
                ObjectId::GenericId(v) => v.serialize(s),
            }
        }
    }

    impl<'de> Deserialize<'de> for ObjectId {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct V;

            impl<'de> Visitor<'de> for V {
                type Value = ObjectId;

                fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.write_str("an OBJECT_ID as a string or a typed object")
                }

                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<ObjectId, E> {
                    infer(v).map_err(E::custom)
                }

                fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<ObjectId, A::Error> {
                    let mut ty: Option<String> = None;
                    let mut value: Option<String> = None;
                    let mut scheme: Option<String> = None;
                    while let Some(key) = map.next_key::<String>()? {
                        match key.as_str() {
                            "_type" => ty = Some(map.next_value()?),
                            "value" => value = Some(map.next_value()?),
                            "scheme" => scheme = Some(map.next_value()?),
                            _ => {
                                map.next_value::<serde::de::IgnoredAny>()?;
                            }
                        }
                    }
                    let value = value.ok_or_else(|| A::Error::missing_field("value"))?;
                    match ty.as_deref() {
                        Some("HIER_OBJECT_ID") => Ok(ObjectId::HierObjectId(
                            value.parse::<HierObjectId>().map_err(A::Error::custom)?,
                        )),
                        Some("OBJECT_VERSION_ID") => Ok(ObjectId::ObjectVersionId(
                            value.parse::<ObjectVersionId>().map_err(A::Error::custom)?,
                        )),
                        Some("ARCHETYPE_ID") => Ok(ObjectId::ArchetypeId(
                            value.parse::<ArchetypeId>().map_err(A::Error::custom)?,
                        )),
                        Some("TEMPLATE_ID") => Ok(ObjectId::TemplateId(
                            value.parse::<TemplateId>().map_err(A::Error::custom)?,
                        )),
                        Some("TERMINOLOGY_ID") => Ok(ObjectId::TerminologyId(
                            value.parse::<TerminologyId>().map_err(A::Error::custom)?,
                        )),
                        Some("GENERIC_ID") => {
                            let scheme = scheme.ok_or_else(|| A::Error::missing_field("scheme"))?;
                            Ok(ObjectId::GenericId(
                                GenericId::new(value, scheme).map_err(A::Error::custom)?,
                            ))
                        }
                        Some(other) => Err(A::Error::custom(format!(
                            "{other} is not an OBJECT_ID class"
                        ))),
                        None => infer(&value).map_err(A::Error::custom),
                    }
                }
            }

            fn infer(value: &str) -> Result<ObjectId, crate::ParseError> {
                Ok(match value.parse::<UidBasedId>()? {
                    UidBasedId::HierObjectId(v) => ObjectId::HierObjectId(v),
                    UidBasedId::ObjectVersionId(v) => ObjectId::ObjectVersionId(v),
                })
            }

            d.deserialize_any(V)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_id_round_trips_exactly() {
        for text in [
            "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::ehr1.nhs.uk::1",
            "87284370-2d4b-4e3d-a3f3-f303d2f4f34b::2.16.840.1::12.3.7",
        ] {
            assert_eq!(text.parse::<ObjectVersionId>().unwrap().to_string(), text);
        }
    }

    #[test]
    fn hier_object_id_rejects_a_double_colon_extension() {
        // Otherwise `a::b::c` would parse as root `a`, extension `b::c`, and
        // print back as something that re-parses into three parts.
        assert!("2.16.840::b::c".parse::<HierObjectId>().is_err());
    }

    #[test]
    fn uid_based_id_infers_by_separator_count() {
        assert!(matches!(
            "1.2.3::x".parse::<UidBasedId>().unwrap(),
            UidBasedId::HierObjectId(_)
        ));
        assert!(matches!(
            "1.2.3::sys.example::4".parse::<UidBasedId>().unwrap(),
            UidBasedId::ObjectVersionId(_)
        ));
    }

    #[test]
    fn archetype_id_round_trips_and_splits() {
        let text = "openEHR-EHR-OBSERVATION.blood_pressure-mine-yours.v1.2.3";
        let id: ArchetypeId = text.parse().unwrap();
        assert_eq!(id.to_string(), text);
        assert_eq!(id.concept_name(), "blood_pressure");
        assert_eq!(id.specialisations().collect::<Vec<_>>(), ["mine", "yours"]);
        assert_eq!(id.major_version(), 1);
    }

    #[test]
    fn archetype_id_rejects_malformed_forms() {
        for text in [
            "openEHR-EHR.blood_pressure.v1",            // two-part rm entity
            "openEHR-EHR-OBSERVATION.blood_pressure",   // no version
            "openEHR-EHR-OBSERVATION.blood_pressure.1", // version without v
            "openEHR-EHR-OBSERVATION.blood_pressure.v", // v without number
            "openEHR-EHR-OBSERVATION..v1",              // empty concept
            "openEHR-EHR-OBSERVATION.a-.v1",            // empty specialisation
            "openEHR-EHR-OBSERVATION.a.v1.2.3.4",       // four version parts
        ] {
            assert!(text.parse::<ArchetypeId>().is_err(), "accepted {text}");
        }
    }

    #[test]
    fn version_tree_next_advances_the_right_counter() {
        let trunk: VersionTreeId = "3".parse().unwrap();
        assert_eq!(trunk.next().to_string(), "4");
        let branch: VersionTreeId = "3.2.9".parse().unwrap();
        assert_eq!(branch.next().to_string(), "3.2.10");
    }
}
