//! A tamper-evident hash chain over an openEHR version history.
//!
//! # What openEHR gives you, and what it does not
//!
//! openEHR's change control is already an audit trail: every version carries
//! [`crate::rm::common::AuditDetails`] saying who committed it, when, and why,
//! and versions are append-only *by convention*. Convention is the gap. Nothing
//! in the model detects a version silently edited in the database, a version
//! removed from the middle of a history, or a `time_committed` moved backwards.
//!
//! This module closes that gap the way it can be closed in-process: each entry
//! digests its predecessor, so altering any entry invalidates every entry after
//! it.
//!
//! # State plainly what an unkeyed chain buys
//!
//! It detects **careless or unaware modification** — a migration, a stray
//! `UPDATE`, a restore from the wrong backup — and it supports an external
//! witness: publish the head digest somewhere the database administrator does
//! not control, and the chain becomes evidence.
//!
//! It does **not** stop an informed attacker with write access. The digests are
//! unkeyed over a published pre-image, so anyone who can rewrite the rows can
//! recompute the chain. For that, add the keyed tag: [`ChainKey`] produces an
//! `HMAC-SHA-256` over the same pre-image, and the key lives in the process,
//! never in the database. A key stored where the attacker already has write
//! access protects nothing.
//!
//! # Rules that are easy to get wrong
//!
//! - **Only a tag mismatch is a forgery finding.** A missing tag, a tag naming
//!   a key this process does not hold, and a malformed tag are each reported as
//!   what they are ([`ChainStatus`]). Reporting a key-distribution problem as
//!   tampering burns an incident response.
//! - **Verification is constant-time.** A timing oracle lets an attacker with
//!   write access recover a valid tag byte by byte.
//! - **Key ids travel with tags**, so rotation is additive. Without the id,
//!   rotating a key would invalidate all history at once — indistinguishable
//!   from mass tampering.
//! - **Never backfill a chain.** A chain assembled after the fact attests only
//!   that the rows look consistent *now*, which is exactly what an attacker who
//!   rewrote them would produce. [`Chain::new`] therefore takes no existing
//!   entries: a chain begins where it begins, and
//!   [`Chain::genesis_after`] records that it began late rather than pretending
//!   otherwise.
//! - **SHA-256, not SHA-1.** openEHR's `integrity_check_algorithms` group names
//!   seven — SHA-1, SHA-224, SHA-256, SHA-384, SHA-512, SHA-512/224, and
//!   SHA-512/256 — and this crate emits exactly one of them. SHA-1 is read,
//!   because openEHR lists it and data exists that names it, and never written:
//!   a clinical record may be retained for decades, longer than anyone can
//!   promise a construction will stand, and SHA-1 has already been outlived.

use crate::security::canonical::to_canonical_bytes;
use core::fmt;
// The trait is imported anonymously so its name does not collide with this
// module's `Mac` type, which is the tag itself rather than the algorithm.
use hmac::{Hmac, Mac as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use zeroize::{Zeroize, ZeroizeOnDrop};

type HmacSha256 = Hmac<Sha256>;

/// A 32-byte digest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Digest256(#[serde(with = "hex_bytes")] [u8; 32]);

impl Digest256 {
    /// The all-zero digest, used as the predecessor of the first entry.
    pub const GENESIS: Self = Self([0u8; 32]);

    /// A digest from its 32 bytes.
    ///
    /// Needed to reconstruct a chain from storage: a store keeps the digest as
    /// 32 raw bytes in a binary column (`db:M3.40`), and verification has to
    /// get them back into the type.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The SHA-256 digest **of these bytes**.
    ///
    /// [`Chain::append`] hashes a value by canonicalising it first, which is
    /// right when the value is in hand. A store verifying what it read back
    /// does not have the value — it has the canonical bytes, and must not
    /// parse them to re-derive it. Parsing loses: `serde_json` reads `1.10`
    /// into an `f64` and writes `1.1`, which is the precision loss `J9.13`
    /// forbids and `db:D-08` found an engine doing.
    ///
    /// So the store hashes the stored bytes directly, through this, rather
    /// than through a round trip that would quietly repair the very tampering
    /// it is looking for.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// The digest's bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The digest as lower-case hex.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex(&self.0)
    }
}

impl fmt::Display for Digest256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Lower-case hex, written with `fmt::Write` rather than by collecting a
/// `format!` per byte: a digest is written on every append and every
/// checkpoint, and 32 short allocations per call is 32 too many.
fn hex(bytes: &[u8]) -> String {
    use core::fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::hex(v))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let text = String::deserialize(d)?;
        if text.len() != 64 {
            return Err(serde::de::Error::custom("digest is not 64 hex characters"));
        }
        let mut out = [0u8; 32];
        for (i, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
                .map_err(|_| serde::de::Error::custom("digest is not hexadecimal"))?;
        }
        Ok(out)
    }
}

/// A keyed-tag key, scrubbed from memory when dropped.
///
/// The key id travels with every tag so that rotation is additive: after
/// rotation, entries written under the old key still verify against it, and
/// entries whose key id this process does not hold are reported as
/// [`ChainStatus::UnknownKey`] rather than as forgeries.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct ChainKey {
    #[zeroize(skip)]
    id: String,
    material: Vec<u8>,
}

impl ChainKey {
    /// The shortest key this type will accept, in bytes.
    ///
    /// 32 bytes is the output size of SHA-256, below which the tag is weaker
    /// than the hash it is built on. It is also long enough that a `changeme`
    /// placeholder reaching production fails loudly rather than yielding tags
    /// an attacker reproduces by guessing.
    pub const MIN_KEY_BYTES: usize = 32;

    /// Builds a key.
    ///
    /// # Errors
    ///
    /// Returns [`crate::ParseError`] if the key id is empty or the material is
    /// shorter than [`ChainKey::MIN_KEY_BYTES`].
    pub fn new(id: impl Into<String>, material: Vec<u8>) -> Result<Self, crate::ParseError> {
        let id = id.into();
        if id.is_empty() {
            return Err(crate::ParseError::invariant("CHAIN_KEY", "Id_valid"));
        }
        if material.len() < Self::MIN_KEY_BYTES {
            return Err(crate::ParseError::invariant("CHAIN_KEY", "Material_length"));
        }
        Ok(Self { id, material })
    }

    /// The key's identifier, which is written into every tag it produces.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    fn tag(&self, pre_image: &[u8]) -> Mac {
        let mut mac =
            HmacSha256::new_from_slice(&self.material).expect("HMAC accepts a key of any length");
        mac.update(pre_image);
        Mac(mac.finalize().into_bytes().into())
    }
}

/// An HMAC-SHA-256 tag, comparable **only** in constant time.
///
/// # Why this is a type and not a `Vec<u8>`
///
/// `X11.12` requires tag comparison to be constant-time, because a byte-by-byte
/// early return is a timing oracle: an attacker with write access recovers a
/// valid tag one byte at a time, and then their forgery verifies.
///
/// This deliberately implements **neither `PartialEq` nor `Eq`**, so
/// `expected == tag.mac` does not compile. The rule was previously kept by one
/// `ct_eq` call and the discipline not to replace it — and `==` is what anyone
/// simplifying this code would reach for first, would find compiles, and would
/// find passes every test.
///
/// It is the same trade as `db:ColTy` not being `#[non_exhaustive]` and as
/// PASETO over JWT (`db:PR12.14`): prefer the design where the dangerous thing
/// cannot be *expressed* over the one where it can be expressed and must be
/// caught.
///
/// It is not absolute, and the limit is worth stating. Nothing stops a
/// determined caller comparing `as_bytes()` directly, and **no test proves the
/// absence of `PartialEq`**: a `compile_fail` doctest was written for it and
/// then deleted, because `ChainKey::tag` is private, so the block failed to
/// compile whether or not `Mac` derived `PartialEq`. It passed for the wrong
/// reason, which a mutation check caught by adding the derive and watching the
/// doctest stay green.
///
/// What is left is a type-level property with one enforcing reader: the only
/// comparison in this crate is [`Mac::matches`], and `==` beside it does not
/// compile. That is why `X11.12` stays **?** in the conformance matrix.
pub struct Mac([u8; 32]);

impl Mac {
    /// The tag's bytes, for storage.
    ///
    /// A store persists these (`db:M3.16`) and reads them back into
    /// [`Tag::from_stored`]. Comparison MUST NOT go through here — use
    /// [`Mac::matches`].
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Whether a candidate tag is this one, compared in constant time.
    ///
    /// A candidate of the wrong length answers `false`; a tag's length is not
    /// secret, so returning early on it leaks nothing.
    #[must_use]
    pub fn matches(&self, candidate: &[u8]) -> bool {
        self.0.as_slice().ct_eq(candidate).unwrap_u8() == 1
    }
}

impl fmt::Debug for Mac {
    /// Prints the length and not the tag. A tag in a log is one an attacker no
    /// longer has to forge, and `{:?}` on a struct holding one is the usual way
    /// it escapes — the same reasoning as [`ChainKey`]'s own `Debug`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Mac").field("bytes", &self.0.len()).finish()
    }
}

impl fmt::Debug for ChainKey {
    /// Prints the key id and never the material. A key in a log is a key
    /// disclosed, and `{:?}` on a struct holding one is the commonest way it
    /// happens.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChainKey")
            .field("id", &self.id)
            .field("material", &"<redacted>")
            .finish()
    }
}

/// A keyed tag, with the id of the key that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    key_id: String,
    #[serde(with = "hex_vec")]
    mac: Vec<u8>,
}

impl Tag {
    /// The tag bytes.
    ///
    /// Exposed so a store can persist the tag; comparison still goes through
    /// [`ChainKey`] in constant time, and a caller comparing these directly
    /// with `==` reintroduces the timing oracle the tag exists to deny.
    #[must_use]
    pub fn mac(&self) -> &[u8] {
        &self.mac
    }

    /// The id of the key that produced the tag.
    #[must_use]
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Rebuilds a tag from its stored columns.
    ///
    /// The counterpart of [`Tag::mac`] and [`Tag::key_id`]: a store persists
    /// the two halves and needs them back in one object to verify. This
    /// constructs an *unverified* tag — it asserts nothing, and
    /// [`Chain::verify`] is what decides whether the key agrees.
    #[must_use]
    pub fn from_stored(key_id: impl Into<String>, mac: impl Into<Vec<u8>>) -> Self {
        Self {
            key_id: key_id.into(),
            mac: mac.into(),
        }
    }
}

mod hex_vec {
    use serde::{Deserialize, Deserializer, Serializer};

    // `&Vec<u8>` rather than `&[u8]`: this is the signature serde's
    // `#[serde(with = ...)]` calls, and it is not ours to choose.
    #[allow(clippy::ptr_arg)]
    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::hex(v))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(d)?;
        if text.len() % 2 != 0 {
            return Err(serde::de::Error::custom(
                "tag has an odd number of hex digits",
            ));
        }
        (0..text.len() / 2)
            .map(|i| {
                u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)
                    .map_err(|_| serde::de::Error::custom("tag is not hexadecimal"))
            })
            .collect()
    }
}

/// One link in the chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainEntry {
    /// The identifier of the version this entry covers, as text.
    pub version_uid: String,
    /// The digest of the previous entry, or [`Digest256::GENESIS`].
    pub previous: Digest256,
    /// The digest of the canonical content of this version.
    pub content: Digest256,
    /// This entry's own digest, over `previous || content || version_uid`.
    pub digest: Digest256,
    /// The keyed tag, if the chain is keyed.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tag: Option<Tag>,
}

impl ChainEntry {
    /// The bytes a digest and a tag are taken over.
    ///
    /// Composed as `previous || content || version_uid` with the uid last and
    /// length-free, because the two digests are fixed-width: no separator is
    /// needed and none is added, so the composition is unambiguous.
    fn pre_image(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + self.version_uid.len());
        out.extend_from_slice(self.previous.as_bytes());
        out.extend_from_slice(self.content.as_bytes());
        out.extend_from_slice(self.version_uid.as_bytes());
        out
    }
}

/// What verification found.
///
/// The variants are deliberately not collapsible into a boolean. Three of the
/// five are *this cannot be checked here*, and reporting any of them as a
/// failure produces an incident response against a configuration problem.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChainStatus {
    /// Every digest links, and every tag verified against a held key.
    Verified,
    /// A digest does not link, or a tag does not verify. **This is a finding.**
    Broken {
        /// Index of the first entry that failed.
        at: usize,
        /// What failed about it.
        reason: BreakReason,
    },
    /// The chain has no entries. Nothing was checked.
    Empty,
    /// The chain is unkeyed: digests link, and nothing attests to *who* wrote
    /// them.
    UnkeyedOnly,
    /// An entry names a key this process does not hold. **Not a forgery.**
    UnknownKey {
        /// Index of the entry.
        at: usize,
        /// The key id it names.
        key_id: String,
    },
}

impl ChainStatus {
    /// Whether the chain is verified end to end with keyed tags.
    ///
    /// [`ChainStatus::UnkeyedOnly`] answers `false`, which is the point: an
    /// unkeyed chain has integrity against accident and not against an
    /// attacker, and code that needs the second must be able to tell.
    #[must_use]
    pub fn is_fully_verified(&self) -> bool {
        matches!(self, Self::Verified)
    }

    /// Whether this outcome is evidence of tampering.
    #[must_use]
    pub fn is_finding(&self) -> bool {
        matches!(self, Self::Broken { .. })
    }
}

/// How a chain entry failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BreakReason {
    /// The entry's `previous` does not match the preceding entry's digest —
    /// an entry was inserted, removed, or reordered.
    PreviousMismatch,
    /// The entry's own digest does not match its contents — the entry was
    /// edited.
    DigestMismatch,
    /// The keyed tag does not verify — the entry was edited by someone without
    /// the key.
    TagMismatch,
}

/// An append-only hash chain over committed versions.
///
/// ```
/// use openehr::security::audit_chain::{Chain, ChainKey, ChainStatus};
///
/// let key = ChainKey::new("k1", vec![7u8; 32]).unwrap();
/// let mut chain = Chain::new();
/// chain.append("uid::sys::1", &serde_json::json!({"a": 1}), Some(&key)).unwrap();
/// chain.append("uid::sys::2", &serde_json::json!({"a": 2}), Some(&key)).unwrap();
///
/// assert_eq!(chain.verify(&[&key]), ChainStatus::Verified);
///
/// // Edit an entry the way a stray UPDATE would, and the chain says so.
/// let mut tampered = chain.clone();
/// tampered.entries_mut()[0].content = openehr::security::audit_chain::Digest256::GENESIS;
/// assert!(tampered.verify(&[&key]).is_finding());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Chain {
    entries: Vec<ChainEntry>,
    /// Set when this chain continues from storage rather than from nothing.
    resumed_head: Option<Digest256>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    genesis_note: Option<String>,
}

impl Chain {
    /// A new, empty chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A new chain that records it began after existing history.
    ///
    /// Use this when adding chaining to a record that already has versions.
    /// The alternative — computing entries for the existing versions — is
    /// backfilling, and a backfilled chain attests only that the rows look
    /// consistent now.
    ///
    /// ```
    /// use openehr::security::audit_chain::Chain;
    ///
    /// let chain = Chain::genesis_after("chain added 2026-07-31; 412 prior versions unchained");
    /// assert!(chain.genesis_note().is_some());
    /// ```
    #[must_use]
    pub fn genesis_after(note: impl Into<String>) -> Self {
        Self {
            entries: Vec::new(),
            resumed_head: None,
            genesis_note: Some(note.into()),
        }
    }

    /// What the chain says about history that predates it.
    #[must_use]
    pub fn genesis_note(&self) -> Option<&str> {
        self.genesis_note.as_deref()
    }

    /// A chain whose next append links to `head`, with no entries in memory.
    ///
    /// This is what a store needs. A database holds the whole history and loads
    /// one predecessor digest to append the next link; reading every prior
    /// entry into memory to add one would make a commit cost the length of the
    /// record's history.
    ///
    /// The returned chain cannot [`Chain::verify`] itself — it has no entries to
    /// verify. Verification is a separate operation over rows read back from
    /// storage, and conflating the two would let a caller believe an append had
    /// checked the history it appended to. It had not.
    #[must_use]
    pub fn resume_from(head: Digest256) -> Self {
        Self {
            entries: Vec::new(),
            resumed_head: Some(head),
            genesis_note: None,
        }
    }

    /// Rebuilds a chain from entries read out of storage, **oldest first**.
    ///
    /// This is how a store verifies: the five chain columns of each row become
    /// a [`ChainEntry`], and [`Chain::verify`] then applies one definition of
    /// what linking means to both the in-memory and the persisted case. A store
    /// that reimplemented the walk would eventually disagree with this one, and
    /// the disagreement would be discovered during an investigation.
    ///
    /// It asserts nothing about the entries. Everything here is untrusted input
    /// — that is the point — and `verify` is what decides.
    ///
    /// **It does not check content.** A `ChainEntry` holds the digest of the
    /// content, never the content, so nothing here can tell whether the stored
    /// document still hashes to it. That check needs the document and belongs
    /// to the store (`db:M3.16`); a chain over rows whose bodies were edited
    /// verifies perfectly.
    #[must_use]
    pub fn from_stored(entries: Vec<ChainEntry>) -> Self {
        Self {
            entries,
            resumed_head: None,
            genesis_note: None,
        }
    }

    /// The entries, oldest first.
    #[must_use]
    pub fn entries(&self) -> &[ChainEntry] {
        &self.entries
    }

    /// Mutable access to the entries.
    ///
    /// Exists so that tests can corrupt a chain and watch verification catch
    /// it — a chain whose failure path is never exercised is a chain nobody
    /// knows works. It is not for production use, and there is no
    /// `push`-shaped alternative that would let a caller add an unlinked entry.
    #[must_use]
    pub fn entries_mut(&mut self) -> &mut [ChainEntry] {
        &mut self.entries
    }

    /// The digest at the head of the chain.
    ///
    /// This is the value to publish to an external witness. It is the whole of
    /// what makes an unkeyed chain useful against an attacker with database
    /// access: they can recompute every digest, and they cannot change what
    /// was already published elsewhere.
    #[must_use]
    pub fn head(&self) -> Digest256 {
        self.entries.last().map_or(
            // A resumed chain's head is the digest it was resumed from, not
            // GENESIS: treating an empty-but-resumed chain as fresh would link
            // the next entry to nothing and silently start a second chain.
            self.resumed_head.unwrap_or(Digest256::GENESIS),
            |e| e.digest,
        )
    }

    /// How many entries the chain holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the chain has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Appends an entry covering one version's content, returning the new head
    /// digest.
    ///
    /// Returning the digest rather than a reference to the entry is what a
    /// caller actually needs: the head is the value published to an external
    /// witness, and handing back a borrow of the chain would prevent appending
    /// the next entry while holding it.
    ///
    /// # Errors
    ///
    /// Returns [`serde_json::Error`] if the content cannot be canonicalised.
    pub fn append<T: serde::Serialize>(
        &mut self,
        version_uid: impl Into<String>,
        content: &T,
        key: Option<&ChainKey>,
    ) -> Result<Digest256, serde_json::Error> {
        let content_digest = Digest256(Sha256::digest(to_canonical_bytes(content)?).into());
        let mut entry = ChainEntry {
            version_uid: version_uid.into(),
            previous: self.head(),
            content: content_digest,
            digest: Digest256::GENESIS,
            tag: None,
        };
        let pre_image = entry.pre_image();
        entry.digest = Digest256(Sha256::digest(&pre_image).into());
        if let Some(key) = key {
            entry.tag = Some(Tag {
                key_id: key.id().to_owned(),
                mac: key.tag(&pre_image).as_bytes().to_vec(),
            });
        }
        let digest = entry.digest;
        self.entries.push(entry);
        Ok(digest)
    }

    /// Verifies the whole chain against the keys this process holds.
    ///
    /// Stops at the first failure and reports its index, because after a break
    /// every later entry fails too and reporting a thousand consequences of one
    /// edit obscures which edit it was.
    #[must_use]
    pub fn verify(&self, keys: &[&ChainKey]) -> ChainStatus {
        if self.entries.is_empty() {
            return ChainStatus::Empty;
        }
        let mut previous = Digest256::GENESIS;
        let mut any_tag = false;
        for (i, entry) in self.entries.iter().enumerate() {
            if entry.previous != previous {
                return ChainStatus::Broken {
                    at: i,
                    reason: BreakReason::PreviousMismatch,
                };
            }
            let pre_image = entry.pre_image();
            let recomputed = Digest256(Sha256::digest(&pre_image).into());
            if recomputed != entry.digest {
                return ChainStatus::Broken {
                    at: i,
                    reason: BreakReason::DigestMismatch,
                };
            }
            if let Some(tag) = &entry.tag {
                any_tag = true;
                let Some(key) = keys.iter().find(|k| k.id() == tag.key_id) else {
                    return ChainStatus::UnknownKey {
                        at: i,
                        key_id: tag.key_id.clone(),
                    };
                };
                let expected = key.tag(&pre_image);
                // Constant time: a byte-by-byte early return here is a timing
                // oracle that hands an attacker with write access a valid tag.
                // Constant time, and `==` on a `Mac` does not compile.
                if !expected.matches(&tag.mac) {
                    return ChainStatus::Broken {
                        at: i,
                        reason: BreakReason::TagMismatch,
                    };
                }
            }
            previous = entry.digest;
        }
        if any_tag {
            ChainStatus::Verified
        } else {
            ChainStatus::UnkeyedOnly
        }
    }

    /// A checkpoint suitable for a long-retention log or an external witness.
    ///
    /// Carries **no patient data** — a count, a head digest, and the last
    /// version's identifier — which is what makes it safe to ship somewhere
    /// clinical data must not go, and what makes a log-based witness practical.
    ///
    /// ```
    /// use openehr::security::audit_chain::Chain;
    ///
    /// let mut chain = Chain::new();
    /// chain.append("uid::sys::1", &serde_json::json!({"name": "A Patient"}), None).unwrap();
    /// let checkpoint = chain.checkpoint();
    /// assert!(!checkpoint.contains("Patient"));
    /// ```
    #[must_use]
    pub fn checkpoint(&self) -> String {
        format!(
            "entries={} head={} last_version={}",
            self.entries.len(),
            self.head(),
            self.entries.last().map_or("-", |e| e.version_uid.as_str())
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn key(id: &str, byte: u8) -> ChainKey {
        ChainKey::new(id, vec![byte; 32]).unwrap()
    }

    #[test]
    fn a_clean_chain_verifies_and_an_edited_one_does_not() {
        let k = key("k1", 7);
        let mut chain = Chain::new();
        chain
            .append("uid::sys::1", &json!({"a": 1}), Some(&k))
            .unwrap();
        chain
            .append("uid::sys::2", &json!({"a": 2}), Some(&k))
            .unwrap();
        assert_eq!(chain.verify(&[&k]), ChainStatus::Verified);

        let mut edited = chain.clone();
        edited.entries_mut()[1].content = Digest256::GENESIS;
        assert_eq!(
            edited.verify(&[&k]),
            ChainStatus::Broken {
                at: 1,
                reason: BreakReason::DigestMismatch
            }
        );
    }

    #[test]
    fn removing_an_entry_from_the_middle_breaks_the_link() {
        let k = key("k1", 7);
        let mut chain = Chain::new();
        for i in 1..=3 {
            chain
                .append(format!("uid::sys::{i}"), &json!({"a": i}), Some(&k))
                .unwrap();
        }
        let mut cut = chain.clone();
        cut.entries.remove(1);
        assert_eq!(
            cut.verify(&[&k]),
            ChainStatus::Broken {
                at: 1,
                reason: BreakReason::PreviousMismatch
            }
        );
    }

    #[test]
    fn an_unheld_key_is_reported_as_such_and_not_as_forgery() {
        let writer = key("k1", 7);
        let other = key("k2", 9);
        let mut chain = Chain::new();
        chain
            .append("uid::sys::1", &json!({"a": 1}), Some(&writer))
            .unwrap();
        // The reader holds a different key. This is a key-distribution problem,
        // and reporting it as tampering would start an incident response.
        let status = chain.verify(&[&other]);
        assert!(matches!(status, ChainStatus::UnknownKey { at: 0, .. }));
        assert!(!status.is_finding());
    }

    #[test]
    fn a_forged_tag_under_a_held_key_is_a_finding() {
        let k = key("k1", 7);
        let mut chain = Chain::new();
        chain
            .append("uid::sys::1", &json!({"a": 1}), Some(&k))
            .unwrap();
        let mut forged = chain.clone();
        forged.entries_mut()[0].tag.as_mut().unwrap().mac[0] ^= 0xFF;
        assert_eq!(
            forged.verify(&[&k]),
            ChainStatus::Broken {
                at: 0,
                reason: BreakReason::TagMismatch
            }
        );
    }

    #[test]
    fn an_unkeyed_chain_does_not_claim_full_verification() {
        let mut chain = Chain::new();
        chain.append("uid::sys::1", &json!({"a": 1}), None).unwrap();
        let status = chain.verify(&[]);
        assert_eq!(status, ChainStatus::UnkeyedOnly);
        assert!(!status.is_fully_verified());
        assert!(!status.is_finding());
    }

    #[test]
    fn key_rotation_is_additive() {
        let old = key("2025", 1);
        let new = key("2026", 2);
        let mut chain = Chain::new();
        chain
            .append("uid::sys::1", &json!({"a": 1}), Some(&old))
            .unwrap();
        chain
            .append("uid::sys::2", &json!({"a": 2}), Some(&new))
            .unwrap();
        // Holding both keys verifies the whole history across the rotation.
        assert_eq!(chain.verify(&[&old, &new]), ChainStatus::Verified);
        // Holding only the new one reports the old entry as unknown-key, not
        // as forgery — which is what makes rotation safe to perform.
        assert!(matches!(
            chain.verify(&[&new]),
            ChainStatus::UnknownKey { at: 0, .. }
        ));
    }

    #[test]
    fn short_keys_and_empty_ids_are_refused() {
        assert!(ChainKey::new("k", vec![0u8; 31]).is_err());
        assert!(ChainKey::new("", vec![0u8; 32]).is_err());
        assert!(ChainKey::new("k", vec![0u8; 32]).is_ok());
    }

    #[test]
    fn key_debug_does_not_print_the_material() {
        let k = ChainKey::new("k1", vec![0xAB; 32]).unwrap();
        let rendered = format!("{k:?}");
        assert!(
            !rendered.contains("171") && !rendered.contains("ab"),
            "{rendered}"
        );
        assert!(rendered.contains("redacted"));
    }

    #[test]
    fn a_checkpoint_carries_no_patient_data() {
        let mut chain = Chain::new();
        chain
            .append(
                "uid::sys::1",
                &json!({"name": {"value": "ZZ-DISTINCTIVE-9999"}}),
                None,
            )
            .unwrap();
        let checkpoint = chain.checkpoint();
        assert!(!checkpoint.contains("ZZ-DISTINCTIVE"), "{checkpoint}");
        assert!(checkpoint.contains("entries=1"));
    }

    #[test]
    fn the_same_content_in_a_different_key_order_chains_identically() {
        // The canonicalisation guarantee, as a chain-level test: two
        // serializers disagreeing on key order must not look like tampering.
        let a: serde_json::Value = serde_json::from_str(r#"{"b":1,"a":2}"#).unwrap();
        let b: serde_json::Value = serde_json::from_str(r#"{"a":2,"b":1}"#).unwrap();
        let mut one = Chain::new();
        let mut two = Chain::new();
        one.append("uid::sys::1", &a, None).unwrap();
        two.append("uid::sys::1", &b, None).unwrap();
        assert_eq!(one.head(), two.head());
    }

    #[test]
    fn an_empty_chain_reports_empty_rather_than_verified() {
        assert_eq!(Chain::new().verify(&[]), ChainStatus::Empty);
        assert!(!Chain::new().verify(&[]).is_fully_verified());
    }
}
