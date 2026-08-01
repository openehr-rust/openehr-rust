//! The openEHR Reference Model.
//!
//! Four packages, in dependency order: [`data_types`] are the leaf values,
//! [`data_structures`] arrange them, [`common`] adds archetyping and change
//! control, and [`ehr`] and [`demographic`] are the two information models
//! built on all three.

pub mod common;
pub mod data_structures;
pub mod data_types;
pub mod demographic;
pub mod ehr;

/// Generates a zero-sized marker that serializes as a fixed `_type` string.
///
/// # Why a marker rather than nothing
///
/// openEHR requires `_type` wherever the declared type is abstract, and this
/// crate gets that for free on the classes modelled as Rust enums — serde's
/// internal tagging writes it. The classes modelled as plain structs
/// (`COMPOSITION`, `HISTORY`, `EVENT_CONTEXT`, …) have no enum above them and
/// so would emit nothing, yet real openEHR payloads and the published JSON
/// Schemas carry `_type` on them: a `COMPOSITION` is the resource a REST
/// client `POST`s, and it is identified by its `_type`.
///
/// The marker is ignored on input. On a concrete declared type the attribute is
/// redundant, and rejecting a payload whose `_type` disagrees would fail on
/// documents every other implementation accepts.
macro_rules! rm_type_tag {
    ($ty:ident, $name:literal) => {
        #[doc = concat!("Serializes as `\"_type\": \"", $name, "\"`; ignored on input.")]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        pub struct $ty;

        impl ::serde::Serialize for $ty {
            fn serialize<S: ::serde::Serializer>(
                &self,
                s: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                s.serialize_str($name)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $ty {
            fn deserialize<D: ::serde::Deserializer<'de>>(
                d: D,
            ) -> ::core::result::Result<Self, D::Error> {
                let _ignored = <::std::string::String as ::serde::Deserialize>::deserialize(d)?;
                Ok(Self)
            }
        }
    };
}

pub(crate) use rm_type_tag;
