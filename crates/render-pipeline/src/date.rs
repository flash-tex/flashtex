//! The document date `\today` renders — carried by the request, never read
//! from the clock here.
//!
//! `docs/contracts/runtime-v1.md` requires byte-identical output for
//! byte-identical input, so this pipeline must not read the wall clock. The
//! caller reads it and sends the answer as an ordinary request field
//! (`payload.date`, `protocol/proposals/runtime-v1-request-date.md`); output
//! stays a pure function of its inputs with the date among them.
//!
//! ## Why this type exists twice
//!
//! `crates/compiler` owns the real [`TodayDate`] and the `\today` formatting.
//! This crate links `vendor/compiler`, a read-only pinned mirror
//! (`vendor/VENDORING.md`) that predates it, so the type is mirrored here until
//! that pin is refreshed. On re-pin this module collapses into a re-export of
//! `flashtex_compiler::date` and the `request-date` feature becomes default —
//! see `RenderOptions::today`.

/// Why a supplied date was refused. Never rounded, clamped, or replaced by the
/// epoch: a caller that sent a date meant it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateError {
    Malformed,
    OutOfRange,
}

impl std::fmt::Display for DateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DateError::Malformed => {
                f.write_str("date must be a civil date in YYYY-MM-DD form, e.g. \"2026-09-13\"")
            }
            DateError::OutOfRange => f.write_str(
                "date names a day that does not exist in the proleptic Gregorian calendar",
            ),
        }
    }
}

/// A proleptic-Gregorian civil date: what `\today` prints.
///
/// A civil date rather than an instant because `\today` is a local calendar
/// date; a timestamp would force this crate to pick a timezone the request does
/// not carry, reintroducing the hidden input the field exists to remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TodayDate {
    year: u16,
    month: u8,
    day: u8,
}

/// LaTeX's kernel month names, `\ifcase\month` order. From `\meaning\today`
/// under TeX Live 2025.
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl TodayDate {
    /// `1970-01-01`. What a request supplying no date compiles with, and what
    /// every committed-expectation harness pins.
    pub const EPOCH: TodayDate = TodayDate {
        year: 1970,
        month: 1,
        day: 1,
    };

    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, DateError> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return Err(DateError::OutOfRange);
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(DateError::OutOfRange);
        }
        Ok(TodayDate { year, month, day })
    }

    /// The wire form: exactly ten ASCII characters, `YYYY-MM-DD`. Deliberately
    /// strict — no time, no timezone, no offset, no alternative separator.
    pub fn parse_iso(text: &str) -> Result<Self, DateError> {
        let b = text.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return Err(DateError::Malformed);
        }
        let digits = |r: std::ops::Range<usize>| -> Option<u32> {
            let mut v: u32 = 0;
            for &c in &b[r] {
                if !c.is_ascii_digit() {
                    return None;
                }
                v = v * 10 + u32::from(c - b'0');
            }
            Some(v)
        };
        let (Some(y), Some(m), Some(d)) = (digits(0..4), digits(5..7), digits(8..10)) else {
            return Err(DateError::Malformed);
        };
        TodayDate::new(y as u16, m as u8, d as u8)
    }

    /// `<MonthName> <day>, <year>`, exactly as LaTeX's kernel `\today` renders
    /// it, with no zero padding on either number.
    pub fn latex_today(&self) -> String {
        format!(
            "{} {}, {}",
            MONTHS[usize::from(self.month) - 1],
            self.day,
            self.year
        )
    }

    pub fn to_iso(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn year(&self) -> u16 {
        self.year
    }

    pub fn month(&self) -> u8 {
        self.month
    }

    pub fn day(&self) -> u8 {
        self.day
    }
}

impl Default for TodayDate {
    fn default() -> Self {
        TodayDate::EPOCH
    }
}

fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// The civil date, in UTC, of a Unix timestamp — the `SOURCE_DATE_EPOCH`
/// conversion the reproducible-builds specification asks for.
///
/// Howard Hinnant's `civil_from_days`, era-based and exact for every value in
/// range; no floating point and no dependency.
pub fn utc_date_of_unix_seconds(seconds: i64) -> Result<TodayDate, DateError> {
    let days = seconds.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    if !(1..=9999).contains(&y) {
        return Err(DateError::OutOfRange);
    }
    TodayDate::new(y as u16, m as u8, d as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_what_the_pipeline_printed_before_the_field_existed() {
        assert_eq!(TodayDate::EPOCH.latex_today(), "January 1, 1970");
        assert_eq!(TodayDate::default(), TodayDate::EPOCH);
    }

    #[test]
    fn latex_today_matches_the_kernel_format() {
        assert_eq!(
            TodayDate::new(2026, 9, 13).unwrap().latex_today(),
            "September 13, 2026"
        );
        assert_eq!(
            TodayDate::new(2026, 1, 5).unwrap().latex_today(),
            "January 5, 2026"
        );
    }

    #[test]
    fn malformed_and_impossible_dates_are_refused() {
        for bad in ["2026-9-13", "26-09-13", "2026/09/13", "", "2026-09-13T00"] {
            assert_eq!(TodayDate::parse_iso(bad), Err(DateError::Malformed), "{bad}");
        }
        for bad in ["2026-02-29", "1900-02-29", "2026-13-01", "2026-04-31"] {
            assert_eq!(
                TodayDate::parse_iso(bad),
                Err(DateError::OutOfRange),
                "{bad}"
            );
        }
        assert!(TodayDate::parse_iso("2000-02-29").is_ok());
        assert!(TodayDate::parse_iso("2024-02-29").is_ok());
    }

    #[test]
    fn source_date_epoch_converts_in_utc() {
        // SOURCE_DATE_EPOCH=0 is the epoch, the date every harness pins.
        assert_eq!(utc_date_of_unix_seconds(0).unwrap(), TodayDate::EPOCH);
        // 2026-09-13T00:00:00Z, and one second before midnight the day before.
        assert_eq!(
            utc_date_of_unix_seconds(1_789_257_600).unwrap().to_iso(),
            "2026-09-13"
        );
        assert_eq!(
            utc_date_of_unix_seconds(1_789_257_599).unwrap().to_iso(),
            "2026-09-12"
        );
        // Leap day, and a leap-century boundary.
        assert_eq!(
            utc_date_of_unix_seconds(951_782_400).unwrap().to_iso(),
            "2000-02-29"
        );
    }

    #[test]
    fn round_trip_every_day_of_a_leap_year() {
        // 2024-01-01T00:00:00Z
        let start = 1_704_067_200i64;
        for day in 0..366 {
            let d = utc_date_of_unix_seconds(start + day * 86_400).unwrap();
            assert_eq!(TodayDate::parse_iso(&d.to_iso()).unwrap(), d);
        }
        assert_eq!(
            utc_date_of_unix_seconds(start + 365 * 86_400)
                .unwrap()
                .to_iso(),
            "2024-12-31"
        );
    }
}
