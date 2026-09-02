//! The openEHR **Data Structures Information Model**: how leaf values are
//! arranged into trees, tables, lists, and time series.
//!
//! ```text
//! DATA_STRUCTURE                  ITEM              EVENT<T>
//! └── ITEM_STRUCTURE              ├── CLUSTER       ├── POINT_EVENT<T>
//!     ├── ITEM_SINGLE             └── ELEMENT       └── INTERVAL_EVENT<T>
//!     ├── ITEM_LIST
//!     ├── ITEM_TABLE          HISTORY<T>
//!     └── ITEM_TREE
//! ```
//!
//! # The four ways to say nothing
//!
//! [`Element`] is where openEHR's most under-appreciated design decision lives.
//! An element either has a `value` **or** a `null_flavour`, never both and
//! never neither, and the null flavour distinguishes four situations that a
//! database `NULL` flattens into one:
//!
//! | Flavour | Means |
//! | --- | --- |
//! | `271｜no information｜` | nobody looked |
//! | `253｜unknown｜` | somebody looked and could not find out |
//! | `272｜masked｜` | the value exists and is withheld |
//! | `273｜not applicable｜` | the question does not arise |
//!
//! "No allergy history recorded" and "no known allergies" are the first and
//! the fourth, and prescribing software that treats them alike will eventually
//! give a penicillin-allergic patient penicillin. This module makes the
//! distinction impossible to drop: [`Element::new_null`] requires a flavour,
//! and [`Element::new`] refuses one.
//!
//! # `INTERVAL_EVENT` is not a `POINT_EVENT` with extra fields
//!
//! An interval event's `time` is the **end** of the interval it summarises, and
//! `math_function` says what the summary is: a maximum, a mean, a total. A
//! reader that treats `time` as an instant places an eight-hour urine output at
//! the moment the bag was measured. [`IntervalEvent::interval_start_time`]
//! exists so the other end is derivable rather than assumed.

use crate::base::iso8601;
use crate::error::ParseError;
use crate::rm::common::{LocatableAttrs, impl_locatable};
use crate::rm::data_types::{DataValue, DvCodedText, DvDateTime, DvDuration, Text};
use crate::rm::rm_type_tag;
use crate::terminology;
use serde::{Deserialize, Serialize};

rm_type_tag!(HistoryTag, "HISTORY");

/// A leaf node holding one data value, or a reason there is none.
///
/// ```
/// use openehr::rm::common::LocatableAttrs;
/// use openehr::rm::data_structures::Element;
/// use openehr::rm::data_types::{DataValue, DvQuantity};
/// use openehr::terminology::null_flavour;
///
/// let systolic = Element::new(
///     LocatableAttrs::named("Systolic", "at0004").unwrap(),
///     DataValue::Quantity(DvQuantity::new(184.0, "mm[Hg]").unwrap()),
/// );
/// assert!(!systolic.is_null());
///
/// let withheld = Element::new_null(
///     LocatableAttrs::named("HIV status", "at0011").unwrap(),
///     null_flavour::MASKED,
/// ).unwrap();
/// assert!(withheld.is_null());
/// assert_eq!(withheld.null_flavour().unwrap().value(), "masked");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Element {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    // Boxed for deserializer frame size; see the note on
    // `LocatableAttrs::archetype_details` and spec/audit.md A-03.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    value: Option<Box<DataValue>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    null_flavour: Option<DvCodedText>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    null_reason: Option<Text>,
}

impl_locatable!(Element, "ELEMENT");

impl Element {
    /// Builds an element with a value.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, value: DataValue) -> Self {
        Self {
            locatable,
            value: Some(Box::new(value)),
            null_flavour: None,
            null_reason: None,
        }
    }

    /// Builds an element with no value and a reason there is none.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the code is not in the `null_flavours` group.
    /// Taking a code rather than a `DV_CODED_TEXT` makes it impossible to
    /// invent a fifth null flavour, which is what happens the moment the type
    /// permits free text here.
    pub fn new_null(
        locatable: LocatableAttrs,
        null_flavour_code: &str,
    ) -> Result<Self, ParseError> {
        let null_flavour = terminology::null_flavour::GROUP
            .coded_text(null_flavour_code)
            .ok_or_else(|| ParseError::invariant("ELEMENT", "Inv_null_flavour_valid"))?;
        Ok(Self {
            locatable,
            value: None,
            null_flavour: Some(null_flavour),
            null_reason: None,
        })
    }

    /// Adds a specific reason to a null element.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the element has a value
    /// (`Null_reason_valid`): a reason for absence on a present value is a
    /// contradiction, and the value is the part a reader will act on.
    pub fn with_null_reason(mut self, reason: Text) -> Result<Self, ParseError> {
        if self.value.is_some() {
            return Err(ParseError::invariant("ELEMENT", "Inv_null_reason_valid"));
        }
        self.null_reason = Some(reason);
        Ok(self)
    }

    /// The value, if there is one.
    #[must_use]
    pub fn value(&self) -> Option<&DataValue> {
        self.value.as_deref()
    }

    /// Why there is no value.
    #[must_use]
    pub fn null_flavour(&self) -> Option<&DvCodedText> {
        self.null_flavour.as_ref()
    }

    /// A specific reason for the absence, beyond the flavour.
    #[must_use]
    pub fn null_reason(&self) -> Option<&Text> {
        self.null_reason.as_ref()
    }

    /// Whether the element has no value.
    #[must_use]
    pub fn is_null(&self) -> bool {
        self.value.is_none()
    }

    /// The null flavour's openEHR code, if the element is null.
    #[must_use]
    pub fn null_flavour_code(&self) -> Option<&str> {
        self.null_flavour
            .as_ref()
            .map(|f| f.defining_code().code_string())
    }

    /// Whether the value exists but is being withheld — `272｜masked｜`.
    ///
    /// Distinguished from every other null because it is the only one that
    /// says *there is something here*. A consent-filtered view produces masked
    /// elements, and a downstream reader must be able to tell that from an
    /// unanswered question.
    #[must_use]
    pub fn is_masked(&self) -> bool {
        self.null_flavour_code() == Some(terminology::null_flavour::MASKED)
    }
}

/// A branch node grouping other items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cluster {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    items: Vec<Item>,
}

impl_locatable!(Cluster, "CLUSTER");

impl Cluster {
    /// Builds a cluster.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the item list is empty (`Items_non_empty`). An
    /// empty cluster is a heading with nothing under it; openEHR's way to say
    /// "this section was not filled in" is a null [`Element`], which carries a
    /// reason.
    pub fn new(locatable: LocatableAttrs, items: Vec<Item>) -> Result<Self, ParseError> {
        if items.is_empty() {
            return Err(ParseError::invariant("CLUSTER", "Items_non_empty"));
        }
        Ok(Self { locatable, items })
    }

    /// The contained items.
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }
}

/// Either kind of item: a branch or a leaf.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
// An ELEMENT holds a DATA_VALUE and is larger than a CLUSTER, which holds a
// Vec. Boxing the element would add an allocation per leaf of every clinical
// structure — the most numerous object there is.
#[allow(clippy::large_enum_variant)]
pub enum Item {
    /// A branch.
    #[serde(rename = "CLUSTER")]
    Cluster(Cluster),
    /// A leaf.
    #[serde(rename = "ELEMENT")]
    Element(Element),
}

impl Item {
    /// The openEHR class name, as it appears in `_type`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Cluster(_) => "CLUSTER",
            Self::Element(_) => "ELEMENT",
        }
    }

    /// The item's `archetype_node_id`.
    #[must_use]
    pub fn archetype_node_id(&self) -> &str {
        use crate::rm::common::Locatable as _;
        match self {
            Self::Cluster(c) => c.archetype_node_id(),
            Self::Element(e) => e.archetype_node_id(),
        }
    }

    /// The item's runtime name.
    #[must_use]
    pub fn name(&self) -> &Text {
        use crate::rm::common::Locatable as _;
        match self {
            Self::Cluster(c) => c.name(),
            Self::Element(e) => e.name(),
        }
    }

    /// Every [`Element`] at or under this item, in document order.
    ///
    /// ```
    /// use openehr::rm::common::LocatableAttrs;
    /// use openehr::rm::data_structures::{Cluster, Element, Item};
    /// use openehr::rm::data_types::{DataValue, DvCount};
    ///
    /// let leaf = |code: &str, n: i64| {
    ///     Item::Element(Element::new(
    ///         LocatableAttrs::named(code, code).unwrap(),
    ///         DataValue::Count(DvCount::new(n)),
    ///     ))
    /// };
    /// let tree = Item::Cluster(
    ///     Cluster::new(LocatableAttrs::named("group", "at0001").unwrap(), vec![
    ///         leaf("at0002", 1),
    ///         Item::Cluster(
    ///             Cluster::new(LocatableAttrs::named("sub", "at0003").unwrap(), vec![leaf("at0004", 2)])
    ///                 .unwrap(),
    ///         ),
    ///     ]).unwrap(),
    /// );
    /// assert_eq!(tree.elements().count(), 2);
    /// ```
    pub fn elements(&self) -> Box<dyn Iterator<Item = &Element> + '_> {
        match self {
            Self::Element(e) => Box::new(core::iter::once(e)),
            Self::Cluster(c) => Box::new(c.items().iter().flat_map(Item::elements)),
        }
    }
}

impl From<Cluster> for Item {
    fn from(v: Cluster) -> Self {
        Self::Cluster(v)
    }
}

impl From<Element> for Item {
    fn from(v: Element) -> Self {
        Self::Element(v)
    }
}

/// A structure holding exactly one element — a weight, a height.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemSingle {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    item: Element,
}

impl_locatable!(ItemSingle, "ITEM_SINGLE");

impl ItemSingle {
    /// Builds a single-item structure.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, item: Element) -> Self {
        Self { locatable, item }
    }

    /// The element.
    #[must_use]
    pub fn item(&self) -> &Element {
        &self.item
    }
}

/// An ordered list of named elements — the parts of an address, a panel of
/// results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemList {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    items: Vec<Element>,
}

impl_locatable!(ItemList, "ITEM_LIST");

impl ItemList {
    /// Builds a list.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, items: Vec<Element>) -> Self {
        Self { locatable, items }
    }

    /// The elements.
    #[must_use]
    pub fn items(&self) -> &[Element] {
        &self.items
    }

    /// How many elements the list holds.
    #[must_use]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// The first element whose runtime name matches.
    ///
    /// Matches on `name`, not `archetype_node_id`, because openEHR's
    /// `named_item` is about the runtime label — in a list built from a
    /// repeating archetype node, every item shares one node id and the names
    /// are what tell them apart.
    #[must_use]
    pub fn named_item(&self, name: &str) -> Option<&Element> {
        self.items.iter().find(|e| {
            use crate::rm::common::Locatable as _;
            e.name().value() == name
        })
    }
}

/// A table of elements: a list of rows, each a [`Cluster`] of cells.
///
/// ```
/// use openehr::rm::common::LocatableAttrs;
/// use openehr::rm::data_structures::{Cluster, Element, ItemTable};
/// use openehr::rm::data_types::{DataValue, DvCount};
///
/// let cell = |name: &str, n: i64| {
///     Element::new(LocatableAttrs::named(name, "at0002").unwrap(), DataValue::Count(DvCount::new(n))).into()
/// };
/// let row = |label: &str, a: i64, b: i64| {
///     Cluster::new(LocatableAttrs::named(label, "at0001").unwrap(), vec![cell("left", a), cell("right", b)])
///         .unwrap()
/// };
/// let table = ItemTable::new(
///     LocatableAttrs::named("Visual acuity", "at0000").unwrap(),
///     vec![row("Uncorrected", 6, 9), row("Corrected", 6, 6)],
/// );
/// assert_eq!(table.row_count(), 2);
/// assert_eq!(table.column_count(), 2);
/// assert!(table.element_at_cell(1, 0).is_some());
/// assert!(table.element_at_cell(2, 0).is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemTable {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    rows: Vec<Cluster>,
}

impl_locatable!(ItemTable, "ITEM_TABLE");

impl ItemTable {
    /// Builds a table.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, rows: Vec<Cluster>) -> Self {
        Self { locatable, rows }
    }

    /// The rows.
    #[must_use]
    pub fn rows(&self) -> &[Cluster] {
        &self.rows
    }

    /// How many rows the table has.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// How many columns the table has.
    ///
    /// Taken from the first row. openEHR's `ITEM_TABLE` is a *regular* table —
    /// every row has the same columns — and this crate does not enforce that
    /// on construction, so a ragged table reports the first row's width and
    /// [`ItemTable::is_regular`] reports the irregularity.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.rows.first().map_or(0, |r| r.items().len())
    }

    /// Whether every row has the same number of cells.
    #[must_use]
    pub fn is_regular(&self) -> bool {
        let width = self.column_count();
        self.rows.iter().all(|r| r.items().len() == width)
    }

    /// The element at a row and column, both zero-based.
    ///
    /// Zero-based, although openEHR's `element_at_cell_ij` is one-based: this
    /// is a Rust API and every other index in it is zero-based. Mixing the two
    /// conventions in one crate produces off-by-one bugs that read as correct.
    #[must_use]
    pub fn element_at_cell(&self, row: usize, column: usize) -> Option<&Element> {
        match self.rows.get(row)?.items().get(column)? {
            Item::Element(e) => Some(e),
            Item::Cluster(_) => None,
        }
    }
}

/// A tree of clusters and elements — the general-purpose structure, and the
/// one most archetypes use.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemTree {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    items: Vec<Item>,
}

impl_locatable!(ItemTree, "ITEM_TREE");

impl ItemTree {
    /// Builds a tree.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, items: Vec<Item>) -> Self {
        Self { locatable, items }
    }

    /// The top-level items.
    #[must_use]
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Every element in the tree, in document order.
    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.items.iter().flat_map(Item::elements)
    }
}

/// Any of the four item structures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
// ITEM_SINGLE embeds an ELEMENT directly and the other three embed Vecs, so the
// variants differ in size by construction. See [`Item`] for why nothing here is
// boxed.
#[allow(clippy::large_enum_variant)]
pub enum ItemStructure {
    /// One element.
    #[serde(rename = "ITEM_SINGLE")]
    Single(ItemSingle),
    /// An ordered list of elements.
    #[serde(rename = "ITEM_LIST")]
    List(ItemList),
    /// A table.
    #[serde(rename = "ITEM_TABLE")]
    Table(ItemTable),
    /// A tree.
    #[serde(rename = "ITEM_TREE")]
    Tree(ItemTree),
}

impl ItemStructure {
    /// The openEHR class name, as it appears in `_type`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Single(_) => "ITEM_SINGLE",
            Self::List(_) => "ITEM_LIST",
            Self::Table(_) => "ITEM_TABLE",
            Self::Tree(_) => "ITEM_TREE",
        }
    }

    /// The locatable attributes.
    #[must_use]
    pub fn locatable(&self) -> &LocatableAttrs {
        use crate::rm::common::Locatable as _;
        match self {
            Self::Single(s) => s.locatable(),
            Self::List(s) => s.locatable(),
            Self::Table(s) => s.locatable(),
            Self::Tree(s) => s.locatable(),
        }
    }

    /// The archetype and template that shaped this node, present only at an
    /// archetype root. A dispatcher of its own, not `self.locatable()
    /// .archetype_details()` — see [`crate::rm::ehr::Entry::archetype_details`]
    /// for why.
    #[must_use]
    pub fn archetype_details(&self) -> Option<&crate::rm::common::Archetyped> {
        use crate::rm::common::Locatable as _;
        match self {
            Self::Single(s) => s.archetype_details(),
            Self::List(s) => s.archetype_details(),
            Self::Table(s) => s.archetype_details(),
            Self::Tree(s) => s.archetype_details(),
        }
    }

    /// Every element in the structure, in document order.
    ///
    /// The one traversal that works across all four shapes, which is what
    /// de-identification and value extraction both need.
    #[must_use]
    pub fn elements(&self) -> Box<dyn Iterator<Item = &Element> + '_> {
        match self {
            Self::Single(s) => Box::new(core::iter::once(s.item())),
            Self::List(s) => Box::new(s.items().iter()),
            Self::Table(s) => Box::new(
                s.rows()
                    .iter()
                    .flat_map(|r| r.items().iter().flat_map(Item::elements)),
            ),
            Self::Tree(s) => Box::new(s.elements()),
        }
    }
}

impl From<ItemSingle> for ItemStructure {
    fn from(v: ItemSingle) -> Self {
        Self::Single(v)
    }
}

impl From<ItemList> for ItemStructure {
    fn from(v: ItemList) -> Self {
        Self::List(v)
    }
}

impl From<ItemTable> for ItemStructure {
    fn from(v: ItemTable) -> Self {
        Self::Table(v)
    }
}

impl From<ItemTree> for ItemStructure {
    fn from(v: ItemTree) -> Self {
        Self::Tree(v)
    }
}

/// An observation at a single instant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointEvent {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    time: DvDateTime,
    // Boxed for deserializer frame size; see the note on
    // `LocatableAttrs::archetype_details` and spec/audit.md A-03.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    state: Option<Box<ItemStructure>>,
    data: ItemStructure,
}

impl_locatable!(PointEvent, "POINT_EVENT");

impl PointEvent {
    /// Builds a point event.
    #[must_use]
    pub fn new(locatable: LocatableAttrs, time: DvDateTime, data: ItemStructure) -> Self {
        Self {
            locatable,
            time,
            state: None,
            data,
        }
    }

    /// Records the subject's state at the time — resting, standing, exercising.
    ///
    /// `state` is not decoration: a blood pressure of 150/95 standing and the
    /// same value lying down are different findings, and openEHR separates
    /// `state` from `data` so the distinction survives into every consumer.
    #[must_use]
    pub fn with_state(mut self, state: ItemStructure) -> Self {
        self.state = Some(Box::new(state));
        self
    }

    /// When the observation was made.
    #[must_use]
    pub fn time(&self) -> &DvDateTime {
        &self.time
    }

    /// The observed data.
    #[must_use]
    pub fn data(&self) -> &ItemStructure {
        &self.data
    }

    /// The subject's state at the time.
    #[must_use]
    pub fn state(&self) -> Option<&ItemStructure> {
        self.state.as_deref()
    }
}

/// An observation summarising a span of time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntervalEvent {
    #[serde(flatten)]
    locatable: LocatableAttrs,
    time: DvDateTime,
    // Boxed for deserializer frame size; see the note on
    // `LocatableAttrs::archetype_details` and spec/audit.md A-03.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    state: Option<Box<ItemStructure>>,
    data: ItemStructure,
    width: DvDuration,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    sample_count: Option<i64>,
    math_function: DvCodedText,
}

impl_locatable!(IntervalEvent, "INTERVAL_EVENT");

impl IntervalEvent {
    /// Builds an interval event.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the math-function code is not in the
    /// `event_math_function` group, or if `width` is negative. A negative width
    /// would put the interval's start after its end.
    pub fn new(
        locatable: LocatableAttrs,
        time: DvDateTime,
        data: ItemStructure,
        width: DvDuration,
        math_function_code: &str,
    ) -> Result<Self, ParseError> {
        if width.value().is_negative() {
            return Err(ParseError::invariant(
                "INTERVAL_EVENT",
                "Width_non_negative",
            ));
        }
        let math_function = terminology::event_math_function::GROUP
            .coded_text(math_function_code)
            .ok_or_else(|| ParseError::invariant("INTERVAL_EVENT", "Math_function_validity"))?;
        Ok(Self {
            locatable,
            time,
            state: None,
            data,
            width,
            sample_count: None,
            math_function,
        })
    }

    /// Records how many samples the summary was computed from.
    #[must_use]
    pub fn with_sample_count(mut self, count: i64) -> Self {
        self.sample_count = Some(count);
        self
    }

    /// The **end** of the interval — see the module header.
    #[must_use]
    pub fn time(&self) -> &DvDateTime {
        &self.time
    }

    /// How long the interval is.
    #[must_use]
    pub fn width(&self) -> &DvDuration {
        &self.width
    }

    /// What the summary computes.
    #[must_use]
    pub fn math_function(&self) -> &DvCodedText {
        &self.math_function
    }

    /// The summarised data.
    #[must_use]
    pub fn data(&self) -> &ItemStructure {
        &self.data
    }

    /// The subject's state over the interval.
    #[must_use]
    pub fn state(&self) -> Option<&ItemStructure> {
        self.state.as_deref()
    }

    /// How many samples the summary came from.
    #[must_use]
    pub fn sample_count(&self) -> Option<i64> {
        self.sample_count
    }

    /// The start of the interval: `time - width`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::Unsupported`] when `width` contains years or
    /// months. Subtracting a calendar duration from an instant needs a calendar
    /// — one month before 31 March is 28 February — and this crate does not
    /// carry one. Returning an approximation would place a clinical event on
    /// the wrong day, silently, in the direction nobody checks.
    pub fn interval_start_time(&self) -> Result<DvDateTime, crate::Error> {
        let width = self.width.value();
        if width.years() > 0 || width.months() > 0 {
            return Err(crate::Error::Unsupported {
                what: "INTERVAL_EVENT.interval_start_time with a calendar-month or -year width",
                spec_ref: "spec/04-data-structures.md R4.9",
            });
        }
        let Some(time) = self.time.value().time() else {
            return Err(crate::Error::Unsupported {
                what: "INTERVAL_EVENT.interval_start_time on a date with no time of day",
                spec_ref: "spec/04-data-structures.md R4.9",
            });
        };
        let seconds = width.approx_seconds();
        let start = subtract_seconds(self.time.value().date(), time, seconds)?;
        DvDateTime::new(&start).map_err(Into::into)
    }
}

/// Subtracts a whole number of seconds from a date and time, in the value's own
/// zone, and formats the result.
fn subtract_seconds(
    date: &iso8601::Date,
    time: &iso8601::Time,
    seconds: f64,
) -> Result<String, crate::Error> {
    let (Some(month), Some(day)) = (date.month(), date.day()) else {
        return Err(crate::Error::Unsupported {
            what: "arithmetic on a date without a day",
            spec_ref: "spec/04-data-structures.md R4.9",
        });
    };
    let total = i64::from(time.hour()) * 3600
        + i64::from(time.minute().unwrap_or(0)) * 60
        + i64::from(time.second().unwrap_or(0));
    #[allow(clippy::cast_possible_truncation)]
    let mut remaining = total - seconds.round() as i64;
    let mut y = date.year();
    let mut m = month;
    let mut d = day;
    while remaining < 0 {
        remaining += 86_400;
        d -= 1;
        if d == 0 {
            m -= 1;
            if m == 0 {
                m = 12;
                y -= 1;
            }
            d = iso8601::days_in_month(y, m);
        }
    }
    let offset = time.offset().map(|o| o.to_string()).unwrap_or_default();
    Ok(format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}{offset}",
        remaining / 3600,
        (remaining % 3600) / 60,
        remaining % 60
    ))
}

/// Either kind of event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "_type")]
// An INTERVAL_EVENT adds a width, a sample count, and a math function. See
// [`Item`] for why nothing here is boxed.
#[allow(clippy::large_enum_variant)]
pub enum Event {
    /// An instant.
    #[serde(rename = "POINT_EVENT")]
    Point(PointEvent),
    /// A summarised span.
    #[serde(rename = "INTERVAL_EVENT")]
    Interval(IntervalEvent),
}

impl Event {
    /// The openEHR class name, as it appears in `_type`.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Point(_) => "POINT_EVENT",
            Self::Interval(_) => "INTERVAL_EVENT",
        }
    }

    /// The event's time. For an interval event this is the interval's **end**.
    #[must_use]
    pub fn time(&self) -> &DvDateTime {
        match self {
            Self::Point(e) => e.time(),
            Self::Interval(e) => e.time(),
        }
    }

    /// The event's data.
    #[must_use]
    pub fn data(&self) -> &ItemStructure {
        match self {
            Self::Point(e) => e.data(),
            Self::Interval(e) => e.data(),
        }
    }

    /// The subject's state during the event.
    #[must_use]
    pub fn state(&self) -> Option<&ItemStructure> {
        match self {
            Self::Point(e) => e.state(),
            Self::Interval(e) => e.state(),
        }
    }

    /// The locatable attributes.
    #[must_use]
    pub fn locatable(&self) -> &LocatableAttrs {
        use crate::rm::common::Locatable as _;
        match self {
            Self::Point(e) => e.locatable(),
            Self::Interval(e) => e.locatable(),
        }
    }

    /// The archetype and template that shaped this node, present only at an
    /// archetype root. A dispatcher of its own, not `self.locatable()
    /// .archetype_details()` — see [`crate::rm::ehr::Entry::archetype_details`]
    /// for why.
    #[must_use]
    pub fn archetype_details(&self) -> Option<&crate::rm::common::Archetyped> {
        use crate::rm::common::Locatable as _;
        match self {
            Self::Point(e) => e.archetype_details(),
            Self::Interval(e) => e.archetype_details(),
        }
    }
}

impl From<PointEvent> for Event {
    fn from(v: PointEvent) -> Self {
        Self::Point(v)
    }
}

impl From<IntervalEvent> for Event {
    fn from(v: IntervalEvent) -> Self {
        Self::Interval(v)
    }
}

/// A time series of events sharing one structure.
///
/// ```
/// use openehr::rm::common::LocatableAttrs;
/// use openehr::rm::data_structures::{Element, History, ItemSingle, PointEvent};
/// use openehr::rm::data_types::{DataValue, DvDateTime, DvQuantity};
///
/// let reading = |t: &str, v: f64| {
///     let data = ItemSingle::new(
///         LocatableAttrs::named("reading", "at0003").unwrap(),
///         Element::new(
///             LocatableAttrs::named("Systolic", "at0004").unwrap(),
///             DataValue::Quantity(DvQuantity::new(v, "mm[Hg]").unwrap()),
///         ),
///     );
///     PointEvent::new(
///         LocatableAttrs::named("any event", "at0006").unwrap(),
///         DvDateTime::new(t).unwrap(),
///         data.into(),
///     ).into()
/// };
///
/// let history = History::new(
///     LocatableAttrs::named("Event Series", "at0001").unwrap(),
///     DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
///     vec![reading("2026-07-31T09:00:00Z", 184.0), reading("2026-07-31T09:05:00Z", 176.0)],
///     None,
/// ).unwrap();
/// assert_eq!(history.events().len(), 2);
/// assert!(!history.is_periodic());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct History {
    #[serde(rename = "_type", default)]
    rm_type: HistoryTag,
    #[serde(flatten)]
    locatable: LocatableAttrs,
    origin: DvDateTime,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    period: Option<DvDuration>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    duration: Option<DvDuration>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    events: Vec<Event>,
    // Boxed for deserializer frame size; see the note on
    // `LocatableAttrs::archetype_details` and spec/audit.md A-03.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    summary: Option<Box<ItemStructure>>,
}

impl_locatable!(History, "HISTORY");

impl History {
    /// Builds a history.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if both `events` and `summary` are absent
    /// (`Events_exists`). A history with neither records that observations were
    /// made and supplies none of them.
    pub fn new(
        locatable: LocatableAttrs,
        origin: DvDateTime,
        events: Vec<Event>,
        summary: Option<ItemStructure>,
    ) -> Result<Self, ParseError> {
        if events.is_empty() && summary.is_none() {
            return Err(ParseError::invariant("HISTORY", "Events_valid"));
        }
        Ok(Self {
            rm_type: HistoryTag,
            locatable,
            origin,
            period: None,
            duration: None,
            events,
            summary: summary.map(Box::new),
        })
    }

    /// Declares the series periodic and states its period.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError`] if the period is zero or negative. A period of
    /// zero describes an infinite number of samples at one instant.
    pub fn with_period(mut self, period: DvDuration) -> Result<Self, ParseError> {
        if period.value().is_negative() || period.value().is_zero() {
            return Err(ParseError::invariant("HISTORY", "Periodic_validity"));
        }
        self.period = Some(period);
        Ok(self)
    }

    /// States how long the series covers.
    #[must_use]
    pub fn with_duration(mut self, duration: DvDuration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// The zero point the events are offset from.
    #[must_use]
    pub fn origin(&self) -> &DvDateTime {
        &self.origin
    }

    /// The events.
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// The aggregate over the whole series, if any.
    #[must_use]
    pub fn summary(&self) -> Option<&ItemStructure> {
        self.summary.as_deref()
    }

    /// The sampling period, if the series is periodic.
    #[must_use]
    pub fn period(&self) -> Option<&DvDuration> {
        self.period.as_ref()
    }

    /// How long the series covers, if recorded.
    #[must_use]
    pub fn duration(&self) -> Option<&DvDuration> {
        self.duration.as_ref()
    }

    /// An event's offset from the history's `origin`, in whole seconds.
    ///
    /// openEHR defines `EVENT.offset` as `time.diff(parent.origin)` — a
    /// *computed* value, not a stored one, which is why it lives on the history
    /// here rather than on the event: an `EVENT` in this crate does not hold a
    /// back-pointer to its parent.
    ///
    /// Returns `None` where the difference is not established — see
    /// [`crate::base::iso8601::DateTime::diff_seconds`].
    #[must_use]
    pub fn offset_seconds(&self, event: &Event) -> Option<i64> {
        event.time().value().diff_seconds(self.origin.value())
    }

    /// Whether every event falls on a multiple of the declared period.
    ///
    /// openEHR's `period_consistency`:
    /// `is_periodic implies events.for_all (e | e.offset.to_seconds.mod(period.to_seconds) = 0)`.
    ///
    /// Returns `None` when the question does not arise or cannot be answered:
    /// the series is not periodic, the period is not a whole number of seconds,
    /// or some event's offset is not established. `Some(false)` is a real
    /// finding — a series declared periodic whose samples are not on the period
    /// is not periodic, and software that resamples or graphs it on the
    /// strength of that declaration will draw the wrong picture.
    #[must_use]
    pub fn is_period_consistent(&self) -> Option<bool> {
        let period = self.period.as_ref()?.value().approx_seconds();
        // A calendar period — months or years — has no fixed length in
        // seconds, so the modulo is not defined. Refuse rather than approximate
        // (`R4.16` takes the same position on interval widths).
        let p = self.period.as_ref()?.value();
        if p.years() > 0 || p.months() > 0 {
            return None;
        }
        #[allow(clippy::cast_possible_truncation)]
        let period = period.round() as i64;
        if period == 0 {
            return None;
        }
        let mut all = true;
        for event in &self.events {
            let offset = self.offset_seconds(event)?;
            if offset % period != 0 {
                all = false;
            }
        }
        Some(all)
    }

    /// Whether the series is periodic.
    ///
    /// openEHR's invariant is `is_periodic xor period = Void`, so this is
    /// exactly "a period was declared" and not a heuristic over the event
    /// times. Inferring periodicity from timestamps would report a series as
    /// periodic because two nurses happened to chart on the hour.
    #[must_use]
    pub fn is_periodic(&self) -> bool {
        self.period.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rm::data_types::{DvCount, DvQuantity};

    fn attrs(name: &str, node: &str) -> LocatableAttrs {
        LocatableAttrs::named(name, node).unwrap()
    }

    fn count_element(name: &str, n: i64) -> Element {
        Element::new(attrs(name, "at0001"), DataValue::Count(DvCount::new(n)))
    }

    #[test]
    fn an_element_is_valued_or_null_and_never_both() {
        let valued = count_element("x", 1);
        assert!(!valued.is_null());
        assert!(valued.null_flavour().is_none());

        let null =
            Element::new_null(attrs("x", "at0001"), terminology::null_flavour::UNKNOWN).unwrap();
        assert!(null.is_null());
        assert!(null.value().is_none());

        // A reason for absence cannot be attached to something present.
        assert!(
            valued
                .with_null_reason(Text::plain("because").unwrap())
                .is_err()
        );
    }

    #[test]
    fn the_four_null_flavours_stay_four() {
        // The failure this prevents: a pipeline mapping every null to `unknown`
        // and losing the distinction between "not asked" and "not applicable".
        let masked =
            Element::new_null(attrs("x", "at0001"), terminology::null_flavour::MASKED).unwrap();
        let unknown =
            Element::new_null(attrs("x", "at0001"), terminology::null_flavour::UNKNOWN).unwrap();
        assert!(masked.is_masked());
        assert!(!unknown.is_masked());
        assert_ne!(masked.null_flavour_code(), unknown.null_flavour_code());

        // And a flavour that is not one of the four is refused outright.
        assert!(Element::new_null(attrs("x", "at0001"), "999").is_err());
    }

    #[test]
    fn interval_start_time_is_derived_where_it_can_be_and_refused_where_it_cannot() {
        let data = ItemSingle::new(attrs("d", "at0002"), count_element("v", 1)).into();
        let event = IntervalEvent::new(
            attrs("8 hour output", "at0006"),
            DvDateTime::new("2026-07-31T08:00:00Z").unwrap(),
            data,
            DvDuration::new("PT8H").unwrap(),
            terminology::event_math_function::TOTAL,
        )
        .unwrap();
        assert_eq!(
            event.interval_start_time().unwrap().as_str(),
            "2026-07-31T00:00:00Z"
        );

        let data2 = ItemSingle::new(attrs("d", "at0002"), count_element("v", 1)).into();
        let calendar = IntervalEvent::new(
            attrs("monthly total", "at0006"),
            DvDateTime::new("2026-03-31T08:00:00Z").unwrap(),
            data2,
            DvDuration::new("P1M").unwrap(),
            terminology::event_math_function::TOTAL,
        )
        .unwrap();
        // One month before 31 March is 28 February, and no fixed number of
        // seconds produces that. The refusal is the correct answer.
        assert!(calendar.interval_start_time().is_err());
    }

    #[test]
    fn interval_start_time_crosses_a_day_boundary_correctly() {
        let data = ItemSingle::new(attrs("d", "at0002"), count_element("v", 1)).into();
        let event = IntervalEvent::new(
            attrs("overnight", "at0006"),
            DvDateTime::new("2026-03-01T06:00:00Z").unwrap(),
            data,
            DvDuration::new("PT12H").unwrap(),
            terminology::event_math_function::MEAN,
        )
        .unwrap();
        // Back across a month boundary in a non-leap year: 2026-02-28.
        assert_eq!(
            event.interval_start_time().unwrap().as_str(),
            "2026-02-28T18:00:00Z"
        );
    }

    #[test]
    fn a_history_needs_events_or_a_summary() {
        assert!(
            History::new(
                attrs("h", "at0001"),
                DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
                Vec::new(),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn element_traversal_reaches_every_leaf_of_every_structure() {
        let tree = ItemTree::new(
            attrs("t", "at0001"),
            vec![
                count_element("a", 1).into(),
                Cluster::new(attrs("c", "at0002"), vec![count_element("b", 2).into()])
                    .unwrap()
                    .into(),
            ],
        );
        assert_eq!(ItemStructure::Tree(tree).elements().count(), 2);

        let table = ItemTable::new(
            attrs("t", "at0001"),
            vec![
                Cluster::new(attrs("r", "at0002"), vec![count_element("a", 1).into()]).unwrap(),
                Cluster::new(attrs("r", "at0002"), vec![count_element("b", 2).into()]).unwrap(),
            ],
        );
        assert_eq!(ItemStructure::Table(table).elements().count(), 2);
    }

    #[test]
    fn structures_round_trip_with_their_type_tags() {
        let s = ItemStructure::Single(ItemSingle::new(
            attrs("s", "at0001"),
            Element::new(
                attrs("q", "at0002"),
                DataValue::Quantity(DvQuantity::new(1.0, "mg").unwrap()),
            ),
        ));
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains(r#""_type":"ITEM_SINGLE""#), "{json}");
        let back: ItemStructure = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }

    /// An `INTERVAL_EVENT` reports where its measurement window started.
    ///
    /// Fifteen mutants lived in this arithmetic and the calendar it used
    /// (`lib:A-09`): every term of the seconds total, the borrow loop that
    /// steps back over a day boundary, and — until `lib:A-33` — a **second
    /// copy** of the Gregorian leap rule that no test had ever run.
    ///
    /// `interval_start_time` is `time - width`: when a measurement began. A
    /// borrow that steps the wrong way puts a clinical event on the wrong day,
    /// which is the failure `R4.9` refuses to approximate for calendar widths
    /// and must not commit for exact ones either.
    ///
    /// The expected values are calendar arithmetic done by hand, not by
    /// running the function.
    #[test]
    fn an_interval_event_reports_when_its_window_opened() {
        let event = |time: &str, width: &str| {
            IntervalEvent::new(
                attrs("summary", "at0100"),
                DvDateTime::new(time).unwrap(),
                ItemTree::new(attrs("data", "at0101"), Vec::new()).into(),
                crate::rm::data_types::DvDuration::new(width).unwrap(),
                terminology::event_math_function::MEAN,
            )
            .unwrap()
        };
        let start = |time: &str, width: &str| {
            event(time, width)
                .interval_start_time()
                .unwrap_or_else(|e| panic!("{time} - {width}: {e}"))
                .as_str()
                .to_owned()
        };

        // Within the day: each component of the seconds total on its own, so
        // no other term can carry the result.
        //
        // The event time itself must carry non-zero minutes *and* seconds, or
        // the `+ minute` and `+ second` terms are added to zero and a `-` is
        // indistinguishable. Every case below with a `:00:00` time leaves that
        // term free, which is how the seconds term survived a first pass.
        assert_eq!(start("2026-08-03T12:34:56Z", "PT1H"), "2026-08-03T11:34:56Z");
        assert_eq!(start("2026-08-03T12:34:56Z", "PT30S"), "2026-08-03T12:34:26Z");
        assert_eq!(start("2026-08-03T00:00:30Z", "PT1M"), "2026-08-02T23:59:30Z");
        assert_eq!(start("2026-08-03T12:00:00Z", "PT1H"), "2026-08-03T11:00:00Z");
        assert_eq!(start("2026-08-03T12:00:00Z", "PT30M"), "2026-08-03T11:30:00Z");
        assert_eq!(start("2026-08-03T12:00:00Z", "PT45S"), "2026-08-03T11:59:15Z");
        assert_eq!(start("2026-08-03T12:00:00Z", "PT0S"), "2026-08-03T12:00:00Z");

        // Across a day boundary, which is the borrow loop.
        assert_eq!(start("2026-08-03T00:30:00Z", "PT1H"), "2026-08-02T23:30:00Z");
        assert_eq!(start("2026-08-03T12:00:00Z", "P1D"), "2026-08-02T12:00:00Z");
        // Several days, so the loop runs more than once.
        assert_eq!(start("2026-08-03T12:00:00Z", "P5D"), "2026-07-29T12:00:00Z");

        // Across a month boundary — this is what `days_in_month` is for, and
        // the length of the *previous* month is what matters.
        assert_eq!(start("2026-08-01T12:00:00Z", "P1D"), "2026-07-31T12:00:00Z");
        assert_eq!(start("2026-03-01T12:00:00Z", "P1D"), "2026-02-28T12:00:00Z");
        // A leap year: 2024-02 has 29 days.
        assert_eq!(start("2024-03-01T12:00:00Z", "P1D"), "2024-02-29T12:00:00Z");
        // A century that is not a leap year: 1900-02 has 28. Dates of birth in
        // 1900 are still in live records, and the copied rule that got this
        // right was never run.
        assert_eq!(start("1900-03-01T12:00:00Z", "P1D"), "1900-02-28T12:00:00Z");
        // A century that is: 2000-02 has 29.
        assert_eq!(start("2000-03-01T12:00:00Z", "P1D"), "2000-02-29T12:00:00Z");

        // Across a year boundary.
        assert_eq!(start("2026-01-01T00:30:00Z", "PT1H"), "2025-12-31T23:30:00Z");

        // The offset is carried through unchanged — the arithmetic is in the
        // value's own zone, so the wall-clock reading moves and the zone does
        // not.
        assert_eq!(
            start("2026-08-03T00:30:00+02:00", "PT1H"),
            "2026-08-02T23:30:00+02:00"
        );

        // A calendar width is refused rather than approximated (`R4.9`): one
        // month before 31 March is 28 February, and no fixed number of seconds
        // produces that.
        assert!(event("2026-03-31T12:00:00Z", "P1M").interval_start_time().is_err());
        assert!(event("2026-03-31T12:00:00Z", "P1Y").interval_start_time().is_err());
        // As is a date with no time of day.
        let dateless = IntervalEvent::new(
            attrs("summary", "at0100"),
            DvDateTime::new("2026-08-03").unwrap(),
            ItemTree::new(attrs("data", "at0101"), Vec::new()).into(),
            crate::rm::data_types::DvDuration::new("PT1H").unwrap(),
            terminology::event_math_function::MEAN,
        )
        .unwrap();
        assert!(dateless.interval_start_time().is_err());

        // A negative width would put the start after the end
        // (`Width_non_negative`).
        assert!(
            IntervalEvent::new(
                attrs("summary", "at0100"),
                DvDateTime::new("2026-08-03T12:00:00Z").unwrap(),
                ItemTree::new(attrs("data", "at0101"), Vec::new()).into(),
                crate::rm::data_types::DvDuration::new("-PT1H").unwrap(),
                terminology::event_math_function::MEAN,
            )
            .is_err()
        );

        // The optional attributes report what was recorded, not a constant.
        let plain = event("2026-08-03T12:00:00Z", "PT1H");
        assert_eq!(plain.sample_count(), None);
        assert_eq!(plain.state(), None);
        let summarised = event("2026-08-03T12:00:00Z", "PT1H").with_sample_count(12);
        assert_eq!(summarised.sample_count(), Some(12));

        // `state` has no builder on an INTERVAL_EVENT — it arrives only by
        // deserialization — so this is the one path that reads it back. It is
        // not decoration: a summary taken while the subject was exercising is a
        // different finding from the same numbers at rest.
        let json = serde_json::to_value(&summarised).expect("serialize");
        let mut with_state = json.as_object().expect("an object").clone();
        with_state.insert(
            "state".to_owned(),
            serde_json::to_value(ItemStructure::from(ItemTree::new(
                attrs("state", "at0102"),
                Vec::new(),
            )))
            .expect("serialize"),
        );
        let revived: IntervalEvent =
            serde_json::from_value(serde_json::Value::Object(with_state)).expect("deserialize");
        assert!(revived.state().is_some(), "a recorded state was dropped");
        assert_eq!(revived.sample_count(), Some(12));
    }

    /// The accessors on `ELEMENT`, `ITEM`, and `ITEM_LIST`.
    ///
    /// Nine mutants (`lib:A-09`). `Item::type_name` is what goes into `_type`
    /// in canonical JSON, so one wrong constant makes a `CLUSTER` deserialize
    /// as an `ELEMENT`; `archetype_node_id` is what every path predicate
    /// matches on; and `named_item`'s `==` could be `!=`, returning the first
    /// element that is *not* the one asked for.
    #[test]
    fn the_accessors_on_an_item_and_a_list_report_what_was_built() {
        use crate::rm::common::Locatable as _;

        // An absent value carries a flavour and, optionally, a reason. The two
        // are different fields: "not measured" and "the cuff was too small".
        let absent = Element::new_null(attrs("bp", "at0004"), "253").unwrap();
        assert!(absent.is_null());
        assert_eq!(absent.null_reason(), None);
        let excused = Element::new_null(attrs("bp", "at0004"), "253")
            .unwrap()
            .with_null_reason(Text::Plain(
                crate::rm::data_types::DvText::new("cuff too small").unwrap(),
            ))
            .unwrap();
        assert_eq!(
            excused.null_reason().map(Text::value),
            Some("cuff too small")
        );
        // A reason on a valued element is refused (`Inv_null_reason_valid`),
        // which is what makes the accessor's answer meaningful.
        assert!(
            count_element("x", 1)
                .with_null_reason(Text::Plain(
                    crate::rm::data_types::DvText::new("why").unwrap()
                ))
                .is_err()
        );

        // ITEM: the two variants must name themselves differently and report
        // their own node id.
        let element_item = Item::Element(count_element("systolic", 120));
        let cluster_item = Item::Cluster(
            Cluster::new(attrs("row", "at0500"), vec![Item::Element(count_element("cell", 1))])
                .unwrap(),
        );
        assert_eq!(element_item.type_name(), "ELEMENT");
        assert_eq!(cluster_item.type_name(), "CLUSTER");
        assert_ne!(element_item.type_name(), cluster_item.type_name());
        assert_eq!(element_item.archetype_node_id(), "at0001");
        assert_eq!(cluster_item.archetype_node_id(), "at0500");

        // ITEM_LIST: the count, and lookup by runtime name rather than node id.
        let list = ItemList::new(
            attrs("readings", "at0600"),
            vec![
                count_element("systolic", 120),
                count_element("diastolic", 80),
                count_element("pulse", 72),
            ],
        );
        assert_eq!(list.item_count(), 3, "the list reported the wrong size");
        assert_eq!(list.items().len(), list.item_count());
        assert_eq!(ItemList::new(attrs("empty", "at0601"), Vec::new()).item_count(), 0);

        // Every item shares `at0001`, so a lookup that matched the node id
        // would return the first regardless — the name is what tells them
        // apart, and a `!=` returns the wrong one rather than none.
        let found = list.named_item("diastolic").expect("diastolic is in the list");
        assert_eq!(found.name().value(), "diastolic");
        assert_eq!(list.named_item("pulse").map(|e| e.name().value()), Some("pulse"));
        assert!(list.named_item("nonesuch").is_none());
    }

    /// A `HISTORY` reports its duration and whether its events fall on the
    /// declared period.
    ///
    /// Four mutants, including the guard that refuses a calendar period
    /// (`lib:A-09`). `Some(false)` from `is_period_consistent` is a real
    /// finding — a series declared periodic whose samples are not on the period
    /// will be resampled or graphed wrongly by anything that trusts the
    /// declaration — so the difference between `None` and `Some(false)` is the
    /// whole point of the method.
    #[test]
    fn a_history_reports_its_duration_and_whether_its_period_holds() {
        let event = |offset_minutes: u32| -> Event {
            PointEvent::new(
                attrs("sample", "at0700"),
                DvDateTime::new(&format!("2026-08-03T09:{offset_minutes:02}:00Z")).unwrap(),
                ItemTree::new(attrs("data", "at0701"), Vec::new()).into(),
            )
            .into()
        };
        let history = |events: Vec<Event>| {
            History::new(
                attrs("series", "at0702"),
                DvDateTime::new("2026-08-03T09:00:00Z").unwrap(),
                events,
                None,
            )
            .unwrap()
        };

        // Duration: absent unless recorded.
        let plain = history(vec![event(0)]);
        assert_eq!(plain.duration(), None);
        let timed = history(vec![event(0)])
            .with_duration(crate::rm::data_types::DvDuration::new("PT30M").unwrap());
        assert_eq!(timed.duration().map(|d| d.value().as_str()), Some("PT30M"));

        // Not periodic: the question does not arise.
        assert!(!plain.is_periodic());
        assert_eq!(plain.is_period_consistent(), None);

        // Periodic and consistent: every offset is a multiple of five minutes.
        let consistent = history(vec![event(0), event(5), event(10)])
            .with_period(crate::rm::data_types::DvDuration::new("PT5M").unwrap())
            .unwrap();
        assert!(consistent.is_periodic());
        assert_eq!(consistent.is_period_consistent(), Some(true));

        // Periodic and *not* consistent: one sample is off the grid. This must
        // be `Some(false)`, not `None` — it is an answer, not an absence.
        let inconsistent = history(vec![event(0), event(5), event(7)])
            .with_period(crate::rm::data_types::DvDuration::new("PT5M").unwrap())
            .unwrap();
        assert_eq!(
            inconsistent.is_period_consistent(),
            Some(false),
            "a series off its declared period was reported consistent"
        );

        // A calendar period has no fixed length in seconds, so the modulo is
        // undefined and the answer is refused rather than approximated. Both
        // halves of the guard matter, so months and years are checked apart.
        for calendar in ["P1M", "P1Y"] {
            let h = history(vec![event(0), event(5)])
                .with_period(crate::rm::data_types::DvDuration::new(calendar).unwrap())
                .unwrap();
            assert!(h.is_periodic());
            assert_eq!(
                h.is_period_consistent(),
                None,
                "{calendar} was treated as a fixed number of seconds"
            );
        }
    }
}
