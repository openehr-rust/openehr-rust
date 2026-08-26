#![no_main]
//! Identifier grammars: every `OBJECT_ID` descendant, and the `UID` family.
//!
//! Totality, plus round-trip through `Display`. An identifier that parses and
//! then renders differently would break every foreign key in the store, because
//! the rendered form is what reaches a column (`db:H5.6`).

#![forbid(unsafe_code)]

use libfuzzer_sys::fuzz_target;
use openehr::base::{
    ArchetypeId, HierObjectId, InternetId, IsoOid, ObjectVersionId, TemplateId, TerminologyId, Uid,
    Uuid, VersionTreeId,
};

macro_rules! round_trip {
    ($text:expr, $($ty:ty),+ $(,)?) => {$(
        if let Ok(v) = $text.parse::<$ty>() {
            assert_eq!(
                v.to_string(),
                $text,
                concat!(stringify!($ty), " did not round-trip through Display"),
            );
        }
    )+};
}

fuzz_target!(|text: &str| {
    round_trip!(
        text,
        Uid,
        IsoOid,
        Uuid,
        InternetId,
        HierObjectId,
        ObjectVersionId,
        VersionTreeId,
        ArchetypeId,
        TemplateId,
        TerminologyId,
    );

    // An OBJECT_VERSION_ID is three parts; reading them must not panic on any
    // input that parsed.
    if let Ok(v) = text.parse::<ObjectVersionId>() {
        let _ = v.object_id().to_string();
        let _ = v.creating_system_id().to_string();
        let tree = v.version_tree_id();
        let _ = (tree.trunk_version(), tree.branch_number(), tree.branch_version());
    }
});
