//! Bounded, typed adapter over [`layout_paragraph`]. The dimension and
//! item-count checks are defined in this module ([`check_dimen`],
//! [`validate_items`], [`validate_params`]) but are invoked directly by
//! [`layout_paragraph`] itself, before the breaker ever runs — see that
//! function's doc for why validation lives there rather than only in this
//! wrapper. What this module adds on top is narrower: it converts a
//! residual internal panic (a breaker bug, not invalid input — that is
//! already a typed rejection by the time this wrapper runs) into a typed
//! error instead of letting it propagate. FT-030 rev 3.
//!
//! ## Why this checks dimensions instead of switching to integer scaled points
//!
//! TeX represents every dimension as a 32-bit integer in scaled points
//! (1pt = 65536sp) so arithmetic is exact and `MAX_DIMEN` (`07777777777`
//! octal = 2^30 - 1 sp = 16383.99998pt, the TeXbook's documented value) is a
//! hard, checkable ceiling. This crate's breaker
//! ([`crate::linebreak`], inherited from the cancelled FT-019 lane) measures
//! in `f64` points throughout — not TeX's integer representation. Rewriting
//! its ~900-line Knuth-Plass core to integer scaled-point arithmetic is a
//! cross-cutting change to already-tested, working code (24 pre-existing
//! tests, a pdflatex oracle comparison) and is out of scope for this pass;
//! doing it without a concrete failure driving it would be exactly the kind
//! of speculative rewrite this crate's "no implicit approximation" and "no
//! copied reference engine" discipline argues against.
//!
//! What this module adds is real, not cosmetic: every dimension entering the
//! breaker through [`try_layout_paragraph`] is checked against the same
//! `MAX_DIMEN` bound TeX uses, in the same unit (pt), and rejected with a
//! typed [`LayoutError`] at or past it — never silently clamped, wrapped, or
//! left to panic partway through the algorithm. A value strictly inside the
//! bound is an integer number of scaled points that fits in 31 bits, which
//! an `f64`'s 53-bit mantissa represents exactly, so nothing here loses
//! precision by staying in pt.
use std::fmt;
use std::panic::{self, AssertUnwindSafe};

use crate::items::{Glue, Item};
use crate::linebreak::{LineBreakParams, Lines, layout_paragraph};

/// TeX's `max_dimen`: `07777777777` octal = 2^30 - 1 scaled points. Also
/// reused, uncoincidentally, as this crate's [`crate::linebreak::AWFUL_BAD`]
/// badness sentinel — the same ceiling TeX itself repurposes.
pub const MAX_DIMEN_SP: i64 = 1_073_741_823;

/// [`MAX_DIMEN_SP`] expressed in points (the TeXbook's documented value,
/// `1_073_741_823.0 / 65536.0` rounded down to avoid overstating the bound).
/// A dimension whose magnitude is at or beyond this is rejected.
pub const MAX_DIMEN_PT: f64 = 16383.99998;

/// Upper bound on the number of horizontal-list items one call will
/// process. [`total_fit_pass`](crate::linebreak) is worst-case quadratic in
/// the number of legal break points versus active nodes; this bound turns
/// "absurdly large input" into a fast, typed rejection instead of an
/// unbounded amount of work. A real paragraph — even an entire chapter set
/// as one paragraph — is far below this.
pub const MAX_ITEMS: usize = 200_000;

/// Everything [`try_layout_paragraph`] can reject before, or instead of,
/// running the breaker.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutError {
    /// A dimension was NaN or +/-infinity.
    NonFiniteDimension { value: f64, context: &'static str },
    /// A dimension's magnitude reached or passed [`MAX_DIMEN_PT`].
    DimensionOverflow { value: f64, context: &'static str },
    /// The item list exceeded [`MAX_ITEMS`].
    TooManyItems { count: usize, limit: usize },
    /// The underlying breaker panicked. This is a defense-in-depth backstop,
    /// not a documented outcome of any known input: every case this crate's
    /// adversarial tests exercise returns `Ok` or one of the variants above
    /// instead. Carries the panic payload's message, if it was a `&str` or
    /// `String`.
    Internal(String),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LayoutError::NonFiniteDimension { value, context } => {
                write!(f, "{context}: dimension {value} is not finite")
            }
            LayoutError::DimensionOverflow { value, context } => {
                write!(
                    f,
                    "{context}: dimension {value}pt reaches or exceeds MAX_DIMEN ({MAX_DIMEN_PT}pt)"
                )
            }
            LayoutError::TooManyItems { count, limit } => {
                write!(f, "{count} items exceeds the bounded limit of {limit}")
            }
            LayoutError::Internal(msg) => write!(f, "internal layout error: {msg}"),
        }
    }
}

impl std::error::Error for LayoutError {}

/// Rejects a dimension that is non-finite or at/past [`MAX_DIMEN_PT`] in
/// magnitude. `context` names the field, for the error message only.
pub fn check_dimen(value: f64, context: &'static str) -> Result<(), LayoutError> {
    if !value.is_finite() {
        return Err(LayoutError::NonFiniteDimension { value, context });
    }
    if value.abs() >= MAX_DIMEN_PT {
        return Err(LayoutError::DimensionOverflow { value, context });
    }
    Ok(())
}

fn check_glue(g: &Glue, context: &'static str) -> Result<(), LayoutError> {
    check_dimen(g.width, context)?;
    check_dimen(g.stretch, context)?;
    check_dimen(g.shrink, context)
}

pub(crate) fn validate_items(items: &[Item]) -> Result<(), LayoutError> {
    if items.len() > MAX_ITEMS {
        return Err(LayoutError::TooManyItems {
            count: items.len(),
            limit: MAX_ITEMS,
        });
    }
    for item in items {
        match item {
            Item::Box(run) => {
                check_dimen(run.width, "box.width")?;
                check_dimen(run.height, "box.height")?;
                check_dimen(run.depth, "box.depth")?;
                for g in &run.glyphs {
                    check_dimen(g.advance, "box.glyph.advance")?;
                    check_dimen(g.kern, "box.glyph.kern")?;
                }
            }
            Item::Glue(g) => check_glue(g, "glue")?,
            Item::Penalty(p) => {
                if let Some(hy) = &p.pre_break {
                    check_dimen(hy.width, "penalty.pre_break.width")?;
                }
                if let Some(hy) = &p.post_break {
                    check_dimen(hy.width, "penalty.post_break.width")?;
                }
            }
            Item::Kern(k) => check_dimen(k.width, "kern.width")?,
        }
    }
    Ok(())
}

pub(crate) fn validate_params(params: &LineBreakParams) -> Result<(), LayoutError> {
    check_dimen(params.line_width, "params.line_width")?;
    check_dimen(params.parindent, "params.parindent")?;
    check_dimen(params.baselineskip, "params.baselineskip")?;
    check_dimen(params.lineskip, "params.lineskip")?;
    check_dimen(params.lineskiplimit, "params.lineskiplimit")?;
    check_dimen(params.emergency_stretch, "params.emergency_stretch")?;
    check_glue(&params.left_skip, "params.left_skip")?;
    check_glue(&params.right_skip, "params.right_skip")
}

/// Fallible, bounded wrapper over [`layout_paragraph`].
///
/// The dimension/item-count validation this doc used to describe as living
/// here now lives in [`layout_paragraph`] itself — see that function's doc
/// for why the raw entry point is the authoritative, single place that
/// decides what counts as valid input. This wrapper's own job is narrower:
/// catch a panic from the breaker (a bug, not a validation failure — real
/// invalid input is now rejected as a typed [`LayoutError`] before the
/// breaker's own algorithm runs) and report it as [`LayoutError::Internal`]
/// instead of unwinding into the caller.
///
/// ```
/// use flashtex_paragraph_layout::adapter::{try_layout_paragraph, MAX_DIMEN_PT, LayoutError};
/// use flashtex_paragraph_layout::core14::Core14Times;
/// use flashtex_paragraph_layout::hyphenate::NoHyphenation;
/// use flashtex_paragraph_layout::items::{Glue, ParagraphBuilder};
/// use flashtex_paragraph_layout::linebreak::LineBreakParams;
///
/// let h = NoHyphenation;
/// let mut b = ParagraphBuilder::new(&h);
/// b.text(&Core14Times::ROMAN, 12.0, "A short paragraph of plain text.", 0).unwrap();
/// let items = b.finish(Glue::fil());
/// let params = LineBreakParams::article_12pt_letter_1in().with_width(200.0);
/// let lines = try_layout_paragraph(&items, &params).expect("well-formed input");
/// assert!(!lines.lines.is_empty());
///
/// // A line width at MAX_DIMEN is rejected rather than fed to the breaker.
/// let bad = LineBreakParams::article_12pt_letter_1in().with_width(MAX_DIMEN_PT);
/// assert_eq!(
///     try_layout_paragraph(&items, &bad),
///     Err(LayoutError::DimensionOverflow {
///         value: MAX_DIMEN_PT,
///         context: "params.line_width",
///     }),
/// );
/// ```
pub fn try_layout_paragraph(
    items: &[Item],
    params: &LineBreakParams,
) -> Result<Lines, LayoutError> {
    match panic::catch_unwind(AssertUnwindSafe(|| layout_paragraph(items, params))) {
        Ok(result) => result,
        Err(payload) => Err(LayoutError::Internal(crate::panic_message(&*payload))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core14::Core14Times;
    use crate::hyphenate::NoHyphenation;
    use crate::items::ParagraphBuilder;
    use crate::linebreak::LineBreakParams;

    fn ok_items() -> Vec<Item> {
        let h = NoHyphenation;
        let mut b = ParagraphBuilder::new(&h);
        b.text(&Core14Times::ROMAN, 12.0, "hello world", 0).unwrap();
        b.finish(Glue::fil())
    }

    #[test]
    fn well_formed_input_is_ok() {
        let items = ok_items();
        let params = LineBreakParams::article_12pt_letter_1in().with_width(200.0);
        assert!(try_layout_paragraph(&items, &params).is_ok());
    }

    #[test]
    fn dimension_just_under_max_dimen_is_ok() {
        assert_eq!(check_dimen(MAX_DIMEN_PT - 0.001, "x"), Ok(()));
        assert_eq!(check_dimen(-(MAX_DIMEN_PT - 0.001), "x"), Ok(()));
    }

    #[test]
    fn dimension_at_max_dimen_is_overflow() {
        assert_eq!(
            check_dimen(MAX_DIMEN_PT, "x"),
            Err(LayoutError::DimensionOverflow {
                value: MAX_DIMEN_PT,
                context: "x"
            })
        );
        assert_eq!(
            check_dimen(-MAX_DIMEN_PT, "x"),
            Err(LayoutError::DimensionOverflow {
                value: -MAX_DIMEN_PT,
                context: "x"
            })
        );
    }

    #[test]
    fn dimension_one_past_max_dimen_is_overflow() {
        assert_eq!(
            check_dimen(MAX_DIMEN_PT + 1.0, "x"),
            Err(LayoutError::DimensionOverflow {
                value: MAX_DIMEN_PT + 1.0,
                context: "x"
            })
        );
    }

    #[test]
    fn nan_and_infinite_are_rejected_before_overflow_check() {
        // NaN != NaN, so this one case is checked by pattern, not equality.
        assert!(matches!(
            check_dimen(f64::NAN, "x"),
            Err(LayoutError::NonFiniteDimension { context: "x", .. })
        ));
        assert_eq!(
            check_dimen(f64::INFINITY, "x"),
            Err(LayoutError::NonFiniteDimension {
                value: f64::INFINITY,
                context: "x"
            })
        );
        assert_eq!(
            check_dimen(f64::NEG_INFINITY, "x"),
            Err(LayoutError::NonFiniteDimension {
                value: f64::NEG_INFINITY,
                context: "x"
            })
        );
    }

    #[test]
    fn overwide_line_width_is_typed_error_not_a_layout_attempt() {
        let items = ok_items();
        let params = LineBreakParams::article_12pt_letter_1in().with_width(MAX_DIMEN_PT);
        assert_eq!(
            try_layout_paragraph(&items, &params),
            Err(LayoutError::DimensionOverflow {
                value: MAX_DIMEN_PT,
                context: "params.line_width"
            })
        );
    }

    #[test]
    fn glue_width_at_bound_inside_items_is_rejected() {
        let mut items = ok_items();
        // Insert an adversarial glue whose width is exactly at MAX_DIMEN.
        items.insert(0, Item::Glue(Glue::fixed(MAX_DIMEN_PT)));
        let params = LineBreakParams::article_12pt_letter_1in().with_width(200.0);
        assert_eq!(
            try_layout_paragraph(&items, &params),
            Err(LayoutError::DimensionOverflow {
                value: MAX_DIMEN_PT,
                context: "glue"
            })
        );
    }

    #[test]
    fn item_count_over_the_bound_is_rejected_without_running_the_breaker() {
        // MAX_ITEMS + 1 trivial items: cheap to build, and must be rejected
        // fast (before any O(items) breaker work) by the count check alone.
        let items: Vec<Item> = (0..=MAX_ITEMS).map(|_| Item::kern(0.0)).collect();
        let params = LineBreakParams::article_12pt_letter_1in();
        assert_eq!(
            try_layout_paragraph(&items, &params),
            Err(LayoutError::TooManyItems {
                count: MAX_ITEMS + 1,
                limit: MAX_ITEMS
            })
        );
    }

    #[test]
    fn item_count_exactly_at_the_bound_is_not_rejected_by_the_count_check() {
        // At the bound (not past it), the item-count gate passes; whatever
        // happens next is the breaker's business, not this typed rejection.
        let items: Vec<Item> = (0..MAX_ITEMS).map(|_| Item::kern(0.0)).collect();
        assert!(validate_items(&items).is_ok());
    }

    /// Regression: the raw [`layout_paragraph`] entry point used to have no
    /// validation of its own, so it silently accepted a NaN dimension that
    /// [`try_layout_paragraph`] correctly rejected — two entry points
    /// disagreeing about what counts as valid input. Both now share the same
    /// validation (this function calls it directly), so a NaN line width is
    /// rejected identically through either entry point instead of being fed
    /// to the breaker's `f64` arithmetic, which produced unspecified geometry
    /// rather than a documented error.
    #[test]
    fn raw_layout_paragraph_rejects_nan_the_same_way_the_checked_entry_point_does() {
        let items = ok_items();
        let mut params = LineBreakParams::article_12pt_letter_1in().with_width(200.0);
        params.line_width = f64::NAN;

        // NaN != NaN, so compare by pattern rather than by `assert_eq!`.
        assert!(matches!(
            crate::linebreak::layout_paragraph(&items, &params),
            Err(LayoutError::NonFiniteDimension {
                context: "params.line_width",
                ..
            })
        ));
        assert!(matches!(
            try_layout_paragraph(&items, &params),
            Err(LayoutError::NonFiniteDimension {
                context: "params.line_width",
                ..
            })
        ));
    }
}
