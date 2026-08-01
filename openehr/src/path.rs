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

/// The `DV_ORDERED` attributes of a value, for the five classes that have them.
fn ordered_attrs_of(value: &DataValue) -> Option<&OrderedAttrs> {
    Some(match value {
        DataValue::Ordinal(v) => v.ordered_attrs(),
        DataValue::Scale(v) => v.ordered_attrs(),
        DataValue::Quantity(v) => v.ordered_attrs(),
        DataValue::Count(v) => v.ordered_attrs(),
        DataValue::Proportion(v) => v.ordered_attrs(),
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
}
