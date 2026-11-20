//! Production-path contracts for the public cADL definition parser.
//!
//! The parser's detailed unit tests live beside its private helpers. Coverage
//! deliberately excludes inline test modules (`W0.41`), so these tests prove
//! the same public boundary from outside the crate: supported grammar families
//! build complete trees and unsupported or malformed input stops at a named
//! boundary (`K15.6`, `K15.7`).

use openehr::am::{CObject, CPrimitive, parse_definition};

fn parses(source: &str) {
    parse_definition(source).unwrap_or_else(|error| panic!("{error}\nsource: {source}"));
}

fn refuses(source: &str, expected: &str) {
    let error = match parse_definition(source) {
        Ok(value) => panic!("unexpected parse: {value:?}\nsource: {source}"),
        Err(error) => error,
    };
    assert!(
        error.reason.contains(expected),
        "expected {expected:?}, got {error}\nsource: {source}"
    );
}

#[test]
fn public_parser_covers_supported_object_families() {
    let sources = [
        "DV_TEXT[id1]",
        r#"CLUSTER[id1] matches {
            items existence matches {0..1}
                cardinality matches {0..*; ordered; unique} matches {
                ELEMENT[id2] occurrences matches {0..1} matches {
                    value matches { DV_TEXT[id3] matches {
                        value matches {"a", "b"; "a"}
                    } }
                }
            }
        }"#,
        r#"CLUSTER[id1] matches { items matches {
            use_archetype CLUSTER[id2, openEHR-EHR-CLUSTER.device.v1]
                occurrences matches {0..1}
        } }"#,
        r#"CLUSTER[id1] matches { items matches {
            use_node ELEMENT[id2] occurrences matches {0..3} /items[id9]
        } }"#,
        r"CLUSTER[id1] matches { items matches {
            allow_archetype CLUSTER[id2] occurrences matches {0..1} matches {
                include
                    archetype_id/value matches {/openEHR-EHR-CLUSTER\.device\..*/}
                    archetype_id/value matches {/openEHR-EHR-CLUSTER\.exposure\..*/}
                exclude
                    archetype_id/value matches {/.*\.experimental\..*/}
            }
        } }",
        "CLUSTER[id1] matches { items matches { allow_archetype CLUSTER[id2] closed } }",
        "CLUSTER[id1] matches { items matches { allow_archetype ELEMENT[id2] } }",
        "OBSERVATION[id1.1] matches { /data/events cardinality matches {2..8; ordered} }",
        "CLUSTER[id1.1] matches { items[id2]/value matches { DV_QUANTITY[id0.16] } }",
        "CLUSTER[id1.1] matches { /items cardinality matches {1..*} }",
        r#"DV_QUANTITY[id1] matches {
            [units, magnitude] matches {
                [{"deg F"}, {212.0}],
                [{"deg C"}, {100.0}]
            }
        }"#,
        r"DV_QUANTITY[id1] matches {
            [units, magnitude] matches { [{/mm\[Hg\]|kPa/}, {|0..300|}] }
        }",
        "DV_ORDINAL[id1] matches { [value, symbol] matches { [{-3}, {[at49]}] } }",
        r"CLUSTER[id1] matches {
            items matches {
                ELEMENT[id2] matches { value matches { DV_TEXT[id29] } }
                ELEMENT[id4] occurrences matches {0..1} matches {
                    value matches { DV_TEXT[id30] }
                }
                allow_archetype CLUSTER[id10] matches {
                    include archetype_id/value matches {/openEHR-EHR-CLUSTER\..*\.v1/}
                }
            }
        }",
    ];
    for source in sources {
        parses(source);
    }

    let root = parse_definition(sources[4]).unwrap();
    let CObject::Slot(slot) = &root.attributes()[0].children()[0] else {
        panic!("expected an archetype slot")
    };
    assert_eq!((slot.includes().len(), slot.excludes().len()), (2, 1));
    assert!(!slot.any_allowed());
    assert_eq!(
        parse_definition(sources[10]).unwrap().attribute_tuples()[0]
            .tuples()
            .len(),
        2
    );
}

#[test]
fn public_parser_covers_primitive_families() {
    let wrapped = [
        ("Boolean", "true, false; true"),
        ("String", r#""a", "b"; "a""#),
        ("Integer", "-3, +2, 7; -3"),
        ("Real", "-3.5, +2.0; 2.0"),
        ("Date", "2024-01-01, 2024-02-02; 2024-01-01"),
        ("Time", "10:30:00, 11:45:00; 10:30:00"),
        ("Date_time", "2024-01-01T00:00:00Z; 2024-01-01T00:00:00Z"),
        ("Duration", "PT0S; PT0S"),
        ("Terminology_code", "[ac1; at2]"),
        ("Date", "yyyy-mm-??"),
        ("Time", "??:??:??; 10:30:00"),
        ("Date_time", "yyyy-??-??T??:??:??"),
        ("Duration", "PYMWDTHMS"),
        ("Integer", "|>0..<10|; 5"),
        ("Real", "|>=0.0|; 1.0"),
        (
            "Date_time",
            "|>2024-01-01T00:00:00Z..<2025-01-01T00:00:00Z|",
        ),
    ];
    for (kind, body) in wrapped {
        parses(&format!(
            "ELEMENT[id1] matches {{ value matches {{ {kind}[id2] \
             occurrences matches {{1}} matches {{{body}}} }} }}"
        ));
    }

    for body in [
        r#""mm[Hg]", "kPa"; "mm[Hg]""#,
        "true",
        "false",
        "-3",
        "-3.5",
        "|0..100|; 10",
        "|0.0..100.0|",
        "|<=10|",
        "2024-06-15",
        "10:30:00",
        "2024-06-15T10:30:00Z",
        "-PT1H",
        "yyyy-mm-??",
        "[at1]",
    ] {
        parses(&format!("WHOLE[id1] matches {{ attr matches {{{body}}} }}"));
    }

    let root = parse_definition("WHOLE[id1] matches { attr matches {-3.5} }").unwrap();
    let CObject::Primitive(value) = &root.attributes()[0].children()[0] else {
        panic!("expected a primitive")
    };
    assert!(matches!(value.constraint(), CPrimitive::Real { .. }));
}

#[test]
fn public_parser_names_malformed_and_unsupported_input() {
    let cases = [
        ("", "RM type name"),
        (":", "RM type name"),
        ("CLUSTER", "expected `[`"),
        ("CLUSTER x", "expected `[`"),
        ("LIST<DV_TEXT>[id1]", "generic RM type parameters"),
        ("CLUSTER[banana]", "not a valid id-"),
        ("CLUSTER[id1", "expected `]`"),
        ("CLUSTER[id1] trailing", "unexpected content"),
        ("CLUSTER[id1] matches {_default = 1}", "default_value"),
        ("CLUSTER[id1] matches {*}", "ADL 1.4's `matches {*}`"),
        (
            "CLUSTER[id1] matches { : matches {} }",
            "not an attribute name",
        ),
        (
            "CLUSTER[id1] matches { /data/ matches {} }",
            "not an attribute name",
        ),
        (
            "CLUSTER[id1] matches { items existence nope {1} }",
            "expected `matches`",
        ),
        (
            "CLUSTER[id1] matches { items existence matches {-1} }",
            "must not be negative",
        ),
        (
            "CLUSTER[id1] matches { items cardinality matches {3..1} }",
            "upper bound is below lower bound",
        ),
        (
            "CLUSTER[id1] matches { items cardinality matches {0..*; banana} }",
            "ordered`, `unordered`, or `unique",
        ),
        (
            "CLUSTER[id1] matches { items matches { after ELEMENT[id2] } }",
            "SIBLING_ORDER",
        ),
        (
            "CLUSTER[id1] matches { items matches { use_node ELEMENT[id2] } }",
            "target ADL_PATH",
        ),
        (
            "CLUSTER[id1] matches { items matches { use_archetype CLUSTER[id2, ] } }",
            "archetype reference",
        ),
        (
            "CLUSTER[id1] matches { items matches { use_archetype ITEM_GROUP[CIMI-CORE-x.v0] } }",
            "expected an id code",
        ),
        (
            "CLUSTER[id1] matches { items matches { use_archetype ITEM_GROUP[banana, x] } }",
            "not a valid id-",
        ),
        (
            r#"CLUSTER[id1] matches { items matches { allow_archetype CLUSTER[id2]
                matches { include archetype_id/value matches {"literal"} } } }"#,
            "CONTAINED_REGEXP",
        ),
        (
            "CLUSTER[id1] matches { items matches { ELEMENT[id2] matches {*} } }",
            "ADL 1.4's `matches {*}`",
        ),
        (
            "WHOLE[id1] matches { attr matches {|2024-01-01..2024-12-31|} }",
            "unwrapped temporal interval",
        ),
        (
            r#"WHOLE[id1] matches { attr matches {|"a".."b"|} }"#,
            "integer or real bound",
        ),
        (
            "WHOLE[id1] matches { attr matches {|>=0..10|} }",
            "relop interval",
        ),
        ("WHOLE[id1] matches { attr matches {|5 +/- 1|} }", "+/-"),
        (
            "ELEMENT[id1] matches { value matches { [at1; at2] } }",
            "may only follow an ac-code",
        ),
        (
            "ELEMENT[id1] matches { value matches { [ac1; ac2] } }",
            "not a valid at-code",
        ),
        ("DV_TEXT[id1] DV_TEXT[id2]", "unexpected content"),
    ];
    for (source, expected) in cases {
        refuses(source, expected);
    }
}
