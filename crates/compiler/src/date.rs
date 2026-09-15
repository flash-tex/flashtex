//! The document date `\today` renders — an *input*, never the wall clock.
//!
//! Real `\today` reads the clock. This compiler must not: `docs/contracts/runtime-v1.md`
//! requires byte-identical output for byte-identical input, and a compiler that
//! reads the clock is not a function of its inputs at all.
//!
//! The resolution is the one reproducible builds already uses: **the clock is
//! read by the caller.** The date arrives as an ordinary field of the compile
//! request (`payload.date`, see `protocol/proposals/runtime-v1-request-date.md`),
//! so the output stays a pure function of `(documents, entry_path, capabilities,
//! date)` while still printing the real date the author expects.
//!
//! A request that supplies no date gets [`TodayDate::EPOCH`], `1970-01-01`,
//! which is what this compiler has always printed. Every existing client and
//! every committed fixture is therefore unchanged, byte for byte.
//!
//! This is a civil date, not an instant: `\today` is a local calendar date, and
//! a timestamp would force this crate to pick a timezone that the request does
//! not carry — reintroducing exactly the hidden input the field removes.

use std::fmt;

/// LaTeX's kernel month names, in `\ifcase\month` order. Verbatim from
/// `\meaning\today` under TeX Live 2025:
///
/// ```text
/// \ifcase\month\or January\or February\or March\or April\or May\or June\or
/// July\or August\or September\or October\or November\or December\fi
/// \space\number\day, \number\year
/// ```
///
/// These are the kernel's English names. `babel`/`polyglossia` date formats are
/// a separate feature and a separate request field; this type carries a date,
/// not a locale.
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

/// Why a supplied date was refused. The compiler never rounds, clamps or falls
/// back to the epoch on a bad value: a caller that sent a date meant it, and
/// silently typesetting a different one is the failure this whole mechanism
/// exists to end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateError {
    /// Not exactly `YYYY-MM-DD` in ASCII digits and two `-` separators.
    Malformed,
    /// Well-formed but names a day that does not exist, or a year outside
    /// `1..=9999` (`\number\year` has no sensible rendering outside it).
    OutOfRange,
}

impl fmt::Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TodayDate {
    year: u16,
    month: u8,
    day: u8,
}

impl TodayDate {
    /// `1970-01-01`, the Unix epoch. The date a request that supplies none is
    /// compiled with, and the date every committed-expectation harness pins so
    /// its reference data stays byte-identical.
    pub const EPOCH: TodayDate = TodayDate {
        year: 1970,
        month: 1,
        day: 1,
    };

    /// A calendar date, or [`DateError::OutOfRange`] if that day does not exist.
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, DateError> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return Err(DateError::OutOfRange);
        }
        if day < 1 || day > days_in_month(year, month) {
            return Err(DateError::OutOfRange);
        }
        Ok(TodayDate { year, month, day })
    }

    /// Parse the wire form: exactly ten ASCII characters, `YYYY-MM-DD`.
    ///
    /// Deliberately strict. No time, no timezone, no offset, no `+`, no other
    /// separator, no shorter or longer year. A caller that cannot produce this
    /// has not decided what date it means, and guessing for it is how the
    /// original bug happened.
    pub fn parse_iso(text: &str) -> Result<Self, DateError> {
        let bytes = text.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(DateError::Malformed);
        }
        let digits = |range: std::ops::Range<usize>| -> Option<u32> {
            let mut value: u32 = 0;
            for &b in &bytes[range] {
                if !b.is_ascii_digit() {
                    return None;
                }
                value = value * 10 + u32::from(b - b'0');
            }
            Some(value)
        };
        let (Some(year), Some(month), Some(day)) = (digits(0..4), digits(5..7), digits(8..10))
        else {
            return Err(DateError::Malformed);
        };
        TodayDate::new(year as u16, month as u8, day as u8)
    }

    /// What `\today` expands to, exactly as LaTeX's kernel renders it:
    /// `<MonthName> <day>, <year>`, with no zero padding on the day or the
    /// year — `September 13, 2026`, `January 1, 1970`.
    pub fn latex_today(&self) -> String {
        format!(
            "{} {}, {}",
            MONTHS[usize::from(self.month) - 1],
            self.day,
            self.year
        )
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

    /// The wire form, round-tripping [`TodayDate::parse_iso`].
    pub fn to_iso(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_renders_what_this_compiler_has_always_printed() {
        assert_eq!(TodayDate::EPOCH.latex_today(), "January 1, 1970");
        assert_eq!(TodayDate::default(), TodayDate::EPOCH);
    }

    #[test]
    fn latex_today_matches_the_kernel_format() {
        // pdflatex 3.141592653-2.6-1.40.27 (TeX Live 2025), \today on this date.
        let d = TodayDate::new(2026, 9, 13).unwrap();
        assert_eq!(d.latex_today(), "September 13, 2026");
        // \number\day and \number\year: no zero padding anywhere.
        assert_eq!(
            TodayDate::new(2026, 1, 5).unwrap().latex_today(),
            "January 5, 2026"
        );
        assert_eq!(
            TodayDate::new(999, 12, 31).unwrap().latex_today(),
            "December 31, 999"
        );
    }

    #[test]
    fn every_month_name_is_the_kernel_name() {
        let names: Vec<String> = (1..=12)
            .map(|m| {
                TodayDate::new(2026, m, 1)
                    .unwrap()
                    .latex_today()
                    .split(' ')
                    .next()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(
            names,
            vec![
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
                "December"
            ]
        );
    }

    #[test]
    fn iso_round_trips() {
        for iso in ["1970-01-01", "2026-09-13", "2024-02-29", "0001-01-01"] {
            assert_eq!(TodayDate::parse_iso(iso).unwrap().to_iso(), iso);
        }
    }

    #[test]
    fn malformed_dates_are_refused_not_guessed() {
        for bad in [
            "",
            "2026-9-13",     // unpadded month
            "26-09-13",      // two-digit year
            "2026/09/13",    // wrong separator
            "2026-09-13T00", // an instant, not a civil date
            "2026-09-13 ",
            " 2026-09-13",
            "+2026-09-13",
            "20260913",
            "abcd-ef-gh",
        ] {
            assert_eq!(
                TodayDate::parse_iso(bad),
                Err(DateError::Malformed),
                "{bad:?} must be refused"
            );
        }
    }

    #[test]
    fn impossible_days_are_refused() {
        for bad in [
            "2026-02-29", // 2026 is not a leap year
            "1900-02-29", // century non-leap
            "2026-00-10",
            "2026-13-01",
            "2026-04-31",
            "2026-01-00",
            "2026-01-32",
            "0000-01-01",
        ] {
            assert_eq!(
                TodayDate::parse_iso(bad),
                Err(DateError::OutOfRange),
                "{bad:?} must be refused"
            );
        }
        // The Gregorian leap rule, both exceptions.
        assert!(TodayDate::parse_iso("2000-02-29").is_ok());
        assert!(TodayDate::parse_iso("2024-02-29").is_ok());
    }
}
