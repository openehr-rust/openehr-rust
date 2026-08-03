//! ISO 8601 dates, times, date-times, and durations, with openEHR's partial
//! precision.
//!
//! # Why this exists instead of a date-time crate
//!
//! `DV_DATE.value` is typed `String` in the reference model, and openEHR means
//! it: `2024`, `2024-05`, and `2024-05-17` are all valid dates, and they are
//! **not the same date**. `2024-05` is a date known to month precision — a
//! birth date on a refugee's record, a diagnosis date recalled as "sometime in
//! May". A general date-time type either refuses these or silently completes
//! them to `2024-05-01`, and a silently completed date is a fabricated clinical
//! fact that no downstream reader can distinguish from a real one.
//!
//! So this module parses into a structure that **keeps the precision** and
//! prints back the exact input.
//!
//! # Comparison is partial, on purpose
//!
//! ```
//! use openehr::base::iso8601::Date;
//!
//! let may: Date = "2024-05".parse().unwrap();
//! let june_first: Date = "2024-06-01".parse().unwrap();
//! let may_17: Date = "2024-05-17".parse().unwrap();
//!
//! assert!(may < june_first);                     // decidable: different months
//! assert_eq!(may.partial_cmp(&may_17), None);    // undecidable: May which day?
//! ```
//!
//! Returning `Less` for the last comparison — the choice a "just complete it to
//! the first of the month" implementation makes — would order a month-precision
//! date before every day in that month, so a query for "events before
//! 2024-05-17" would include an event that may have happened after it.
//!
//! # Scope
//!
//! Extended format only (`2024-05-17`, not `20240517`). The basic format is
//! valid ISO 8601 and openEHR does not forbid it; it does not appear in openEHR
//! canonical JSON, and accepting it would make `2024` ambiguous between a
//! year and a four-digit basic-format nothing. Recorded as a limitation in
//! `spec/audit.md` rather than left to be discovered.

use crate::error::ParseError;
use core::cmp::Ordering;
use core::fmt;
use core::str::FromStr;

/// How much of a date is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DatePrecision {
    /// `2024`
    Year,
    /// `2024-05`
    Month,
    /// `2024-05-17`
    Day,
}

/// How much of a time is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TimePrecision {
    /// `10`
    Hour,
    /// `10:30`
    Minute,
    /// `10:30:15`
    Second,
    /// `10:30:15.250`
    Fraction,
}

/// A UTC offset, or `Z`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Offset {
    /// Signed offset from UTC in minutes.
    minutes: i32,
    /// Whether the source wrote `Z` rather than `+00:00`. Both mean UTC; both
    /// must print back as they came, because the string is the stored value.
    zulu: bool,
}

impl Offset {
    /// UTC written as `Z`.
    pub const UTC: Self = Self {
        minutes: 0,
        zulu: true,
    };

    /// The offset from UTC in minutes, east positive.
    #[must_use]
    pub fn minutes(&self) -> i32 {
        self.minutes
    }
}

impl fmt::Display for Offset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.zulu {
            return f.write_str("Z");
        }
        let sign = if self.minutes < 0 { '-' } else { '+' };
        let abs = self.minutes.abs();
        write!(f, "{sign}{:02}:{:02}", abs / 60, abs % 60)
    }
}

/// An ISO 8601 date, to year, month, or day precision.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Date {
    year: i32,
    month: Option<u8>,
    day: Option<u8>,
    text: String,
}

impl Date {
    /// The year.
    #[must_use]
    pub fn year(&self) -> i32 {
        self.year
    }

    /// The month, if known.
    #[must_use]
    pub fn month(&self) -> Option<u8> {
        self.month
    }

    /// The day, if known.
    #[must_use]
    pub fn day(&self) -> Option<u8> {
        self.day
    }

    /// How much of the date is known.
    #[must_use]
    pub fn precision(&self) -> DatePrecision {
        match (self.month, self.day) {
            (None, _) => DatePrecision::Year,
            (Some(_), None) => DatePrecision::Month,
            (Some(_), Some(_)) => DatePrecision::Day,
        }
    }

    /// The exact text this date was parsed from.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    fn key(&self) -> (i32, u8, u8) {
        (self.year, self.month.unwrap_or(0), self.day.unwrap_or(0))
    }
}

impl PartialOrd for Date {
    /// Compares on the components both dates know.
    ///
    /// Returns `None` when the known components are equal but one date knows
    /// more than the other — see the module header for why that is not `Less`.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let common = self.precision().min(other.precision());
        let (a, b) = (self.key(), other.key());
        let ord = match common {
            DatePrecision::Year => a.0.cmp(&b.0),
            DatePrecision::Month => (a.0, a.1).cmp(&(b.0, b.1)),
            DatePrecision::Day => a.cmp(&b),
        };
        match ord {
            Ordering::Equal if self.precision() != other.precision() => None,
            other_ord => Some(other_ord),
        }
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl FromStr for Date {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the text is not `YYYY`, `YYYY-MM`, or
    /// `YYYY-MM-DD` with in-range components. Day ranges are checked against
    /// the month, including February in leap years.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();
        let bad = |reason| ParseError::new("ISO8601 date", reason, s);

        let year_text = parts.first().copied().unwrap_or_default();
        if year_text.len() != 4 || !year_text.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad("year is not four digits"));
        }
        let year: i32 = year_text.parse().map_err(|_| bad("year is not a number"))?;

        let two = |text: &str, what| -> Result<u8, ParseError> {
            if text.len() != 2 || !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad(what));
            }
            text.parse().map_err(|_| bad(what))
        };

        let (month, day) = match parts.as_slice() {
            [_] => (None, None),
            [_, m] => (Some(two(m, "month is not two digits")?), None),
            [_, m, d] => (
                Some(two(m, "month is not two digits")?),
                Some(two(d, "day is not two digits")?),
            ),
            _ => return Err(bad("too many `-`-separated components")),
        };

        if month.is_some_and(|m| !(1..=12).contains(&m)) {
            return Err(bad("month is out of range"));
        }
        if let (Some(m), Some(d)) = (month, day)
            && (d < 1 || d > days_in_month(year, m))
        {
            return Err(bad("day is out of range for the month"));
        }

        Ok(Self {
            year,
            month,
            day,
            text: s.to_owned(),
        })
    }
}

/// Days from the civil epoch (1970-01-01) for a proleptic Gregorian date.
///
/// Howard Hinnant's `days_from_civil`, which is exact for the whole range of
/// `i32` years and needs no lookup table. Used only for differencing two
/// instants — this crate does not otherwise convert dates to numbers, because
/// doing so is what loses partial precision (`D3.9`).
fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let y = i64::from(year) - i64::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(month);
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Days in a month, Gregorian, with the full leap rule.
///
/// The century exceptions matter here: 1900 was not a leap year, and dates of
/// birth in 1900 are still in live records.
///
/// `pub(crate)` because `rm::data_structures` needs it for interval-event
/// arithmetic and used to carry **its own copy** — identical but for the
/// fallback arm, and entirely untested (`lib:A-33`). One calendar, one place:
/// a leap rule fixed in one of two copies is a rule that disagrees with itself.
///
/// Returns `0` for a month outside 1–12, which is not a length but a refusal;
/// every caller must already have validated the month.
pub(crate) fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

/// An ISO 8601 time of day, to hour, minute, second, or fractional precision,
/// with an optional UTC offset.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Time {
    hour: u8,
    minute: Option<u8>,
    second: Option<u8>,
    /// Kept as text: `.5` and `.50` are the same quantity and different
    /// strings, and the string is what the record stores.
    fraction: Option<String>,
    offset: Option<Offset>,
    text: String,
}

impl Time {
    /// The hour, 0–23.
    #[must_use]
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// The minute, if known.
    #[must_use]
    pub fn minute(&self) -> Option<u8> {
        self.minute
    }

    /// The second, if known. `60` is accepted, for leap seconds.
    #[must_use]
    pub fn second(&self) -> Option<u8> {
        self.second
    }

    /// The fractional second as written, without its separator.
    #[must_use]
    pub fn fraction(&self) -> Option<&str> {
        self.fraction.as_deref()
    }

    /// The UTC offset, if the time carries one.
    #[must_use]
    pub fn offset(&self) -> Option<Offset> {
        self.offset
    }

    /// How much of the time is known.
    #[must_use]
    pub fn precision(&self) -> TimePrecision {
        match (self.minute, self.second, &self.fraction) {
            (None, _, _) => TimePrecision::Hour,
            (Some(_), None, _) => TimePrecision::Minute,
            (Some(_), Some(_), None) => TimePrecision::Second,
            (Some(_), Some(_), Some(_)) => TimePrecision::Fraction,
        }
    }

    /// The exact text this time was parsed from.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Milliseconds since midnight, in the time's own zone.
    ///
    /// # Panics
    ///
    /// Never: the parser bounds every component.
    #[must_use]
    fn millis_local(&self) -> i64 {
        let frac_ms = self.fraction.as_deref().map_or(0, |f| {
            // Pad or truncate to exactly three digits; anything finer than a
            // millisecond does not affect an ordering at millisecond
            // resolution, and anything coarser must not be read as smaller.
            let mut digits: String = f.chars().take(3).collect();
            while digits.len() < 3 {
                digits.push('0');
            }
            digits.parse::<i64>().unwrap_or(0)
        });
        i64::from(self.hour) * 3_600_000
            + i64::from(self.minute.unwrap_or(0)) * 60_000
            + i64::from(self.second.unwrap_or(0)) * 1_000
            + frac_ms
    }
}

impl PartialOrd for Time {
    /// Compares on the components both times know, after normalising to UTC.
    ///
    /// Returns `None` if exactly one side carries a UTC offset: a local time
    /// with no offset is unanchored, and could be up to 26 hours either side of
    /// the offset one. Also returns `None` when the known components agree but
    /// the precisions differ, for the reason given in the module header.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.offset, other.offset) {
            (Some(_), None) | (None, Some(_)) => return None,
            _ => {}
        }
        let a = self.millis_local() - i64::from(self.offset.map_or(0, |o| o.minutes)) * 60_000;
        let b = other.millis_local() - i64::from(other.offset.map_or(0, |o| o.minutes)) * 60_000;
        match a.cmp(&b) {
            Ordering::Equal if self.precision() != other.precision() => None,
            ord => Some(ord),
        }
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl FromStr for Time {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the text is not `hh[:mm[:ss[.sss]]]` with an
    /// optional `Z` or `±hh:mm` suffix, or if a component is out of range.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = |reason| ParseError::new("ISO8601 time", reason, s);
        let (body, offset) = split_offset(s).ok_or_else(|| bad("malformed UTC offset"))?;

        let (body, fraction) = match body.split_once('.') {
            None => (body, None),
            Some((head, frac)) => {
                if frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(bad("fractional second is not a number"));
                }
                (head, Some(frac.to_owned()))
            }
        };

        let parts: Vec<&str> = body.split(':').collect();
        let two = |text: &str, what| -> Result<u8, ParseError> {
            if text.len() != 2 || !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad(what));
            }
            text.parse().map_err(|_| bad(what))
        };

        let (hour, minute, second) = match parts.as_slice() {
            [h] => (two(h, "hour is not two digits")?, None, None),
            [h, m] => (
                two(h, "hour is not two digits")?,
                Some(two(m, "minute is not two digits")?),
                None,
            ),
            [h, m, sec] => (
                two(h, "hour is not two digits")?,
                Some(two(m, "minute is not two digits")?),
                Some(two(sec, "second is not two digits")?),
            ),
            _ => return Err(bad("too many `:`-separated components")),
        };

        if fraction.is_some() && second.is_none() {
            return Err(bad("fractional part without a second"));
        }
        if hour > 23 {
            return Err(bad("hour is out of range"));
        }
        if minute.is_some_and(|m| m > 59) {
            return Err(bad("minute is out of range"));
        }
        // 60 is permitted: a leap second is a real instant, and a record that
        // captured one must not be rejected on read-back.
        if second.is_some_and(|sec| sec > 60) {
            return Err(bad("second is out of range"));
        }

        Ok(Self {
            hour,
            minute,
            second,
            fraction,
            offset,
            text: s.to_owned(),
        })
    }
}

/// Splits a trailing UTC offset from a time. Returns `None` if a suffix looks
/// like an offset but is malformed.
fn split_offset(s: &str) -> Option<(&str, Option<Offset>)> {
    if let Some(head) = s.strip_suffix('Z') {
        return Some((head, Some(Offset::UTC)));
    }
    // Search from the right, and only in the last 6 characters, so the `-` in a
    // date does not get mistaken for a sign when this is called on a date-time.
    // No sign at all is the common case — a local time — not a failure.
    let Some((idx, _)) = s
        .char_indices()
        .rev()
        .take(6)
        .find(|(_, c)| *c == '+' || *c == '-')
    else {
        return Some((s, None));
    };
    if idx == 0 {
        // The whole string is a sign and digits, which is an offset with no
        // time in front of it.
        return None;
    }
    let (head, tail) = s.split_at(idx);
    let sign = if tail.starts_with('-') { -1 } else { 1 };
    let digits = &tail[1..];
    // Checked before any split: `len()` counts *bytes*, so a single four-byte
    // character satisfies `digits.len() == 4` below and `split_at(2)` then
    // lands inside it and panics rather than returning `None` (**A-16**). An
    // offset is ASCII digits and an optional colon by definition, so rejecting
    // everything else here removes the whole class instead of that one index.
    if !digits.bytes().all(|b| b.is_ascii_digit() || b == b':') {
        return None;
    }
    let (hh, mm) = match digits.split_once(':') {
        Some((hh, mm)) => (hh, mm),
        None if digits.len() == 4 => digits.split_at(2),
        None if digits.len() == 2 => (digits, "00"),
        None => return None,
    };
    if hh.len() != 2 || mm.len() != 2 {
        return None;
    }
    let hh: i32 = hh.parse().ok()?;
    let mm: i32 = mm.parse().ok()?;
    if hh > 14 || mm > 59 {
        return None;
    }
    Some((
        head,
        Some(Offset {
            minutes: sign * (hh * 60 + mm),
            zulu: false,
        }),
    ))
}

/// An ISO 8601 date-time: a [`Date`], optionally with a `T` and a [`Time`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DateTime {
    date: Date,
    time: Option<Time>,
    text: String,
}

impl DateTime {
    /// The date part.
    #[must_use]
    pub fn date(&self) -> &Date {
        &self.date
    }

    /// The time part, if the value carries one.
    #[must_use]
    pub fn time(&self) -> Option<&Time> {
        self.time.as_ref()
    }

    /// The UTC offset, if the value carries one.
    #[must_use]
    pub fn offset(&self) -> Option<Offset> {
        self.time.as_ref().and_then(Time::offset)
    }

    /// The exact text this date-time was parsed from.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl DateTime {
    /// The difference from `other` in whole seconds, positive when `self` is
    /// later.
    ///
    /// Returns `None` where the difference is not established, for the same
    /// reasons [`DateTime`]'s ordering is partial: either value lacking a time
    /// of day or a full date, or exactly one of them carrying a UTC offset. A
    /// difference computed across an unanchored local time would be wrong by up
    /// to 26 hours.
    ///
    /// ```
    /// use openehr::base::iso8601::DateTime;
    ///
    /// let a: DateTime = "2026-07-31T09:00:00Z".parse().unwrap();
    /// let b: DateTime = "2026-07-31T17:30:00Z".parse().unwrap();
    /// assert_eq!(b.diff_seconds(&a), Some(30_600));
    /// assert_eq!(a.diff_seconds(&b), Some(-30_600));
    ///
    /// // Across a month boundary, and across the offset.
    /// let c: DateTime = "2026-08-01T00:00:00+01:00".parse().unwrap();
    /// assert_eq!(c.diff_seconds(&b), Some(19_800));
    ///
    /// // Not established: one local, one anchored.
    /// let local: DateTime = "2026-07-31T09:00:00".parse().unwrap();
    /// assert_eq!(local.diff_seconds(&a), None);
    /// ```
    #[must_use]
    pub fn diff_seconds(&self, other: &Self) -> Option<i64> {
        match (self.offset(), other.offset()) {
            (Some(_), None) | (None, Some(_)) => return None,
            _ => {}
        }
        Some(self.epoch_seconds()? - other.epoch_seconds()?)
    }

    /// Seconds from the civil epoch, normalised to UTC where an offset is
    /// present. `None` unless the value has a full date and a time of day.
    fn epoch_seconds(&self) -> Option<i64> {
        let (month, day) = (self.date.month()?, self.date.day()?);
        let time = self.time.as_ref()?;
        let days = days_from_civil(self.date.year(), month, day);
        let seconds = i64::from(time.hour()) * 3600
            + i64::from(time.minute().unwrap_or(0)) * 60
            + i64::from(time.second().unwrap_or(0));
        let offset = i64::from(time.offset().map_or(0, |o| o.minutes())) * 60;
        Some(days * 86_400 + seconds - offset)
    }
}

impl PartialOrd for DateTime {
    /// Compares the dates first; compares times only when the dates are equal
    /// to day precision. `None` propagates from either part.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.date.partial_cmp(&other.date)? {
            Ordering::Equal => {}
            ord => return Some(ord),
        }
        match (&self.time, &other.time) {
            (None, None) => Some(Ordering::Equal),
            // Same day, one with a time and one without: the timeless one could
            // be any instant that day.
            (None, Some(_)) | (Some(_), None) => None,
            (Some(a), Some(b)) => a.partial_cmp(b),
        }
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl FromStr for DateTime {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if either half is malformed, or if a `T`
    /// separator is present with nothing after it.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (date_text, time_text) = match s.split_once('T') {
            None => (s, None),
            Some((d, t)) => {
                if t.is_empty() {
                    return Err(ParseError::new(
                        "ISO8601 date-time",
                        "`T` with no time after it",
                        s,
                    ));
                }
                (d, Some(t))
            }
        };
        let date: Date = date_text
            .parse()
            .map_err(|e: ParseError| ParseError::new("ISO8601 date-time", e.reason, s))?;
        let time = time_text
            .map(str::parse::<Time>)
            .transpose()
            .map_err(|e: ParseError| ParseError::new("ISO8601 date-time", e.reason, s))?;
        // A time on a date that does not name a day places an instant inside a
        // month. Nothing can resolve that later, and it is far more often a
        // string-concatenation bug than a real partial value.
        if time.is_some() && date.precision() != DatePrecision::Day {
            return Err(ParseError::new(
                "ISO8601 date-time",
                "a time requires a full date",
                s,
            ));
        }
        Ok(Self {
            date,
            time,
            text: s.to_owned(),
        })
    }
}

/// An ISO 8601 duration: `[-]PnYnMnWnDTnHnMnS`.
///
/// openEHR permits the leading minus, which plain ISO 8601 does not, because
/// `HISTORY` offsets and `INSTRUCTION` timings need to express "two hours
/// before".
///
/// ```
/// use openehr::base::iso8601::Duration;
///
/// let d: Duration = "P1Y2M10DT2H30M".parse().unwrap();
/// assert_eq!(d.years(), 1);
/// assert_eq!(d.minutes(), 30);
/// assert!(!d.is_negative());
///
/// let back: Duration = "-PT2H".parse().unwrap();
/// assert!(back.is_negative());
/// assert_eq!(back.to_string(), "-PT2H");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Duration {
    negative: bool,
    years: u32,
    months: u32,
    weeks: u32,
    days: u32,
    hours: u32,
    minutes: u32,
    /// Seconds including any fractional part, kept as text for the same reason
    /// as [`Time::fraction`].
    seconds: Option<String>,
    text: String,
}

impl Duration {
    /// Whether the duration runs backwards.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// Years component.
    #[must_use]
    pub fn years(&self) -> u32 {
        self.years
    }

    /// Months component.
    #[must_use]
    pub fn months(&self) -> u32 {
        self.months
    }

    /// Weeks component.
    #[must_use]
    pub fn weeks(&self) -> u32 {
        self.weeks
    }

    /// Days component.
    #[must_use]
    pub fn days(&self) -> u32 {
        self.days
    }

    /// Hours component.
    #[must_use]
    pub fn hours(&self) -> u32 {
        self.hours
    }

    /// Minutes component.
    #[must_use]
    pub fn minutes(&self) -> u32 {
        self.minutes
    }

    /// Seconds component, as written.
    #[must_use]
    pub fn seconds(&self) -> Option<&str> {
        self.seconds.as_deref()
    }

    /// Whether every component is zero.
    #[must_use]
    pub fn is_zero(&self) -> bool {
        self.years == 0
            && self.months == 0
            && self.weeks == 0
            && self.days == 0
            && self.hours == 0
            && self.minutes == 0
            && self
                .seconds
                .as_deref()
                .is_none_or(|s| s.parse::<f64>().is_ok_and(|v| v == 0.0))
    }

    /// The exact text this duration was parsed from.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// An **approximate** length in seconds, using 365.2425-day years and
    /// 30.436875-day months.
    ///
    /// Deliberately named `approx_`: a duration containing years or months has
    /// no exact length without an anchor date, because February is 28 or 29
    /// days and a year is 365 or 366. Use this for ordering and rough bucketing,
    /// never for computing a due date — `P1M` after 31 January is 28 February,
    /// which no fixed number of seconds produces.
    #[must_use]
    pub fn approx_seconds(&self) -> f64 {
        let secs = f64::from(self.years) * 365.2425 * 86_400.0
            + f64::from(self.months) * 30.436_875 * 86_400.0
            + f64::from(self.weeks) * 7.0 * 86_400.0
            + f64::from(self.days) * 86_400.0
            + f64::from(self.hours) * 3_600.0
            + f64::from(self.minutes) * 60.0
            + self
                .seconds
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
        if self.negative { -secs } else { secs }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl PartialOrd for Duration {
    /// Orders by [`Duration::approx_seconds`], and refuses to order two
    /// durations whose approximations agree but whose component shapes differ —
    /// `P1M` and `P30D` are not comparable without a calendar anchor.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let (a, b) = (self.approx_seconds(), other.approx_seconds());
        let calendarish = |d: &Self| d.years > 0 || d.months > 0;
        match a.partial_cmp(&b)? {
            Ordering::Equal
                if (calendarish(self) || calendarish(other)) && self.text != other.text =>
            {
                None
            }
            ord => Some(ord),
        }
    }
}

impl FromStr for Duration {
    type Err = ParseError;

    /// # Errors
    ///
    /// Returns [`ParseError`] if the text does not start with `P` (after an
    /// optional `-`), if a designator appears out of order or twice, if a
    /// designator has no number, or if the duration has no components at all.
    #[allow(clippy::too_many_lines)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bad = |reason| ParseError::new("ISO8601 duration", reason, s);
        let (negative, rest) = match s.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, s),
        };
        let Some(rest) = rest.strip_prefix('P') else {
            return Err(bad("does not start with `P`"));
        };

        let (date_part, time_part) = match rest.split_once('T') {
            None => (rest, ""),
            Some((d, t)) => {
                if t.is_empty() {
                    return Err(bad("`T` with no time components after it"));
                }
                (d, t)
            }
        };

        let mut out = Self {
            negative,
            years: 0,
            months: 0,
            weeks: 0,
            days: 0,
            hours: 0,
            minutes: 0,
            seconds: None,
            text: s.to_owned(),
        };
        let mut any = false;

        // Designators must appear in order and at most once. `P1D1D` and `P1M1Y`
        // are both rejected: the first double-counts, the second means a reader
        // that scans left to right and one that sums by designator disagree.
        let mut scan =
            |part: &str, designators: &[char], is_time: bool| -> Result<(), ParseError> {
                let mut next_allowed = 0usize;
                let mut number = String::new();
                for ch in part.chars() {
                    if ch.is_ascii_digit() || (ch == '.' && is_time) {
                        number.push(ch);
                        continue;
                    }
                    let Some(pos) = designators.iter().position(|d| *d == ch) else {
                        return Err(bad("unknown duration designator"));
                    };
                    if pos < next_allowed {
                        return Err(bad("duration designators are out of order or repeated"));
                    }
                    if number.is_empty() {
                        return Err(bad("designator with no number"));
                    }
                    next_allowed = pos + 1;
                    any = true;
                    if is_time && ch == 'S' {
                        if number.parse::<f64>().is_err() {
                            return Err(bad("seconds is not a number"));
                        }
                        out.seconds = Some(core::mem::take(&mut number));
                    } else {
                        if number.contains('.') {
                            return Err(bad("only seconds may be fractional"));
                        }
                        let value: u32 = number
                            .parse()
                            .map_err(|_| bad("component does not fit in u32"))?;
                        number.clear();
                        match ch {
                            'Y' => out.years = value,
                            'M' if is_time => out.minutes = value,
                            'M' => out.months = value,
                            'W' => out.weeks = value,
                            'D' => out.days = value,
                            'H' => out.hours = value,
                            _ => return Err(bad("unknown duration designator")),
                        }
                    }
                }
                if number.is_empty() {
                    Ok(())
                } else {
                    Err(bad("number with no designator"))
                }
            };

        scan(date_part, &['Y', 'M', 'W', 'D'], false)?;
        scan(time_part, &['H', 'M', 'S'], true)?;

        if !any {
            return Err(bad("no components"));
        }
        Ok(out)
    }
}

crate::impl_string_serde!(Date, "ISO8601 date");
crate::impl_string_serde!(Time, "ISO8601 time");
crate::impl_string_serde!(DateTime, "ISO8601 date-time");
crate::impl_string_serde!(Duration, "ISO8601 duration");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_dates_keep_their_precision_and_text() {
        for text in ["2024", "2024-05", "2024-05-17"] {
            let d: Date = text.parse().unwrap();
            assert_eq!(d.to_string(), text);
        }
        assert_eq!(
            "2024".parse::<Date>().unwrap().precision(),
            DatePrecision::Year
        );
        assert_eq!(
            "2024-05".parse::<Date>().unwrap().precision(),
            DatePrecision::Month
        );
    }

    #[test]
    fn a_coarser_date_is_not_ordered_inside_its_own_range() {
        let may: Date = "2024-05".parse().unwrap();
        let may_17: Date = "2024-05-17".parse().unwrap();
        // Mutation check for that behaviour: an implementation that completed
        // the missing day to 0 or 1 would answer Some(Less) here.
        assert_eq!(may.partial_cmp(&may_17), None);
        // And the coarser value is still ordered where the answer is decidable.
        let april: Date = "2024-04".parse().unwrap();
        assert_eq!(april.partial_cmp(&may_17), Some(Ordering::Less));
    }

    #[test]
    fn leap_day_validity_follows_the_gregorian_rule() {
        assert!("2024-02-29".parse::<Date>().is_ok());
        assert!("2023-02-29".parse::<Date>().is_err());
        assert!("2000-02-29".parse::<Date>().is_ok()); // divisible by 400
        assert!("1900-02-29".parse::<Date>().is_err()); // divisible by 100
    }

    #[test]
    fn offsets_normalise_before_comparison() {
        let london: Time = "12:00:00+01:00".parse().unwrap();
        let utc: Time = "11:00:00Z".parse().unwrap();
        assert_eq!(london.partial_cmp(&utc), Some(Ordering::Equal));
    }

    #[test]
    fn a_local_time_is_not_comparable_with_an_anchored_one() {
        let local: Time = "11:00:00".parse().unwrap();
        let utc: Time = "11:00:00Z".parse().unwrap();
        assert_eq!(local.partial_cmp(&utc), None);
    }

    #[test]
    fn date_time_rejects_a_time_on_a_partial_date() {
        assert!("2024-05T10:00:00".parse::<DateTime>().is_err());
        assert!("2024-05-17T10:00:00Z".parse::<DateTime>().is_ok());
    }

    #[test]
    fn differencing_two_instants_crosses_months_and_offsets() {
        let dt = |t: &str| t.parse::<DateTime>().unwrap();
        assert_eq!(
            dt("2026-07-31T17:30:00Z").diff_seconds(&dt("2026-07-31T09:00:00Z")),
            Some(30_600)
        );
        // Across a month boundary and a leap day.
        assert_eq!(
            dt("2024-03-01T00:00:00Z").diff_seconds(&dt("2024-02-28T00:00:00Z")),
            Some(172_800)
        );
        assert_eq!(
            dt("2023-03-01T00:00:00Z").diff_seconds(&dt("2023-02-28T00:00:00Z")),
            Some(86_400)
        );
        // Across a year and a century-rule non-leap year.
        assert_eq!(
            dt("1900-03-01T00:00:00Z").diff_seconds(&dt("1900-02-28T00:00:00Z")),
            Some(86_400)
        );
        // Offsets normalise before differencing.
        assert_eq!(
            dt("2026-07-31T10:00:00+01:00").diff_seconds(&dt("2026-07-31T09:00:00Z")),
            Some(0)
        );
        // Not established.
        assert_eq!(
            dt("2026-07-31T09:00:00").diff_seconds(&dt("2026-07-31T09:00:00Z")),
            None
        );
        assert_eq!(dt("2026-07-31").diff_seconds(&dt("2026-07-30")), None);
    }

    #[test]
    fn durations_round_trip_and_reject_disorder() {
        for text in ["P1Y", "P1Y2M3W4DT5H6M7.5S", "-PT30M", "PT0S"] {
            assert_eq!(text.parse::<Duration>().unwrap().to_string(), text);
        }
        for text in ["P1M1Y", "P1D1D", "1Y", "P", "PT", "P1.5D", "PY"] {
            assert!(text.parse::<Duration>().is_err(), "accepted {text}");
        }
    }

    #[test]
    fn duration_minute_and_month_are_disambiguated_by_the_t() {
        let month: Duration = "P1M".parse().unwrap();
        assert_eq!((month.months(), month.minutes()), (1, 0));
        let minute: Duration = "PT1M".parse().unwrap();
        assert_eq!((minute.months(), minute.minutes()), (0, 1));
    }

    #[test]
    fn calendar_durations_are_not_ordered_against_equivalent_day_counts() {
        // P12M and P1Y have identical approximations by construction, so a
        // naive comparison reports Equal — and they are not equal, because
        // "twelve months from 31 January" and "one year from 31 January" land
        // on the same day only by the accident of that anchor.
        let twelve_months: Duration = "P12M".parse().unwrap();
        let one_year: Duration = "P1Y".parse().unwrap();
        assert_eq!(twelve_months.partial_cmp(&one_year), None);

        // Where a calendar component is present but the approximations differ,
        // the answer is still decidable.
        let one_month: Duration = "P1M".parse().unwrap();
        let thirty_days: Duration = "P30D".parse().unwrap();
        assert_eq!(one_month.partial_cmp(&thirty_days), Some(Ordering::Greater));
    }

    /// A UTC offset renders with the sign it was parsed with.
    ///
    /// The comparison that picks `-` or `+` could be inverted, made `==`, or
    /// widened to `<=`, and no test noticed (`lib:A-09`). A flipped sign on
    /// `-05:00` is a ten-hour error in a clinical timestamp, and this crate
    /// stores the lexical form as authoritative precisely so the offset a
    /// caller wrote survives (`db:M3.28`).
    ///
    /// `Z` and `+00:00` are also distinguished here: both are UTC, and both
    /// must print back as they came, because the text *is* the stored value.
    #[test]
    fn an_offset_prints_back_with_the_sign_it_arrived_with() {
        let offset_of = |text: &str| {
            text.parse::<Time>()
                .unwrap_or_else(|e| panic!("{text}: {e}"))
                .offset()
                .unwrap_or_else(|| panic!("{text} carries no offset"))
        };

        for (text, minutes, rendered) in [
            ("12:00:00-05:00", -300, "-05:00"),
            ("12:00:00+05:30", 330, "+05:30"),
            ("12:00:00-00:30", -30, "-00:30"),
            // Zero, both ways round. `+00:00` is not `-00:00`, and neither is
            // `Z` — the `<` could become `<=` and turn `+00:00` into
            // `-00:00` with nothing failing.
            ("12:00:00+00:00", 0, "+00:00"),
            ("12:00:00Z", 0, "Z"),
        ] {
            let offset = offset_of(text);
            assert_eq!(offset.minutes(), minutes, "{text}");
            assert_eq!(offset.to_string(), rendered, "{text}");
            // The whole value round-trips too: the lexical form is what is
            // stored, so a re-rendering that differs is a rewritten record.
            assert_eq!(text.parse::<Time>().unwrap().as_str(), text);
        }
    }

    /// The civil-date-to-epoch conversion, against values computed elsewhere.
    ///
    /// Every arithmetic operation in `days_from_civil` survived mutation
    /// (`lib:A-09`): the era division, the `y - 399` correction for negative
    /// years, the day-of-year term, and the two constants at the end.
    ///
    /// **Called directly, not through `diff_seconds`.** That is the whole
    /// point. Its only caller differences two of these, and a difference
    /// cancels every constant — `+ day - 1` could become `+ day + 1` and
    /// `- 719_468` could become `+ 719_468`, shifting every date by the same
    /// amount, with no comparison able to tell. Testing a function only
    /// through the caller that cancels half of it is how five of these
    /// survived a table of nine dates.
    ///
    /// This is the conversion behind the derived UTC column (`db:M3.28`), and
    /// time-ranged queries must use that column rather than the lexical one
    /// (`db:P6.14`). A wrong day here is a query that returns the wrong
    /// encounters and says nothing.
    ///
    /// **The expected values are from Python's `datetime`, not from this
    /// code.** A table generated by running the function under test would
    /// confirm only that it still does what it did. Year 0 is not
    /// representable in `datetime`; those rows come from their year-400
    /// counterparts less one 400-year cycle of 146,097 days, which is exact by
    /// construction and was checked against a representable pair.
    #[test]
    fn days_from_the_civil_epoch_match_an_independent_calendar() {
        for (year, month, day, days) in [
            (1970, 1, 1, 0),
            (1969, 12, 31, -1),
            // A leap day in a century that is a leap year, and the day after
            // one that is not: 2000 was, 1900 was not. Dates of birth in 1900
            // are still in live records.
            (2000, 2, 29, 11_016),
            (1900, 3, 1, -25_508),
            (2026, 8, 3, 20_668),
            // Before the epoch by centuries, where the era division stops
            // cancelling.
            (1582, 10, 15, -141_427),
            (1600, 2, 29, -135_081),
            (2400, 2, 29, 157_113),
            (1, 1, 1, -719_162),
            (1, 3, 1, -719_103),
            // Year 0 and earlier is where `y` goes negative and the `y - 399`
            // correction is the only thing keeping the era division truncating
            // the right way. Nothing reached it before: `0001-01-01` gives
            // `y = 0`, which is still the non-negative branch.
            (0, 1, 1, -719_528),
            (0, 2, 29, -719_469),
            (0, 3, 1, -719_468),
            (0, 12, 31, -719_163),
        ] {
            assert_eq!(
                days_from_civil(year, month, day),
                days,
                "{year:04}-{month:02}-{day:02}"
            );
        }

        // And the caller still agrees, which is what the rest of the crate
        // uses.
        let a: DateTime = "2026-08-03T00:00:00Z".parse().unwrap();
        let epoch: DateTime = "1970-01-01T00:00:00Z".parse().unwrap();
        assert_eq!(a.diff_seconds(&epoch), Some(20_668 * 86_400));
    }

    /// A malformed date component is refused, each on its own account.
    ///
    /// Both `||`s in the parser could become `&&`, which accepts a year or a
    /// month of the right *length* made of anything at all — `20x6-01-01` and
    /// `2026-ab-01`. A date is the field openEHR stores as authoritative text
    /// (`db:M3.28`), so what is accepted here is what is stored forever.
    #[test]
    fn a_date_component_must_be_the_right_length_and_all_digits() {
        for text in [
            "20x6-01-01", // four characters, not four digits
            "202-01-01",  // three digits
            "20266-01-01",
            "2026-ab-01", // two characters, not two digits
            "2026-1-01",  // one digit
            "2026-011-01",
            "2026-01-ab",
            "2026-01-1",
        ] {
            assert!(
                text.parse::<Date>().is_err(),
                "{text} was accepted as a date"
            );
        }
        // And the well-formed partial precisions still parse.
        for text in ["2026", "2026-01", "2026-01-31"] {
            assert_eq!(text.parse::<Date>().unwrap().as_str(), text);
        }
    }

    /// A UTC offset in every form the parser accepts, and every form it must
    /// refuse.
    ///
    /// `split_offset` is where an offset becomes a number, and nineteen of its
    /// mutants survived (`lib:A-09`): the sign, the two length guards that tell
    /// `+0530` from `+05`, and both halves of the range check. It is also where
    /// `lib:A-16` lived — a multi-byte character satisfying `len() == 4` and
    /// `split_at(2)` landing inside it — so what this function refuses is a
    /// safety property, not only a correctness one.
    #[test]
    fn every_offset_form_is_read_and_every_malformed_one_refused() {
        let minutes = |text: &str| {
            text.parse::<Time>()
                .unwrap_or_else(|e| panic!("{text}: {e}"))
                .offset()
                .map(|o| o.minutes())
        };

        // Extended, basic, and hour-only — the three shapes, each with both
        // signs. The `digits.len() == 4` and `== 2` guards are what separate
        // them, and both were free.
        assert_eq!(minutes("12:00:00+05:30"), Some(330));
        assert_eq!(minutes("12:00:00+0530"), Some(330));
        assert_eq!(minutes("12:00:00+05"), Some(300));
        assert_eq!(minutes("12:00:00-05:30"), Some(-330));
        assert_eq!(minutes("12:00:00-0530"), Some(-330));
        assert_eq!(minutes("12:00:00-05"), Some(-300));
        // No offset at all is an unanchored local time, not a zero offset.
        assert_eq!(minutes("12:00:00"), None);

        // The boundaries of the range check. `+14:00` is the largest offset in
        // use (Kiribati); `59` is the largest minute. Both `>` could become
        // `>=` and reject a legal offset, or `==` and admit an absurd one.
        assert_eq!(minutes("12:00:00+14:00"), Some(840));
        assert_eq!(minutes("12:00:00+13:59"), Some(839));
        for bad in [
            "12:00:00+15:00", // hours out of range
            "12:00:00+14:60", // minutes out of range
            "12:00:00+99:00",
            "12:00:00+5:30",  // one digit
            "12:00:00+05:3",
            "12:00:00+053",   // three digits: neither shape
            "12:00:00+05300",
            "12:00:00+ab:cd",
            // The `lib:A-16` shape: four *bytes*, one character. A length
            // check in bytes with a split in characters panicked here.
            "12:00:00+\u{1F600}",
        ] {
            assert!(bad.parse::<Time>().is_err(), "{bad} was accepted");
        }
    }

    /// A time at each precision, and how the parser refuses a malformed one.
    ///
    /// The `[h]` and `[h, m]` arms could each be deleted — hour-only and
    /// hour-and-minute times are legal ISO 8601 and openEHR stores them — and
    /// both length-and-digit guards could be widened to accept `1x:00`.
    #[test]
    fn a_time_parses_at_each_precision_and_keeps_its_text() {
        for (text, hour, minute, second, fraction) in [
            ("09", 9u8, None, None, None),
            ("09:30", 9, Some(30u8), None, None),
            ("09:30:15", 9, Some(30), Some(15u8), None),
            ("09:30:15.250", 9, Some(30), Some(15), Some("250")),
            // A leap second is accepted, as the accessor's documentation says.
            ("23:59:60", 23, Some(59), Some(60), None),
        ] {
            let t: Time = text.parse().unwrap_or_else(|e| panic!("{text}: {e}"));
            assert_eq!(t.hour(), hour, "{text}");
            assert_eq!(t.minute(), minute, "{text}");
            assert_eq!(t.second(), second, "{text}");
            assert_eq!(t.fraction(), fraction, "{text}");
            // The lexical form is the stored value (`db:M3.28`).
            assert_eq!(t.as_str(), text);
        }

        for bad in [
            "1x:00", "9:00", "090", "09:0", "09:000", "09:30:1",
            "09:30:15.", "09:30:15.x", "09:30:15:00",
        ] {
            assert!(bad.parse::<Time>().is_err(), "{bad} was accepted as a time");
        }
    }

    /// Ordering two times, and the millisecond arithmetic underneath it.
    ///
    /// Every term of `millis_local` survived — the hour, minute and second
    /// multipliers and all three additions — because the existing comparisons
    /// differed in a component that dominates whatever the others do. A
    /// fraction shorter than three digits is padded, not read as smaller:
    /// `.5` is 500ms, and the loop that pads it was free to run never or
    /// forever.
    #[test]
    fn times_order_by_the_instant_they_denote() {
        let t = |s: &str| s.parse::<Time>().unwrap();

        // Each component in isolation, so no other term can carry the result.
        assert!(t("09:00:00") < t("10:00:00"));
        assert!(t("09:00:00") < t("09:01:00"));
        assert!(t("09:00:00") < t("09:00:01"));
        assert!(t("09:00:00.100") < t("09:00:00.200"));

        // Across components, which is the only way the *scale* of each term is
        // tested. Comparing `09:00:00` with `09:01:00` cannot tell `m * 60_000`
        // from `m + 60_000` — addition is monotonic too, so the ordering comes
        // out the same. These pairs cross a boundary, so a term with the wrong
        // magnitude reverses them.
        assert!(t("00:02:00") > t("00:00:59"), "a minute is not sixty seconds");
        assert!(t("00:00:02") > t("00:00:00.500"), "a second is not 1000ms");
        assert!(t("01:00:00") > t("00:59:59"), "an hour is not sixty minutes");
        // The same for the fraction's padding: `.5` is 500ms, not 5000ms, so
        // it is *less* than a whole second. A pad loop that ran once too often
        // makes it greater.
        assert!(t("09:00:00.5") < t("09:00:01"), "`.5` was read as five seconds");

        // A short fraction is padded to milliseconds, not treated as smaller.
        // Compared through `partial_cmp` rather than `==`: `Eq` here is
        // lexical identity, and the two are deliberately different values that
        // denote the same instant (`lib:A-32`).
        assert!(t("09:00:00.5") > t("09:00:00.499"));
        let same = Some(Ordering::Equal);
        assert_eq!(t("09:00:00.5").partial_cmp(&t("09:00:00.500")), same);
        // And a longer one is truncated at millisecond resolution rather than
        // overflowing into the seconds.
        assert_eq!(t("09:00:00.5009").partial_cmp(&t("09:00:00.500")), same);

        // Offsets are normalised before comparing (`D3.18`): the same instant
        // written two ways orders equal, and a later local time can be the
        // earlier instant.
        assert_eq!(t("12:00:00+01:00").partial_cmp(&t("11:00:00Z")), same);
        assert!(t("12:00:00+01:00") < t("12:00:00Z"));

        // Equal on what both know, but different precision: unordered. A
        // 09:00 that might be 09:00:59 is not "the same as" 09:00:00.
        assert_eq!(t("09:00").partial_cmp(&t("09:00:00")), None);
        // One anchored and one not: unordered, whatever the components say.
        assert_eq!(t("12:00:00Z").partial_cmp(&t("12:00:00")), None);
    }

    /// `Eq` and `PartialOrd` answer different questions, and disagree.
    ///
    /// `Eq` is derived and therefore **lexical**: two values are equal when
    /// they were written the same way. `PartialOrd` normalises to UTC
    /// (`D3.18`) and compares instants. So `11:00:00Z` and `12:00:00+01:00`
    /// order `Equal` while `==` says they differ.
    ///
    /// That contradicts the standard library's requirement that the two agree
    /// — `a == b` if and only if `partial_cmp` is `Some(Equal)` — and it is
    /// pinned here rather than fixed, because both halves are wanted and
    /// neither can move without losing something. Lexical equality is record
    /// identity: the text is the stored value (`db:M3.28`), `.5` and `.50` are
    /// different strings, and `Hash` must agree with `Eq`. Instant ordering is
    /// what a query needs. See `lib:A-32` and `D3.18a`.
    ///
    /// **What a caller must not do:** sort, `dedup`, or `binary_search` a
    /// collection of these and expect the two notions to coincide. `dedup`
    /// after `sort` keeps both spellings, because `dedup` uses `==`.
    #[test]
    fn lexical_equality_and_instant_ordering_are_different_questions() {
        let t = |s: &str| s.parse::<Time>().unwrap();
        let (utc, plus_one) = (t("11:00:00Z"), t("12:00:00+01:00"));

        assert_eq!(utc.partial_cmp(&plus_one), Some(Ordering::Equal));
        assert_ne!(utc, plus_one, "`Eq` is lexical, and these differ");
        // The consequence, stated so it cannot surprise anyone twice: none of
        // the three comparison operators is true for a pair that `partial_cmp`
        // calls equal. Written with `matches!` because clippy rightly objects
        // to negating `<` on a partially ordered type — the objection is the
        // point, and the operators are exactly what a caller reaches for.
        assert!(matches!(utc.partial_cmp(&plus_one), Some(Ordering::Equal)));
        assert!(!matches!(utc.partial_cmp(&plus_one), Some(Ordering::Less)));
        assert!(!matches!(
            utc.partial_cmp(&plus_one),
            Some(Ordering::Greater)
        ));
        assert_ne!(utc, plus_one);

        // The same for a fraction written two ways.
        assert_eq!(
            t("09:00:00.5").partial_cmp(&t("09:00:00.50")),
            Some(Ordering::Equal)
        );
        assert_ne!(t("09:00:00.5"), t("09:00:00.50"));

        // And identical text is equal by both notions, which is the case the
        // rest of the crate relies on.
        assert_eq!(utc, t("11:00:00Z"));
        assert_eq!(utc.partial_cmp(&t("11:00:00Z")), Some(Ordering::Equal));
    }

    /// Every component of a duration, and the approximate length it implies.
    ///
    /// The `weeks`, `days` and `seconds` accessors could each return a
    /// constant, and four of the six additions in `approx_seconds` could be a
    /// subtraction or a multiplication, with nothing failing (`lib:A-09`).
    /// `approx_seconds` is what orders durations, and a wrong sign on the
    /// weeks term makes `P2W` shorter than `P1W`.
    ///
    /// The expected totals are computed here from the documented constants —
    /// 365.2425-day years and 30.436875-day months — rather than by running
    /// the function, so an inverted operator has nowhere to hide.
    #[test]
    fn a_duration_reports_each_component_and_an_approximate_length() {
        let d: Duration = "P1Y2M3W4DT5H6M7.5S".parse().unwrap();
        assert!(!d.is_negative());
        assert_eq!(d.years(), 1);
        assert_eq!(d.months(), 2);
        assert_eq!(d.weeks(), 3);
        assert_eq!(d.days(), 4);
        assert_eq!(d.hours(), 5);
        assert_eq!(d.minutes(), 6);
        assert_eq!(d.seconds(), Some("7.5"));
        assert_eq!(d.as_str(), "P1Y2M3W4DT5H6M7.5S");

        let want = 365.2425 * 86_400.0
            + 2.0 * 30.436_875 * 86_400.0
            + 3.0 * 7.0 * 86_400.0
            + 4.0 * 86_400.0
            + 5.0 * 3_600.0
            + 6.0 * 60.0
            + 7.5;
        assert!(
            (d.approx_seconds() - want).abs() < 1e-6,
            "{} != {want}",
            d.approx_seconds()
        );

        // Each term alone, so no other can carry the result — this is what the
        // combined case above cannot show.
        for (text, seconds) in [
            ("P1Y", 365.2425 * 86_400.0),
            ("P1M", 30.436_875 * 86_400.0),
            ("P1W", 7.0 * 86_400.0),
            ("P1D", 86_400.0),
            ("PT1H", 3_600.0),
            ("PT1M", 60.0),
            ("PT1S", 1.0),
            ("PT0S", 0.0),
        ] {
            let d: Duration = text.parse().unwrap_or_else(|e| panic!("{text}: {e}"));
            assert!(
                (d.approx_seconds() - seconds).abs() < 1e-6,
                "{text}: {} != {seconds}",
                d.approx_seconds()
            );
        }

        // Each is longer than one of itself, which a flipped sign breaks.
        for (more, less) in [("P2W", "P1W"), ("P2D", "P1D"), ("PT2S", "PT1S")] {
            let (a, b): (Duration, Duration) = (more.parse().unwrap(), less.parse().unwrap());
            assert!(
                a.approx_seconds() > b.approx_seconds(),
                "{more} is not longer than {less}"
            );
        }

        // A duration with no seconds component reports none, rather than zero:
        // "no seconds were written" and "zero seconds" are different records.
        let d: Duration = "P1D".parse().unwrap();
        assert_eq!(d.seconds(), None);
        assert_eq!(d.weeks(), 0);
        assert_eq!(d.days(), 1);
    }

    /// Ordering durations, including the pairs that must refuse to order.
    ///
    /// Seven mutants survived here (`lib:A-09`). Both halves of
    /// `d.years > 0 || d.months > 0` could be any comparison, and the guard
    /// that refuses `P1M` against `P30D` could be `true` — which makes every
    /// equal-length pair incomparable — or have its `&&`/`||` swapped.
    ///
    /// The refusal is the point of the impl, and it had no test. A month is 28
    /// to 31 days, so `P1M` and `P30D` have no order without a calendar anchor;
    /// answering `Equal` because the approximations agree would let a caller
    /// sort a medication interval into the wrong place.
    // Exact float comparison is deliberate and load-bearing here: the guard
    // under test fires only when `partial_cmp` says `Equal`, which is itself an
    // exact comparison. An epsilon would let the fixture drift until the two
    // approximations merely nearly agree, at which point the pair is ordered
    // rather than refused and the test silently stops testing the guard.
    #[test]
    #[allow(clippy::float_cmp)]
    fn durations_order_by_length_and_refuse_when_a_calendar_is_needed() {
        let d = |s: &str| s.parse::<Duration>().unwrap_or_else(|e| panic!("{s}: {e}"));

        // Plainly ordered: no calendar component on either side.
        assert!(d("PT1H") < d("PT2H"));
        assert!(d("P1D") < d("P1W"));
        assert!(d("PT59S") < d("PT1M"));
        // Equal and identically written: ordered, not refused.
        assert_eq!(d("P1D").partial_cmp(&d("P1D")), Some(Ordering::Equal));
        assert_eq!(d("P1M").partial_cmp(&d("P1M")), Some(Ordering::Equal));

        // Calendar against non-calendar, approximations equal but shapes
        // different: no order. `P1M` is 30.436875 days by the constant, so
        // this is the pair the guard exists for.
        let month = d("P1M");
        let same_days = d("PT2629746S"); // 30.436875 days in seconds
        assert_eq!(
            month.approx_seconds(),
            same_days.approx_seconds(),
            "fixture no longer pins the guard"
        );
        assert_eq!(month.partial_cmp(&same_days), None, "P1M vs an equal span");
        assert_eq!(same_days.partial_cmp(&month), None, "and the other way");

        // A calendar component on *either* side is enough — the `||`. And both
        // halves of `years > 0 || months > 0` matter, so a year-only duration
        // must behave the same way as a month-only one.
        let year = d("P1Y");
        let year_in_seconds = d("PT31556952S"); // 365.2425 days
        assert_eq!(year.approx_seconds(), year_in_seconds.approx_seconds());
        assert_eq!(year.partial_cmp(&year_in_seconds), None);

        // Neither side calendarish: equal approximations *are* an order, and
        // the `&&` is what keeps this from being refused too.
        assert_eq!(
            d("P1W").partial_cmp(&d("P7D")),
            Some(Ordering::Equal),
            "a week is exactly seven days, with no calendar involved"
        );

        // Ordered pairs stay ordered even with a calendar component: the guard
        // only fires on `Equal`.
        assert!(d("P1M") < d("P2M"));
        assert!(d("P1M") > d("P1D"));
    }

    /// A negative duration is negative, and orders below every positive one.
    ///
    /// `if self.negative { -secs } else { secs }` could drop its sign
    /// (`lib:A-09`), which makes `-P1D` and `P1D` the same length — and then
    /// the ordering guard refuses to order them at all, because their texts
    /// differ. openEHR permits negative durations (`SPECRM-96`), and a
    /// medication interval of minus one day is not the same as one day.
    #[test]
    fn a_negative_duration_is_shorter_than_zero() {
        let d = |s: &str| s.parse::<Duration>().unwrap_or_else(|e| panic!("{s}: {e}"));

        let minus_day = d("-P1D");
        assert!(minus_day.is_negative());
        // A day is a whole number of seconds, so this is exact — but say so
        // rather than relying on it.
        assert!((minus_day.approx_seconds() - -86_400.0).abs() < f64::EPSILON);
        assert_eq!(minus_day.as_str(), "-P1D");

        assert!(!d("P1D").is_negative());
        assert!(minus_day < d("PT0S"));
        assert!(minus_day < d("P1D"));
        assert!(d("-P2D") < d("-P1D"), "more negative is less");
    }

    /// Seconds are added to an instant, not subtracted from it.
    ///
    /// The `+ second` term of `epoch_seconds` could be a `-`, and every
    /// existing difference used midnight, where the seconds term is zero.
    #[test]
    fn epoch_seconds_adds_each_part_of_the_time_of_day() {
        let epoch: DateTime = "1970-01-01T00:00:00Z".parse().unwrap();
        for (text, seconds) in [
            ("1970-01-01T00:00:07Z", 7),
            ("1970-01-01T00:03:00Z", 180),
            ("1970-01-01T04:00:00Z", 14_400),
            ("1970-01-01T04:03:07Z", 14_400 + 180 + 7),
            // With an offset, which is subtracted rather than added.
            ("1970-01-01T04:03:07+01:00", 14_400 + 180 + 7 - 3_600),
        ] {
            let t: DateTime = text.parse().unwrap();
            assert_eq!(t.diff_seconds(&epoch), Some(seconds), "{text}");
        }
    }
}
