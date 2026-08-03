//! Verifying a container's history against what is stored.
//!
//! # What the chain alone does not prove
//!
//! [`openehr::security::audit_chain::Chain::verify`] walks the entries and
//! checks that each links to the last and that each digest is the hash of its
//! own pre-image. That is a check of the chain's *internal consistency*, and it
//! is complete for what a `ChainEntry` holds.
//!
//! A `ChainEntry` holds the **digest** of the content and never the content.
//! So an attacker who edits a stored document and leaves the five chain columns
//! untouched produces a history that `Chain::verify` pronounces `Verified`.
//! Every link matches, every digest recomputes, and the record says something
//! else than it did.
//!
//! [`verify_versions`] closes that by recomputing the content digest **from the
//! stored bytes**, which is the one check that requires the store and cannot
//! live in the library.
//!
//! # Why the stored bytes, and not the stored object
//!
//! Because re-deriving the document — parsing `data_json` back into a value and
//! canonicalising it again — would repair the tampering it is looking for.
//! `serde_json` reads `1.10` into an `f64` and writes `1.1`; a deserializer
//! drops attributes it does not model (`lib:J9.9`). Both turn an altered
//! document into a plausible one *before* it is hashed.
//!
//! Hashing what the column returned makes the check exact and makes it depend
//! on `M3.43`: the column has to give back the bytes it was given, which two
//! engines did not until `D-08`.
//!
//! # What it still does not prove
//!
//! **Truncation.** Delete the newest versions and what remains is a shorter
//! history that verifies perfectly. Only a checkpoint published where the
//! database administrator cannot reach it closes that (`M3.16c`,
//! [`crate::Store::chain_checkpoint`]).
//!
//! **Wholesale rewriting.** An attacker who edits a document *and* recomputes
//! every downstream digest produces a valid unkeyed chain. That is what the
//! keyed tag is for, and a keyed chain is only as good as a key the database
//! does not hold.

use crate::record::VersionRow;
use openehr::security::audit_chain::{
    BreakReason, Chain, ChainEntry, ChainKey, ChainStatus, Digest256, Tag,
};

/// What verifying a stored history found.
///
/// Not a boolean, for the same reason [`ChainStatus`] is not: two of these
/// variants mean *this could not be checked*, and reporting either as "fine"
/// is how a system comes to believe it has evidence it never had.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Integrity {
    /// Content, links, digests, and every tag verified against a held key.
    Verified,
    /// Content, links, and digests verified. **Nothing attests to who wrote
    /// them.**
    ///
    /// An unkeyed chain detects an edit by someone who cannot recompute the
    /// digests. It does not detect one by an attacker who can — which is
    /// anybody holding the database, since the algorithm is public. Reported
    /// separately from [`Integrity::Verified`] so that a report cannot say
    /// "verified" about a chain nobody signed.
    Unkeyed,
    /// The container has no versions. Nothing was checked.
    Empty,
    /// A breach. **This is a finding.**
    Broken {
        /// Position in the history, oldest first.
        at: usize,
        /// The version identifier. Design-time or system-minted, never
        /// clinical content, so naming it is safe (`X11.7a`).
        uid: String,
        /// What failed.
        reason: Breach,
    },
    /// The library reported a status this build does not recognise.
    ///
    /// [`ChainStatus`] is `#[non_exhaustive]`, so a wildcard arm is required —
    /// the opposite trade-off from [`crate::ColTy`], which is deliberately open
    /// to nothing so that adding a variant breaks all six dialects at compile
    /// time (`M3.30`). There a wildcard hides a defect; here the enum is open
    /// by design and the wildcard must land somewhere.
    ///
    /// It lands here, and here is **not a pass**. A build that met a status it
    /// was not written for has not verified anything, and folding that into
    /// [`Integrity::Verified`] is how a newer library's new failure mode would
    /// arrive as good news.
    Unrecognised,
    /// A tag names a key this process does not hold, so it could not be
    /// checked. **Not a pass.**
    UnknownKey {
        /// Position in the history, oldest first.
        at: usize,
        /// The version identifier.
        uid: String,
        /// The key named by the tag.
        key_id: String,
    },
}

/// What was wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Breach {
    /// **The stored document does not hash to its recorded content digest.**
    ///
    /// The row was edited. This is the one the chain cannot find on its own,
    /// and the one an investigation is usually looking for.
    ContentAltered,
    /// A version does not follow the one before it.
    ///
    /// A version was removed from the middle, reordered, or inserted.
    PreviousMismatch,
    /// An entry's digest is not the hash of its own pre-image.
    ///
    /// One of the chain columns was edited without the others.
    DigestMismatch,
    /// A tag does not verify against the key it names.
    ///
    /// The strongest finding available: it means the edit was made by someone
    /// without the key, and survived everything an unkeyed chain would catch.
    TagMismatch,
}

impl Integrity {
    /// Whether this is a finding that must be escalated.
    ///
    /// [`Integrity::Unkeyed`] is **not** a breach: an unsigned history that
    /// checks out is intact, merely unattested.
    #[must_use]
    pub const fn is_breach(&self) -> bool {
        matches!(self, Self::Broken { .. })
    }

    /// Whether the history may be described as intact.
    ///
    /// The deliberate asymmetry with [`Integrity::is_breach`]: three variants
    /// are neither. [`Integrity::Empty`] checked nothing,
    /// [`Integrity::UnknownKey`] could not finish, and
    /// [`Integrity::Unrecognised`] was not understood. `!is_breach()` is
    /// therefore **not** a licence to report a history as sound, and a caller
    /// that writes `if !is_breach()` has quietly turned three "cannot say"
    /// answers into a yes.
    #[must_use]
    pub const fn is_intact(&self) -> bool {
        matches!(self, Self::Verified | Self::Unkeyed)
    }
}

/// The canonical bytes a version's content was hashed over.
///
/// A version with no content — a deletion (`H5.2`) — hashed the canonical form
/// of `null`, because [`crate::record::VersionRow::project`] passes
/// `Option<&T>` and serde writes `None` as `null`. Getting this wrong would
/// make every deleted version report as altered, which is a false accusation
/// against exactly the rows an audit looks at hardest.
fn hashed_bytes(row: &VersionRow) -> &[u8] {
    row.data_json.as_deref().map_or(b"null", str::as_bytes)
}

/// Verifies a container's stored history.
///
/// `rows` must be every version of one container, **oldest first** — which is
/// the order [`crate::Store::all_versions`] returns (`V8.7a`). A caller that
/// sorts them differently is asking a different question and will get
/// [`Breach::PreviousMismatch`].
///
/// Stops at the first breach. After a break every later entry fails too, and
/// reporting a thousand consequences of one edit obscures which edit it was.
///
/// # Errors
///
/// Never returns `Err`; the result is the finding. Failures are values here
/// because "the history is broken" is an answer, not an error condition.
///
/// # Examples
///
/// ```
/// use openehr_store::integrity::{Integrity, verify_versions};
/// # use openehr_store::record::VersionRow;
/// # fn rows() -> Vec<VersionRow> { Vec::new() }
/// // An empty container is `Empty`, never `Verified`: nothing was checked.
/// assert_eq!(verify_versions(&rows(), &[]), Integrity::Empty);
/// ```
#[must_use]
pub fn verify_versions(rows: &[VersionRow], keys: &[&ChainKey]) -> Integrity {
    // No early return for an empty `rows`. There was one, and mutation testing
    // showed it made the `ChainStatus::Empty` arm below unreachable: deleting
    // that arm changed nothing. An empty slice hashes nothing, builds an empty
    // chain, and `verify` reports `Empty` — one path instead of two saying the
    // same thing (`lib:A-09`).

    // The check the library cannot make, made first: it is the one that finds
    // an edited record, and running it before the link walk means an altered
    // row is reported as altered rather than as whatever it breaks downstream.
    for (at, row) in rows.iter().enumerate() {
        if Digest256::of(hashed_bytes(row)) != Digest256::from_bytes(row.chain.content) {
            return Integrity::Broken {
                at,
                uid: row.uid.clone(),
                reason: Breach::ContentAltered,
            };
        }
    }

    // Links, digests, and tags go through the library's own walk, so that a
    // history in memory and a history in a database are judged by one
    // definition rather than two that agree until they do not.
    let chain = Chain::from_stored(
        rows.iter()
            .map(|row| ChainEntry {
                version_uid: row.uid.clone(),
                previous: Digest256::from_bytes(row.chain.previous),
                content: Digest256::from_bytes(row.chain.content),
                digest: Digest256::from_bytes(row.chain.digest),
                tag: row
                    .chain
                    .tag_key_id
                    .as_ref()
                    .zip(row.chain.tag_mac)
                    .map(|(key_id, mac)| Tag::from_stored(key_id, mac.to_vec())),
            })
            .collect(),
    );

    let uid = |at: usize| rows.get(at).map_or_else(String::new, |r| r.uid.clone());
    match chain.verify(keys) {
        ChainStatus::Verified => Integrity::Verified,
        ChainStatus::UnkeyedOnly => Integrity::Unkeyed,
        ChainStatus::Empty => Integrity::Empty,
        ChainStatus::UnknownKey { at, key_id } => Integrity::UnknownKey {
            at,
            uid: uid(at),
            key_id,
        },
        ChainStatus::Broken { at, reason } => Integrity::Broken {
            at,
            uid: uid(at),
            reason: match reason {
                BreakReason::PreviousMismatch => Breach::PreviousMismatch,
                BreakReason::TagMismatch => Breach::TagMismatch,
                // Every other break is a digest that does not recompute. Not a
                // wildcard for convenience: `BreakReason` is the library's
                // enum, and a variant added there must arrive here as *some*
                // breach rather than be silently dropped into a pass.
                _ => Breach::DigestMismatch,
            },
        },
        // Required, because `ChainStatus` is `#[non_exhaustive]`. Never a pass
        // — see `Integrity::Unrecognised`.
        _ => Integrity::Unrecognised,
    }
}

#[cfg(test)]
mod tests {
    use super::{Breach, Integrity, verify_versions};
    use crate::conformance::{RECORD, SYSTEM, sample_version};
    use crate::record::VersionRow;
    use openehr::security::Digest256;

    /// Why these exist, when `openehr-sqlite/tests/tamper.rs` already drives
    /// this module against a real database.
    ///
    /// Because it drives it from **another crate**. `cargo mutants` runs the
    /// tests of the crate it mutates, and on this file it missed **15 of 15**
    /// viable mutants: `is_breach` could return `true` for everything,
    /// `is_intact` could return either constant, the content-digest comparison
    /// could be inverted, and every match arm could be deleted, with
    /// `openehr-store`'s suite green throughout.
    ///
    /// The engine tests would have caught each one — in a different crate's
    /// job, after this crate had already reported success. A conformance suite
    /// shared by engines is the right place for *engine* behaviour; this file
    /// is pure logic and needs no engine (`lib:A-09`).
    fn rows(n: u32) -> Vec<VersionRow> {
        let mut previous = None;
        let mut out = Vec::new();
        for v in 1..=n {
            let version = sample_version(v, (v > 1).then(|| v - 1), v * 5);
            let row = VersionRow::project(&version, "c1", previous, None).expect("projects");
            previous = Some(row.chain.digest);
            out.push(row);
        }
        out
    }

    #[test]
    fn an_untouched_history_is_unkeyed_rather_than_verified() {
        let verdict = verify_versions(&rows(3), &[]);
        // Not `Verified`: nothing signed these entries, and a report that said
        // otherwise would claim a key backs them.
        assert_eq!(verdict, Integrity::Unkeyed);
        assert!(verdict.is_intact());
        assert!(!verdict.is_breach());
    }

    #[test]
    fn an_empty_container_is_neither_intact_nor_a_breach() {
        let verdict = verify_versions(&[], &[]);
        assert_eq!(verdict, Integrity::Empty);
        // The asymmetry `is_intact` exists for: `!is_breach()` is **not** a
        // licence to report a history as sound. Nothing was checked.
        assert!(!verdict.is_breach());
        assert!(!verdict.is_intact());
    }

    #[test]
    fn an_edited_document_is_reported_as_altered_content() {
        let mut rows = rows(3);
        // The chain columns are left exactly as written, so every link matches
        // and every digest recomputes. Only re-hashing the stored bytes finds
        // this.
        rows[1].data_json = Some(r#"{"altered":true}"#.to_owned());

        match verify_versions(&rows, &[]) {
            Integrity::Broken { at, uid, reason } => {
                assert_eq!(reason, Breach::ContentAltered);
                assert_eq!(at, 1);
                assert_eq!(uid, format!("{RECORD}::{SYSTEM}::2"));
            }
            other => panic!("an edited document was not detected: {other:?}"),
        }
    }

    #[test]
    fn a_deletion_hashes_as_null_rather_than_reporting_as_altered() {
        // A version with no content hashed the canonical form of `null`.
        // Getting this wrong would accuse every deleted version in every
        // record — exactly the rows an investigation reads hardest.
        let mut rows = rows(1);
        rows[0].data_json = None;
        rows[0].chain.content = *Digest256::of(b"null").as_bytes();
        // The entry digest no longer matches its own pre-image, so the content
        // check must pass and the *digest* check must be what fails.
        match verify_versions(&rows, &[]) {
            Integrity::Broken { reason, .. } => assert_eq!(reason, Breach::DigestMismatch),
            other => panic!("expected a digest mismatch, got {other:?}"),
        }
    }

    #[test]
    fn a_removed_version_breaks_the_link_rather_than_the_content() {
        let mut rows = rows(3);
        rows.remove(1);
        match verify_versions(&rows, &[]) {
            Integrity::Broken { at, reason, .. } => {
                assert_eq!(reason, Breach::PreviousMismatch);
                assert_eq!(at, 1);
            }
            other => panic!("a removed version was not detected: {other:?}"),
        }
    }

    #[test]
    fn a_rewritten_entry_digest_is_reported_as_a_digest_mismatch() {
        let mut rows = rows(2);
        rows[0].chain.digest = [0u8; 32];
        match verify_versions(&rows, &[]) {
            Integrity::Broken { at, reason, .. } => {
                assert_eq!(reason, Breach::DigestMismatch);
                assert_eq!(at, 0);
            }
            other => panic!("a rewritten digest was not detected: {other:?}"),
        }
    }

    #[test]
    fn a_tag_naming_a_key_this_process_does_not_hold_is_not_a_pass() {
        let mut rows = rows(1);
        rows[0].chain.tag_key_id = Some("k-unheld".to_owned());
        rows[0].chain.tag_mac = Some([9u8; 32]);

        match verify_versions(&rows, &[]) {
            Integrity::UnknownKey { at, uid, key_id } => {
                assert_eq!(at, 0);
                assert_eq!(key_id, "k-unheld");
                assert_eq!(uid, format!("{RECORD}::{SYSTEM}::1"));
            }
            other => panic!("expected UnknownKey, got {other:?}"),
        }
        // Not a breach and not intact: a check that could not be completed
        // must not report as one that was.
        let verdict = verify_versions(&rows, &[]);
        assert!(!verdict.is_breach());
        assert!(!verdict.is_intact());
    }

    /// A keyed history, and a keyed history whose tag was forged.
    ///
    /// Both `ChainStatus::Verified` and `BreakReason::TagMismatch` survived
    /// mutation until this existed: every earlier test used an unkeyed chain,
    /// so the two arms that only a **signed** history reaches were never taken.
    #[test]
    fn a_signed_history_verifies_and_a_forged_tag_does_not() {
        use openehr::security::ChainKey;

        let key = ChainKey::new("k1", vec![7u8; 32]).expect("key");
        let signed = |n: u32| {
            let mut previous = None;
            let mut out = Vec::new();
            for v in 1..=n {
                let version = sample_version(v, (v > 1).then(|| v - 1), v * 5);
                let row =
                    VersionRow::project(&version, "c1", previous, Some(&key)).expect("projects");
                previous = Some(row.chain.digest);
                out.push(row);
            }
            out
        };

        // Signed and held: the only path to `Verified`.
        let verdict = verify_versions(&signed(2), &[&key]);
        assert_eq!(verdict, Integrity::Verified);
        assert!(verdict.is_intact());
        assert!(!verdict.is_breach());

        // The tag rewritten under a key this process *does* hold. That is the
        // strongest finding available: it means the edit was made by someone
        // without the key and survived everything an unkeyed chain would catch.
        let mut forged = signed(2);
        forged[1].chain.tag_mac = Some([0u8; 32]);
        match verify_versions(&forged, &[&key]) {
            Integrity::Broken { at, reason, .. } => {
                assert_eq!(reason, Breach::TagMismatch);
                assert_eq!(at, 1);
            }
            other => panic!("a forged tag was not detected: {other:?}"),
        }
        // And a breach answers `is_breach`, which nothing had asserted in the
        // true direction.
        assert!(verify_versions(&forged, &[&key]).is_breach());
    }
}
