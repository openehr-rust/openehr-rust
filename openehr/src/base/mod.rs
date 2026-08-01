//! The openEHR **BASE** component: identifiers, references, intervals, and the
//! ISO 8601 primitives everything else is built from.
//!
//! Nothing here is clinical. These are the types the Reference Model assumes
//! already exist — which is precisely why getting them wrong is expensive: an
//! identifier type that normalises where it should not, or an interval that
//! lets `lower_unbounded` disagree with `lower`, corrupts every RM class that
//! embeds it.
//!
//! | Module | openEHR package |
//! | --- | --- |
//! | [`uid`] | Base Types → Identification (primitive UIDs) |
//! | [`object_id`] | Base Types → Identification (`OBJECT_ID` and descendants) |
//! | [`object_ref`] | Base Types → Identification (`OBJECT_REF` and descendants) |
//! | [`interval`] | Foundation Types → Interval |
//! | [`iso8601`] | Foundation Types → Time |

pub mod interval;
pub mod iso8601;
pub mod object_id;
pub mod object_ref;
#[doc(hidden)]
pub mod serde_support;
pub mod uid;

pub use interval::Interval;
pub use iso8601::{Date, DatePrecision, DateTime, Duration, Offset, Time, TimePrecision};
pub use object_id::{
    ArchetypeId, GenericId, HierObjectId, ObjectId, ObjectVersionId, TemplateId, TerminologyId,
    UidBasedId, VersionTreeId,
};
pub use object_ref::{
    AccessGroupRef, LocatableRef, NAMESPACE_LOCAL, NAMESPACE_UNKNOWN, ObjectRef, PartyRef,
    validate_namespace,
};
pub use uid::{InternetId, IsoOid, Uid, Uuid};
