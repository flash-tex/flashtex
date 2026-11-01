//! Resolving the date `\today` renders — the one place in FlashTeX that is
//! *allowed* to read the clock.
//!
//! `docs/contracts/runtime-v1.md` requires byte-identical output for
//! byte-identical input, so the compiler and the render pipeline must never
//! read the wall clock. The CLI is the **caller**: it reads the clock and sends
//! the answer as an ordinary request input (`payload.date` /
//! `RenderOptions::today`, `protocol/proposals/runtime-v1-request-date.md`).
//! That is how reproducible builds already works, and it is what lets `\today`
//! print the real date without anything downstream becoming non-deterministic.
//!
//! Precedence, highest first:
//!
//! 1. `--date YYYY-MM-DD` — an explicit override, for harnesses and for anyone
//!    who wants an exact answer.
//! 2. `SOURCE_DATE_EPOCH` — the reproducible-builds variable. Its civil date
//!    **in UTC**, per that specification.
//! 3. the machine's current date.
//!
//! Note a deliberate divergence from pdfTeX: pdfTeX honours `SOURCE_DATE_EPOCH`
//! for `\today` only when `FORCE_SOURCE_DATE=1` is *also* set (alone it changes
//! only the PDF `/CreationDate`). FlashTeX honours it on its own, which is what
//! the reproducible-builds specification asks for and costs nothing here. A
//! harness comparing against pdflatex must therefore set both variables for the
//! oracle and pass the date explicitly to FlashTeX.

use flashtex_render_pipeline::date::{utc_date_of_unix_seconds, TodayDate};

/// Why the CLI could not resolve a date it was explicitly given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateArgError(pub String);

impl std::fmt::Display for DateArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Resolve the date to send, given an optional `--date` value and the process
/// environment. `env` is passed in rather than read here so the precedence is
/// testable without mutating global state.
pub fn resolve(
    explicit: Option<&str>,
    source_date_epoch: Option<&str>,
    now_unix_seconds: i64,
) -> Result<TodayDate, DateArgError> {
    if let Some(text) = explicit {
        return TodayDate::parse_iso(text)
            .map_err(|e| DateArgError(format!("--date {text:?}: {e}")));
    }
    if let Some(raw) = source_date_epoch {
        let trimmed = raw.trim();
        // The reproducible-builds spec: a non-negative decimal integer of
        // seconds. A malformed value is an error, not something to quietly
        // ignore -- a build that thinks it pinned the date and did not is
        // exactly the failure this variable exists to prevent.
        let seconds: i64 = trimmed.parse().map_err(|_| {
            DateArgError(format!(
                "SOURCE_DATE_EPOCH {raw:?} is not a decimal number of seconds"
            ))
        })?;
        if seconds < 0 {
            return Err(DateArgError(format!(
                "SOURCE_DATE_EPOCH {raw:?} must not be negative"
            )));
        }
        return utc_date_of_unix_seconds(seconds)
            .map_err(|e| DateArgError(format!("SOURCE_DATE_EPOCH {raw:?}: {e}")));
    }
    // No override: the machine's own date. Falling back to the epoch here would
    // silently reintroduce the bug this whole mechanism fixes, so a clock that
    // cannot be read is reported rather than guessed at.
    utc_date_of_unix_seconds(now_unix_seconds)
        .map_err(|e| DateArgError(format!("system clock is outside the representable range: {e}")))
}

/// Seconds since the Unix epoch, now.
///
/// **This is UTC, not the machine's local timezone.** `std` exposes no local
/// time and this crate is deliberately dependency-light, so a user east or west
/// of UTC can see yesterday's or tomorrow's date near midnight. `--date` is the
/// exact answer in the meantime, and the Mac app — the path the reported bug
/// came from — resolves a true local date through `Calendar.current` and sends
/// it in `payload.date`, so it is unaffected by this limitation.
pub fn now_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Read `SOURCE_DATE_EPOCH` from the real environment.
pub fn source_date_epoch_from_env() -> Option<String> {
    std::env::var("SOURCE_DATE_EPOCH").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_789_300_000; // 2026-09-13T11:46:40Z

    #[test]
    fn without_any_override_the_clock_wins() {
        let d = resolve(None, None, NOW).unwrap();
        assert_eq!(d.to_iso(), "2026-09-13");
        assert_eq!(d.latex_today(), "September 13, 2026");
    }

    #[test]
    fn source_date_epoch_overrides_the_clock() {
        let d = resolve(None, Some("0"), NOW).unwrap();
        assert_eq!(d, TodayDate::EPOCH);
        assert_eq!(d.latex_today(), "January 1, 1970");
    }

    /// The pin harnesses rely on: `SOURCE_DATE_EPOCH=0` must give the epoch no
    /// matter what day the machine thinks it is.
    #[test]
    fn source_date_epoch_zero_pins_the_epoch_whatever_the_clock_says() {
        for now in [0, NOW, 4_102_444_800] {
            assert_eq!(resolve(None, Some("0"), now).unwrap(), TodayDate::EPOCH);
        }
    }

    #[test]
    fn an_explicit_date_beats_source_date_epoch_and_the_clock() {
        let d = resolve(Some("2001-02-03"), Some("0"), NOW).unwrap();
        assert_eq!(d.to_iso(), "2001-02-03");
    }

    #[test]
    fn a_malformed_explicit_date_is_an_error_not_a_fallback() {
        let e = resolve(Some("13-09-2026"), None, NOW).unwrap_err();
        assert!(e.0.contains("--date"), "{e}");
        assert!(resolve(Some("2026-02-29"), None, NOW).is_err());
    }

    /// A build that believes it pinned the date and did not is the failure this
    /// variable exists to prevent, so a bad value is refused rather than ignored.
    #[test]
    fn a_malformed_source_date_epoch_is_an_error_not_a_silent_fallback() {
        for bad in ["", "abc", "-1", "1.5", "0x10"] {
            let r = resolve(None, Some(bad), NOW);
            assert!(r.is_err(), "SOURCE_DATE_EPOCH {bad:?} was accepted: {r:?}");
        }
        // Whitespace around an otherwise valid value is tolerated.
        assert_eq!(resolve(None, Some(" 0 "), NOW).unwrap(), TodayDate::EPOCH);
    }
}
