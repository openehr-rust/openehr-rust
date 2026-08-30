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
//! definitions. It is not archetype validation — checking that
//! `openEHR-EHR-OBSERVATION.blood_pressure.v2` permits exactly these elements
//! at these node ids needs the archetype, and that check lives in
//! [`crate::am::validate_against_archetype`], as a separate verdict (`K15.19`),
//! not here.
//!
//! That split used to rest on a decision this crate no longer holds — `S1.4`,
//! the crate MUST NOT implement the Archetype Model, withdrawn 2026-08-26 — and
//! `K15.18`–`K15.23` are now implemented, **against an archetype already held
//! in memory**: this crate does not yet parse ADL (`K15.5`) or flatten a
//! specialised archetype (`K15.11`), so
//! [`crate::am::validate_against_archetype`] validates the definition as given,
//! and a construct it cannot check is reported unchecked rather than passed
//! (`K15.20`). `L10.2` splits the two verdicts, and this sentence stays until
//! the conformance matrix says every requirement in §15 is satisfied, not
//! merely the ones that are.
//!
//! **So a composition that passes here can still violate its archetype**, and
//! passing [`crate::am::validate_against_archetype`] too still does
//! not mean it conforms to what the *published* archetype requires unless the
//! `Archetype` in hand already carries everything an ADL parser and a
//! flattening step would otherwise have merged in. The documentation says so
//! rather than letting "valid" be read as more than it is.
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
use crate::rm::data_types::{
    DataValue, DvCodedText, DvEhrUri, DvOrdered, DvQuantity, DvUri, OrderedAttrs, Text,
};
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
    // Links are checked here, on the `LOCATABLE` that carries them, so that
    // every node in every structure is covered by one call rather than by a
    // rule repeated per class. `LINK.target` is where a `DV_EHR_URI` actually
    // reaches this crate from outside (`M5.9`), and nothing validated it until
    // `A-36`.
    for (i, link) in node.links().iter().enumerate() {
        ctx.nested(format!("/links[{i}]"), |c| {
            c.nested("/target".to_owned(), |c| check_ehr_uri(link.target(), c));
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
/// `DV_URI`, on data that arrived rather than data the program built.
///
/// The constructor is the only definition of "well formed" — this re-runs it
/// rather than restating its rules, so the two gates cannot drift into
/// disagreeing about one URI (`W0.1`). What it adds is the *reporting*: a
/// constructor returns one `ParseError` and stops, and validation has to name
/// the class, the invariant, and the path (`L10.4`), and collect rather than
/// abort (`L10.3`).
///
/// Two invariants, deliberately distinct:
///
/// * `Value_valid` is openEHR's own, and is only `not value.is_empty`.
/// * `Uri_well_formed` is a **crate-added** check (`L10.9`, `D3.30`): a scheme
///   matching RFC 3986, and no spaces or control characters. openEHR does not
///   require it, this crate's constructor always has, and until `A-36` nothing
///   required it of a document read from a wire.
fn check_uri(uri: &DvUri, ctx: &mut Context) {
    if uri.value().is_empty() {
        ctx.violation("DV_URI", "Value_valid", "value is empty");
        // Nothing further is meaningful, and `new` would report the same thing
        // under the added name as well.
        return;
    }
    if DvUri::new(uri.value()).is_err() {
        ctx.violation(
            "DV_URI",
            "Uri_well_formed",
            "no RFC 3986 scheme, or a space or control character in the value",
        );
    }
}

/// `DV_EHR_URI`, on data that arrived.
///
/// `LINK.target` is typed `DV_EHR_URI` precisely so that a link cannot point
/// out of the record without saying so (`D3.31`, `M5.9`) — and the type's own
/// doctest asserts that `"https://example.org/x"` fails to *parse*. It
/// deserialized without complaint, which made the guarantee true of links this
/// program builds and false of links it is sent. That is the whole of `A-36`.
fn check_ehr_uri(uri: &DvEhrUri, ctx: &mut Context) {
    check_uri(uri.as_uri(), ctx);
    if uri.scheme() != DvEhrUri::SCHEME {
        ctx.violation(
            "DV_EHR_URI",
            "Scheme_valid",
            "the scheme is not `ehr`, so the target is outside the record",
        );
    }
}

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
    // `REFERENCE_RANGE.Range_is_simple`: a reference range's endpoints must
    // themselves be simple — carrying no normal range and no reference ranges
    // of their own.
    //
    // The model really is this cyclic: a quantity's normal range is expressed
    // as quantities, each of which can carry its own reference ranges. Without
    // this rule a range can nest to any depth, and a renderer walking it to
    // show "normal: 4.0–6.0" either recurses forever or silently shows the
    // outermost layer.
    for range in attrs.other_reference_ranges() {
        // `DataValue` has no single accessor for ordered attributes — only the
        // ordered variants carry them — so the check names them. Written out
        // rather than wildcarded: a new ordered variant should be a decision
        // here, not a silent pass.
        let simple = |bound: Option<&crate::rm::data_types::DataValue>| {
            use crate::rm::data_types::DataValue;
            let attrs = match bound {
                Some(DataValue::Quantity(v)) => Some(v.ordered_attrs()),
                Some(DataValue::Count(v)) => Some(v.ordered_attrs()),
                Some(DataValue::Proportion(v)) => Some(v.ordered_attrs()),
                Some(DataValue::Ordinal(v)) => Some(v.ordered_attrs()),
                Some(DataValue::Scale(v)) => Some(v.ordered_attrs()),
                _ => None,
            };
            attrs.is_none_or(|o| {
                o.normal_range().is_none() && o.other_reference_ranges().is_empty()
            })
        };
        if !simple(range.range().lower()) || !simple(range.range().upper()) {
            ctx.violation(
                "REFERENCE_RANGE",
                "Range_is_simple",
                "a reference range's endpoint carries reference ranges of its own",
            );
        }
    }

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
            Self::Uri(u) => check_uri(u, ctx),
            Self::EhrUri(u) => check_ehr_uri(u, ctx),
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
                event.time().semantic_cmp(self.origin()),
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
                end.semantic_cmp(self.start_time()),
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
        // `Is_archetype_root`: openEHR states it as a bare assertion — every
        // ENTRY *is* the root of an entry archetype. One without
        // `archetype_details` cannot say which archetype shaped it, and nothing
        // downstream can validate it against one. Already checked for
        // COMPOSITION and EHR_STATUS; this is the third class that asserts it
        // and the one carrying the clinical statement.
        // `Subject_validity`: `subject_is_self implies subject.generating_type
        // = "PARTY_SELF"`. The BMM documents `subject_is_self` as *"True if
        // this Entry is about the subject of the EHR, in which case the subject
        // attribute is of type PARTY_SELF"* — so in openEHR the implication
        // holds by construction.
        //
        // It does not hold here. `PartyProxy::is_subject` answers true for a
        // `PARTY_RELATED` whose relationship is `self`, which is a legitimate
        // way to say "the patient" and is **not** a `PARTY_SELF`. So an entry
        // can claim to be about the record subject while naming a related
        // party, and the two readings of "who is this about" diverge silently.
        if self.subject().is_subject() && self.subject().type_name() != "PARTY_SELF" {
            ctx.violation(
                "ENTRY",
                "Subject_validity",
                "the subject is about the record subject but is not a PARTY_SELF",
            );
        }
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
    // `ENTRY.Is_archetype_root` is checked here rather than beside the other
    // `ENTRY` rules, because `Locatable` is in scope only once the variant is
    // known. openEHR states it as a bare assertion — every ENTRY *is* the root
    // of an entry archetype — and one without `archetype_details` cannot say
    // which archetype shaped it, so nothing downstream can validate it against
    // one. Already checked for COMPOSITION and EHR_STATUS; this is the class
    // carrying the clinical statement.
    let root = |e: &dyn Fn() -> bool, ctx: &mut Context| {
        if !e() {
            ctx.violation(
                "ENTRY",
                "Is_archetype_root",
                "an entry has no archetype_details, so it is not an archetype root",
            );
        }
    };
    match entry {
        Entry::Observation(e) => {
            check_locatable(e, ctx);
            root(&|| e.is_archetype_root(), ctx);
        }
        Entry::Evaluation(e) => {
            check_locatable(e, ctx);
            root(&|| e.is_archetype_root(), ctx);
        }
        Entry::Instruction(e) => {
            check_locatable(e, ctx);
            root(&|| e.is_archetype_root(), ctx);
        }
        Entry::Action(e) => {
            check_locatable(e, ctx);
            root(&|| e.is_archetype_root(), ctx);
        }
        Entry::AdminEntry(e) => {
            check_locatable(e, ctx);
            root(&|| e.is_archetype_root(), ctx);
        }
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

/// `EHR_ACCESS.Is_archetype_root`.
///
/// The fourth class to assert it, after `COMPOSITION`, `EHR_STATUS`, and
/// `ENTRY`. An `EHR_ACCESS` is a versioned, archetyped object in its own right,
/// and one without `archetype_details` cannot say which archetype shaped it.
///
/// `Scheme_valid` is **not** checked here, and the reason is a departure worth
/// naming. openEHR derives `scheme` from the concrete `settings` instance and
/// requires it non-empty, which means an `EHR_ACCESS` must always carry a
/// policy. This crate's `EhrAccess::new` deliberately builds one with **no
/// policy recorded**, because "no access policy has been set" and "the policy
/// is deny-all" are different facts and collapsing them would invent one. The
/// divergence is recorded rather than enforced away.
impl Validate for crate::security::access::EhrAccess {
    fn visit(&self, ctx: &mut Context) {
        check_locatable(self, ctx);
        if !self.is_archetype_root() {
            ctx.violation(
                "EHR_ACCESS",
                "Is_archetype_root",
                "an EHR_ACCESS has no archetype_details, so it is not an archetype root",
            );
        }
    }
}

/// `PARTY.Is_archetype_root`.
///
/// A demographic `PARTY` is the root of a party archetype, exactly as a
/// `COMPOSITION` is the root of a composition archetype.
impl Validate for crate::rm::demographic::Party {
    fn visit(&self, ctx: &mut Context) {
        use crate::rm::demographic::Party;
        // Per variant, because `Locatable` is in scope only once the concrete
        // type is known — the same shape as `check_locatable_entry`.
        let check = |rooted: bool, ctx: &mut Context| {
            if !rooted {
                ctx.violation(
                    "PARTY",
                    "Is_archetype_root",
                    "a party has no archetype_details, so it is not an archetype root",
                );
            }
        };
        match self {
            Party::Person(p) => {
                check_locatable(p, ctx);
                check(p.is_archetype_root(), ctx);
            }
            Party::Organisation(p) => {
                check_locatable(p, ctx);
                check(p.is_archetype_root(), ctx);
            }
            Party::Group(p) => {
                check_locatable(p, ctx);
                check(p.is_archetype_root(), ctx);
            }
            Party::Agent(p) => {
                check_locatable(p, ctx);
                check(p.is_archetype_root(), ctx);
            }
            // A ROLE is a PARTY too, and openEHR declares the invariant on
            // PARTY, so the violation is reported against PARTY rather than
            // against the subtype a reader happens to be holding.
            Party::Role(p) => {
                check_locatable(p, ctx);
                check(p.is_archetype_root(), ctx);
            }
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
            Evaluation::new(
                attrs("Problem", "openEHR-EHR-EVALUATION.problem.v1").with_archetype_details(
                    Archetyped::new("openEHR-EHR-EVALUATION.problem.v1", "1.1.0").unwrap(),
                ),
                entry_attrs(),
                structure(),
            )
            .into(),
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
        let observation = Observation::new(
            attrs("Obs", "openEHR-EHR-OBSERVATION.blood_pressure.v2").with_archetype_details(
                Archetyped::new("openEHR-EHR-OBSERVATION.blood_pressure.v2", "1.1.0").unwrap(),
            ),
            entry_attrs(),
            history,
        );
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

    #[test]
    fn an_entry_that_is_not_an_archetype_root_is_reported() {
        // openEHR states `ENTRY.Is_archetype_root` as a bare assertion: an
        // ENTRY *is* the root of an entry archetype. Enforcing it found seven
        // fixtures that were not, including the README's example of "a
        // composition another implementation wrote" — the `A-21` shape again.
        let entry = Entry::from(Evaluation::new(
            attrs("Problem", "at0000"),
            entry_attrs(),
            structure(),
        ));
        let report = entry.validate();
        assert!(
            report
                .violations()
                .iter()
                .any(|v| v.invariant == "Is_archetype_root"),
            "{report:?}"
        );
    }

    #[test]
    fn an_entry_about_the_record_subject_must_name_a_party_self() {
        use crate::rm::common::{PartyRelated, PartySelf};

        let rooted = || {
            attrs("Problem", "openEHR-EHR-EVALUATION.problem.v1").with_archetype_details(
                Archetyped::new("openEHR-EHR-EVALUATION.problem.v1", "1.1.0").unwrap(),
            )
        };
        let about = |subject: crate::rm::common::PartyProxy| {
            Entry::from(Evaluation::new(
                rooted(),
                crate::rm::ehr::EntryAttrs::about(
                    CodePhrase::new("ISO_639-1", "en").unwrap(),
                    CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
                    subject,
                ),
                structure(),
            ))
        };

        // A PARTY_SELF subject is the ordinary case and says nothing.
        assert!(
            !about(PartySelf::anonymous().into())
                .validate()
                .violations()
                .iter()
                .any(|v| v.invariant == "Subject_validity")
        );

        // `PARTY_RELATED` with a `self` relationship answers true to
        // `is_subject` and is not a `PARTY_SELF`. openEHR's BMM documents
        // `subject_is_self` as implying the type, so in openEHR the two agree
        // by construction; here they can diverge, and an entry can claim to be
        // about the patient while naming a related party.
        let related = PartyRelated::new(
            PartyIdentified::named("The patient").unwrap(),
            crate::terminology::subject_relationship::SELF,
        )
        .unwrap();
        let report = about(related.into()).validate();
        assert!(
            report
                .violations()
                .iter()
                .any(|v| v.invariant == "Subject_validity"),
            "{report:?}"
        );
    }

    #[test]
    fn a_reference_range_endpoint_may_not_carry_reference_ranges_of_its_own() {
        use crate::base::Interval;
        use crate::rm::data_types::{DataValue, DvQuantity, ReferenceRange};

        // The model is genuinely this cyclic: a quantity's reference range is
        // expressed as quantities, each of which can carry reference ranges.
        // Without `Range_is_simple` a range nests to any depth, and a renderer
        // showing "normal: 4.0–6.0" either recurses forever or shows only the
        // outermost layer.
        let plain = |v: f64| DvQuantity::new(v, "mmol/l").unwrap();
        let nested = plain(4.0).with_other_reference_range(ReferenceRange::new(
            DvText::new("inner").unwrap(),
            Interval::closed(
                DataValue::Quantity(plain(1.0)),
                DataValue::Quantity(plain(2.0)),
            )
            .unwrap(),
        ));

        let subject = plain(5.0).with_other_reference_range(ReferenceRange::new(
            DvText::new("normal").unwrap(),
            Interval::closed(
                DataValue::Quantity(nested),
                DataValue::Quantity(plain(6.0)),
            )
            .unwrap(),
        ));
        let report = DataValue::Quantity(subject).validate();
        assert!(
            report
                .violations()
                .iter()
                .any(|v| v.invariant == "Range_is_simple"),
            "{report:?}"
        );

        // A simple range says nothing.
        let ok = plain(5.0).with_other_reference_range(ReferenceRange::new(
            DvText::new("normal").unwrap(),
            Interval::closed(
                DataValue::Quantity(plain(4.0)),
                DataValue::Quantity(plain(6.0)),
            )
            .unwrap(),
        ));
        assert!(
            !DataValue::Quantity(ok)
                .validate()
                .violations()
                .iter()
                .any(|v| v.invariant == "Range_is_simple")
        );
    }

    /// The version envelope, checked here rather than only from the crates
    /// that call it.
    ///
    /// `A-23` added this impl and its tests live in `openehr-loco` and
    /// `openehr-sqlite`. `cargo mutants` runs the tests of the crate it
    /// mutates, so replacing the whole `visit` body with `()` left `openehr`
    /// green — the fix for a High finding, removable without this crate
    /// noticing (`lib:A-09`).
    #[test]
    fn a_version_envelope_is_checked_on_data_that_arrived_as_json() {
        // Deserialization never calls a constructor, which is the whole of
        // `A-23`. Each of these is well-formed JSON and an impossible version.
        let base = |lifecycle: &str, extra: &str| {
            format!(
                r#"{{
                  "_type": "ORIGINAL_VERSION",
                  "uid": {{"_type":"OBJECT_VERSION_ID","value":"87284370-2D4B-4E3D-A3F3-F303D2F4F34B::s::1"}},
                  "lifecycle_state": {{"_type":"DV_CODED_TEXT","value":"x",
                    "defining_code":{{"_type":"CODE_PHRASE","terminology_id":{{"value":"openehr"}},"code_string":"{lifecycle}"}}}},
                  "commit_audit": {{"_type":"AUDIT_DETAILS","system_id":"s",
                    "time_committed":{{"_type":"DV_DATE_TIME","value":"2026-08-01T09:00:00Z"}},
                    "change_type":{{"_type":"DV_CODED_TEXT","value":"creation",
                      "defining_code":{{"_type":"CODE_PHRASE","terminology_id":{{"value":"openehr"}},"code_string":"249"}}}},
                    "committer":{{"_type":"PARTY_IDENTIFIED","name":"N"}}}},
                  "contribution": {{"_type":"OBJECT_REF","namespace":"local","type":"EHR",
                    "id":{{"_type":"HIER_OBJECT_ID","value":"87284370-2D4B-4E3D-A3F3-F303D2F4F34B"}}}}
                  {extra}
                }}"#
            )
        };
        let parse = |json: &str| {
            serde_json::from_str::<crate::rm::common::Version<crate::rm::ehr::Composition>>(json)
                .expect("deserialization is lenient by design (J9.9)")
        };
        let reports = |json: &str| parse(json).validate();

        // `Lifecycle_state_valid`: a code openEHR does not define.
        assert!(
            reports(&base("9999", ""))
                .violations()
                .iter()
                .any(|v| v.invariant == "Lifecycle_state_valid")
        );

        // `Data_valid`: `complete` with no data. 532 is `complete`.
        assert!(
            reports(&base("532", ""))
                .violations()
                .iter()
                .any(|v| v.invariant == "Data_valid")
        );

        // `Preceding_version_uid_validity`: version 1 naming a predecessor.
        let with_predecessor = base(
            "532",
            r#", "preceding_version_uid": {"_type":"OBJECT_VERSION_ID","value":"87284370-2D4B-4E3D-A3F3-F303D2F4F34B::s::9"}"#,
        );
        assert!(
            reports(&with_predecessor)
                .violations()
                .iter()
                .any(|v| v.invariant == "Preceding_version_uid_validity")
        );

        // `validate_ok` must actually return `Err`, which nothing asserted —
        // it could have returned `Ok(())` unconditionally.
        assert!(parse(&base("9999", "")).validate_ok().is_err());

        // And a deleted version with no data is the one legal absence. 523 is
        // `deleted`; this is the case `Data_valid` exists to permit, so it must
        // not be reported.
        let deleted = parse(&base("523", ""));
        assert!(
            !deleted
                .validate()
                .violations()
                .iter()
                .any(|v| v.invariant == "Data_valid"),
            "a deleted version may carry no data"
        );
    }

    /// `EHR_ACCESS` and `PARTY` archetype-root checks.
    ///
    /// Both impls were added in the same sitting as this comment **with no
    /// test at all** — the whole of each `visit` could be replaced with `()`
    /// and nothing failed. Mutation testing found it; reading the diff had
    /// not, including by the person who wrote it.
    #[test]
    fn an_ehr_access_and_a_party_must_be_archetype_roots() {
        use crate::rm::demographic::{Party, PartyAttrs, PartyIdentity, Person};
        use crate::security::access::EhrAccess;

        let rooted = |archetype: &str| {
            attrs("subject", archetype).with_archetype_details(
                Archetyped::new(archetype, "1.1.0").expect("literal"),
            )
        };

        let bare = EhrAccess::new(attrs("access", "at0000"));
        assert!(
            bare.validate()
                .violations()
                .iter()
                .any(|v| v.invariant == "Is_archetype_root" && v.class == "EHR_ACCESS")
        );
        assert!(
            EhrAccess::new(rooted("openEHR-EHR-EHR_ACCESS.generic.v1"))
                .validate()
                .is_empty()
        );

        let identity = PartyIdentity::new(
            attrs("legal name", "at0001"),
            crate::rm::data_structures::ItemStructure::Single(
                crate::rm::data_structures::ItemSingle::new(
                    attrs("name", "at0002"),
                    Element::new(
                        attrs("full name", "at0003"),
                        DataValue::Text(DvText::new("A Patient").expect("literal")),
                    ),
                ),
            ),
        );
        // A `PARTY` must carry a uid (`PARTY.Uid_mandatory`), which is a
        // separate rule from being an archetype root and is already enforced
        // in the constructor.
        let person = |locatable: crate::rm::common::LocatableAttrs| {
            let with_uid = locatable.with_uid(crate::base::UidBasedId::HierObjectId(
                crate::base::HierObjectId::from_uid_str("6BA7B810-9DAD-11D1-80B4-00C04FD430C8")
                    .expect("literal"),
            ));
            Party::Person(Person::new(
                PartyAttrs::new(with_uid, vec![identity.clone()]).expect("literal"),
            ))
        };
        assert!(
            person(attrs("patient", "at0000"))
                .validate()
                .violations()
                .iter()
                .any(|v| v.invariant == "Is_archetype_root" && v.class == "PARTY")
        );
        assert!(
            person(rooted("openEHR-DEMOGRAPHIC-PERSON.person.v1"))
                .validate()
                .is_empty()
        );
    }

    /// Validation descends through a `SECTION` and through a `HISTORY`'s
    /// events.
    ///
    /// `Section::visit`, `ContentItem::visit` and `Event::visit` could each be
    /// replaced with `()` and nothing failed: every existing test puts its
    /// entry directly in `content` and its element directly in a tree, so the
    /// two nesting paths a real composition uses were never walked
    /// (`lib:A-09`).
    ///
    /// A composition organised into sections is the ordinary case, not an edge
    /// one, and an unreported violation inside a section is a document that
    /// validates and is wrong.
    #[test]
    fn a_violation_nested_in_a_section_or_an_event_is_still_reported() {
        use crate::rm::data_structures::{History, PointEvent};
        use crate::rm::ehr::{ContentItem, Evaluation, Observation, Section};

        // An element whose name is empty — `DV_TEXT.Valid_value`, the same
        // violation the flat tests use, so only the *path* to it is new.
        // Violations that a *constructor* permits, since the point is the path
        // the walk takes and not the rule. An `ITEM_TABLE` whose rows have
        // different widths breaks `Rows_regular`, and it is checked by
        // validation rather than refused at construction.
        let irregular = || -> crate::rm::data_structures::ItemStructure {
            let cell = |node: &str| {
                Element::new(attrs("cell", node), DataValue::Count(DvCount::new(1))).into()
            };
            crate::rm::data_structures::ItemTable::new(
                attrs("table", "at0009"),
                vec![
                    Cluster::new(attrs("row 1", "at0010"), vec![cell("at0011")]).expect("literal"),
                    Cluster::new(
                        attrs("row 2", "at0012"),
                        vec![cell("at0013"), cell("at0014")],
                    )
                    .expect("literal"),
                ],
            )
            .into()
        };
        let entry_root = |archetype: &str| {
            attrs("nested", archetype)
                .with_archetype_details(Archetyped::new(archetype, "1.1.0").expect("literal"))
        };

        // Through a SECTION: composition -> section -> entry -> element.
        let sectioned = composition().with_content(
            Section::new(
                attrs("Findings", "at0002"),
                vec![ContentItem::Entry(
                    Evaluation::new(
                        entry_root("openEHR-EHR-EVALUATION.problem.v1"),
                        entry_attrs(),
                        irregular(),
                    )
                    .into(),
                )],
            )
            .into(),
        );
        let report = sectioned.validate();
        assert!(
            report
                .violations()
                .iter()
                .any(|v| v.path.contains("/items")),
            "nothing inside the section was walked: {report}"
        );

        // Through an EVENT: observation -> history -> event -> element.
        let observed = Observation::new(
            entry_root("openEHR-EHR-OBSERVATION.blood_pressure.v2"),
            entry_attrs(),
            History::new(
                attrs("Event Series", "at0001"),
                DvDateTime::new("2026-07-31T09:00:00Z").expect("literal"),
                vec![
                    PointEvent::new(
                        attrs("any event", "at0006"),
                        DvDateTime::new("2026-07-31T09:15:00Z").expect("literal"),
                        irregular(),
                    )
                    .into(),
                ],
                None,
            )
            .expect("literal"),
        );
        let report = Entry::from(observed).validate();
        assert!(
            report
                .violations()
                .iter()
                .any(|v| v.path.contains("/events")),
            "nothing inside the event was walked: {report}"
        );
    }

    /// Every ordered data value has its `normal_status` checked, not just
    /// `DV_QUANTITY`.
    ///
    /// `check_ordered` is reached through a `match` with one arm per ordered
    /// variant, and deleting the `Count`, `Proportion`, `Ordinal` or `Scale`
    /// arm changed nothing: every existing test used a quantity. A count with
    /// a normal status outside openEHR's code set is a result a renderer shows
    /// verbatim beside a number, and nothing was checking it (`lib:A-09`).
    #[test]
    fn a_bad_normal_status_is_reported_for_every_ordered_type() {
        use crate::rm::data_types::{DvOrdinal, DvProportion, DvScale, ProportionKind};

        // Not in the openEHR normal-statuses code set.
        let bogus = || CodePhrase::new("openehr", "ZZ").expect("literal");
        let symbol = || {
            DvCodedText::new("symbol", CodePhrase::new("local", "at0001").expect("literal"))
                .expect("literal")
        };

        let values = [
            DataValue::Count(DvCount::new(1).with_normal_status(bogus())),
            DataValue::Proportion(
                DvProportion::new(1.0, 2.0, ProportionKind::Ratio)
                    .expect("literal")
                    .with_normal_status(bogus()),
            ),
            DataValue::Ordinal(DvOrdinal::new(1, symbol()).with_normal_status(bogus())),
            DataValue::Scale(
                DvScale::new(1.0, symbol())
                    .expect("literal")
                    .with_normal_status(bogus()),
            ),
            DataValue::Quantity(
                DvQuantity::new(1.0, "mm[Hg]")
                    .expect("literal")
                    .with_normal_status(bogus()),
            ),
        ];

        for value in values {
            let kind = value.type_name();
            let report = value.validate();
            assert!(
                report
                    .violations()
                    .iter()
                    .any(|v| v.invariant == "Normal_status_validity"),
                "{kind} did not have its normal_status checked: {report}"
            );
        }
    }
}
