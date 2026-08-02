//! Encapsulated data: `DV_MULTIMEDIA` and `DV_PARSABLE`.
//!
//! These are the two data values whose content this crate does not understand:
//! a JPEG and an XML fragment. That makes them the two most dangerous, and the
//! module is built around that.
//!
//! # Bytes are not printed, ever
//!
//! A `DV_MULTIMEDIA` holds an image that may be a photograph of a patient.
//! Neither [`Display`](core::fmt::Display) nor [`Debug`](core::fmt::Debug)
//! renders the bytes here — `Debug` is hand-written to print the media type and
//! a length. This is the one place the crate overrides its usual rule that
//! `Debug` may show everything, because "everything" is a megabyte of base64
//! that will be pasted into a ticket.
//!
//! # Integrity checks are checked, not trusted
//!
//! `DV_MULTIMEDIA` carries `integrity_check` and `integrity_check_algorithm`.
//! A field saying "SHA-256: …" that nothing ever verifies is worse than no
//! field, because it reads as an assurance. [`DvMultimedia::verify_integrity`]
//! does the comparison, and reports *not checked* separately from *passed*.

use super::text::CodePhrase;
use super::uri::DvUri;
use crate::error::ParseError;
use core::fmt;
use serde::{Deserialize, Serialize};
use sha2::Digest as _;

/// Base64 is how canonical openEHR JSON carries `Array<Byte>`.
mod base64 {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
        for chunk in bytes.chunks(3) {
            let b = [
                chunk[0],
                chunk.get(1).copied().unwrap_or(0),
                chunk.get(2).copied().unwrap_or(0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            for i in 0..4 {
                if i <= chunk.len() {
                    out.push(ALPHABET[((n >> (18 - i * 6)) & 0x3F) as usize] as char);
                } else {
                    out.push('=');
                }
            }
        }
        out
    }

    pub fn decode(text: &str) -> Option<Vec<u8>> {
        let mut acc: u32 = 0;
        let mut bits = 0u32;
        let mut out = Vec::with_capacity(text.len() / 4 * 3);
        for c in text.bytes() {
            if c == b'=' {
                break;
            }
            // Whitespace inside base64 is common in XML-derived payloads and
            // carries no information; rejecting it would fail on data that
            // every other implementation reads.
            if c.is_ascii_whitespace() {
                continue;
            }
            let v = u32::try_from(ALPHABET.iter().position(|a| *a == c)?).ok()?;
            acc = (acc << 6) | v;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push(((acc >> bits) & 0xFF) as u8);
            }
        }
        Some(out)
    }
}

mod bytes_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    // `&Option<Vec<u8>>` rather than `Option<&[u8]>`: this is the signature
    // serde's `#[serde(with = ...)]` calls, and it is not ours to choose.
    #[allow(clippy::ref_option)]
    pub fn serialize<S: Serializer>(v: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            Some(bytes) => s.serialize_str(&super::base64::encode(bytes)),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        let raw = Option::<String>::deserialize(d)?;
        raw.map(|text| {
            super::base64::decode(&text).ok_or_else(|| serde::de::Error::custom("not valid base64"))
        })
        .transpose()
    }
}

/// The attributes every `DV_ENCAPSULATED` carries.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EncapsulatedAttrs {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    charset: Option<CodePhrase>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    language: Option<CodePhrase>,
}

impl EncapsulatedAttrs {
    /// The character set, if recorded.
    #[must_use]
    pub fn charset(&self) -> Option<&CodePhrase> {
        self.charset.as_ref()
    }

    /// The language, if recorded.
    #[must_use]
    pub fn language(&self) -> Option<&CodePhrase> {
        self.language.as_ref()
    }
}

/// Whether an integrity check could be performed, and whether it passed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum IntegrityCheck {
    /// The digest matches the data.
    Passed,
    /// The digest does not match the data. This is a finding.
    Failed,
    /// No digest was recorded. Nothing was checked, and nothing is claimed.
    NotRecorded,
    /// The data is not inline — only a URI is present — so there is nothing
    /// here to hash.
    NoInlineData,
    /// A digest was recorded under an algorithm this crate does not implement.
    ///
    /// Deliberately not `Failed`. Reporting an unsupported algorithm as a
    /// tamper finding burns an incident response on a dependency gap.
    UnsupportedAlgorithm,
}

impl IntegrityCheck {
    /// Whether this outcome is a positive assurance.
    ///
    /// Only [`IntegrityCheck::Passed`] is. In particular
    /// [`IntegrityCheck::NotRecorded`] is not, which is the distinction the
    /// enum exists to force.
    #[must_use]
    pub fn is_verified(self) -> bool {
        self == Self::Passed
    }
}

/// Binary content: an image, a waveform, a scanned document.
///
/// ```
/// use openehr::rm::data_types::{CodePhrase, DvMultimedia, IntegrityCheck};
///
/// let png = DvMultimedia::inline(
///     CodePhrase::new("IANA_media-types", "image/png").unwrap(),
///     b"\x89PNG\r\n\x1a\n".to_vec(),
/// );
/// assert_eq!(png.verify_integrity(), IntegrityCheck::NotRecorded);
///
/// let sealed = png.clone().with_sha256_integrity();
/// assert_eq!(sealed.verify_integrity(), IntegrityCheck::Passed);
/// ```
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvMultimedia {
    #[serde(flatten)]
    encapsulated: EncapsulatedAttrs,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    alternate_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    uri: Option<DvUri>,
    #[serde(skip_serializing_if = "Option::is_none", default, with = "bytes_serde")]
    data: Option<Vec<u8>>,
    media_type: CodePhrase,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    compression_algorithm: Option<CodePhrase>,
    #[serde(skip_serializing_if = "Option::is_none", default, with = "bytes_serde")]
    integrity_check: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    integrity_check_algorithm: Option<CodePhrase>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    thumbnail: Option<Box<DvMultimedia>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    size: Option<i64>,
}

// Omitting fields from Debug is the entire point of this impl, so the lint
// that checks for completeness is the wrong check here. The omitted fields are
// `data` (patient imagery), `thumbnail` (the same, smaller), and the
// encapsulated attributes, which are uninteresting rather than sensitive.
#[allow(clippy::missing_fields_in_debug)]
impl fmt::Debug for DvMultimedia {
    /// Prints shape, never content. See the module header.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DvMultimedia")
            .field("media_type", &self.media_type.code_string())
            .field("data_len", &self.data.as_ref().map(Vec::len))
            .field("uri", &self.uri.as_ref().map(DvUri::value))
            .field("has_integrity_check", &self.integrity_check.is_some())
            .finish()
    }
}

impl DvMultimedia {
    /// Builds a multimedia value holding its content inline.
    #[must_use]
    pub fn inline(media_type: CodePhrase, data: Vec<u8>) -> Self {
        Self {
            encapsulated: EncapsulatedAttrs::default(),
            alternate_text: None,
            uri: None,
            size: Some(i64::try_from(data.len()).unwrap_or(i64::MAX)),
            data: Some(data),
            media_type,
            compression_algorithm: None,
            integrity_check: None,
            integrity_check_algorithm: None,
            thumbnail: None,
        }
    }

    /// Builds a multimedia value that points at content held elsewhere.
    #[must_use]
    pub fn external(media_type: CodePhrase, uri: DvUri) -> Self {
        Self {
            encapsulated: EncapsulatedAttrs::default(),
            alternate_text: None,
            uri: Some(uri),
            data: None,
            media_type,
            compression_algorithm: None,
            integrity_check: None,
            integrity_check_algorithm: None,
            thumbnail: None,
            size: None,
        }
    }

    /// Records a text alternative, for accessibility and for readers that
    /// cannot render the media type.
    #[must_use]
    pub fn with_alternate_text(mut self, text: impl Into<String>) -> Self {
        self.alternate_text = Some(text.into());
        self
    }

    /// Computes and attaches a SHA-256 integrity check over the inline data.
    ///
    /// SHA-256 and not SHA-1, although openEHR's terminology lists both:
    /// SHA-1 collisions are published and a clinical record may be retained for
    /// decades. Reading an instance that names SHA-1 still works; this crate
    /// just will not create one.
    ///
    /// Does nothing if the value has no inline data — there is nothing to hash,
    /// and writing a digest of the empty string would be an assurance about
    /// content that is not here.
    #[must_use]
    pub fn with_sha256_integrity(mut self) -> Self {
        if let Some(data) = &self.data {
            let digest = sha2::Sha256::digest(data);
            self.integrity_check = Some(digest.to_vec());
            self.integrity_check_algorithm = CodePhrase::new(
                "openehr_integrity_check_algorithms",
                crate::terminology::integrity_check_algorithm::SHA_256,
            )
            .ok();
        }
        self
    }

    /// Verifies the recorded integrity check against the inline data.
    ///
    /// See [`IntegrityCheck`] for why "no digest" is a distinct outcome from
    /// "digest matched".
    #[must_use]
    pub fn verify_integrity(&self) -> IntegrityCheck {
        let Some(expected) = &self.integrity_check else {
            return IntegrityCheck::NotRecorded;
        };
        let Some(data) = &self.data else {
            return IntegrityCheck::NoInlineData;
        };
        let algorithm = self.integrity_check_algorithm.as_ref().map_or(
            crate::terminology::integrity_check_algorithm::SHA_256,
            |c| c.code_string(),
        );
        let actual = match algorithm {
            "SHA-256" => sha2::Sha256::digest(data).to_vec(),
            "SHA-512" => sha2::Sha512::digest(data).to_vec(),
            _ => return IntegrityCheck::UnsupportedAlgorithm,
        };
        // Constant time is not required here: the digest is public, and the
        // data it covers is already in hand. `subtle` is reserved for the
        // audit chain, where the comparison is against a secret.
        if actual == *expected {
            IntegrityCheck::Passed
        } else {
            IntegrityCheck::Failed
        }
    }

    /// The media type.
    #[must_use]
    pub fn media_type(&self) -> &CodePhrase {
        &self.media_type
    }

    /// The inline content, if any.
    #[must_use]
    pub fn data(&self) -> Option<&[u8]> {
        self.data.as_deref()
    }

    /// The external location, if any.
    #[must_use]
    pub fn uri(&self) -> Option<&DvUri> {
        self.uri.as_ref()
    }

    /// The text alternative, if recorded.
    #[must_use]
    pub fn alternate_text(&self) -> Option<&str> {
        self.alternate_text.as_deref()
    }

    /// The thumbnail, if any.
    #[must_use]
    pub fn thumbnail(&self) -> Option<&DvMultimedia> {
        self.thumbnail.as_deref()
    }

    /// The compression algorithm, if the content is compressed.
    #[must_use]
    pub fn compression_algorithm(&self) -> Option<&CodePhrase> {
        self.compression_algorithm.as_ref()
    }

    /// The algorithm that produced [`DvMultimedia::integrity_check`].
    #[must_use]
    pub fn integrity_check_algorithm(&self) -> Option<&CodePhrase> {
        self.integrity_check_algorithm.as_ref()
    }

    /// The recorded integrity check, if one was supplied.
    ///
    /// A digest, not content — safe to return, unlike [`DvMultimedia::data`].
    #[must_use]
    pub fn integrity_check(&self) -> Option<&[u8]> {
        self.integrity_check.as_deref()
    }

    /// The recorded size in bytes, if any.
    #[must_use]
    pub fn size(&self) -> Option<i64> {
        self.size
    }

    /// Whether the value has content to render: inline data or a URI.
    ///
    /// openEHR's invariant is that at least one is present. A `DV_MULTIMEDIA`
    /// with neither is an empty box with a media type on it.
    #[must_use]
    pub fn has_content(&self) -> bool {
        self.data.is_some() || self.uri.is_some()
    }
}

/// Content in a formalism this crate does not parse: an ADL fragment, an XML
/// document, an HL7 v2 message.
///
/// ```
/// use openehr::rm::data_types::DvParsable;
///
/// let timing = DvParsable::new("R2/2026-07-31T09:00:00Z/PT12H", "ISO8601").unwrap();
/// assert_eq!(timing.formalism(), "ISO8601");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DvParsable {
    #[serde(flatten)]
    encapsulated: EncapsulatedAttrs,
    value: String,
    formalism: String,
}

impl DvParsable {
    /// Builds parsable content.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the value or the formalism is empty. An empty
    /// formalism is the worse of the two: it leaves content that nothing can
    /// decide how to read.
    pub fn new(value: impl Into<String>, formalism: impl Into<String>) -> Result<Self, ParseError> {
        let value = value.into();
        let formalism = formalism.into();
        if value.is_empty() {
            return Err(ParseError::invariant("DV_PARSABLE", "Value_valid"));
        }
        if formalism.is_empty() {
            return Err(ParseError::invariant("DV_PARSABLE", "Formalism_valid"));
        }
        Ok(Self {
            encapsulated: EncapsulatedAttrs::default(),
            value,
            formalism,
        })
    }

    /// The content.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// The formalism the content is written in.
    #[must_use]
    pub fn formalism(&self) -> &str {
        &self.formalism
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media() -> CodePhrase {
        CodePhrase::new("IANA_media-types", "image/png").unwrap()
    }

    #[test]
    fn base64_round_trips_including_padding_lengths() {
        for len in 0..8usize {
            let bytes: Vec<u8> = (0..len)
                .map(|i| u8::try_from(i * 37 % 251).unwrap())
                .collect();
            let text = base64::encode(&bytes);
            assert_eq!(base64::decode(&text).unwrap(), bytes, "len {len}");
        }
        assert_eq!(
            base64::encode(b"any carnal pleasure."),
            "YW55IGNhcm5hbCBwbGVhc3VyZS4="
        );
        assert_eq!(
            base64::decode("YW55IGNhcm5hbCBwbGVhc3VyZS4=").unwrap(),
            b"any carnal pleasure."
        );
    }

    #[test]
    fn integrity_outcomes_are_distinguished() {
        let bare = DvMultimedia::inline(media(), b"abc".to_vec());
        assert_eq!(bare.verify_integrity(), IntegrityCheck::NotRecorded);
        assert!(!bare.verify_integrity().is_verified());

        let sealed = bare.clone().with_sha256_integrity();
        assert_eq!(sealed.verify_integrity(), IntegrityCheck::Passed);

        let mut tampered = sealed.clone();
        tampered.data = Some(b"abd".to_vec());
        assert_eq!(tampered.verify_integrity(), IntegrityCheck::Failed);

        // A digest recorded against content that is not here: the digest
        // cannot be checked, and reporting that as a pass would be the exact
        // false assurance this enum exists to prevent.
        let mut external =
            DvMultimedia::external(media(), DvUri::new("https://example.org/x.png").unwrap());
        assert_eq!(external.verify_integrity(), IntegrityCheck::NotRecorded);
        external.integrity_check = Some(sha2::Sha256::digest(b"abc").to_vec());
        assert_eq!(external.verify_integrity(), IntegrityCheck::NoInlineData);
        assert!(!external.verify_integrity().is_verified());
    }

    #[test]
    fn debug_does_not_print_the_bytes() {
        let marker = b"ZZ-DISTINCTIVE-9999";
        let m = DvMultimedia::inline(media(), marker.to_vec());
        let rendered = format!("{m:?}");
        assert!(!rendered.contains("ZZ-DISTINCTIVE"), "{rendered}");
        assert!(rendered.contains("data_len"));
    }

    #[test]
    fn sha1_is_readable_but_not_writable() {
        // Reading an instance that names SHA-1 must not panic or mis-report.
        let mut m = DvMultimedia::inline(media(), b"abc".to_vec());
        m.integrity_check = Some(vec![0; 20]);
        m.integrity_check_algorithm =
            CodePhrase::new("openehr_integrity_check_algorithms", "SHA-1").ok();
        assert_eq!(m.verify_integrity(), IntegrityCheck::UnsupportedAlgorithm);

        // And nothing this crate builds names it.
        let ours = DvMultimedia::inline(media(), b"abc".to_vec()).with_sha256_integrity();
        assert_eq!(
            ours.integrity_check_algorithm.unwrap().code_string(),
            "SHA-256"
        );
    }
}
