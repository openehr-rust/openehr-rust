//! The openEHR support terminology.
//!
//! The reference model refers to controlled vocabularies **by code**, not by
//! name: `AUDIT_DETAILS.change_type` is a `DV_CODED_TEXT` whose
//! `defining_code.code_string` is `"249"` and whose `value` is `"creation"`.
//! Instances therefore carry bare numbers, and code that does not have the
//! terminology in front of it produces records that look valid and mean
//! nothing.
//!
//! # Source and verification
//!
//! Every group here was transcribed from the computable terminology in
//! [`openEHR/specifications-TERM`][term] (`computable/XML/en/openehr_terminology.xml`,
//! read 2026-07-31), which is the normative machine-readable artifact. It is
//! **not** derived from the older `openEHR/terminology` repository, which is
//! still online and disagrees: that copy gives `435` for `episodic` where the
//! current one gives `451`, omits `815|report|`, and omits `816|restoration|`
//! and `817|format conversion|` from the change types. A crate seeded from the
//! wrong copy would reject valid instances and mint invalid ones, so the
//! provenance is recorded here rather than assumed.
//!
//! [term]: https://github.com/openEHR/specifications-TERM
//!
//! # What this module does not do
//!
//! It does not resolve external terminologies. `SNOMED-CT`, `LOINC`, and
//! `ICD-10` codes appear throughout openEHR instances and this crate treats
//! them as opaque: it checks the `TERMINOLOGY_ID` grammar and nothing else.
//! Validating a SNOMED expression requires a terminology server, and a crate
//! that pretended otherwise would report "valid" for a code that does not
//! exist.
//!
//! ```
//! use openehr::terminology::{self, audit_change_type};
//!
//! assert_eq!(terminology::rubric(audit_change_type::GROUP, "249"), Some("creation"));
//! assert_eq!(terminology::rubric(audit_change_type::GROUP, "999"), None);
//!
//! // The constants are the readable way to say the same thing.
//! assert_eq!(audit_change_type::CREATION, "249");
//! ```

use crate::rm::data_types::{CodePhrase, DvCodedText};

/// One code and its rubric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Concept {
    /// The code, as it appears in `code_string`.
    pub code: &'static str,
    /// The English rubric, as it appears in `DV_CODED_TEXT.value`.
    pub rubric: &'static str,
}

/// A named set of codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Group {
    /// The group's identifier, such as `audit_change_type`.
    pub id: &'static str,
    /// The concepts in the group.
    pub concepts: &'static [Concept],
}

impl Group {
    /// The rubric for a code in this group.
    #[must_use]
    pub fn rubric(&self, code: &str) -> Option<&'static str> {
        self.concepts
            .iter()
            .find(|c| c.code == code)
            .map(|c| c.rubric)
    }

    /// Whether the code belongs to this group.
    #[must_use]
    pub fn contains(&self, code: &str) -> bool {
        self.rubric(code).is_some()
    }

    /// Builds the `DV_CODED_TEXT` openEHR expects for a code in this group.
    ///
    /// Returns `None` for a code the group does not define, rather than
    /// constructing a coded text with an invented rubric. A `DV_CODED_TEXT`
    /// whose `value` does not match its `defining_code`'s rubric is exactly the
    /// defect `DvCodedText`'s own invariant exists to catch.
    ///
    /// ```
    /// use openehr::terminology::{self, null_flavour};
    ///
    /// let masked = null_flavour::GROUP.coded_text(null_flavour::MASKED).unwrap();
    /// assert_eq!(masked.value(), "masked");
    /// assert_eq!(masked.defining_code().terminology_id().name(), "openehr");
    /// ```
    #[must_use]
    pub fn coded_text(&self, code: &str) -> Option<DvCodedText> {
        let rubric = self.rubric(code)?;
        let phrase = CodePhrase::openehr(code).ok()?;
        DvCodedText::new(rubric, phrase).ok()
    }
}

/// The rubric for a code in a group.
#[must_use]
pub fn rubric(group: Group, code: &str) -> Option<&'static str> {
    group.rubric(code)
}

/// Every group defined by the openEHR support terminology that this crate
/// carries.
///
/// The three `extract_*` groups the terminology also defines are absent: the
/// EHR Extract model is out of scope (`S1.6`), and shipping its codes would
/// imply support for a model this crate cannot build.
pub const GROUPS: &[Group] = &[
    audit_change_type::GROUP,
    attestation_reason::GROUP,
    composition_category::GROUP,
    setting::GROUP,
    null_flavour::GROUP,
    version_lifecycle_state::GROUP,
    event_math_function::GROUP,
    term_mapping_purpose::GROUP,
    subject_relationship::GROUP,
    participation_function::GROUP,
    participation_mode::GROUP,
    instruction_state::GROUP,
    instruction_transition::GROUP,
    normal_status::GROUP,
    compression_algorithm::GROUP,
    integrity_check_algorithm::GROUP,
];

/// Finds the group with the given identifier.
#[must_use]
pub fn group(id: &str) -> Option<Group> {
    GROUPS.iter().copied().find(|g| g.id == id)
}

macro_rules! group {
    (
        $(#[$attr:meta])*
        $module:ident, $id:literal, { $( $konst:ident = $code:literal => $rubric:literal ),* $(,)? }
    ) => {
        $(#[$attr])*
        pub mod $module {
            use super::{Concept, Group};

            /// The group.
            pub const GROUP: Group = Group {
                id: $id,
                concepts: &[ $( Concept { code: $code, rubric: $rubric } ),* ],
            };

            $(
                #[doc = concat!("`", $code, "|", $rubric, "|`")]
                pub const $konst: &str = $code;
            )*
        }
    };
}

group! {
    /// Why a version was committed — `AUDIT_DETAILS.change_type`.
    ///
    /// `CONTRIBUTION.audit.change_type` is restricted to a subset of these; see
    /// [`crate::rm::common::Contribution`].
    audit_change_type, "audit_change_type", {
        CREATION = "249" => "creation",
        AMENDMENT = "250" => "amendment",
        MODIFICATION = "251" => "modification",
        SYNTHESIS = "252" => "synthesis",
        DELETED = "523" => "deleted",
        ATTESTATION = "666" => "attestation",
        RESTORATION = "816" => "restoration",
        FORMAT_CONVERSION = "817" => "format conversion",
        UNKNOWN = "253" => "unknown",
    }
}

group! {
    /// Why content was attested — `ATTESTATION.reason`.
    attestation_reason, "attestation_reason", {
        SIGNED = "240" => "signed",
        WITNESSED = "648" => "witnessed",
    }
}

group! {
    /// How long a composition's content stays current —
    /// `COMPOSITION.category`.
    ///
    /// This is the attribute that decides whether a composition is a snapshot
    /// of an encounter or a running list. Getting it wrong makes a medication
    /// list behave like a one-off note.
    composition_category, "composition_category", {
        PERSISTENT = "431" => "persistent",
        EVENT = "433" => "event",
        EPISODIC = "451" => "episodic",
        REPORT = "815" => "report",
    }
}

group! {
    /// The care setting of an event — `EVENT_CONTEXT.setting`.
    setting, "setting", {
        HOME = "225" => "home",
        EMERGENCY_CARE = "227" => "emergency care",
        PRIMARY_MEDICAL_CARE = "228" => "primary medical care",
        PRIMARY_NURSING_CARE = "229" => "primary nursing care",
        PRIMARY_ALLIED_HEALTH_CARE = "230" => "primary allied health care",
        MIDWIFERY_CARE = "231" => "midwifery care",
        SECONDARY_MEDICAL_CARE = "232" => "secondary medical care",
        SECONDARY_NURSING_CARE = "233" => "secondary nursing care",
        SECONDARY_ALLIED_HEALTH_CARE = "234" => "secondary allied health care",
        COMPLEMENTARY_HEALTH_CARE = "235" => "complementary health care",
        DENTAL_CARE = "236" => "dental care",
        NURSING_HOME_CARE = "237" => "nursing home care",
        MENTAL_HEALTHCARE = "802" => "mental healthcare",
        OTHER_CARE = "238" => "other care",
    }
}

group! {
    /// Why an `ELEMENT` has no value — `ELEMENT.null_flavour`.
    ///
    /// These four are not interchangeable and the difference is clinical.
    /// `masked` means the value exists and is withheld; `no information` means
    /// nobody looked; `unknown` means somebody looked and could not find out;
    /// `not applicable` means the question does not arise. A pipeline that maps
    /// all four to SQL `NULL` has destroyed the distinction between "we have no
    /// allergy history" and "the patient has no allergies".
    null_flavour, "null_flavours", {
        NO_INFORMATION = "271" => "no information",
        UNKNOWN = "253" => "unknown",
        MASKED = "272" => "masked",
        NOT_APPLICABLE = "273" => "not applicable",
    }
}

group! {
    /// A version's completeness — `VERSION.lifecycle_state`.
    version_lifecycle_state, "version_lifecycle_state", {
        COMPLETE = "532" => "complete",
        INCOMPLETE = "553" => "incomplete",
        DELETED = "523" => "deleted",
        INACTIVE = "800" => "inactive",
        ABANDONED = "801" => "abandoned",
    }
}

group! {
    /// What an interval event summarises — `INTERVAL_EVENT.math_function`.
    event_math_function, "event_math_function", {
        MINIMUM = "145" => "minimum",
        MAXIMUM = "144" => "maximum",
        MODE = "267" => "mode",
        MEDIAN = "268" => "median",
        MEAN = "146" => "mean",
        CHANGE = "147" => "change",
        TOTAL = "148" => "total",
        VARIATION = "149" => "variation",
        DECREASE = "521" => "decrease",
        INCREASE = "522" => "increase",
        ACTUAL = "640" => "actual",
    }
}

group! {
    /// Why a term mapping was made — `TERM_MAPPING.purpose`.
    term_mapping_purpose, "term_mapping_purpose", {
        PUBLIC_HEALTH = "669" => "public health",
        REIMBURSEMENT = "670" => "reimbursement",
        RESEARCH_STUDY = "671" => "research study",
    }
}

group! {
    /// How an entry's subject relates to the record's subject —
    /// `PARTY_RELATED.relationship`.
    ///
    /// `0|self|` is the default and the one that matters most: an entry about
    /// anyone else in the record — a family history, a donor, a foetus — must
    /// say so, or it reads as a finding about the patient.
    subject_relationship, "subject_relationship", {
        SELF = "0" => "self",
        FOETUS = "3" => "foetus",
        DONOR = "6" => "donor",
        MATERNAL_GRANDMOTHER = "7" => "maternal grandmother",
        MATERNAL_GRANDFATHER = "8" => "maternal grandfather",
        FATHER = "9" => "father",
        MOTHER = "10" => "mother",
        PARTNER_SPOUSE = "22" => "partner/spouse",
        BROTHER = "23" => "brother",
        SISTER = "24" => "sister",
        STEP_OR_HALF_BROTHER = "25" => "step or half brother",
        STEP_OR_HALF_SISTER = "26" => "step or half sister",
        SIBLING = "27" => "sibling",
        CHILD = "28" => "child",
        DAUGHTER = "29" => "daughter",
        SON = "31" => "son",
        PATERNAL_GRANDFATHER = "36" => "paternal grandfather",
        PATERNAL_GRANDMOTHER = "37" => "paternal grandmother",
        MATERNAL_UNCLE = "38" => "maternal uncle",
        MATERNAL_AUNT = "39" => "maternal aunt",
        PATERNAL_UNCLE = "40" => "paternal uncle",
        PATERNAL_AUNT = "41" => "paternal aunt",
        NEONATE = "189" => "neonate",
        UNKNOWN = "253" => "unknown",
        PARENT = "254" => "parent",
        BIOLOGICAL_MOTHER = "255" => "biological mother",
        BIOLOGICAL_FATHER = "256" => "biological father",
        COUSIN = "257" => "cousin",
        ADOPTIVE_MOTHER = "258" => "adoptive mother",
        ADOPTIVE_FATHER = "259" => "adoptive father",
        ADOPTED_SON = "260" => "adopted son",
        ADOPTED_DAUGHTER = "261" => "adopted daughter",
        STEP_MOTHER = "262" => "step mother",
        STEP_FATHER = "263" => "step father",
        GUARDIAN = "264" => "guardian",
        COHABITEE = "265" => "cohabitee",
    }
}

group! {
    /// `PARTICIPATION.function`.
    ///
    /// openEHR defines exactly one code here. `PARTICIPATION.function` is typed
    /// `DV_TEXT`, not `DV_CODED_TEXT`, precisely because the useful vocabulary
    /// lives in external terminologies; the single `unknown` code is the
    /// fallback, not a vocabulary.
    participation_function, "participation_function", {
        UNKNOWN = "253" => "unknown",
    }
}

group! {
    /// How a participant took part — `PARTICIPATION.mode`.
    participation_mode, "participation_mode", {
        NOT_SPECIFIED = "193" => "not specified",
        ASYNCHRONOUS_AUDIOVISUAL = "194" => "asynchronous audiovisual",
        LIVE_AUDIOVISUAL = "195" => "live audiovisual",
        RECORDED_VIDEO = "196" => "recorded video",
        VIDEOPHONE = "197" => "videophone",
        VIDEOCONFERENCING = "198" => "videoconferencing",
        ASYNCHRONOUS_AUDIO_ONLY = "199" => "asynchronous audio-only",
        DICTATED = "200" => "dictated",
        VOICE_MAIL = "201" => "voice-mail",
        LIVE_AUDIO_ONLY = "202" => "live audio-only",
        TELECONFERENCE = "203" => "teleconference",
        TELEPHONE = "204" => "telephone",
        INTERNET_TELEPHONE = "205" => "internet telephone",
        ASYNCHRONOUS_TEXT = "206" => "asynchronous text",
        EMAIL = "207" => "email",
        FACSIMILE_TELEFAX = "208" => "facsimile/telefax",
        SMS_MESSAGE = "209" => "SMS message",
        PRINTED_TYPED_LETTER = "210" => "printed/typed letter",
        HANDWRITTEN_NOTE = "211" => "handwritten note",
        LIVE_TEXT_ONLY = "212" => "live text-only",
        INTERNET_CHAT = "213" => "internet chat",
        SMS_CHAT = "214" => "SMS chat",
        INTERACTIVE_WRITTEN_NOTE = "215" => "interactive written note",
        FACE_TO_FACE_COMMUNICATION = "216" => "face-to-face communication",
        SIGNING_FACE_TO_FACE = "217" => "signing face-to-face",
        SIGNING_OVER_VIDEO = "218" => "signing over video",
        PHYSICALLY_PRESENT = "219" => "physically present",
        PHYSICALLY_REMOTE = "220" => "physically remote",
        TRANSLATED_TEXT = "221" => "translated text",
        INTERPRETED_AUDIO_ONLY = "222" => "interpreted audio-only",
        INTERPRETED_FACE_TO_FACE_COMMUNICATION = "223" => "interpreted face-to-face communication",
        INTERPRETED_VIDEO_COMMUNICATION = "224" => "interpreted video communication",
    }
}

group! {
    /// States of the Instruction State Machine — `ISM_TRANSITION.current_state`.
    ///
    /// These are the states an order can be in, and they are the reason
    /// `ACTION` exists as a separate class from `INSTRUCTION`: the order says
    /// what should happen, and each `ACTION` records one transition of the
    /// state machine that says what did.
    instruction_state, "instruction_states", {
        ACTIVE = "245" => "active",
        INITIAL = "524" => "initial",
        PLANNED = "526" => "planned",
        POSTPONED = "527" => "postponed",
        CANCELLED = "528" => "cancelled",
        SCHEDULED = "529" => "scheduled",
        SUSPENDED = "530" => "suspended",
        ABORTED = "531" => "aborted",
        COMPLETED = "532" => "completed",
        EXPIRED = "533" => "expired",
    }
}

group! {
    /// Transitions of the Instruction State Machine — `ISM_TRANSITION.transition`.
    instruction_transition, "instruction_transitions", {
        CANCEL = "166" => "cancel",
        SCHEDULED_STEP = "534" => "scheduled step",
        INITIATE = "535" => "initiate",
        PLAN_STEP = "536" => "plan step",
        POSTPONE = "537" => "postpone",
        RESTORE = "538" => "restore",
        SCHEDULE = "539" => "schedule",
        START = "540" => "start",
        DO = "541" => "do",
        POSTPONED_STEP = "542" => "postponed step",
        ACTIVE_STEP = "543" => "active step",
        SUSPEND = "544" => "suspend",
        SUSPENDED_STEP = "545" => "suspended step",
        RESUME = "546" => "resume",
        ABORT = "547" => "abort",
        FINISH = "548" => "finish",
        TIME_OUT = "549" => "time out",
        NOTIFY_ABORTED = "550" => "notify aborted",
        NOTIFY_COMPLETED = "551" => "notify completed",
        NOTIFY_CANCELLED = "552" => "notify cancelled",
    }
}

group! {
    /// Where a measurement sits relative to its reference range —
    /// `DV_QUANTIFIED.normal_status`.
    ///
    /// These are the codes themselves, not numbers: openEHR uses the HL7 v2
    /// abnormal-flag letters here.
    normal_status, "normal_statuses", {
        VERY_HIGH = "HHH" => "HHH",
        HIGH_HIGH = "HH" => "HH",
        HIGH = "H" => "H",
        NORMAL = "N" => "N",
        LOW = "L" => "L",
        LOW_LOW = "LL" => "LL",
        VERY_LOW = "LLL" => "LLL",
    }
}

group! {
    /// `DV_ENCAPSULATED.compression_algorithm`.
    compression_algorithm, "compression_algorithms", {
        COMPRESS = "compress" => "compress",
        DEFLATE = "deflate" => "deflate",
        GZIP = "gzip" => "gzip",
        ZLIB = "zlib" => "zlib",
        OTHER = "other" => "other",
    }
}

group! {
    /// `DV_MULTIMEDIA.integrity_check_algorithm`.
    ///
    /// `SHA-1` is in the group because openEHR lists it, and this crate will
    /// read an instance that names it. It will not *emit* it: see
    /// [`crate::security::audit_chain`] for the argument, which is that a
    /// clinical record outlives any promise about a hash construction, and
    /// SHA-1 has already been outlived.
    integrity_check_algorithm, "integrity_check_algorithms", {
        SHA_1 = "SHA-1" => "SHA-1",
        SHA_224 = "SHA-224" => "SHA-224",
        SHA_256 = "SHA-256" => "SHA-256",
        SHA_384 = "SHA-384" => "SHA-384",
        SHA_512 = "SHA-512" => "SHA-512",
        SHA_512_224 = "SHA-512/224" => "SHA-512/224",
        SHA_512_256 = "SHA-512/256" => "SHA-512/256",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_group_is_reachable_by_id_and_has_unique_codes() {
        for g in GROUPS {
            assert_eq!(group(g.id), Some(*g), "group {} not findable", g.id);
            let mut codes: Vec<&str> = g.concepts.iter().map(|c| c.code).collect();
            codes.sort_unstable();
            let before = codes.len();
            codes.dedup();
            assert_eq!(before, codes.len(), "duplicate code in {}", g.id);
            assert!(!g.concepts.is_empty(), "empty group {}", g.id);
        }
    }

    #[test]
    fn the_codes_that_disagree_between_terminology_repositories_are_the_current_ones() {
        // This test is the provenance claim in the module header, made
        // checkable. If someone re-seeds this module from the older
        // openEHR/terminology repository, these three assertions fail.
        assert_eq!(composition_category::EPISODIC, "451"); // not 435
        assert!(composition_category::GROUP.contains("815")); // report
        assert!(audit_change_type::GROUP.contains("816")); // restoration
    }

    #[test]
    fn coded_text_refuses_a_code_the_group_does_not_define() {
        assert!(null_flavour::GROUP.coded_text("999").is_none());
        assert!(
            null_flavour::GROUP
                .coded_text(null_flavour::UNKNOWN)
                .is_some()
        );
    }

    #[test]
    fn deleted_shares_a_code_across_two_groups() {
        // 523 means "deleted" as a change type and as a lifecycle state. That
        // is the terminology's design, not a transcription slip, and a lookup
        // keyed on the code alone would be ambiguous — which is why `rubric`
        // takes a group.
        assert_eq!(audit_change_type::DELETED, version_lifecycle_state::DELETED);
    }
}
