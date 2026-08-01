//! Serde glue for the identifier types.
//!
//! openEHR identifiers have two JSON shapes, and which one applies depends on
//! where the identifier sits:
//!
//! | Shape | Where | Example |
//! | --- | --- | --- |
//! | bare string | inside another identifier's `value` | `"87284370-…::ehr1.nhs.uk::2"` |
//! | typed object | as an RM attribute | `{"_type":"HIER_OBJECT_ID","value":"…"}` |
//!
//! Two macros, therefore. Both are `#[doc(hidden)]` implementation details.
//!
//! # Reading is lenient, writing is canonical
//!
//! [`crate::impl_valued_serde`] **accepts** a bare string where an object is
//! expected.
//! Real openEHR payloads do this constantly — `{"terminology_id": "SNOMED-CT"}`
//! appears in template tooling output, and rejecting it converts a cosmetic
//! divergence into an import failure. It always **writes** the typed object,
//! so nothing this crate emits inherits the leniency, and a round trip
//! normalises rather than propagating.
//!
//! The `_type` on input is checked when present and defaulted when absent: a
//! `_type` naming a *different* class is a genuine error, because the caller
//! believes the value is something it is not.

/// Serialize/deserialize a type as a bare JSON string via `Display`/`FromStr`.
#[doc(hidden)]
#[macro_export]
macro_rules! impl_string_serde {
    ($ty:ty, $name:expr) => {
        impl ::serde::Serialize for $ty {
            fn serialize<S: ::serde::Serializer>(
                &self,
                s: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $ty {
            fn deserialize<D: ::serde::Deserializer<'de>>(
                d: D,
            ) -> ::core::result::Result<Self, D::Error> {
                let raw = <::std::string::String as ::serde::Deserialize>::deserialize(d)?;
                raw.parse().map_err(|e: $crate::ParseError| {
                    ::serde::de::Error::custom(format!("{}: {}", $name, e.reason))
                })
            }
        }
    };
}

/// Serialize/deserialize an `OBJECT_ID` descendant as `{"_type": …, "value": …}`.
///
/// Extra keys carried by the sender are ignored rather than rejected: openEHR
/// adds attributes to identifier classes between minor releases (`OBJECT_REF`
/// gained `namespace`), and a strict reader rejects tomorrow's payload for
/// containing something it does not need.
#[doc(hidden)]
#[macro_export]
macro_rules! impl_valued_serde {
    ($ty:ty, $name:literal) => {
        impl ::serde::Serialize for $ty {
            fn serialize<S: ::serde::Serializer>(
                &self,
                s: S,
            ) -> ::core::result::Result<S::Ok, S::Error> {
                use ::serde::ser::SerializeStruct as _;
                let mut st = s.serialize_struct($name, 2)?;
                st.serialize_field("_type", $name)?;
                st.serialize_field("value", &self.to_string())?;
                st.end()
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $ty {
            fn deserialize<D: ::serde::Deserializer<'de>>(
                d: D,
            ) -> ::core::result::Result<Self, D::Error> {
                let raw = $crate::base::serde_support::ValuedId::deserialize_checked(d, $name)?;
                raw.parse().map_err(|e: $crate::ParseError| {
                    ::serde::de::Error::custom(format!("{}: {}", $name, e.reason))
                })
            }
        }
    };
}

/// The permissive input form behind [`crate::impl_valued_serde`].
#[doc(hidden)]
pub struct ValuedId;

impl ValuedId {
    /// Reads either `"text"` or `{"_type": …, "value": "text"}` and returns the
    /// `value`.
    ///
    /// # Errors
    ///
    /// Fails if the input is neither shape, if `value` is missing, or if a
    /// present `_type` names a different class.
    #[doc(hidden)]
    pub fn deserialize_checked<'de, D: serde::Deserializer<'de>>(
        d: D,
        expected: &'static str,
    ) -> Result<String, D::Error> {
        use serde::de::{Error as _, MapAccess, Visitor};

        struct V(&'static str);

        impl<'de> Visitor<'de> for V {
            type Value = String;

            fn expecting(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "a string or a {{\"value\": …}} object for {}", self.0)
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<String, E> {
                Ok(v.to_owned())
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<String, A::Error> {
                let mut value: Option<String> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "value" => value = Some(map.next_value()?),
                        "_type" => {
                            let ty: String = map.next_value()?;
                            // A generic id declared as something else is a
                            // caller error, not a formatting quirk: silently
                            // reinterpreting it would hand back an identifier
                            // of a class the sender did not mean.
                            if ty != self.0 {
                                return Err(A::Error::custom(format!(
                                    "_type is {ty}, expected {}",
                                    self.0
                                )));
                            }
                        }
                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                value.ok_or_else(|| A::Error::missing_field("value"))
            }
        }

        d.deserialize_any(V(expected))
    }
}
