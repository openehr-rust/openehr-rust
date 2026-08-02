//! Reference Model invariant checking.
//!
//! # Why this exists when the constructors already check
//!
//! Every builder in this crate enforces the invariants of the class it builds.
//! **Deserialization does not.** `serde` writes fields directly, so a
//! `COMPOSITION` read from JSON has never passed through `Composition::new`,
//! and an `ELEMENT` read from JSON can carry a `value` *and* a `null_flavour`
//! — a combination no constructor here will produce and every openEHR
//! implementation must be prepared to receive.
//!
//! So there are two gates, and they are not redundant:
//!
//! | Gate | Covers | Enforced by |
//! | --- | --- | --- |
//! | construction | data this program builds | `Result`-returning builders |
//! | validation | data this program **receives** | this module |
//!
//! A service that deserializes a composition and stores it without calling
//! [`Validate::validate`] has no invariant checking at all, whatever its
//! constructors do.
//!
//! # What it reports, and what it cannot
//!
//! Validation here is **RM-level**: the invariants stated in the openEHR class
//! definitions. It is not archetype validation. Checking that
//! `openEHR-EHR-OBSERVATION.blood_pressure.v2` permits exactly these elements
//! at these node ids requires the archetype, and archetypes are out of scope
//! (`S1.4`). A composition that passes here can still violate its archetype,
//! and the documentation says so rather than letting "valid" be read as more
//! than it is.
//!
//! ```
//! use openehr::validation::Validate;
//! use openehr::rm::data_structures::Element;
//!
//! // An ELEMENT that serde built with both a value and a null flavour: no
//! // constructor produces it, and a sender can still send it.
//! let json = r#"{
//!   "name": {"value": "Systolic"},
//!   "archetype_node_id": "at0004",
//!   "value": {"_type": "DV_COUNT", "magnitude": 1},
//!   "null_flavour": {"value": "unknown", "defining_code":
//!     {"terminology_id": {"value": "openehr"}, "code_string": "253"}}
//! }"#;
//! let element: Element = serde_json::from_str(json).unwrap();
//! let report = element.validate();
//! assert!(!report.is_empty());
//! assert_eq!(report.violations()[0].invariant, "Inv_null_flavour_indicated");
//! ```

use crate::error::{ValidationReport, Violation};
use crate::rm::common::{Attestation, Locatable as _, Participation, PartyProxy};
use crate::rm::data_structures::{Cluster, Element, Event, History, Item, ItemStructure};
use crate::rm::data_types::{DataValue, DvCodedText, DvOrdered, DvQuantity, OrderedAttrs, Text};
use crate::rm::ehr::{Composition, ContentItem, EhrStatus, Entry, EventContext, Section};
use crate::terminology;

/// Accumulates violations while walking a structure, tracking where it is.
#[derive(Debug, Default)]
pub struct Context {
    path: Vec<String>,
    report: ValidationReport,
}

impl Context {
    /// A fresh context rooted at the object being validated.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a violation at the current path.
    pub fn violation(
        &mut self,
        class: &'static str,
        invariant: &'static str,
        detail: &'static str,
    ) {
        self.report.push(Violation {
            path: self.path.join(""),
            class,
            invariant,
            detail,
        });
    }

    /// Runs `f` with one more path segment pushed.
    fn nested<F: FnOnce(&mut Self)>(&mut self, segment: String, f: F) {
        self.path.push(segment);
        f(self);
        self.path.pop();
    }

    /// The accumulated report.
    #[must_use]
    pub fn finish(self) -> ValidationReport {
        self.report
    }
}

/// Checks a structure's Reference Model invariants.
pub trait Validate {
    /// Walks the structure, recording every violation found.
    fn visit(&self, ctx: &mut Context);

    /// Validates the structure and returns everything that failed.
    fn validate(&self) -> ValidationReport {
        let mut ctx = Context::new();
        self.visit(&mut ctx);
        ctx.finish()
    }

    /// Validates and converts to a `Result`.
    ///
    /// # Errors
    ///
    /// Returns the [`ValidationReport`] if any invariant failed.
    fn validate_ok(&self) -> Result<(), ValidationReport> {
        self.validate().into_result()
    }
}

/// Checks the invariants shared by every `LOCATABLE`.
fn check_locatable<L: crate::rm::common::Locatable>(node: &L, ctx: &mut Context) {
    if node.archetype_node_id().is_empty() {
        ctx.violation(
            "LOCATABLE",
            "Archetype_node_id_valid",
            "archetype_node_id is empty, so the node cannot be reached by any path",
        );
    }
    // openEHR's `LOCATABLE.Name_valid` is only `name /= Void` — an *empty* name
    // breaks `DV_TEXT.Value_valid` instead, and attributing it to `Name_valid`
    // would send a reader to the wrong class definition (`L10.4`).
    check_text(node.name().as_text(), ctx);
    if node.name().value().is_empty() {
        ctx.nested("/name".to_owned(), |c| {
            c.violation("DV_TEXT", "Valid_value", "value is empty");
        });
    }
    // An archetype root whose archetype id constrains a different RM class is
    // the single most common structural error in hand-built or
    // transform-produced instances: an OBSERVATION archetype id on an
    // EVALUATION node. Nothing downstream detects it, and the data looks fine.
    if let Some(details) = node.archetype_details() {
        let entity = details.archetype_id().rm_entity();
        if entity != node.rm_type_name() {
            ctx.violation(
                "ARCHETYPED",
                "Archetype_id_rm_entity_matches",
                "archetype id constrains a different RM class than the node it annotates",
            );
        }
        if details.rm_version().is_empty() {
            ctx.violation("ARCHETYPED", "Rm_version_valid", "rm_version is empty");
        }
    }
}

/// Checks a `DV_CODED_TEXT` against the openEHR support terminology.
/// `TERM_MAPPING.Purpose_valid` — a mapping's purpose must come from openEHR's
/// own group.
///
/// The group has shipped as `terminology::term_mapping_purpose` since the
/// terminology was written, and nothing consulted it (`lib:A-24`). That is the
/// same shape as `A-22`, which found three `DV_MULTIMEDIA` rules unenforced
/// while the crate carried exactly the code sets they needed.
///
/// Reached from [`check_locatable`] for every node name and from
/// [`check_coded_text`] for every coded text — `DV_CODED_TEXT` embeds a
/// `DV_TEXT`, so one helper covers both. A `DV_TEXT` inside a data value the
/// walk does not descend into is not reached, which is a property of the walk
/// rather than of this rule.
fn check_text(text: &crate::rm::data_types::DvText, ctx: &mut Context) {
    for mapping in text.mappings() {
        if let Some(purpose) = mapping.purpose()
            && !crate::terminology::term_mapping_purpose::GROUP
                .contains(purpose.defining_code().code_string())
        {
            ctx.violation(
                "TERM_MAPPING",
                "Purpose_valid",
                "a term mapping's purpose is not from the openEHR                  term_mapping_purpose group",
            );
        }
    }
}

fn check_coded_text(coded: &DvCodedText, ctx: &mut Context) {
    check_text(coded.as_text(), ctx);
    if coded.check_openehr_rubric() == Some(false) {
        ctx.violation(
            "DV_CODED_TEXT",
            "Value_is_rubric",
            "value does not match the openEHR rubric for defining_code",
        );
    }
}

/// Checks the two `DV_ORDERED` invariants that relate a value to its own
/// recorded reference range.
///
/// `is_abnormal` is the value's own answer to "am I outside my normal range?",
/// which is `None` when no range was recorded — see [`DvOrdered::is_abnormal`].
fn check_ordered(attrs: &OrderedAttrs, is_abnormal: Option<bool>, ctx: &mut Context) {
    let Some(status) = attrs.normal_status() else {
        return;
    };
    // `Normal_status_validity`: the status comes from the openEHR
    // "normal statuses" code set — the HL7 abnormal-flag letters. A status
    // outside it is not comparable with anything, and a renderer will show it
    // verbatim next to a result.
    if !crate::terminology::normal_status::GROUP.contains(status.code_string()) {
        ctx.violation(
            "DV_ORDERED",
            "Normal_status_validity",
            "normal_status is not from the openEHR normal-statuses code set",
        );
    }
    // `Normal_range_and_status_consistency`: with both a range and a status
    // present, `N` and "inside the range" must agree. They disagree when a
    // result is copied from one system and its flag from another — exactly the
    // case where a clinician sees a normal flag beside an abnormal number.
    if let Some(abnormal) = is_abnormal {
        let says_normal = status.code_string() == crate::terminology::normal_status::NORMAL;
        if says_normal == abnormal {
            ctx.violation(
                "DV_ORDERED",
                "Normal_range_and_status_consistency",
                "normal_status and the value's position in its normal_range disagree",
            );
        }
    }
}

/// Checks a `DV_TEXT`-typed attribute that openEHR requires to come from a
/// terminology group **when it happens to be coded**.
///
/// `PARTICIPATION.function`, `PARTICIPATION.mode`, and `ATTESTATION.reason` are
/// all shaped this way: the attribute is `DV_TEXT` because the useful
/// vocabulary lives in external terminologies, and the openEHR group applies
/// only if the value is a `DV_CODED_TEXT` from openEHR's own terminology.
/// Checking unconditionally would reject a SNOMED-coded participation function,
/// which is the commonest real case.
fn check_optional_group(
    text: &Text,
    group: crate::terminology::Group,
    class: &'static str,
    invariant: &'static str,
    ctx: &mut Context,
) {
    let Text::Coded(coded) = text else {
        return;
    };
    check_coded_text(coded, ctx);
    if coded.defining_code().is_openehr() && !group.contains(coded.defining_code().code_string()) {
        ctx.violation(
            class,
            invariant,
            "coded value is not from its openEHR group",
        );
    }
}

/// Checks one `PARTICIPATION`.
fn check_participation(participation: &Participation, ctx: &mut Context) {
    check_optional_group(
        participation.function(),
        crate::terminology::participation_function::GROUP,
        "PARTICIPATION",
        "Function_valid",
        ctx,
    );
    if let Some(mode) = participation.mode() {
        check_coded_text(mode, ctx);
        if mode.defining_code().is_openehr()
            && !crate::terminology::participation_mode::GROUP
                .contains(mode.defining_code().code_string())
        {
            ctx.violation(
                "PARTICIPATION",
                "Mode_valid",
                "mode is not from the openEHR participation-mode group",
            );
        }
    }
    check_party(
        participation.performer(),
        "PARTICIPATION",
        "Performer_valid",
        ctx,
    );
}

/// Checks one `ATTESTATION`.
///
/// Not reachable from a `COMPOSITION`: attestations hang off a `VERSION`, which
/// is why this is public — a caller validating a version calls it directly.
pub fn check_attestation(attestation: &Attestation, ctx: &mut Context) {
    check_optional_group(
        attestation.reason(),
        crate::terminology::attestation_reason::GROUP,
        "ATTESTATION",
        "Reason_valid",
        ctx,
    );
    check_party(
        attestation.audit().committer(),
        "AUDIT_DETAILS",
        "Committer_valid",
        ctx,
    );
}

impl Validate for DataValue {
    fn visit(&self, ctx: &mut Context) {
        match self {
            Self::CodedText(t) => check_coded_text(t, ctx),
            Self::Ordinal(o) => {
                check_coded_text(o.symbol(), ctx);
                check_ordered(o.ordered_attrs(), o.is_abnormal(), ctx);
            }
            Self::Scale(s) => {
                check_coded_text(s.symbol(), ctx);
                check_ordered(s.ordered_attrs(), s.is_abnormal(), ctx);
            }
            Self::Count(c) => check_ordered(c.ordered_attrs(), c.is_abnormal(), ctx),
            Self::Quantity(q) => {
                if !q.magnitude().is_finite() {
                    ctx.violation(
                        "DV_QUANTITY",
                        "Magnitude_finite",
                        "magnitude is not a finite number",
                    );
                }
                if q.units().is_empty() {
                    ctx.violation("DV_QUANTITY", "Units_valid", "units is empty");
                }
                // `precision >= -1`, not `>= 0`: -1 is openEHR's stated
                // "unlimited decimal places", not an out-of-range value.
                if q.precision()
                    .is_some_and(|p| p < DvQuantity::UNLIMITED_PRECISION)
                {
                    ctx.violation(
                        "DV_QUANTITY",
                        "Precision_valid",
                        "precision is below -1, the unlimited-precision sentinel",
                    );
                }
                check_ordered(q.ordered_attrs(), q.is_abnormal(), ctx);
            }
            Self::Proportion(p) => {
                if p.denominator() == 0.0 {
                    ctx.violation("DV_PROPORTION", "Valid_denominator", "denominator is zero");
                }
                if p.kind().requires_integral() && !p.is_integral() {
                    ctx.violation(
                        "DV_PROPORTION",
                        "Fraction_validity",
                        "a fractional kind has a non-integral numerator or denominator",
                    );
                }
                // `precision = 0 implies is_integral`: declaring precision zero
                // asserts the parts really are whole numbers.
                if p.precision() == Some(0) && !p.is_integral() {
                    ctx.violation(
                        "DV_PROPORTION",
                        "Precision_validity",
                        "precision is 0 but a part has a fractional component",
                    );
                }
                check_ordered(p.ordered_attrs(), p.is_abnormal(), ctx);
            }
            Self::Multimedia(m) => check_multimedia(m, ctx),
            Self::Text(t) if t.value().is_empty() => {
                ctx.violation("DV_TEXT", "Valid_value", "value is empty");
            }
            _ => {}
        }
    }
}

impl Validate for Element {
    fn visit(&self, ctx: &mut Context) {
        check_locatable(self, ctx);
        // openEHR: `is_null xor null_flavour = Void`. Both present and both
        // absent are each a violation, and they fail differently: both present
        // is a contradiction, both absent is an element that says nothing.
        match (self.value().is_some(), self.null_flavour().is_some()) {
            (true, true) => ctx.violation(
                "ELEMENT",
                "Inv_null_flavour_indicated",
                "an element has both a value and a null_flavour",
            ),
            (false, false) => ctx.violation(
                "ELEMENT",
                "Inv_null_flavour_indicated",
                "an element has neither a value nor a null_flavour",
            ),
            _ => {}
        }
        if self.null_reason().is_some() && self.value().is_some() {
            ctx.violation(
                "ELEMENT",
                "Inv_null_reason_valid",
                "a reason for absence is recorded on an element that has a value",
            );
        }
        if let Some(flavour) = self.null_flavour() {
            if !flavour.defining_code().is_openehr()
                || !crate::terminology::null_flavour::GROUP
                    .contains(flavour.defining_code().code_string())
            {
                ctx.violation(
                    "ELEMENT",
                    "Inv_null_flavour_valid",
                    "null_flavour is not one of the four openEHR null flavours",
                );
            }
            check_coded_text(flavour, ctx);
        }
        if let Some(value) = self.value() {
            ctx.nested("/value".to_owned(), |c| value.visit(c));
        }
    }
}

impl Validate for Cluster {
    fn visit(&self, ctx: &mut Context) {
        check_locatable(self, ctx);
        if self.items().is_empty() {
            ctx.violation("CLUSTER", "Items_non_empty", "a cluster has no items");
        }
        for (i, item) in self.items().iter().enumerate() {
            ctx.nested(format!("/items[{i}]"), |c| item.visit(c));
        }
    }
}

impl Validate for Item {
    fn visit(&self, ctx: &mut Context) {
        match self {
            Self::Cluster(v) => v.visit(ctx),
            Self::Element(v) => v.visit(ctx),
        }
    }
}

impl Validate for ItemStructure {
    fn visit(&self, ctx: &mut Context) {
        match self {
            Self::Single(s) => {
                check_locatable(s, ctx);
                ctx.nested("/item".to_owned(), |c| s.item().visit(c));
            }
            Self::List(s) => {
                check_locatable(s, ctx);
                for (i, e) in s.items().iter().enumerate() {
                    ctx.nested(format!("/items[{i}]"), |c| e.visit(c));
                }
            }
            Self::Table(s) => {
                check_locatable(s, ctx);
                if !s.is_regular() {
                    ctx.violation(
                        "ITEM_TABLE",
                        "Rows_regular",
                        "rows do not all have the same number of columns",
                    );
                }
                for (i, r) in s.rows().iter().enumerate() {
                    // `Valid_structure`: a table's cells are ELEMENTs. A row is
                    // a CLUSTER and a CLUSTER may hold either, so the type
                    // system permits a table cell that is itself a cluster —
                    // a nested structure in what a reader will render as one
                    // cell, and what a path expression will not find.
                    if r.items()
                        .iter()
                        .any(|item| !matches!(item, crate::rm::data_structures::Item::Element(_)))
                    {
                        ctx.nested(format!("/rows[{i}]"), |c| {
                            c.violation(
                                "ITEM_TABLE",
                                "Valid_structure",
                                "a row holds something other than ELEMENTs",
                            );
                        });
                    }
                    ctx.nested(format!("/rows[{i}]"), |c| r.visit(c));
                }
            }
            Self::Tree(s) => {
                check_locatable(s, ctx);
                for (i, item) in s.items().iter().enumerate() {
                    ctx.nested(format!("/items[{i}]"), |c| item.visit(c));
                }
            }
        }
    }
}

impl Validate for Event {
    fn visit(&self, ctx: &mut Context) {
        match self {
            Self::Point(e) => {
                check_locatable(e, ctx);
                ctx.nested("/data".to_owned(), |c| e.data().visit(c));
                if let Some(state) = e.state() {
                    ctx.nested("/state".to_owned(), |c| state.visit(c));
                }
            }
            Self::Interval(e) => {
                check_locatable(e, ctx);
                if e.width().value().is_negative() {
                    ctx.violation(
                        "INTERVAL_EVENT",
                        "Width_non_negative",
                        "width is negative, so the interval ends before it starts",
                    );
                }
                check_coded_text(e.math_function(), ctx);
                ctx.nested("/data".to_owned(), |c| e.data().visit(c));
                if let Some(state) = e.state() {
                    ctx.nested("/state".to_owned(), |c| state.visit(c));
                }
            }
        }
    }
}

impl Validate for History {
    fn visit(&self, ctx: &mut Context) {
        check_locatable(self, ctx);
        if self.events().is_empty() && self.summary().is_none() {
            ctx.violation(
                "HISTORY",
                "Events_valid",
                "a history has neither events nor a summary",
            );
        }
        if self
            .period()
            .is_some_and(|p| p.value().is_zero() || p.value().is_negative())
        {
            ctx.violation(
                "HISTORY",
                "Periodic_validity",
                "a periodic history declares a zero or negative period",
            );
        }
        // `period_consistency`: in a periodic series every event must fall on a
        // multiple of the period. A series that says it is periodic and is not
        // will be resampled or graphed on the strength of that declaration.
        if self.is_period_consistent() == Some(false) {
            ctx.violation(
                "HISTORY",
                "Period_consistency",
                "a periodic history has an event whose offset is not a multiple of the period",
            );
        }
        for (i, event) in self.events().iter().enumerate() {
            // Every event must be at or after the history's origin: an event
            // before the zero point has a negative offset, and `EVENT.offset`
            // is defined as `time - origin` with no provision for one.
            if matches!(
                event.time().partial_cmp(self.origin()),
                Some(core::cmp::Ordering::Less)
            ) {
                ctx.nested(format!("/events[{i}]"), |c| {
                    c.violation(
                        "EVENT",
                        "Time_after_origin",
                        "event time is before the history origin",
                    );
                });
            }
            ctx.nested(format!("/events[{i}]"), |c| event.visit(c));
        }
        if let Some(summary) = self.summary() {
            ctx.nested("/summary".to_owned(), |c| summary.visit(c));
        }
    }
}

impl Validate for EventContext {
    fn visit(&self, ctx: &mut Context) {
        check_coded_text(self.setting(), ctx);
        if !self.setting().defining_code().is_openehr()
            || !crate::terminology::setting::GROUP
                .contains(self.setting().defining_code().code_string())
        {
            ctx.violation(
                "EVENT_CONTEXT",
                "Setting_valid",
                "setting is not from the openEHR setting group",
            );
        }
        if let Some(end) = self.end_time()
            && matches!(
                end.partial_cmp(self.start_time()),
                Some(core::cmp::Ordering::Less)
            )
        {
            ctx.violation(
                "EVENT_CONTEXT",
                "End_time_valid",
                "end_time is before start_time",
            );
        }
        if self.location().is_some_and(str::is_empty) {
            ctx.violation(
                "EVENT_CONTEXT",
                "location_valid",
                "location is present and empty",
            );
        }
        for (i, participation) in self.participations().iter().enumerate() {
            ctx.nested(format!("/participations[{i}]"), |c| {
                check_participation(participation, c);
            });
        }
        if let Some(other) = self.other_context() {
            ctx.nested("/other_context".to_owned(), |c| other.visit(c));
        }
    }
}

impl Validate for Entry {
    fn visit(&self, ctx: &mut Context) {
        check_locatable_entry(self, ctx);
        check_party(self.subject(), "ENTRY", "Subject_valid", ctx);
        if let Some(provider) = self.entry_attrs().provider() {
            check_party(provider, "ENTRY", "Provider_valid", ctx);
        }
        for (i, participation) in self.entry_attrs().other_participations().iter().enumerate() {
            ctx.nested(format!("/other_participations[{i}]"), |c| {
                check_participation(participation, c);
            });
        }
        match self {
            Self::Observation(o) => {
                ctx.nested("/data".to_owned(), |c| o.data().visit(c));
                if let Some(state) = o.state() {
                    ctx.nested("/state".to_owned(), |c| state.visit(c));
                }
                if let Some(protocol) = o.care_entry().protocol() {
                    ctx.nested("/protocol".to_owned(), |c| protocol.visit(c));
                }
            }
            Self::Evaluation(e) => ctx.nested("/data".to_owned(), |c| e.data().visit(c)),
            Self::AdminEntry(a) => ctx.nested("/data".to_owned(), |c| a.data().visit(c)),
            Self::Instruction(i) => {
                if i.activities().is_empty() {
                    ctx.violation(
                        "INSTRUCTION",
                        "Activities_valid",
                        "an instruction has no activities",
                    );
                }
                if i.narrative().value().is_empty() {
                    ctx.violation(
                        "INSTRUCTION",
                        "Narrative_valid",
                        "an instruction has an empty narrative",
                    );
                }
                for (n, activity) in i.activities().iter().enumerate() {
                    ctx.nested(format!("/activities[{n}]/description"), |c| {
                        activity.description().visit(c);
                    });
                }
            }
            Self::Action(a) => {
                check_coded_text(a.ism_transition().current_state(), ctx);
                if !crate::terminology::instruction_state::GROUP.contains(
                    a.ism_transition()
                        .current_state()
                        .defining_code()
                        .code_string(),
                ) {
                    ctx.violation(
                        "ISM_TRANSITION",
                        "Current_state_valid",
                        "current_state is not an Instruction State Machine state",
                    );
                }
                ctx.nested("/description".to_owned(), |c| a.description().visit(c));
            }
        }
    }
}

fn check_locatable_entry(entry: &Entry, ctx: &mut Context) {
    match entry {
        Entry::Observation(e) => check_locatable(e, ctx),
        Entry::Evaluation(e) => check_locatable(e, ctx),
        Entry::Instruction(e) => check_locatable(e, ctx),
        Entry::Action(e) => check_locatable(e, ctx),
        Entry::AdminEntry(e) => check_locatable(e, ctx),
    }
}

fn check_party(
    party: &PartyProxy,
    class: &'static str,
    invariant: &'static str,
    ctx: &mut Context,
) {
    let identified = match party {
        PartyProxy::SelfParty(_) => return,
        PartyProxy::Identified(p) => p,
        PartyProxy::Related(p) => {
            check_coded_text(p.relationship(), ctx);
            // `Relationship_valid`: the relationship comes from the openEHR
            // `subject_relationship` group. This is the attribute that says
            // whom an entry is about, so an unrecognised code means a finding
            // may be attributed to the wrong person.
            let code = p.relationship().defining_code();
            if code.is_openehr()
                && !crate::terminology::subject_relationship::GROUP.contains(code.code_string())
            {
                ctx.violation(
                    "PARTY_RELATED",
                    "Relationship_valid",
                    "relationship is not from the openEHR subject-relationship group",
                );
            }
            p.as_identified()
        }
    };
    if identified.name().is_none()
        && identified.identifiers().is_empty()
        && identified.external_ref().is_none()
    {
        ctx.violation(
            class,
            invariant,
            "an identified party has no name, no identifiers, and no external reference",
        );
    }
}

impl Validate for Section {
    fn visit(&self, ctx: &mut Context) {
        check_locatable(self, ctx);
        for (i, item) in self.items().iter().enumerate() {
            ctx.nested(format!("/items[{i}]"), |c| item.visit(c));
        }
    }
}

impl Validate for ContentItem {
    fn visit(&self, ctx: &mut Context) {
        match self {
            Self::Section(s) => s.visit(ctx),
            Self::Entry(e) => e.visit(ctx),
        }
    }
}

impl Validate for EhrStatus {
    fn visit(&self, ctx: &mut Context) {
        check_locatable(self, ctx);
        // `Is_archetype_root`, as for `COMPOSITION`: an EHR_STATUS is a
        // versioned, archetyped object in its own right.
        if !self.is_archetype_root() {
            ctx.violation(
                "EHR_STATUS",
                "Is_archetype_root",
                "an EHR_STATUS has no archetype_details, so it is not an archetype root",
            );
        }
        if let Some(other) = self.other_details() {
            ctx.nested("/other_details".to_owned(), |c| other.visit(c));
        }
    }
}

impl Validate for Composition {
    fn visit(&self, ctx: &mut Context) {
        check_locatable(self, ctx);
        check_coded_text(self.category(), ctx);
        if !crate::terminology::composition_category::GROUP.contains(self.category_code()) {
            ctx.violation(
                "COMPOSITION",
                "Category_validity",
                "category is not from the openEHR composition_category group",
            );
        }
        check_party(self.composer(), "COMPOSITION", "Composer_valid", ctx);
        // `Is_archetype_root`: a COMPOSITION is by definition the root of a
        // composition archetype. One without `archetype_details` cannot say
        // which archetype shaped it, and nothing downstream can validate it
        // against one.
        if !self.is_archetype_root() {
            ctx.violation(
                "COMPOSITION",
                "Is_archetype_root",
                "a composition has no archetype_details, so it is not an archetype root",
            );
        }
        // `Is_persistent_validity`: a persistent composition is a running
        // summary across many encounters, so one encounter's context would
        // assert that the whole list belongs to that visit.
        if self.is_persistent() && self.context().is_some() {
            ctx.violation(
                "COMPOSITION",
                "Is_persistent_validity",
                "a persistent composition carries an event context",
            );
        }
        if let Some(context) = self.context() {
            ctx.nested("/context".to_owned(), |c| context.visit(c));
        }
        for (i, item) in self.content().iter().enumerate() {
            ctx.nested(format!("/content[{i}]"), |c| item.visit(c));
        }
    }
}

/// `DV_MULTIMEDIA`'s invariants.
///
/// Lifted out of `visit` because it grew past the length lint, and because
/// these are the checks most likely to be extended: three of them compare
/// against openEHR code sets, and openEHR adds to those.
fn check_multimedia(m: &crate::rm::data_types::DvMultimedia, ctx: &mut Context) {
    if !m.has_content() {
        ctx.violation(
            "DV_MULTIMEDIA",
            "Not_empty",
            "neither inline data nor a uri is present",
        );
    }
    // A crate addition (`L10.9`), not openEHR's
    // `Integrity_check_validity` — which says only that a check
    // implies an algorithm, checked below. Reporting a digest
    // mismatch under that name sent a reader to an invariant about
    // something else (`A-22`).
    if m.verify_integrity() == crate::rm::data_types::IntegrityCheck::Failed {
        ctx.violation(
            "DV_MULTIMEDIA",
            "Integrity_check_matches",
            "the recorded digest does not match the inline data",
        );
    }
    // openEHR's actual `Integrity_check_validity`: a check with no
    // algorithm cannot be verified by anyone, so it is a claim of
    // integrity that nothing can act on.
    if m.integrity_check().is_some() && m.integrity_check_algorithm().is_none() {
        ctx.violation(
            "DV_MULTIMEDIA",
            "Integrity_check_validity",
            "an integrity check is present with no algorithm naming how it was made",
        );
    }
    if let Some(algorithm) = m.integrity_check_algorithm()
        && terminology::integrity_check_algorithm::GROUP
            .rubric(algorithm.code_string())
            .is_none()
    {
        ctx.violation(
            "DV_MULTIMEDIA",
            "Integrity_check_algorithm_validity",
            "the integrity check algorithm is not one openEHR names",
        );
    }
    if let Some(algorithm) = m.compression_algorithm()
        && terminology::compression_algorithm::GROUP
            .rubric(algorithm.code_string())
            .is_none()
    {
        ctx.violation(
            "DV_MULTIMEDIA",
            "Compression_algorithm_validity",
            "the compression algorithm is not one openEHR names",
        );
    }
    if m.size().is_some_and(|s| s < 0) {
        ctx.violation("DV_MULTIMEDIA", "Size_valid", "size is negative");
    }
}

/// Checks a `VERSION`'s own invariants — the envelope, not the content.
///
/// # Why this exists
///
/// [`OriginalVersion::new`] checks `Lifecycle_state_valid`, `Data_valid`, and
/// `Preceding_version_uid_validity`. **Deserialization checked none of them**,
/// because the type carries a derived `Deserialize` that writes the fields
/// straight in, and nothing else validated a version at all — the store
/// validated the composition inside it and never the envelope around it.
///
/// So a version arriving as JSON could name a lifecycle state openEHR does not
/// define, or claim `complete` and carry no content, and be committed. That is
/// `A-23`, and this is the half of the fix that covers the path an HTTP service
/// actually takes.
///
/// Validating rather than refusing to deserialize is deliberate and matches the
/// rest of the crate: reading is lenient so that a document can be inspected,
/// repaired, or reported on (`J9.9`), and `validate()` is where a caller finds
/// out what is wrong with it. What was missing was any way to ask.
impl<T: Validate> Validate for crate::rm::common::Version<T> {
    fn visit(&self, ctx: &mut Context) {
        if !crate::terminology::version_lifecycle_state::GROUP.contains(self.lifecycle_state_code())
        {
            ctx.violation(
                "ORIGINAL_VERSION",
                "Lifecycle_state_valid",
                "lifecycle_state is not from the openEHR version_lifecycle_state group",
            );
        }
        // A version with no data and a non-deleted state claims its content is
        // finished and then does not supply it.
        if self.data().is_none()
            && self.lifecycle_state_code() != crate::terminology::version_lifecycle_state::DELETED
        {
            ctx.violation(
                "ORIGINAL_VERSION",
                "Data_valid",
                "a version with no data must carry the deleted lifecycle state",
            );
        }
        // `uid.version_tree_id.is_first xor preceding_version_uid /= Void`.
        // Both halves are wrong in their own way: a first version naming a
        // predecessor claims a history that does not exist, and a successor
        // naming none starts a second history inside one container.
        if self.uid().version_tree_id().is_first() == self.preceding_version_uid().is_some() {
            ctx.violation(
                "VERSION",
                "Preceding_version_uid_validity",
                "the first version must have no preceding_version_uid and every \
                 later version must have one",
            );
        }
        // `Attestations_valid` and `Other_input_version_uids_valid` are
        // `X /= Void implies not X.is_empty`, which cannot fail here: both are
        // `Vec`, and an empty `Vec` *is* the absent case. Named so that a
        // reader looking for them finds why they are not checked rather than
        // concluding they were missed.
        if let Some(data) = self.data() {
            ctx.nested("/data".to_owned(), |c| data.visit(c));
        }
    }
}

/// Checks an `EHR`'s four reference collections and its two required
/// references.
///
/// # Why the constructor was not enough
///
/// `A-21` made `Ehr::new` check `Ehr_status_valid` and `Ehr_access_valid`. The
/// four collections are filled by infallible `with_*` builders, which a
/// constructor cannot see, and `Deserialize` is derived — so an `EHR` read from
/// JSON reached neither check. That is `A-23`'s shape again, in a second class,
/// and it is why both the constructor's rules are repeated here rather than
/// assumed.
///
/// What these catch is a reference pointing at the wrong kind of thing: an
/// `EHR` whose `compositions` list names a `CONTRIBUTION`, or whose `directory`
/// names a versioned composition. Rust's type system cannot tell them apart —
/// every one is an `OBJECT_REF` — so the type name is the only thing that can.
impl Validate for crate::rm::ehr::Ehr {
    fn visit(&self, ctx: &mut Context) {
        // Written out rather than looped, because the class and invariant must
        // be **literals** at the call site. `openehr-assets` reads these two
        // call forms to decide which invariants the crate can name, and a
        // violation raised through a variable is one it cannot see — which is
        // how the first version of this impl left all four rules counted as
        // unnamed (`lib:A-25`).
        if self.ehr_status().type_name() != "VERSIONED_EHR_STATUS" {
            ctx.violation(
                "EHR",
                "Ehr_status_valid",
                "ehr_status does not reference a VERSIONED_EHR_STATUS",
            );
        }
        if self.ehr_access().type_name() != "VERSIONED_EHR_ACCESS" {
            ctx.violation(
                "EHR",
                "Ehr_access_valid",
                "ehr_access does not reference a VERSIONED_EHR_ACCESS",
            );
        }
        if self
            .compositions()
            .iter()
            .any(|r| r.type_name() != "VERSIONED_COMPOSITION")
        {
            ctx.violation(
                "EHR",
                "Compositions_valid",
                "a composition reference does not name a VERSIONED_COMPOSITION",
            );
        }
        if self
            .contributions()
            .iter()
            .any(|r| r.type_name() != "CONTRIBUTION")
        {
            ctx.violation(
                "EHR",
                "Contributions_valid",
                "a contribution reference does not name a CONTRIBUTION",
            );
        }
        if self
            .folders()
            .iter()
            .any(|r| r.type_name() != "VERSIONED_FOLDER")
        {
            ctx.violation(
                "EHR",
                "Folders_valid",
                "a folder reference does not name a VERSIONED_FOLDER",
            );
        }
        if self
            .directory()
            .is_some_and(|d| d.type_name() != "VERSIONED_FOLDER")
        {
            ctx.violation(
                "EHR",
                "Directory_valid",
                "directory does not reference a VERSIONED_FOLDER",
            );
        }
    }
}

/// Checks a `VERSIONED_OBJECT`'s identity and every version it holds.
///
/// `Uid_validity` requires the container's uid to carry **no extension**. A
/// `HIER_OBJECT_ID` may have one — `root::extension` — and openEHR forbids it
/// here because the extension is what distinguishes two objects sharing a root,
/// and a version container's identity is the root. One with an extension names
/// something narrower than the thing all its versions belong to.
///
/// `All_versions_valid`, `All_version_ids_valid`, and `Latest_version_valid`
/// are not checked and cannot fail: `version_count()` returns `versions.len()`
/// and `latest_version()` returns `versions.last()`, so each is true by
/// construction.
impl<T: Validate> Validate for crate::rm::common::VersionedObject<T> {
    fn visit(&self, ctx: &mut Context) {
        if self.uid().extension().is_some() {
            ctx.violation(
                "VERSIONED_OBJECT",
                "Uid_validity",
                "the container's uid carries an extension, so it names something \
                 narrower than the object its versions belong to",
            );
        }
        for (i, version) in self.all_versions().iter().enumerate() {
            ctx.nested(format!("/versions[{i}]"), |c| version.visit(c));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Interval;
    use crate::rm::common::{Archetyped, LocatableAttrs, PartyIdentified};
    use crate::rm::data_structures::{ItemSingle, PointEvent};
    use crate::rm::data_types::{CodePhrase, DvCount, DvDateTime, DvQuantity, DvText};
    use crate::rm::ehr::{EntryAttrs, Evaluation, Observation};
    use crate::terminology;

    fn attrs(name: &str, node: &str) -> LocatableAttrs {
        LocatableAttrs::named(name, node).unwrap()
    }

    fn structure() -> ItemStructure {
        ItemSingle::new(
            attrs("d", "at0001"),
            Element::new(attrs("v", "at0002"), DataValue::Count(DvCount::new(1))),
        )
        .into()
    }

    fn entry_attrs() -> EntryAttrs {
        EntryAttrs::about_subject(
            CodePhrase::new("ISO_639-1", "en").unwrap(),
            CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
        )
    }

    fn composition() -> Composition {
        Composition::new(
            attrs("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1").with_archetype_details(
                Archetyped::new("openEHR-EHR-COMPOSITION.encounter.v1", "1.1.0").unwrap(),
            ),
            terminology::composition_category::EVENT,
            PartyIdentified::named("Dr A Nurse").unwrap().into(),
            CodePhrase::new("ISO_639-1", "en").unwrap(),
            CodePhrase::new("ISO_3166-1", "GB").unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn a_well_formed_composition_has_no_violations() {
        let c = composition().with_content(
            Evaluation::new(attrs("Problem", "at0000"), entry_attrs(), structure()).into(),
        );
        let report = c.validate();
        assert!(report.is_empty(), "{report}");
    }

    #[test]
    fn deserialization_bypasses_constructors_and_validation_catches_it() {
        // The mutation check for this whole module: if `visit` stopped
        // checking the null invariant, this element would validate clean.
        let json = r#"{
          "name": {"value": "Systolic"},
          "archetype_node_id": "at0004",
          "value": {"_type": "DV_COUNT", "magnitude": 1},
          "null_flavour": {"value": "unknown", "defining_code":
            {"terminology_id": {"value": "openehr"}, "code_string": "253"}}
        }"#;
        let element: Element = serde_json::from_str(json).unwrap();
        let report = element.validate();
        assert_eq!(report.len(), 1);
        assert_eq!(
            report.violations()[0].invariant,
            "Inv_null_flavour_indicated"
        );
    }

    #[test]
    fn an_element_with_nothing_at_all_is_also_a_violation() {
        let json = r#"{"name": {"value": "Systolic"}, "archetype_node_id": "at0004"}"#;
        let element: Element = serde_json::from_str(json).unwrap();
        assert_eq!(element.validate().len(), 1);
    }

    #[test]
    fn an_archetype_id_on_the_wrong_class_is_reported() {
        let wrong = Evaluation::new(
            attrs("Problem", "openEHR-EHR-OBSERVATION.blood_pressure.v2").with_archetype_details(
                Archetyped::new("openEHR-EHR-OBSERVATION.blood_pressure.v2", "1.1.0").unwrap(),
            ),
            entry_attrs(),
            structure(),
        );
        let report = Entry::from(wrong).validate();
        assert_eq!(report.len(), 1);
        assert_eq!(
            report.violations()[0].invariant,
            "Archetype_id_rm_entity_matches"
        );
    }

    #[test]
    fn a_coded_text_that_contradicts_its_own_code_is_reported() {
        let json = r#"{
          "name": {"value": "Category"},
          "archetype_node_id": "at0001",
          "value": {"_type": "DV_CODED_TEXT", "value": "deletion",
            "defining_code": {"terminology_id": {"value": "openehr"}, "code_string": "249"}}
        }"#;
        let element: Element = serde_json::from_str(json).unwrap();
        let report = element.validate();
        assert_eq!(report.len(), 1);
        assert_eq!(report.violations()[0].invariant, "Value_is_rubric");
        // The path locates the offending node without repeating its content.
        assert_eq!(report.violations()[0].path, "/value");
    }

    #[test]
    fn violation_paths_locate_the_node_and_never_the_value() {
        let marker = "ZZ-DISTINCTIVE-9999";
        let bad_element: Element = serde_json::from_str(&format!(
            r#"{{"name": {{"value": "{marker}"}}, "archetype_node_id": "at0004"}}"#
        ))
        .unwrap();
        let structure: ItemStructure = ItemSingle::new(attrs("d", "at0001"), bad_element).into();
        let report = structure.validate();
        assert_eq!(report.violations()[0].path, "/item");
        assert!(!report.to_string().contains(marker), "{report}");
    }

    #[test]
    fn an_event_before_its_history_origin_is_reported() {
        let event = PointEvent::new(
            attrs("early", "at0006"),
            DvDateTime::new("2026-07-31T08:00:00Z").unwrap(),
            structure(),
        );
        let history = History::new(
            attrs("Event Series", "at0001"),
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            vec![event.into()],
            None,
        )
        .unwrap();
        let observation = Observation::new(attrs("Obs", "at0000"), entry_attrs(), history);
        let report = Entry::from(observation).validate();
        assert_eq!(report.len(), 1);
        assert_eq!(report.violations()[0].invariant, "Time_after_origin");
        assert_eq!(report.violations()[0].path, "/data/events[0]");
    }

    #[test]
    fn every_violation_is_reported_not_just_the_first() {
        let json = r#"{
          "name": {"value": ""},
          "archetype_node_id": "",
          "value": {"_type": "DV_QUANTITY", "magnitude": 1.0, "units": "", "precision": -3}
        }"#;
        let element: Element = serde_json::from_str(json).unwrap();
        let report = element.validate();
        // empty node id, empty name, empty units, negative precision.
        assert_eq!(report.len(), 4, "{report}");
    }

    #[test]
    fn a_normal_flag_beside_an_abnormal_number_is_reported() {
        // `DV_ORDERED.Normal_range_and_status_consistency`. This is the
        // real-world shape of the defect: a result arrives from one system and
        // its abnormal flag from another, and the clinician sees "N" next to a
        // potassium of 9.9.
        let range = Interval::closed(
            DataValue::Quantity(DvQuantity::new(3.5, "mmol/l").unwrap()),
            DataValue::Quantity(DvQuantity::new(5.5, "mmol/l").unwrap()),
        )
        .unwrap();
        let lying = DvQuantity::new(9.9, "mmol/l")
            .unwrap()
            .with_normal_range(range.clone())
            .with_normal_status(CodePhrase::openehr(terminology::normal_status::NORMAL).unwrap());
        let report = Element::new(attrs("K", "at0004"), DataValue::Quantity(lying)).validate();
        assert_eq!(report.len(), 1, "{report}");
        assert_eq!(
            report.violations()[0].invariant,
            "Normal_range_and_status_consistency"
        );

        // The same value flagged `HHH` is consistent, and so is an in-range
        // value flagged `N`.
        let truthful = DvQuantity::new(9.9, "mmol/l")
            .unwrap()
            .with_normal_range(range.clone())
            .with_normal_status(
                CodePhrase::openehr(terminology::normal_status::VERY_HIGH).unwrap(),
            );
        assert!(
            Element::new(attrs("K", "at0004"), DataValue::Quantity(truthful))
                .validate()
                .is_empty()
        );
        let in_range = DvQuantity::new(4.2, "mmol/l")
            .unwrap()
            .with_normal_range(range)
            .with_normal_status(CodePhrase::openehr(terminology::normal_status::NORMAL).unwrap());
        assert!(
            Element::new(attrs("K", "at0004"), DataValue::Quantity(in_range))
                .validate()
                .is_empty()
        );
    }

    #[test]
    fn a_normal_status_outside_the_code_set_is_reported() {
        // `DV_ORDERED.Normal_status_validity`. A renderer shows this letter
        // verbatim beside a result, so an invented one reaches a clinician.
        let odd = DvQuantity::new(4.2, "mmol/l")
            .unwrap()
            .with_normal_status(CodePhrase::openehr("VERY-HIGH-INDEED").unwrap());
        let report = Element::new(attrs("K", "at0004"), DataValue::Quantity(odd)).validate();
        assert_eq!(report.len(), 1, "{report}");
        assert_eq!(report.violations()[0].invariant, "Normal_status_validity");
    }

    #[test]
    fn the_unlimited_precision_sentinel_validates() {
        // The mutation check for the A-01 fix: a validator requiring
        // `precision >= 0` reports a violation here, and the data is
        // conformant.
        let unlimited = DvQuantity::new(1.5, "mg")
            .unwrap()
            .with_precision(-1)
            .unwrap();
        assert!(
            Element::new(attrs("q", "at0004"), DataValue::Quantity(unlimited))
                .validate()
                .is_empty()
        );
    }

    #[test]
    fn a_valid_quantity_with_a_normal_range_still_validates() {
        let q = DvQuantity::new(4.2, "mmol/l").unwrap();
        let element = Element::new(attrs("Glucose", "at0004"), DataValue::Quantity(q));
        assert!(element.validate().is_empty());
        let _ = DvText::new("unused");
    }

    #[test]
    fn an_ehr_reference_pointing_at_the_wrong_kind_of_thing_is_a_violation() {
        use crate::base::{HierObjectId, ObjectId, ObjectRef};
        use crate::rm::data_types::DvDateTime;
        use crate::rm::ehr::Ehr;

        let uid = HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap();
        let reference =
            |ty: &str| ObjectRef::new("local", ty, ObjectId::HierObjectId(uid.clone())).unwrap();
        let ehr = || {
            Ehr::new(
                HierObjectId::from_uid_str("11111111-2222-3333-4444-555555555555").unwrap(),
                uid.clone(),
                reference("VERSIONED_EHR_STATUS"),
                reference("VERSIONED_EHR_ACCESS"),
                DvDateTime::new("2026-08-01T09:00:00Z").unwrap(),
            )
            .unwrap()
        };

        assert!(ehr().validate().is_empty());

        // Every one of these is an OBJECT_REF, so nothing but the type name can
        // tell them apart. The builders are infallible, which is why the
        // constructor could never have caught them.
        let cases = [
            (
                ehr().with_composition(reference("CONTRIBUTION")),
                "Compositions_valid",
            ),
            (
                ehr().with_contribution(reference("VERSIONED_COMPOSITION")),
                "Contributions_valid",
            ),
            // `with_folders` sets `directory` to the first folder, so one bad
            // folder reference trips both rules — which is the model's own
            // doing (`Directory_in_folders`) and worth seeing.
            (
                ehr()
                    .with_folders(vec![reference("VERSIONED_COMPOSITION")])
                    .unwrap(),
                "Folders_valid",
            ),
            (
                ehr()
                    .with_folders(vec![reference("VERSIONED_COMPOSITION")])
                    .unwrap(),
                "Directory_valid",
            ),
        ];
        for (subject, invariant) in cases {
            let report = subject.validate();
            assert!(
                report.violations().iter().any(|v| v.invariant == invariant),
                "{invariant} not reported: {report:?}"
            );
        }
    }

    #[test]
    fn an_ehr_that_never_went_through_the_constructor_is_still_checked() {
        use crate::rm::ehr::Ehr;

        // `A-21` made `Ehr::new` refuse these two. Deserialization does not call
        // it, so until `Validate for Ehr` existed an EHR read from JSON was
        // checked by nothing at all — the same hole as `A-23`, one class along.
        let json = r#"{
          "system_id": {"_type":"HIER_OBJECT_ID","value":"11111111-2222-3333-4444-555555555555"},
          "ehr_id": {"_type":"HIER_OBJECT_ID","value":"87284370-2D4B-4E3D-A3F3-F303D2F4F34B"},
          "ehr_status": {"_type":"OBJECT_REF","namespace":"local","type":"EHR",
            "id":{"_type":"HIER_OBJECT_ID","value":"87284370-2D4B-4E3D-A3F3-F303D2F4F34B"}},
          "ehr_access": {"_type":"OBJECT_REF","namespace":"local","type":"EHR",
            "id":{"_type":"HIER_OBJECT_ID","value":"87284370-2D4B-4E3D-A3F3-F303D2F4F34B"}},
          "time_created": {"_type":"DV_DATE_TIME","value":"2026-08-01T09:00:00Z"}
        }"#;
        let ehr: Ehr = serde_json::from_str(json).expect("deserialization is lenient (J9.9)");
        let report = ehr.validate();
        assert!(
            report
                .violations()
                .iter()
                .any(|v| v.invariant == "Ehr_status_valid"),
            "{report:?}"
        );
        assert!(
            report
                .violations()
                .iter()
                .any(|v| v.invariant == "Ehr_access_valid"),
            "{report:?}"
        );
    }

    #[test]
    fn an_interval_reports_openehrs_own_name_for_its_bounds_rule() {
        use crate::base::Interval;

        // `lib:A-24`: enforced all along, reported as `INTERVAL` with prose, so
        // a reader could not find the rule in any class definition (`L10.4`).
        let err = Interval::closed(10i64, 1i64).unwrap_err().to_string();
        assert!(err.contains("DV_INTERVAL"), "{err}");
        assert!(err.contains("Limits_consistent"), "{err}");
    }

    #[test]
    fn a_term_mapping_purpose_outside_openehrs_group_is_a_violation() {
        use crate::rm::common::LocatableAttrs;
        use crate::rm::data_structures::Element;
        use crate::rm::data_types::{
            CodePhrase, DataValue, DvCodedText, DvText, MappingMatch, TermMapping,
        };

        let mapped = |purpose_code: &str| {
            let mapping = TermMapping::new(
                CodePhrase::new("SNOMED-CT", "73211009").unwrap(),
                MappingMatch::Equivalent,
            )
            .with_purpose(
                DvCodedText::new("purpose", CodePhrase::new("openehr", purpose_code).unwrap())
                    .unwrap(),
            );
            // Carried on the node's *name*, which is where `check_locatable`
            // reaches a text.
            let name = DvText::new("Diabetes").unwrap().with_mapping(mapping);
            Element::new(
                LocatableAttrs::new(name.into(), "at0004").unwrap(),
                DataValue::Text(DvText::new("present").unwrap()),
            )
        };

        // `669` is `public health` — a real member of the group that has
        // shipped all along while nothing consulted it (`A-24`).
        let ok = mapped("669");
        assert!(ok.validate().is_empty(), "{:?}", ok.validate());

        let bad = mapped("9999");
        let report = bad.validate();
        assert!(
            report
                .violations()
                .iter()
                .any(|v| v.invariant == "Purpose_valid"),
            "{report:?}"
        );
    }

    #[test]
    fn a_versioned_object_uid_may_not_carry_an_extension() {
        use crate::base::{HierObjectId, ObjectId, ObjectRef};
        use crate::rm::common::VersionedObject;
        use crate::rm::data_types::DvDateTime;

        let owner = ObjectRef::new(
            "local",
            "EHR",
            ObjectId::HierObjectId(
                HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap(),
            ),
        )
        .unwrap();
        let at = DvDateTime::new("2026-08-01T09:00:00Z").unwrap();

        // `Composition` rather than `String`: the impl requires `T: Validate`,
        // because a container that checked its own uid and not its versions
        // would be a walk that stops at the interesting part.
        let plain: VersionedObject<crate::rm::ehr::Composition> = VersionedObject::new(
            HierObjectId::from_uid_str("87284370-2D4B-4E3D-A3F3-F303D2F4F34B").unwrap(),
            owner.clone(),
            at.clone(),
        );
        assert!(plain.validate().is_empty());

        // The extension distinguishes two objects sharing a root, and a version
        // container's identity is the root.
        let extended: VersionedObject<crate::rm::ehr::Composition> = VersionedObject::new(
            "87284370-2D4B-4E3D-A3F3-F303D2F4F34B::narrower"
                .parse()
                .unwrap(),
            owner,
            at,
        );
        let report = extended.validate();
        assert!(
            report
                .violations()
                .iter()
                .any(|v| v.invariant == "Uid_validity"),
            "{report:?}"
        );
    }
}
