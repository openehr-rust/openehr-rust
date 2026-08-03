//! openEHR paths: parsing, and navigation over a composition.
//!
//! A path names a node inside an archetyped structure:
//!
//! ```text
//! /content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/data/events[at0006]/data/items[at0004]/value/magnitude
//! ```
//!
//! Each segment is an **RM attribute name** optionally followed by a
//! **predicate** narrowing which of the repeated values under that attribute is
//! meant. The predicate forms this module accepts:
//!
//! | Form | Meaning |
//! | --- | --- |
//! | `[at0004]` | the child whose `archetype_node_id` is `at0004` |
//! | `[openEHR-EHR-OBSERVATION.x.v2]` | the same, with a full archetype id |
//! | `['Systolic']` | the child whose `name/value` is `Systolic` |
//! | `[at0004, 'Systolic']` | both |
//! | `[archetype_node_id='at0004']` | the long form of the first |
//! | `[name/value='Systolic']` | the long form of the second |
//! | `[2]` | the child at index 2, **zero-based** |
//! | `[a and b]` | both conditions |
//!
//! # Why `path_exists` and `path_unique` are separate questions
//!
//! openEHR asks them separately and so does this module. A path can match three
//! `ELEMENT`s — a repeating archetype node with three instances — and code that
//! calls something like `item_at_path` and takes the first will silently read
//! one of three diagnoses. [`Pathable::item_at_path`] therefore **fails** on an
//! ambiguous path and [`Pathable::items_at_path`] returns all of them, so
//! choosing is something a caller does deliberately.
//!
//! # Index predicates are zero-based here
//!
//! openEHR's own examples are inconsistent and AQL implementations differ. This
//! module documents zero-based and means it, because every other index in this
//! crate is zero-based and one crate with two conventions produces off-by-one
//! errors that read as correct code. `spec/12-paths-and-query.md` `Q12.4`
//! records the choice.

use crate::error::PathError;
use crate::rm::common::Locatable as _;
use crate::rm::data_structures::{Cluster, Element, Event, History, Item, ItemStructure};
use crate::rm::data_types::{
    DataValue, DvCodedText, DvInterval, DvOrdered as _, DvText, OrderedAttrs, ReferenceRange, Text,
};
use crate::rm::ehr::{Composition, ContentItem, Entry, Section};
use core::fmt;

/// One condition inside a predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    /// Matches `archetype_node_id`.
    NodeId(String),
    /// Matches `name/value`.
    Name(String),
    /// Matches by position, zero-based.
    Index(usize),
}

impl fmt::Display for Condition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeId(v) => write!(f, "archetype_node_id='{v}'"),
            Self::Name(v) => write!(f, "name/value='{v}'"),
            Self::Index(i) => write!(f, "{i}"),
        }
    }
}

/// One `/attribute[predicate]` step of a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The RM attribute name.
    pub attribute: String,
    /// The conditions, all of which must hold.
    pub conditions: Vec<Condition>,
}

impl Segment {
    /// Whether a locatable child satisfies every condition.
    fn matches(&self, node_id: &str, name: &str, index: usize) -> bool {
        self.conditions.iter().all(|c| match c {
            Condition::NodeId(v) => node_id == v,
            Condition::Name(v) => name == v,
            Condition::Index(i) => index == *i,
        })
    }
}

impl fmt::Display for Segment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.attribute)?;
        if self.conditions.is_empty() {
            return Ok(());
        }
        write!(f, "[")?;
        for (i, c) in self.conditions.iter().enumerate() {
            if i > 0 {
                write!(f, " and ")?;
            }
            write!(f, "{c}")?;
        }
        write!(f, "]")
    }
}

/// A parsed openEHR path.
///
/// ```
/// use openehr::path::Path;
///
/// let p: Path = "/data[at0001]/events[at0006, 'any event']/data/items[at0004]/value/magnitude"
///     .parse()
///     .unwrap();
/// assert_eq!(p.segments().len(), 6);
/// assert_eq!(p.segments()[0].attribute, "data");
/// // Printing a parsed path normalises the shorthand into the long form.
/// assert!(p.to_string().starts_with("/data[archetype_node_id='at0001']"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    segments: Vec<Segment>,
}

impl Path {
    /// The path's segments, in order.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Whether the path is the empty path, which denotes the root object.
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segments.is_empty() {
            return f.write_str("/");
        }
        for s in &self.segments {
            write!(f, "/{s}")?;
        }
        Ok(())
    }
}

impl core::str::FromStr for Path {
    type Err = PathError;

    /// # Errors
    ///
    /// Returns [`PathError::Malformed`] with the offset at which parsing
    /// stopped.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_path(s)
    }
}

fn parse_path(input: &str) -> Result<Path, PathError> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return Ok(Path {
            segments: Vec::new(),
        });
    }
    let body = trimmed.strip_prefix('/').unwrap_or(trimmed);
    let base = trimmed.len() - body.len();

    let mut segments = Vec::new();
    let bytes = body.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let start = i;
        while i < bytes.len() && bytes[i] != b'[' && bytes[i] != b'/' {
            i += 1;
        }
        let attribute = &body[start..i];
        if attribute.is_empty() {
            return Err(PathError::Malformed {
                offset: base + i,
                reason: "empty attribute name",
            });
        }
        let mut conditions = Vec::new();
        if i < bytes.len() && bytes[i] == b'[' {
            // Find the matching `]`, respecting quotes so that a name
            // containing `]` — rare but legal — does not truncate the
            // predicate.
            let open = i;
            i += 1;
            let mut quote: Option<u8> = None;
            loop {
                if i >= bytes.len() {
                    return Err(PathError::Malformed {
                        offset: base + open,
                        reason: "unclosed `[`",
                    });
                }
                match (quote, bytes[i]) {
                    (None, b'\'' | b'"') => quote = Some(bytes[i]),
                    (Some(q), c) if c == q => quote = None,
                    (None, b']') => break,
                    _ => {}
                }
                i += 1;
            }
            conditions = parse_predicate(&body[open + 1..i], base + open + 1)?;
            i += 1;
        }
        segments.push(Segment {
            attribute: attribute.to_owned(),
            conditions,
        });
        if i < bytes.len() {
            if bytes[i] != b'/' {
                return Err(PathError::Malformed {
                    offset: base + i,
                    reason: "expected `/` after a segment",
                });
            }
            i += 1;
            if i == bytes.len() {
                return Err(PathError::Malformed {
                    offset: base + i,
                    reason: "trailing `/`",
                });
            }
        }
    }
    Ok(Path { segments })
}

fn parse_predicate(text: &str, offset: usize) -> Result<Vec<Condition>, PathError> {
    let mut conditions = Vec::new();
    // `and` and `,` are both conjunctions here: openEHR's shorthand
    // `[at0004, 'Systolic']` and the long form `[a and b]` mean the same thing.
    for part in split_conjunctions(text) {
        let part = part.trim();
        if part.is_empty() {
            return Err(PathError::Malformed {
                offset,
                reason: "empty predicate term",
            });
        }
        conditions.push(parse_condition(part, offset)?);
    }
    if conditions.is_empty() {
        return Err(PathError::Malformed {
            offset,
            reason: "empty predicate",
        });
    }
    Ok(conditions)
}

fn split_conjunctions(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let bytes = text.as_bytes();
    let mut quote: Option<u8> = None;
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        match (quote, bytes[i]) {
            (None, b'\'' | b'"') => quote = Some(bytes[i]),
            (Some(q), c) if c == q => quote = None,
            (None, b',') => {
                parts.push(&text[start..i]);
                start = i + 1;
            }
            (None, b'a') if text[i..].starts_with("and") && is_word_boundary(text, i, 3) => {
                parts.push(&text[start..i]);
                i += 3;
                start = i;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(&text[start..]);
    parts
}

fn is_word_boundary(text: &str, at: usize, len: usize) -> bool {
    let before = at == 0 || text.as_bytes()[at - 1].is_ascii_whitespace();
    let after = at + len >= text.len() || text.as_bytes()[at + len].is_ascii_whitespace();
    before && after
}

fn parse_condition(text: &str, offset: usize) -> Result<Condition, PathError> {
    if let Some((lhs, rhs)) = text.split_once('=') {
        let value = unquote(rhs.trim());
        return match lhs.trim() {
            "archetype_node_id" => Ok(Condition::NodeId(value)),
            "name/value" | "name" => Ok(Condition::Name(value)),
            _ => Err(PathError::Malformed {
                offset,
                reason: "predicate attribute is not archetype_node_id or name/value",
            }),
        };
    }
    // Quoted shorthand is always a name; an unquoted integer is an index;
    // anything else unquoted is a node id. Checking the quote *before* the
    // integer matters: `['3']` is an element named "3", not the fourth child.
    if text.starts_with('\'') || text.starts_with('"') {
        return Ok(Condition::Name(unquote(text)));
    }
    if let Ok(index) = text.parse::<usize>() {
        return Ok(Condition::Index(index));
    }
    Ok(Condition::NodeId(text.to_owned()))
}

fn unquote(text: &str) -> String {
    let trimmed = text.trim();
    for q in ['\'', '"'] {
        if trimmed.starts_with(q) && trimmed.ends_with(q) && trimmed.len() >= 2 {
            return trimmed[1..trimmed.len() - 1].to_owned();
        }
    }
    trimmed.to_owned()
}

/// A node reached by a path.
///
/// Borrowed rather than owned: navigation is a read over an existing
/// composition, and copying a subtree to answer "what is at this path" would
/// be the expensive part of every query.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Node<'a> {
    /// A `COMPOSITION`.
    Composition(&'a Composition),
    /// A `SECTION`.
    Section(&'a Section),
    /// An `ENTRY` of any kind.
    Entry(&'a Entry),
    /// An `ITEM_STRUCTURE` of any kind.
    ItemStructure(&'a ItemStructure),
    /// A `CLUSTER`.
    Cluster(&'a Cluster),
    /// An `ELEMENT`.
    Element(&'a Element),
    /// A `HISTORY`.
    History(&'a History),
    /// An `EVENT` of either kind.
    Event(&'a Event),
    /// A `DATA_VALUE` of any kind.
    DataValue(&'a DataValue),
    /// A `DV_TEXT` or `DV_CODED_TEXT` reached through a `name` attribute.
    Text(&'a Text),
    /// A `DV_CODED_TEXT` reached through an attribute typed as one —
    /// `COMPOSITION.category`, `DV_ORDINAL.symbol`, `ISM_TRANSITION.current_state`.
    CodedText(&'a DvCodedText),
    /// A `DV_TEXT` reached through an attribute typed as one, such as
    /// `REFERENCE_RANGE.meaning`.
    PlainText(&'a DvText),
    /// A `DV_INTERVAL` — an `ELEMENT`'s own value, or a `DV_ORDERED`'s
    /// `normal_range`.
    Interval(&'a DvInterval),
    /// A `REFERENCE_RANGE` from `DV_ORDERED.other_reference_ranges`.
    ReferenceRange(&'a ReferenceRange),
    /// A primitive reached inside a data value, such as `value/magnitude`.
    Scalar(Scalar<'a>),
}

/// A primitive value at the end of a path.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Scalar<'a> {
    /// A string.
    Str(&'a str),
    /// A number.
    Number(f64),
    /// An integer.
    Integer(i64),
    /// A boolean.
    Boolean(bool),
}

impl Scalar<'_> {
    /// The value rendered as text.
    #[must_use]
    pub fn to_display_string(&self) -> String {
        match self {
            Self::Str(v) => (*v).to_owned(),
            Self::Number(v) => v.to_string(),
            Self::Integer(v) => v.to_string(),
            Self::Boolean(v) => v.to_string(),
        }
    }
}

impl<'a> Node<'a> {
    /// The openEHR class name of the node, or the primitive's kind.
    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Composition(_) => "COMPOSITION",
            Self::Section(_) => "SECTION",
            Self::Entry(e) => e.type_name(),
            Self::ItemStructure(s) => s.type_name(),
            Self::Cluster(_) => "CLUSTER",
            Self::Element(_) => "ELEMENT",
            Self::History(_) => "HISTORY",
            Self::Event(e) => e.type_name(),
            Self::DataValue(v) => v.type_name(),
            Self::Text(t) => t.type_name(),
            Self::CodedText(_) => "DV_CODED_TEXT",
            Self::PlainText(_) => "DV_TEXT",
            Self::Interval(_) => "DV_INTERVAL",
            Self::ReferenceRange(_) => "REFERENCE_RANGE",
            Self::Scalar(_) => "primitive",
        }
    }

    /// The node's `archetype_node_id`, where it has one.
    #[must_use]
    pub fn archetype_node_id(&self) -> Option<&'a str> {
        Some(match self {
            Self::Composition(c) => c.archetype_node_id(),
            Self::Section(s) => s.archetype_node_id(),
            Self::Entry(e) => e.locatable().archetype_node_id(),
            Self::ItemStructure(s) => s.locatable().archetype_node_id(),
            Self::Cluster(c) => c.archetype_node_id(),
            Self::Element(e) => e.archetype_node_id(),
            Self::History(h) => h.archetype_node_id(),
            Self::Event(e) => e.locatable().archetype_node_id(),
            Self::DataValue(_)
            | Self::Text(_)
            | Self::CodedText(_)
            | Self::PlainText(_)
            | Self::Interval(_)
            | Self::ReferenceRange(_)
            | Self::Scalar(_) => return None,
        })
    }

    /// The node's runtime name, where it has one.
    #[must_use]
    pub fn name(&self) -> Option<&'a str> {
        Some(match self {
            Self::Composition(c) => c.name().value(),
            Self::Section(s) => s.name().value(),
            Self::Entry(e) => e.locatable().name().value(),
            Self::ItemStructure(s) => s.locatable().name().value(),
            Self::Cluster(c) => c.name().value(),
            Self::Element(e) => e.name().value(),
            Self::History(h) => h.name().value(),
            Self::Event(e) => e.locatable().name().value(),
            Self::DataValue(_)
            | Self::Text(_)
            | Self::CodedText(_)
            | Self::PlainText(_)
            | Self::Interval(_)
            | Self::ReferenceRange(_)
            | Self::Scalar(_) => return None,
        })
    }

    /// The children reachable through one RM attribute.
    ///
    /// Returns an empty vector for an attribute this node does not have, which
    /// is how a path that names a wrong attribute resolves to no match rather
    /// than to an error. The distinction is reported by
    /// [`Pathable::item_at_path`] as [`PathError::NoMatch`].
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn children(&self, attribute: &str) -> Vec<Node<'a>> {
        match self {
            Self::Composition(c) => match attribute {
                "content" => c.content().iter().map(content_item_node).collect(),
                "name" => vec![Node::Text(c.name())],
                "category" => vec![Node::CodedText(c.category())],
                _ => Vec::new(),
            },
            Self::Section(s) => match attribute {
                "items" => s.items().iter().map(content_item_node).collect(),
                "name" => vec![Node::Text(s.name())],
                _ => Vec::new(),
            },
            Self::Entry(e) => entry_children(e, attribute),
            Self::ItemStructure(s) => item_structure_children(s, attribute),
            Self::Cluster(c) => match attribute {
                "items" => c.items().iter().map(item_node).collect(),
                "name" => vec![Node::Text(c.name())],
                _ => Vec::new(),
            },
            Self::Element(e) => match attribute {
                "value" => e.value().map(Node::DataValue).into_iter().collect(),
                "null_flavour" => e
                    .null_flavour()
                    .map(|f| Node::Scalar(Scalar::Str(f.defining_code().code_string())))
                    .into_iter()
                    .collect(),
                "name" => vec![Node::Text(e.name())],
                _ => Vec::new(),
            },
            Self::History(h) => match attribute {
                "events" => h.events().iter().map(Node::Event).collect(),
                "summary" => h.summary().map(Node::ItemStructure).into_iter().collect(),
                "origin" => vec![Node::Scalar(Scalar::Str(h.origin().as_str()))],
                "name" => vec![Node::Text(h.name())],
                _ => Vec::new(),
            },
            Self::Event(e) => match attribute {
                "data" => vec![Node::ItemStructure(e.data())],
                "state" => e.state().map(Node::ItemStructure).into_iter().collect(),
                "time" => vec![Node::Scalar(Scalar::Str(e.time().as_str()))],
                "name" => vec![Node::Text(e.locatable().name())],
                _ => Vec::new(),
            },
            Self::DataValue(v) => data_value_children(v, attribute),
            Self::Text(t) => match attribute {
                "value" => vec![Node::Scalar(Scalar::Str(t.value()))],
                "defining_code" => t
                    .defining_code()
                    .map(|c| Node::Scalar(Scalar::Str(c.code_string())))
                    .into_iter()
                    .collect(),
                _ => Vec::new(),
            },
            Self::CodedText(t) => match attribute {
                "value" => vec![Node::Scalar(Scalar::Str(t.value()))],
                "defining_code" => {
                    vec![Node::Scalar(Scalar::Str(t.defining_code().code_string()))]
                }
                _ => Vec::new(),
            },
            Self::PlainText(t) => match attribute {
                "value" => vec![Node::Scalar(Scalar::Str(t.value()))],
                _ => Vec::new(),
            },
            Self::Interval(i) => interval_children(i, attribute),
            Self::ReferenceRange(r) => match attribute {
                "meaning" => vec![Node::PlainText(r.meaning())],
                "range" => vec![Node::Interval(r.range())],
                _ => Vec::new(),
            },
            Self::Scalar(_) => Vec::new(),
        }
    }
}

fn content_item_node(item: &ContentItem) -> Node<'_> {
    match item {
        ContentItem::Section(s) => Node::Section(s),
        ContentItem::Entry(e) => Node::Entry(e),
    }
}

fn item_node(item: &Item) -> Node<'_> {
    match item {
        Item::Cluster(c) => Node::Cluster(c),
        Item::Element(e) => Node::Element(e),
    }
}

fn entry_children<'a>(entry: &'a Entry, attribute: &str) -> Vec<Node<'a>> {
    match (entry, attribute) {
        (Entry::Observation(o), "data") => vec![Node::History(o.data())],
        (Entry::Observation(o), "state") => o.state().map(Node::History).into_iter().collect(),
        (Entry::Observation(o), "protocol") => o
            .care_entry()
            .protocol()
            .map(Node::ItemStructure)
            .into_iter()
            .collect(),
        (Entry::Evaluation(e), "data") => vec![Node::ItemStructure(e.data())],
        (Entry::AdminEntry(a), "data") => vec![Node::ItemStructure(a.data())],
        (Entry::Instruction(i), "narrative") => vec![Node::Text(i.narrative())],
        (Entry::Instruction(i), "activities") => i
            .activities()
            .iter()
            .map(|a| Node::ItemStructure(a.description()))
            .collect(),
        (Entry::Action(a), "description") => vec![Node::ItemStructure(a.description())],
        (Entry::Action(a), "time") => vec![Node::Scalar(Scalar::Str(a.time().as_str()))],
        (_, "name") => vec![Node::Text(entry.locatable().name())],
        _ => Vec::new(),
    }
}

fn item_structure_children<'a>(structure: &'a ItemStructure, attribute: &str) -> Vec<Node<'a>> {
    match (structure, attribute) {
        // Both spellings on an ITEM_SINGLE: openEHR names the attribute `item`,
        // but paths written against a template routinely say `items`, and the
        // two cannot mean anything different when there is exactly one.
        (ItemStructure::Single(s), "item" | "items") => vec![Node::Element(s.item())],
        (ItemStructure::List(l), "items") => l.items().iter().map(Node::Element).collect(),
        (ItemStructure::Table(t), "rows") => t.rows().iter().map(Node::Cluster).collect(),
        (ItemStructure::Tree(t), "items") => t.items().iter().map(item_node).collect(),
        (_, "name") => vec![Node::Text(structure.locatable().name())],
        _ => Vec::new(),
    }
}

/// The navigable attributes of a `DV_INTERVAL`.
///
/// The `*_unbounded` flags are derived rather than stored (see
/// [`crate::base::interval`]), and they are navigable anyway: an AQL query may
/// legitimately ask whether a range is open at one end, and the fact that the
/// answer is computed rather than looked up is not something a path should have
/// to know.
fn interval_children<'a>(interval: &'a DvInterval, attribute: &str) -> Vec<Node<'a>> {
    match attribute {
        "lower" => interval.lower().map(Node::DataValue).into_iter().collect(),
        "upper" => interval.upper().map(Node::DataValue).into_iter().collect(),
        "lower_unbounded" => vec![Node::Scalar(Scalar::Boolean(interval.lower_unbounded()))],
        "upper_unbounded" => vec![Node::Scalar(Scalar::Boolean(interval.upper_unbounded()))],
        "lower_included" => interval
            .lower_included()
            .map(|v| Node::Scalar(Scalar::Boolean(v)))
            .into_iter()
            .collect(),
        "upper_included" => interval
            .upper_included()
            .map(|v| Node::Scalar(Scalar::Boolean(v)))
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

/// The `DV_ORDERED` attributes, navigable from any of the five ordered data
/// values.
fn ordered_children<'a>(attrs: &'a OrderedAttrs, attribute: &str) -> Vec<Node<'a>> {
    match attribute {
        "normal_range" => attrs
            .normal_range()
            .map(Node::Interval)
            .into_iter()
            .collect(),
        "other_reference_ranges" => attrs
            .other_reference_ranges()
            .iter()
            .map(Node::ReferenceRange)
            .collect(),
        "normal_status" => attrs
            .normal_status()
            .map(|c| Node::Scalar(Scalar::Str(c.code_string())))
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn data_value_children<'a>(value: &'a DataValue, attribute: &str) -> Vec<Node<'a>> {
    // The three `DV_ORDERED` attributes first: they are shared by five classes,
    // and matching them per class below would be five chances to omit one.
    if let Some(attrs) = ordered_attrs_of(value) {
        let ordered = ordered_children(attrs, attribute);
        if !ordered.is_empty() {
            return ordered;
        }
    }
    if let DataValue::Interval(i) = value {
        return interval_children(i, attribute);
    }
    let scalar = match (value, attribute) {
        (DataValue::Quantity(q), "magnitude") => Scalar::Number(q.magnitude()),
        (DataValue::Quantity(q), "units") => Scalar::Str(q.units()),
        (DataValue::Quantity(q), "precision") => match q.precision() {
            Some(p) => Scalar::Integer(i64::from(p)),
            None => return Vec::new(),
        },
        (DataValue::Count(c), "magnitude") => Scalar::Integer(c.magnitude()),
        (DataValue::Ordinal(o), "value") => Scalar::Integer(o.value()),
        (DataValue::Ordinal(o), "symbol") => return vec![Node::CodedText(o.symbol())],
        (DataValue::Scale(s), "symbol") => return vec![Node::CodedText(s.symbol())],
        (DataValue::Scale(s), "value") => Scalar::Number(s.value()),
        (DataValue::Proportion(p), "numerator") => Scalar::Number(p.numerator()),
        (DataValue::Proportion(p), "denominator") => Scalar::Number(p.denominator()),
        (DataValue::Boolean(b), "value") => Scalar::Boolean(b.value()),
        (DataValue::Text(t), "value") => Scalar::Str(t.value()),
        (DataValue::CodedText(t), "value") => Scalar::Str(t.value()),
        (DataValue::CodedText(t), "defining_code") => Scalar::Str(t.defining_code().code_string()),
        (DataValue::Date(d), "value") => Scalar::Str(d.as_str()),
        (DataValue::Time(t), "value") => Scalar::Str(t.as_str()),
        (DataValue::DateTime(t), "value") => Scalar::Str(t.as_str()),
        (DataValue::Duration(d), "value") => Scalar::Str(d.as_str()),
        (DataValue::Uri(u), "value") => Scalar::Str(u.value()),
        (DataValue::EhrUri(u), "value") => Scalar::Str(u.value()),
        (DataValue::Identifier(i), "id") => Scalar::Str(i.id()),
        _ => return Vec::new(),
    };
    vec![Node::Scalar(scalar)]
}

/// The `DV_ORDERED` attributes of a value, for the nine classes that have them.
///
/// The four temporal types were missing here until `lib:A-29`. They implement
/// [`DvOrdered`] and carry the attributes like any other, so a `DV_DATE` could
/// hold a normal range that no path could reach — and `Q12.7a` requires
/// navigation to reach them. "Results outside their own normal range" is the
/// query that requirement exists for, and a due date or a gestational age is
/// exactly the kind of value a clinician asks it about.
fn ordered_attrs_of(value: &DataValue) -> Option<&OrderedAttrs> {
    Some(match value {
        DataValue::Ordinal(v) => v.ordered_attrs(),
        DataValue::Scale(v) => v.ordered_attrs(),
        DataValue::Quantity(v) => v.ordered_attrs(),
        DataValue::Count(v) => v.ordered_attrs(),
        DataValue::Proportion(v) => v.ordered_attrs(),
        DataValue::Date(v) => v.ordered_attrs(),
        DataValue::Time(v) => v.ordered_attrs(),
        DataValue::DateTime(v) => v.ordered_attrs(),
        DataValue::Duration(v) => v.ordered_attrs(),
        _ => return None,
    })
}

/// Navigation by path.
///
/// Implemented for the classes a path can be rooted at. Rooting at a
/// [`Composition`] is the usual case and the one AQL assumes.
pub trait Pathable {
    /// This object as a [`Node`].
    fn as_node(&self) -> Node<'_>;

    /// Every node the path matches, in document order.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::Malformed`] if the path text is not a path.
    fn items_at_path(&self, path: &str) -> Result<Vec<Node<'_>>, PathError> {
        let parsed: Path = path.parse()?;
        Ok(resolve(self.as_node(), &parsed))
    }

    /// The single node the path matches.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::NoMatch`] if nothing matched, or
    /// [`PathError::NotUnique`] if more than one node did. The second is the
    /// one that matters: taking the first of several is how a repeated
    /// archetype node silently yields one of three answers.
    fn item_at_path(&self, path: &str) -> Result<Node<'_>, PathError> {
        let matches = self.items_at_path(path)?;
        match matches.len() {
            0 => Err(PathError::NoMatch {
                path: path.to_owned(),
            }),
            1 => Ok(matches[0]),
            n => Err(PathError::NotUnique {
                path: path.to_owned(),
                count: n,
            }),
        }
    }

    /// Whether the path matches anything.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::Malformed`] if the path text is not a path.
    fn path_exists(&self, path: &str) -> Result<bool, PathError> {
        Ok(!self.items_at_path(path)?.is_empty())
    }

    /// Whether the path matches exactly one node.
    ///
    /// # Errors
    ///
    /// Returns [`PathError::Malformed`] if the path text is not a path.
    fn path_unique(&self, path: &str) -> Result<bool, PathError> {
        Ok(self.items_at_path(path)?.len() == 1)
    }
}

fn resolve<'a>(root: Node<'a>, path: &Path) -> Vec<Node<'a>> {
    let mut current = vec![root];
    for segment in path.segments() {
        let mut next = Vec::new();
        for node in current {
            let children = node.children(&segment.attribute);
            for (index, child) in children.into_iter().enumerate() {
                let node_id = child.archetype_node_id().unwrap_or_default();
                let name = child.name().unwrap_or_default();
                if segment.matches(node_id, name, index) {
                    next.push(child);
                }
            }
        }
        if next.is_empty() {
            return Vec::new();
        }
        current = next;
    }
    current
}

impl Pathable for Composition {
    fn as_node(&self) -> Node<'_> {
        Node::Composition(self)
    }
}

impl Pathable for Section {
    fn as_node(&self) -> Node<'_> {
        Node::Section(self)
    }
}

impl Pathable for Entry {
    fn as_node(&self) -> Node<'_> {
        Node::Entry(self)
    }
}

impl Pathable for ItemStructure {
    fn as_node(&self) -> Node<'_> {
        Node::ItemStructure(self)
    }
}

impl Pathable for Cluster {
    fn as_node(&self) -> Node<'_> {
        Node::Cluster(self)
    }
}

impl Pathable for Element {
    fn as_node(&self) -> Node<'_> {
        Node::Element(self)
    }
}

impl Pathable for History {
    fn as_node(&self) -> Node<'_> {
        Node::History(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rm::common::{LocatableAttrs, PartyIdentified};
    use crate::rm::data_structures::{ItemTree, PointEvent};
    use crate::rm::data_types::{CodePhrase, DvDateTime, DvQuantity};
    use crate::rm::ehr::{EntryAttrs, Observation};
    use crate::terminology;

    fn attrs(name: &str, node: &str) -> LocatableAttrs {
        LocatableAttrs::named(name, node).unwrap()
    }

    fn blood_pressure() -> Composition {
        let element = |name: &str, node: &str, v: f64| {
            Item::Element(Element::new(
                attrs(name, node),
                DataValue::Quantity(DvQuantity::new(v, "mm[Hg]").unwrap()),
            ))
        };
        let data = ItemTree::new(
            attrs("blood pressure", "at0003"),
            vec![
                element("Systolic", "at0004", 184.0),
                element("Diastolic", "at0005", 96.0),
            ],
        );
        let event = PointEvent::new(
            attrs("any event", "at0006"),
            DvDateTime::new("2026-07-31T09:15:00Z").unwrap(),
            data.into(),
        );
        let history = History::new(
            attrs("Event Series", "at0001"),
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            vec![event.into()],
            None,
        )
        .unwrap();
        let observation = Observation::new(
            attrs(
                "Blood pressure",
                "openEHR-EHR-OBSERVATION.blood_pressure.v2",
            ),
            EntryAttrs::about_subject(
                CodePhrase::new("ISO_639-1", "en").unwrap(),
                CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
            ),
            history,
        );
        Composition::new(
            attrs("Encounter", "openEHR-EHR-COMPOSITION.encounter.v1"),
            terminology::composition_category::EVENT,
            PartyIdentified::named("Dr A Nurse").unwrap().into(),
            CodePhrase::new("ISO_639-1", "en").unwrap(),
            CodePhrase::new("ISO_3166-1", "GB").unwrap(),
        )
        .unwrap()
        .with_content(observation.into())
    }

    #[test]
    fn a_full_path_reaches_a_magnitude() {
        let c = blood_pressure();
        let node = c
            .item_at_path(
                "/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/data/events[at0006]/data/items[at0004]/value/magnitude",
            )
            .unwrap();
        assert_eq!(node, Node::Scalar(Scalar::Number(184.0)));
    }

    #[test]
    fn an_ambiguous_path_fails_instead_of_taking_the_first() {
        let c = blood_pressure();
        // Two elements under `items`, no predicate: this is the case where
        // returning the first would silently pick systolic over diastolic.
        let err = c
            .item_at_path("/content/data/events/data/items/value/magnitude")
            .unwrap_err();
        assert!(
            matches!(err, PathError::NotUnique { count: 2, .. }),
            "{err:?}"
        );
        assert_eq!(
            c.items_at_path("/content/data/events/data/items/value/magnitude")
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn name_predicates_select_the_right_repeat() {
        let c = blood_pressure();
        let diastolic = c
            .item_at_path("/content/data/events/data/items['Diastolic']/value/magnitude")
            .unwrap();
        assert_eq!(diastolic, Node::Scalar(Scalar::Number(96.0)));
        let combined = c
            .item_at_path("/content/data/events/data/items[at0004, 'Systolic']/value/magnitude")
            .unwrap();
        assert_eq!(combined, Node::Scalar(Scalar::Number(184.0)));
    }

    #[test]
    fn index_predicates_are_zero_based_and_quoted_digits_are_names() {
        let c = blood_pressure();
        let first = c
            .item_at_path("/content/data/events/data/items[0]/value/magnitude")
            .unwrap();
        assert_eq!(first, Node::Scalar(Scalar::Number(184.0)));
        // `['0']` is a name, not an index, and nothing here is named "0".
        assert!(
            !c.path_exists("/content/data/events/data/items['0']")
                .unwrap()
        );
    }

    #[test]
    fn a_wrong_attribute_is_no_match_not_an_error() {
        let c = blood_pressure();
        assert!(
            !c.path_exists("/content/data/events/data/nonexistent")
                .unwrap()
        );
        assert!(matches!(
            c.item_at_path("/content/data/events/data/nonexistent"),
            Err(PathError::NoMatch { .. })
        ));
    }

    #[test]
    fn reference_ranges_are_navigable() {
        use crate::base::Interval;
        use crate::rm::data_types::{DvText, ReferenceRange};

        let bound = |v: f64| DataValue::Quantity(DvQuantity::new(v, "mmol/l").unwrap());
        let quantity = DvQuantity::new(9.9, "mmol/l")
            .unwrap()
            .with_normal_range(Interval::closed(bound(3.5), bound(5.5)).unwrap())
            .with_other_reference_range(ReferenceRange::new(
                DvText::new("therapeutic").unwrap(),
                Interval::closed(bound(4.0), bound(5.0)).unwrap(),
            ))
            .with_normal_status(
                crate::rm::data_types::CodePhrase::openehr(
                    crate::terminology::normal_status::VERY_HIGH,
                )
                .unwrap(),
            );
        let element: ItemStructure = crate::rm::data_structures::ItemSingle::new(
            attrs("s", "at0001"),
            Element::new(attrs("K", "at0004"), DataValue::Quantity(quantity)),
        )
        .into();

        // Into the normal range and out the other side to a magnitude.
        assert_eq!(
            element
                .item_at_path("/item/value/normal_range/lower/magnitude")
                .unwrap(),
            Node::Scalar(Scalar::Number(3.5))
        );
        assert_eq!(
            element
                .item_at_path("/item/value/normal_range/upper_included")
                .unwrap(),
            Node::Scalar(Scalar::Boolean(true))
        );
        // A named reference range, and the text inside it.
        assert_eq!(
            element
                .item_at_path("/item/value/other_reference_ranges/meaning/value")
                .unwrap(),
            Node::Scalar(Scalar::Str("therapeutic"))
        );
        assert_eq!(
            element
                .item_at_path("/item/value/other_reference_ranges/range/upper/magnitude")
                .unwrap(),
            Node::Scalar(Scalar::Number(5.0))
        );
        // And the abnormal flag.
        assert_eq!(
            element.item_at_path("/item/value/normal_status").unwrap(),
            Node::Scalar(Scalar::Str("HHH"))
        );
    }

    #[test]
    fn an_interval_valued_element_is_navigable() {
        use crate::base::Interval;

        // A DV_INTERVAL as an ELEMENT's own value: a blood-pressure target of
        // 130–140 is one value, not two.
        let bound = |v: f64| DataValue::Quantity(DvQuantity::new(v, "mm[Hg]").unwrap());
        let target = DataValue::Interval(Box::new(
            Interval::closed(bound(130.0), bound(140.0)).unwrap(),
        ));
        let structure: ItemStructure = crate::rm::data_structures::ItemSingle::new(
            attrs("s", "at0001"),
            Element::new(attrs("Target", "at0004"), target),
        )
        .into();

        assert_eq!(
            structure
                .item_at_path("/item/value/lower/magnitude")
                .unwrap(),
            Node::Scalar(Scalar::Number(130.0))
        );
        assert_eq!(
            structure.item_at_path("/item/value/upper/units").unwrap(),
            Node::Scalar(Scalar::Str("mm[Hg]"))
        );
        assert_eq!(
            structure
                .item_at_path("/item/value/lower_unbounded")
                .unwrap(),
            Node::Scalar(Scalar::Boolean(false))
        );
        // An attribute an interval does not have is still no match, not an
        // error.
        assert!(!structure.path_exists("/item/value/middle").unwrap());
    }

    #[test]
    fn malformed_paths_report_where_they_broke() {
        for text in ["/content[at0001", "/content//data", "/content/", "//"] {
            let err = text.parse::<Path>();
            assert!(err.is_err(), "accepted {text}");
        }
        assert!("".parse::<Path>().unwrap().is_root());
        assert!("/".parse::<Path>().unwrap().is_root());
    }

    #[test]
    fn predicates_round_trip_through_display_in_long_form() {
        let p: Path = "/data[at0001]/events[at0006, 'any event']".parse().unwrap();
        let printed = p.to_string();
        let reparsed: Path = printed.parse().unwrap();
        assert_eq!(reparsed, p);
    }

    /// Renders whatever a navigation step returned, so a table can state the
    /// expected result as one string.
    ///
    /// A node that is not a scalar renders as its class name: the point of the
    /// table below is *which attribute reaches what*, and re-asserting a
    /// quantity's magnitude under `normal_range/lower` would only restate the
    /// row above it.
    fn rendered(nodes: &[Node<'_>]) -> String {
        nodes
            .iter()
            .map(|n| match n {
                Node::Scalar(s) => s.to_display_string(),
                other => other.type_name().to_owned(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Every attribute a `DATA_VALUE` can be navigated through, and what each
    /// one reaches.
    ///
    /// This exists because mutation testing deleted **nineteen** arms of
    /// `data_value_children` one at a time and the suite stayed green
    /// (`lib:A-09`). A deleted arm does not error: `children` returns an empty
    /// vector for an attribute a node does not have, which
    /// [`Pathable::item_at_path`] reports as `NoMatch`. So the failure mode is
    /// a query that silently finds nothing — the shape of defect `db:P6.15`
    /// exists to forbid one level up, because "this patient has no systolic
    /// reading" and "I could not follow the path" are different answers and
    /// only one of them is true.
    ///
    /// Written as canonical JSON rather than constructors on purpose: it is
    /// the form these values actually arrive in, and it keeps a row to a line.
    // Long because it is a table; the alternative is several tables that each
    // cover part of one function, which is how an arm gets missed.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn every_navigable_attribute_of_a_data_value_reaches_its_value() {
        const QUANTITY: &str = r#"{"_type":"DV_QUANTITY","magnitude":140.0,"units":"mm[Hg]","precision":1,
            "normal_range":{"lower":{"_type":"DV_QUANTITY","magnitude":90.0,"units":"mm[Hg]"},
                            "upper":{"_type":"DV_QUANTITY","magnitude":120.0,"units":"mm[Hg]"},
                            "lower_unbounded":false,"upper_unbounded":false},
            "normal_status":{"terminology_id":{"value":"openehr"},"code_string":"H"},
            "other_reference_ranges":[{"meaning":{"value":"therapeutic"},
                "range":{"lower":{"_type":"DV_QUANTITY","magnitude":80.0,"units":"mm[Hg]"},
                          "lower_unbounded":false,"upper_unbounded":true}}]}"#;
        const ORDINAL: &str = r#"{"_type":"DV_ORDINAL","value":2,
            "symbol":{"value":"moderate","defining_code":{"terminology_id":{"value":"local"},"code_string":"at0003"}}}"#;
        const SCALE: &str = r#"{"_type":"DV_SCALE","value":2.5,
            "symbol":{"value":"often","defining_code":{"terminology_id":{"value":"local"},"code_string":"at0004"}}}"#;
        const CODED: &str = r#"{"_type":"DV_CODED_TEXT","value":"female",
            "defining_code":{"terminology_id":{"value":"local"},"code_string":"at0022"}}"#;
        // Unbounded above, and `upper_included` therefore absent rather than
        // false — the two are different facts (`crate::base::interval`).
        const INTERVAL: &str = r#"{"_type":"DV_INTERVAL",
            "lower":{"_type":"DV_COUNT","magnitude":1},"lower_included":true,
            "lower_unbounded":false,"upper_unbounded":true}"#;

        // The same `normal_status` on each of the other DV_ORDERED classes.
        let ranged = |body: &str| {
            format!(
                r#"{{{body},"normal_status":{{"terminology_id":{{"value":"openehr"}},"code_string":"L"}}}}"#
            )
        };
        let ordinal_ranged = ranged(
            r#""_type":"DV_ORDINAL","value":1,"symbol":{"value":"mild","defining_code":{"terminology_id":{"value":"local"},"code_string":"at0002"}}"#,
        );
        let scale_ranged = ranged(
            r#""_type":"DV_SCALE","value":1.0,"symbol":{"value":"rarely","defining_code":{"terminology_id":{"value":"local"},"code_string":"at0005"}}"#,
        );
        let count_ranged = ranged(r#""_type":"DV_COUNT","magnitude":3"#);
        let proportion_ranged =
            ranged(r#""_type":"DV_PROPORTION","numerator":1.0,"denominator":4.0,"type":0"#);
        let (rd, rt, rdt, rdur) = (
            ranged(r#""_type":"DV_DATE","value":"2026-08-03""#),
            ranged(r#""_type":"DV_TIME","value":"09:30:00""#),
            ranged(r#""_type":"DV_DATE_TIME","value":"2026-08-03T09:30:00Z""#),
            ranged(r#""_type":"DV_DURATION","value":"P1D""#),
        );
        let (ranged_date, ranged_time, ranged_date_time, ranged_duration) =
            (rd.as_str(), rt.as_str(), rdt.as_str(), rdur.as_str());
        let (ranged_ordinal, ranged_scale, ranged_count, ranged_proportion) = (
            ordinal_ranged.as_str(),
            scale_ranged.as_str(),
            count_ranged.as_str(),
            proportion_ranged.as_str(),
        );

        let cases: &[(&str, &str, &str)] = &[
            // DV_QUANTITY, and through it the three DV_ORDERED attributes that
            // five classes share.
            (QUANTITY, "magnitude", "140"),
            (QUANTITY, "units", "mm[Hg]"),
            (QUANTITY, "precision", "1"),
            (QUANTITY, "normal_range", "DV_INTERVAL"),
            (QUANTITY, "normal_status", "H"),
            (QUANTITY, "other_reference_ranges", "REFERENCE_RANGE"),
            (QUANTITY, "nonesuch", ""),
            // The other four classes that carry the `DV_ORDERED` attributes.
            // `ordered_attrs_of` maps each to the same shared block, and a
            // deleted arm silently removes a class's reference ranges — the
            // thing that tells a clinician whether a value is normal
            // (`Q12.7a`). Testing only DV_QUANTITY left four unguarded.
            (ranged_ordinal, "normal_status", "L"),
            (ranged_scale, "normal_status", "L"),
            (ranged_count, "normal_status", "L"),
            (ranged_proportion, "normal_status", "L"),
            // The four temporal types. They are `DV_ORDERED` too, and until
            // `lib:A-29` a normal range on one of them was unreachable
            // although the model carried it.
            (ranged_date, "normal_status", "L"),
            (ranged_time, "normal_status", "L"),
            (ranged_date_time, "normal_status", "L"),
            (ranged_duration, "normal_status", "L"),
            // DV_COUNT, DV_ORDINAL, DV_SCALE, DV_PROPORTION.
            (r#"{"_type":"DV_COUNT","magnitude":7}"#, "magnitude", "7"),
            (ORDINAL, "value", "2"),
            (ORDINAL, "symbol", "DV_CODED_TEXT"),
            (SCALE, "value", "2.5"),
            (SCALE, "symbol", "DV_CODED_TEXT"),
            (
                r#"{"_type":"DV_PROPORTION","numerator":1.0,"denominator":4.0,"type":0}"#,
                "numerator",
                "1",
            ),
            (
                r#"{"_type":"DV_PROPORTION","numerator":1.0,"denominator":4.0,"type":0}"#,
                "denominator",
                "4",
            ),
            // The text and scalar values.
            (r#"{"_type":"DV_BOOLEAN","value":true}"#, "value", "true"),
            (r#"{"_type":"DV_TEXT","value":"free text"}"#, "value", "free text"),
            (CODED, "value", "female"),
            (CODED, "defining_code", "at0022"),
            (r#"{"_type":"DV_DATE","value":"2026-08-03"}"#, "value", "2026-08-03"),
            (r#"{"_type":"DV_TIME","value":"09:30:00"}"#, "value", "09:30:00"),
            (
                r#"{"_type":"DV_DATE_TIME","value":"2026-08-03T09:30:00Z"}"#,
                "value",
                "2026-08-03T09:30:00Z",
            ),
            (r#"{"_type":"DV_DURATION","value":"P1D"}"#, "value", "P1D"),
            (
                r#"{"_type":"DV_URI","value":"https://example.org/x"}"#,
                "value",
                "https://example.org/x",
            ),
            (
                r#"{"_type":"DV_EHR_URI","value":"ehr://x/y"}"#,
                "value",
                "ehr://x/y",
            ),
            (
                r#"{"_type":"DV_IDENTIFIER","id":"NHS-12345","issuer":"","assigner":"","type":""}"#,
                "id",
                "NHS-12345",
            ),
            // DV_INTERVAL as an ELEMENT's own value. The `*_unbounded` flags
            // are derived and navigable anyway; `upper_included` is absent
            // because there is no upper bound to include.
            (INTERVAL, "lower", "DV_COUNT"),
            (INTERVAL, "upper", ""),
            (INTERVAL, "lower_included", "true"),
            (INTERVAL, "upper_included", ""),
            (INTERVAL, "lower_unbounded", "false"),
            (INTERVAL, "upper_unbounded", "true"),
        ];

        for (json, attribute, want) in cases {
            let value: DataValue = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("fixture does not deserialize: {e}\n{json}"));
            let got = rendered(&Node::DataValue(&value).children(attribute));
            assert_eq!(
                got, *want,
                "navigating `{attribute}` reached {got:?}, wanted {want:?}"
            );
        }
    }

    /// Every attribute of every structural node, and what each one reaches.
    ///
    /// The companion to the data-value table above, and it exists for the same
    /// reason: mutation testing deleted **twenty-nine** arms across
    /// `Node::children`, `entry_children` and `item_structure_children`, one at
    /// a time, and nothing failed (`lib:A-09`).
    ///
    /// The consequence of a deleted arm is worth stating plainly, because it is
    /// not a crash. `children` answers an unknown attribute with an empty
    /// vector — deliberately, so that a wrong attribute is `NoMatch` rather
    /// than an error. So a lost arm turns a path that *should* resolve into a
    /// path that finds nothing, and an AQL query returns no rows. That is the
    /// clinically dangerous direction: an empty result set reads as "there is
    /// no such record".
    ///
    /// `name` is checked on every one of them. It is the arm most likely to be
    /// deleted as duplication — it appears eleven times — and it is the arm
    /// every archetype predicate in a path depends on.
    // Long because it builds one of every structural node before it can ask
    // anything of them, and splitting the fixtures out would only move the
    // lines somewhere the borrows do not reach. `Node::children` carries the
    // same allow for the same reason.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn every_navigable_attribute_of_a_structural_node_reaches_its_value() {
        use crate::rm::data_structures::{Cluster, ItemList, ItemSingle, ItemTable};
        use crate::rm::data_structures::Event;
        use crate::rm::ehr::{
            Action, AdminEntry, CareEntryAttrs, Evaluation, Instruction, IsmTransition, Section,
        };

        let entry_attrs = || {
            EntryAttrs::about_subject(
                CodePhrase::new("ISO_639-1", "en").unwrap(),
                CodePhrase::new("IANA_character-sets", "UTF-8").unwrap(),
            )
        };
        let element = |name: &str, node: &str, v: f64| {
            Element::new(
                attrs(name, node),
                DataValue::Quantity(DvQuantity::new(v, "mm[Hg]").unwrap()),
            )
        };
        let single = |name: &str| ItemSingle::new(attrs(name, "at0100"), element("s", "at0101", 1.0));

        // --- COMPOSITION, SECTION -------------------------------------------
        let composition = blood_pressure();
        let coded_name = Text::Coded(
            DvCodedText::new(
                "Systolic",
                CodePhrase::new("local", "at0004").unwrap(),
            )
            .unwrap(),
        );
        let section = Section::new(
            attrs("Findings", "at0200"),
            vec![crate::rm::ehr::ContentItem::Section(Section::new(
                attrs("Nested", "at0201"),
                vec![],
            ))],
        );

        // --- the five ENTRY kinds -------------------------------------------
        // OBSERVATION carries `state` and `protocol` as well as `data`; both
        // are optional and both were unreachable-and-unnoticed.
        let state = History::new(
            attrs("State Series", "at0300"),
            DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
            vec![PointEvent::new(
                attrs("state event", "at0301"),
                DvDateTime::new("2026-07-31T09:05:00Z").unwrap(),
                ItemTree::new(attrs("position", "at0302"), vec![]).into(),
            )
            .into()],
            None,
        )
        .unwrap();
        let observation = Observation::new(
            attrs("Blood pressure", "openEHR-EHR-OBSERVATION.blood_pressure.v2"),
            entry_attrs(),
            History::new(
                attrs("Event Series", "at0001"),
                DvDateTime::new("2026-07-31T09:00:00Z").unwrap(),
                vec![PointEvent::new(
                    attrs("any event", "at0006"),
                    DvDateTime::new("2026-07-31T09:15:00Z").unwrap(),
                    ItemTree::new(attrs("bp", "at0003"), vec![]).into(),
                )
                .into()],
                None,
            )
            .unwrap(),
        )
        .with_state(state)
        .with_care_entry(CareEntryAttrs::default().with_protocol(single("protocol").into()));
        let evaluation = Evaluation::new(
            attrs("Problem", "openEHR-EHR-EVALUATION.problem_diagnosis.v1"),
            entry_attrs(),
            single("evaluation data").into(),
        );
        let admin = AdminEntry::new(
            attrs("Admission", "openEHR-EHR-ADMIN_ENTRY.admission.v0"),
            entry_attrs(),
            single("admin data").into(),
        );
        let instruction = Instruction::new(
            attrs("Order", "openEHR-EHR-INSTRUCTION.medication_order.v3"),
            entry_attrs(),
            Text::Plain(DvText::new("Give 5mg at once").unwrap()),
            vec![crate::rm::ehr::Activity::new(
                attrs("activity", "at0400"),
                single("activity description").into(),
                "openEHR-EHR-ACTION.medication.v1",
            )
            .unwrap()],
        )
        .unwrap();
        let action = Action::new(
            attrs("Given", "openEHR-EHR-ACTION.medication.v1"),
            entry_attrs(),
            DvDateTime::new("2026-07-31T10:00:00Z").unwrap(),
            single("action description").into(),
            IsmTransition::new("532").unwrap(),
        );

        // --- the four ITEM_STRUCTUREs, CLUSTER, ELEMENT ---------------------
        let item_single = single("single");
        let item_list = ItemList::new(attrs("list", "at0500"), vec![element("l", "at0501", 2.0)]);
        let item_table = ItemTable::new(
            attrs("table", "at0600"),
            vec![Cluster::new(
                attrs("row", "at0601"),
                vec![Item::Element(element("cell", "at0602", 6.0))],
            )
            .unwrap()],
        );
        let item_tree = ItemTree::new(
            attrs("tree", "at0700"),
            vec![Item::Element(element("t", "at0701", 3.0))],
        );
        let cluster = Cluster::new(
            attrs("cluster", "at0800"),
            vec![Item::Element(element("c", "at0801", 4.0))],
        )
        .unwrap();
        let valued = element("valued", "at0900", 5.0);
        // An ELEMENT with no value and a reason there is none. `null_flavour`
        // is the attribute that distinguishes "not measured" from "measured as
        // nothing", and it was reachable by no test.
        let absent = Element::new_null(attrs("absent", "at0901"), "253").unwrap();

        // --- HISTORY, EVENT --------------------------------------------------
        let history = History::new(
            attrs("Series", "at1000"),
            DvDateTime::new("2026-07-31T08:00:00Z").unwrap(),
            vec![PointEvent::new(
                attrs("point", "at1001"),
                DvDateTime::new("2026-07-31T08:30:00Z").unwrap(),
                item_single.clone().into(),
            )
            .into()],
            Some(single("summary").into()),
        )
        .unwrap();
        let event: Event = PointEvent::new(
            attrs("point", "at1100"),
            DvDateTime::new("2026-07-31T08:30:00Z").unwrap(),
            item_single.clone().into(),
        )
        .into();
        let event_with_state = PointEvent::new(
            attrs("point", "at1101"),
            DvDateTime::new("2026-07-31T08:35:00Z").unwrap(),
            item_single.clone().into(),
        )
        .with_state(single("event state").into());

        let observation_entry = Entry::Observation(observation);
        let evaluation_entry = Entry::Evaluation(evaluation);
        let admin_entry = Entry::AdminEntry(admin);
        let instruction_entry = Entry::Instruction(instruction);
        let action_entry = Entry::Action(action);
        let single_structure: ItemStructure = item_single.clone().into();
        let list_structure: ItemStructure = item_list.into();
        let table_structure: ItemStructure = item_table.into();
        let tree_structure: ItemStructure = item_tree.into();
        let event_state: Event = event_with_state.into();

        let cases: &[(Node<'_>, &str, &str)] = &[
            (Node::Composition(&composition), "content", "OBSERVATION"),
            (Node::Composition(&composition), "name", "DV_TEXT"),
            (Node::Composition(&composition), "category", "DV_CODED_TEXT"),
            (Node::Composition(&composition), "nonesuch", ""),
            // Through a `name` and a `category`, one step further. A DV_TEXT
            // name has no `defining_code`; a DV_CODED_TEXT category always
            // has one. Nothing had ever navigated past a name.
            (Node::Text(composition.name()), "value", "Encounter"),
            (Node::Text(composition.name()), "defining_code", ""),
            // A `name` that *is* coded. An archetype's node names are
            // routinely coded, and this is the only way the `defining_code`
            // arm of `Node::Text` is reached at all — the plain case above
            // yields nothing whether the arm is there or not.
            (Node::Text(&coded_name), "value", "Systolic"),
            (Node::Text(&coded_name), "defining_code", "at0004"),
            (Node::CodedText(composition.category()), "value", "event"),
            (Node::CodedText(composition.category()), "defining_code", "433"),
            (Node::Section(&section), "items", "SECTION"),
            (Node::Section(&section), "name", "DV_TEXT"),
            // The five ENTRY kinds. `name` is shared and comes last in the
            // match, so every kind must still reach it.
            (Node::Entry(&observation_entry), "data", "HISTORY"),
            (Node::Entry(&observation_entry), "state", "HISTORY"),
            (Node::Entry(&observation_entry), "protocol", "ITEM_SINGLE"),
            (Node::Entry(&observation_entry), "name", "DV_TEXT"),
            (Node::Entry(&evaluation_entry), "data", "ITEM_SINGLE"),
            (Node::Entry(&evaluation_entry), "name", "DV_TEXT"),
            (Node::Entry(&admin_entry), "data", "ITEM_SINGLE"),
            (Node::Entry(&admin_entry), "name", "DV_TEXT"),
            (Node::Entry(&instruction_entry), "narrative", "DV_TEXT"),
            (Node::Entry(&instruction_entry), "activities", "ITEM_SINGLE"),
            (Node::Entry(&instruction_entry), "name", "DV_TEXT"),
            (Node::Entry(&action_entry), "description", "ITEM_SINGLE"),
            (Node::Entry(&action_entry), "time", "2026-07-31T10:00:00Z"),
            (Node::Entry(&action_entry), "name", "DV_TEXT"),
            // An OBSERVATION has no `narrative`; an INSTRUCTION has no `data`.
            // Both must be no-match, not a wrong node.
            (Node::Entry(&observation_entry), "narrative", ""),
            (Node::Entry(&instruction_entry), "data", ""),
            // The four ITEM_STRUCTUREs. `item` and `items` are both accepted on
            // an ITEM_SINGLE, which is a deliberate departure documented at the
            // match arm.
            (Node::ItemStructure(&single_structure), "item", "ELEMENT"),
            (Node::ItemStructure(&single_structure), "items", "ELEMENT"),
            (Node::ItemStructure(&single_structure), "name", "DV_TEXT"),
            (Node::ItemStructure(&list_structure), "items", "ELEMENT"),
            (Node::ItemStructure(&list_structure), "name", "DV_TEXT"),
            (Node::ItemStructure(&table_structure), "rows", "CLUSTER"),
            (Node::ItemStructure(&table_structure), "name", "DV_TEXT"),
            (Node::ItemStructure(&tree_structure), "items", "ELEMENT"),
            (Node::ItemStructure(&tree_structure), "name", "DV_TEXT"),
            // A list has no rows and a table has no items.
            (Node::ItemStructure(&list_structure), "rows", ""),
            (Node::ItemStructure(&table_structure), "items", ""),
            (Node::Cluster(&cluster), "items", "ELEMENT"),
            (Node::Cluster(&cluster), "name", "DV_TEXT"),
            (Node::Element(&valued), "value", "DV_QUANTITY"),
            (Node::Element(&valued), "name", "DV_TEXT"),
            (Node::Element(&valued), "null_flavour", ""),
            (Node::Element(&absent), "value", ""),
            (Node::Element(&absent), "null_flavour", "253"),
            (Node::History(&history), "events", "POINT_EVENT"),
            (Node::History(&history), "summary", "ITEM_SINGLE"),
            (Node::History(&history), "origin", "2026-07-31T08:00:00Z"),
            (Node::History(&history), "name", "DV_TEXT"),
            (Node::Event(&event), "data", "ITEM_SINGLE"),
            (Node::Event(&event), "time", "2026-07-31T08:30:00Z"),
            (Node::Event(&event), "name", "DV_TEXT"),
            (Node::Event(&event), "state", ""),
            (Node::Event(&event_state), "state", "ITEM_SINGLE"),
        ];

        for (node, attribute, want) in cases {
            let got = rendered(&node.children(attribute));
            assert_eq!(
                got,
                *want,
                "{}/{attribute} reached {got:?}, wanted {want:?}",
                node.type_name()
            );
        }
    }

    /// The path parser's predicate scanner, and the offsets it reports.
    ///
    /// Fourteen mutants in `parse_path` survived (`lib:A-09`): every `+` and
    /// `-` in the offset arithmetic, and the branch that tracks whether the
    /// scanner is inside a quoted string. The scanner is what decides where a
    /// predicate **ends** — a quoted name may legally contain `]`, and if the
    /// quote tracking is wrong the predicate is truncated and the path silently
    /// selects the wrong node, or none.
    ///
    /// `Q12.2` and `Q12.12` require the offset to say where parsing stopped.
    /// Every offset here was free to be any other number.
    #[test]
    fn a_predicate_is_scanned_through_quotes_and_errors_report_where() {
        // A quoted name containing the character that would otherwise close
        // the predicate.
        let p: Path = "/items['a]b']/value".parse().unwrap();
        assert_eq!(p.segments().len(), 2);
        assert_eq!(p.segments()[0].conditions, vec![Condition::Name("a]b".into())]);
        // And with double quotes, which is the other branch of the same arm.
        let p: Path = r#"/items["a]b"]/value"#.parse().unwrap();
        assert_eq!(p.segments()[0].conditions, vec![Condition::Name("a]b".into())]);

        // Offsets. `base` is the length of the leading `/` the parser strips,
        // so the same malformed path with and without one must report offsets
        // that differ by exactly one.
        let offset_of = |text: &str| match text.parse::<Path>() {
            Err(PathError::Malformed { offset, .. }) => offset,
            other => panic!("{text} did not fail as malformed: {other:?}"),
        };
        assert_eq!(offset_of("/items[at0001"), 6, "unclosed `[` points at the `[`");
        assert_eq!(offset_of("items[at0001"), 5, "without the leading slash");
        assert_eq!(offset_of("/items/"), 7, "trailing `/` points past the end");
        assert_eq!(offset_of("/items//value"), 7, "an empty attribute name");
        assert_eq!(
            offset_of("/items[at0001]x/value"),
            14,
            "a segment must be followed by `/`"
        );
        // A predicate whose contents are bad reports a position *inside* the
        // brackets, not the start of the path.
        assert_eq!(
            offset_of("/items[bad/attr='x']"),
            7,
            "a bad predicate points just past the `[`"
        );
        // The same, one segment further in, so a mutated `+` shows up.
        assert_eq!(offset_of("/content/items[bad/attr='x']"), 15);
    }

    /// A predicate holding more than one condition.
    ///
    /// `split_conjunctions` and `is_word_boundary` are eleven surviving
    /// mutants between them. They decide where one condition ends and the next
    /// begins, and they must not split inside a quoted value — a name
    /// containing the word "and", or a comma, is ordinary text.
    #[test]
    fn conditions_are_split_on_and_and_comma_but_not_inside_a_name() {
        let conditions = |text: &str| text.parse::<Path>().unwrap().segments()[0].conditions.clone();

        // Both separators, and both together.
        assert_eq!(
            conditions("/items[archetype_node_id='at0004' and name/value='Systolic']"),
            vec![
                Condition::NodeId("at0004".into()),
                Condition::Name("Systolic".into()),
            ]
        );
        assert_eq!(
            conditions("/items[archetype_node_id='at0004', name/value='Systolic']"),
            vec![
                Condition::NodeId("at0004".into()),
                Condition::Name("Systolic".into()),
            ]
        );

        // A quoted value containing a comma, and one containing ` and `. If
        // the scanner splits inside the quotes it produces two conditions
        // neither of which matches anything.
        assert_eq!(
            conditions("/items[name/value='Weight, standing']"),
            vec![Condition::Name("Weight, standing".into())]
        );
        assert_eq!(
            conditions("/items[name/value='Signs and symptoms']"),
            vec![Condition::Name("Signs and symptoms".into())]
        );

        // `and` must be a whole word. A name beginning or ending with those
        // three letters is not a separator — this is `is_word_boundary`, whose
        // every operator survived mutation.
        assert_eq!(
            conditions("/items[name/value=band]"),
            vec![Condition::Name("band".into())],
            "`band` was split at its `and`"
        );
        assert_eq!(
            conditions("/items[name/value=andy]"),
            vec![Condition::Name("andy".into())],
            "`andy` was split at its `and`"
        );
        // A predicate that is nothing but a separator. `is_word_boundary`'s
        // `at + len` could become `at * len`, and the only inputs that tell
        // the two apart are the short ones: at index 1 in a four-character
        // predicate, `1 + 3 >= 4` holds and `1 * 3 >= 4` does not. So without
        // this the arithmetic there is free.
        assert!(
            "/items[ and]".parse::<Path>().is_err(),
            "a predicate of only a separator was accepted as a node id"
        );

        // `and` at the very end of the predicate is a boundary on both sides
        // — the `at + len >= text.len()` arm — so it splits, and the empty
        // term that leaves is refused rather than quietly dropped. Without
        // that arm the whole thing parses as one condition named
        // `'at0004' and`, which matches nothing and says nothing.
        let err = "/items[archetype_node_id='at0004' and]"
            .parse::<Path>()
            .expect_err("a dangling `and` leaves an empty term");
        assert!(
            matches!(err, PathError::Malformed { reason, .. } if reason.contains("empty")),
            "{err:?}"
        );
    }

    /// Quote removal only strips a *matching* pair.
    ///
    /// Both halves of `unquote`'s condition survived being widened to `||`,
    /// which would strip a lone leading quote and lose a character from a name
    /// that legitimately starts or ends with one.
    #[test]
    fn a_value_is_unquoted_only_when_both_quotes_are_there() {
        let name = |text: &str| match text.parse::<Path>().unwrap().segments()[0]
            .conditions
            .first()
            .cloned()
        {
            Some(Condition::Name(v)) => v,
            other => panic!("{text} did not yield a name: {other:?}"),
        };
        assert_eq!(name("/items[name/value='Systolic']"), "Systolic");
        assert_eq!(name(r#"/items[name/value="Systolic"]"#), "Systolic");

        // A quote at one end only. Both are written with a *balanced* pair
        // inside the value, because the predicate scanner tracks quotes and an
        // odd one makes the predicate unclosed before `unquote` ever sees it —
        // so these are the only shapes that reach the condition at all.
        //
        // Stripping one side here would silently rename the node the path
        // selects, and lose a character from the other end while doing it.
        assert_eq!(name("/items[name/value='Sys'tolic]"), "'Sys'tolic");
        assert_eq!(name("/items[name/value=Sys'tolic']"), "Sys'tolic'");
    }

    /// The empty path is the root, and `path_unique` counts.
    ///
    /// `is_root` could return `true` for every path, and `path_unique` could
    /// return `Ok(false)` always — the answer a caller uses to decide whether a
    /// path identifies one node or many.
    #[test]
    fn the_root_path_is_the_only_root_and_uniqueness_is_counted() {
        for text in ["", "/", "  "] {
            assert!(text.parse::<Path>().unwrap().is_root(), "{text:?}");
        }
        for text in ["/content", "/content/name/value"] {
            assert!(!text.parse::<Path>().unwrap().is_root(), "{text:?}");
        }

        let c = blood_pressure();
        // One node.
        assert!(c.path_unique("/content").unwrap());
        // Two: the systolic and diastolic elements under the same attribute.
        let many = "/content/data/events/data/items";
        assert_eq!(c.items_at_path(many).unwrap().len(), 2);
        assert!(!c.path_unique(many).unwrap());
        // None.
        assert!(!c.path_unique("/content/nonesuch").unwrap());
    }
}
