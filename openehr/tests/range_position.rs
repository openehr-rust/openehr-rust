//! `lib:D3.14b` / `A-86`: a qualified result against its normal range.
//!
//! The audit's table, row for row, plus the boundaries a ray can meet a range
//! at, incomparable units, no range, and the `D3.24b` flag check that uses the
//! same answer.

use openehr::base::Interval;
use openehr::rm::data_types::{
    CodePhrase, DataValue, DvCount, DvOrdered as _, DvQuantity, Indeterminate, MagnitudeStatus,
    RangePosition,
};
use openehr::validation::Validate as _;

fn mg(v: f64) -> DvQuantity {
    DvQuantity::new(v, "mg/L").expect("literal")
}

fn range(lo: f64, hi: f64) -> Interval<DataValue> {
    Interval::closed(DataValue::Quantity(mg(lo)), DataValue::Quantity(mg(hi))).expect("literal")
}

fn at(v: f64, status: Option<MagnitudeStatus>, r: Interval<DataValue>) -> Option<RangePosition> {
    let q = mg(v).with_normal_range(r);
    match status {
        Some(s) => q.with_magnitude_status(s),
        None => q,
    }
    .range_position()
}

use MagnitudeStatus::{Approximate, Equal, GreaterOrEqual, GreaterThan, LessOrEqual, LessThan};
use RangePosition::{Inside, Outside};
const UNDECIDED: RangePosition = RangePosition::Indeterminate(Indeterminate::Qualified);

#[test]
fn the_audit_table_against_three_to_five() {
    // value, qualifier, position, is_abnormal
    for (v, s, position, abnormal) in [
        (4.0, None, Inside, false),
        (4.0, Some(Equal), Inside, false),
        (4.0, Some(LessThan), UNDECIDED, true),
        (4.0, Some(GreaterThan), UNDECIDED, true),
        (2.0, Some(LessThan), Outside, true),
        (6.0, Some(LessThan), UNDECIDED, true),
        (6.0, Some(GreaterThan), Outside, true),
        (2.0, None, Outside, true),
    ] {
        let q = {
            let q = mg(v).with_normal_range(range(3.0, 5.0));
            match s {
                Some(s) => q.with_magnitude_status(s),
                None => q,
            }
        };
        assert_eq!(q.range_position(), Some(position), "{s:?} {v}");
        assert_eq!(q.is_abnormal(), Some(abnormal), "{s:?} {v}");
        // The authored qualifier is not rewritten by being assessed.
        assert_eq!(q.magnitude_status(), s);
    }
}

#[test]
fn a_ray_touching_a_bound_is_decided_by_inclusion() {
    // `< 3` against [3, 5]: every admitted value is below 3: outside.
    assert_eq!(at(3.0, Some(LessThan), range(3.0, 5.0)), Some(Outside));
    // `<= 3` admits 3 itself, which is inside: undecided.
    assert_eq!(at(3.0, Some(LessOrEqual), range(3.0, 5.0)), Some(UNDECIDED));
    // `> 5` and `>= 5` mirror them.
    assert_eq!(at(5.0, Some(GreaterThan), range(3.0, 5.0)), Some(Outside));
    assert_eq!(
        at(5.0, Some(GreaterOrEqual), range(3.0, 5.0)),
        Some(UNDECIDED)
    );
    // Against an open bound, `<= 3` admits nothing inside: outside.
    let open = Interval::new(
        Some(DataValue::Quantity(mg(3.0))),
        Some(DataValue::Quantity(mg(5.0))),
        Some(false),
        Some(false),
    )
    .expect("literal");
    assert_eq!(at(3.0, Some(LessOrEqual), open.clone()), Some(Outside));
    assert_eq!(at(5.0, Some(GreaterOrEqual), open), Some(Outside));
}

#[test]
fn a_ray_fits_only_a_range_open_on_its_side() {
    // "Below 10" with nothing below it: `< 8` is wholly inside, `< 12` is not.
    let below_ten = Interval::new(None, Some(DataValue::Quantity(mg(10.0))), None, Some(false))
        .expect("literal");
    assert_eq!(at(8.0, Some(LessThan), below_ten.clone()), Some(Inside));
    assert_eq!(at(10.0, Some(LessThan), below_ten.clone()), Some(Inside));
    assert_eq!(
        at(10.0, Some(LessOrEqual), below_ten.clone()),
        Some(UNDECIDED)
    );
    assert_eq!(at(12.0, Some(LessThan), below_ten.clone()), Some(UNDECIDED));
    assert_eq!(
        at(12.0, Some(GreaterThan), below_ten.clone()),
        Some(Outside)
    );
    let above_two =
        Interval::new(Some(DataValue::Quantity(mg(2.0))), None, Some(true), None).expect("literal");
    assert_eq!(at(3.0, Some(GreaterThan), above_two.clone()), Some(Inside));
    assert_eq!(
        at(2.0, Some(GreaterOrEqual), above_two.clone()),
        Some(Inside)
    );
    assert_eq!(
        at(1.0, Some(GreaterThan), above_two.clone()),
        Some(UNDECIDED)
    );
    assert_eq!(at(1.0, Some(LessThan), above_two), Some(Outside));
    // Against an *open* lower bound, `> 2` is still wholly inside: every value
    // it admits is above 2. `>= 2` admits 2 itself, which is not.
    let above_two_open = Interval::new(Some(DataValue::Quantity(mg(2.0))), None, Some(false), None)
        .expect("literal");
    assert_eq!(
        at(2.0, Some(GreaterThan), above_two_open.clone()),
        Some(Inside)
    );
    assert_eq!(
        at(2.0, Some(GreaterOrEqual), above_two_open),
        Some(UNDECIDED)
    );
    // A bounded range cannot hold a ray.
    assert_eq!(
        at(4.0, Some(GreaterOrEqual), range(3.0, 5.0)),
        Some(UNDECIDED)
    );
}

#[test]
fn approximate_incomparable_and_absent_are_their_own_answers() {
    assert_eq!(
        at(4.0, Some(Approximate), range(3.0, 5.0)),
        Some(RangePosition::Indeterminate(Indeterminate::Approximate))
    );
    let millilitres = DvQuantity::new(4.0, "mL")
        .expect("literal")
        .with_normal_range(range(3.0, 5.0));
    assert_eq!(
        millilitres.range_position(),
        Some(RangePosition::Indeterminate(Indeterminate::Incomparable))
    );
    // One bound is enough to make the range unable to judge: a range bounded
    // only above, in other units.
    let only_above =
        Interval::new(None, Some(DataValue::Quantity(mg(5.0))), None, Some(true)).expect("literal");
    assert_eq!(
        DvQuantity::new(4.0, "mL")
            .expect("literal")
            .with_normal_range(only_above)
            .range_position(),
        Some(RangePosition::Indeterminate(Indeterminate::Incomparable))
    );
    // Fail closed, as `D3.14a` specifies: not established inside.
    assert_eq!(millilitres.is_abnormal(), Some(true));
    assert_eq!(mg(4.0).range_position(), None);
    assert_eq!(mg(4.0).is_abnormal(), None);
}

#[test]
fn a_count_is_judged_the_same_way() {
    let r = Interval::closed(
        DataValue::Count(DvCount::new(10)),
        DataValue::Count(DvCount::new(20)),
    )
    .expect("literal");
    let fewer_than_twelve = DvCount::new(12)
        .with_magnitude_status(LessThan)
        .with_normal_range(r);
    assert_eq!(fewer_than_twelve.range_position(), Some(UNDECIDED));
}

/// `D3.24b` with the same answer: a flag is only contradicted by a decided
/// position.
#[test]
fn the_flag_check_is_contradicted_only_by_a_decided_position() {
    let flagged = |v: f64, s: Option<MagnitudeStatus>, flag: &str| {
        let q = mg(v)
            .with_normal_range(range(3.0, 5.0))
            .with_normal_status(CodePhrase::new("openehr_normal_statuses", flag).expect("literal"));
        let q = match s {
            Some(s) => q.with_magnitude_status(s),
            None => q,
        };
        DataValue::Quantity(q)
            .validate()
            .violations()
            .iter()
            .any(|v| v.invariant == "Normal_range_and_status_consistency")
    };
    // Decided: the flag must agree.
    assert!(flagged(2.0, None, "N"));
    assert!(!flagged(4.0, None, "N"));
    assert!(flagged(4.0, None, "L"));
    // Undecided: neither N nor L is contradicted. `< 6` beside `N` used to be
    // reported as a disagreement, and `< 4` beside `L` too.
    assert!(!flagged(6.0, Some(LessThan), "N"));
    assert!(!flagged(4.0, Some(LessThan), "L"));
    assert!(!flagged(4.0, Some(LessThan), "N"));
}

/// An ordered type defined outside this crate, written against the trait as
/// it was before `D3.14b`: only `is_abnormal`. The provided
/// `range_position` derives a two-state answer from it, so such an
/// implementation keeps compiling and gives a consistent answer.
struct Legacy {
    attrs: openehr::rm::data_types::OrderedAttrs,
    abnormal: Option<bool>,
}

impl openehr::rm::data_types::DvOrdered for Legacy {
    fn ordered_attrs(&self) -> &openehr::rm::data_types::OrderedAttrs {
        &self.attrs
    }
    fn semantic_cmp(&self, _: &Self) -> Option<core::cmp::Ordering> {
        None
    }
    fn is_abnormal(&self) -> Option<bool> {
        self.abnormal
    }
}

#[test]
fn an_implementation_outside_the_crate_gets_a_two_state_position() {
    let legacy = |abnormal| Legacy {
        attrs: openehr::rm::data_types::OrderedAttrs::default(),
        abnormal,
    };
    assert_eq!(legacy(Some(true)).range_position(), Some(Outside));
    assert_eq!(legacy(Some(false)).range_position(), Some(Inside));
    assert_eq!(legacy(None).range_position(), None);
}
