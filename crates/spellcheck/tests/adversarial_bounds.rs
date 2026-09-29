//! Adversarial bounds: hostile inputs aimed at the checker's cost model,
//! its malformed-input handling, and its capacity limits. Every case here
//! must (a) not panic, (b) not hang, and (c) demonstrate that the bound in
//! question actually binds -- by measuring elapsed time against a generous
//! ceiling, by counting how many times a callback fires, or by asserting an
//! exact typed error/outcome -- rather than merely "seeming fast" on one run.

use flashtex_spellcheck::{
    CheckOutcome, Dictionary, SpellChecker, SpellCheckerConfig, UserDictionary,
    UserDictionaryError, USER_DICTIONARY_DEFAULT_MAX_ENTRIES,
};
use std::cell::Cell;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Generous wall-clock ceiling for a single adversarial case. Any of these
/// inputs completing within this bound (on top of debug-build overhead) is
/// strong evidence there is no accidental exponential or quadratic blowup;
/// none of them are expected to take more than a small fraction of this in
/// practice.
const BOUND: Duration = Duration::from_secs(10);

fn dict(words: &[&str]) -> HashSet<String> {
    words.iter().map(|w| w.to_string()).collect()
}

/// Wraps a dictionary and counts `contains` lookups. Every raw candidate
/// the suggestion search generates is looked up in the dictionary (bar
/// in-search duplicates), so the lookup count is a deterministic work
/// counter for suggestion generation: unlike wall-clock time it does not
/// depend on build profile or on how loaded a shared CI runner is.
struct CountingDict<'a> {
    inner: &'a HashSet<String>,
    lookups: Cell<usize>,
}

impl<'a> CountingDict<'a> {
    fn new(inner: &'a HashSet<String>) -> Self {
        Self {
            inner,
            lookups: Cell::new(0),
        }
    }
}

impl Dictionary for CountingDict<'_> {
    fn contains(&self, word: &str) -> bool {
        self.lookups.set(self.lookups.get() + 1);
        self.inner.contains(word)
    }
}

/// Raw-candidate count `edits1` can produce for an `n`-char word over the
/// 26-letter edit alphabet: deletions + transpositions + substitutions +
/// insertions.
fn edits1_candidates(n: usize) -> usize {
    n + n.saturating_sub(1) + 26 * n + 26 * (n + 1)
}

fn assert_bounded<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    let elapsed = start.elapsed();
    assert!(
        elapsed < BOUND,
        "{label} took {elapsed:?}, exceeding the {BOUND:?} bound -- suspected unbounded blowup"
    );
    out
}

// ---------------------------------------------------------------------
// 1. A document of megabyte scale.
// ---------------------------------------------------------------------

#[test]
fn megabyte_scale_document_is_bounded_and_exact() {
    let d = dict(&["hello", "world"]);
    let checker = SpellChecker::default();

    // "hello wrold " is 12 bytes; 100_000 repeats is 1_200_000 bytes.
    const REPEATS: usize = 100_000;
    let text = "hello wrold ".repeat(REPEATS);
    assert!(
        text.len() > 1_000_000,
        "test setup must exceed one megabyte"
    );

    let out = assert_bounded("megabyte-scale document", || checker.check(&text, &d));

    // Every "wrold" is flagged (distance-1 hit against "world"), every
    // "hello" is not -- proving the bound doesn't come at the cost of
    // correctness at scale.
    assert_eq!(out.len(), REPEATS);
    assert!(out.iter().all(|m| m.word == "wrold"));
    assert!(out
        .iter()
        .all(|m| m.suggestions == vec!["world".to_string()]));
}

// ---------------------------------------------------------------------
// 2. A single word of enormous length.
// ---------------------------------------------------------------------

#[test]
fn single_enormous_word_is_bounded_and_gets_no_suggestions() {
    let d = dict(&["hello"]);
    let checker = SpellChecker::default();

    let huge_word = "x".repeat(3_000_000);
    let text = format!("hello {huge_word}");

    let out = assert_bounded("single enormous word", || checker.check(&text, &d));

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].word, huge_word);
    assert!(
        out[0].suggestions.is_empty(),
        "a word this far past max_word_length_for_suggestions must get zero suggestions"
    );
}

// ---------------------------------------------------------------------
// 3. Text that is entirely math delimiters.
// ---------------------------------------------------------------------

#[test]
fn text_of_only_dollar_delimiters_is_bounded_and_produces_no_words() {
    let d = dict(&["anything"]);
    let checker = SpellChecker::default();

    let text = "$".repeat(200_000);
    let out = assert_bounded("all-dollar-delimiter text", || checker.check(&text, &d));
    assert!(
        out.is_empty(),
        "a text with no alphabetic runs can flag nothing: {out:?}"
    );
}

#[test]
fn text_of_only_mixed_delimiters_is_bounded_and_produces_no_words() {
    let d = dict(&["anything"]);
    let checker = SpellChecker::default();

    let text = r"\(\)\[\]$$".repeat(50_000);
    let out = assert_bounded("all-mixed-delimiter text", || checker.check(&text, &d));
    assert!(
        out.is_empty(),
        "no alphabetic content exists to flag: {out:?}"
    );
}

// ---------------------------------------------------------------------
// 4. Unbalanced and nested math delimiters.
// ---------------------------------------------------------------------

#[test]
fn unbalanced_and_nested_math_delimiters_do_not_panic_and_are_bounded() {
    let d = dict(&["hello"]);
    let checker = SpellChecker::default();

    // Opens paren-math, then dollar-math nested inside it (dollar-math has
    // no nesting concept of its own -- this exercises the "close whichever
    // kind is open" state machine with genuinely contradictory input),
    // followed by a bracket-close with no matching open, repeated many
    // times to also exercise scale.
    let unit = r"$a \( b $ c \) d \[ e $$ f \] g ";
    let text = unit.repeat(20_000);

    let out = assert_bounded("unbalanced/nested math delimiters", || {
        checker.check(&text, &d)
    });
    // No panic and a plain Vec came back is the assertion; ranges must
    // still all be char-boundary safe and round-trip to their own word.
    for m in &out {
        assert!(text.is_char_boundary(m.range.start));
        assert!(text.is_char_boundary(m.range.end));
        assert_eq!(&text[m.range.clone()], m.word);
    }
}

#[test]
fn deeply_unbalanced_single_run_reaches_end_of_input_without_panicking() {
    let d = dict(&["hello"]);
    let checker = SpellChecker::default();
    // One opening delimiter of each kind, never closed, then real prose.
    let text = r"$ \( \[ start of math that never closes hello";
    let out = checker.check(text, &d);
    assert!(
        out.is_empty(),
        "everything after the first '$' is excluded to end of input: {out:?}"
    );
}

// ---------------------------------------------------------------------
// 5. An unterminated command (runs off the absolute end of input).
// ---------------------------------------------------------------------

#[test]
fn command_that_runs_off_the_end_of_input_does_not_panic() {
    let d = dict(&["hello"]);
    let checker = SpellChecker::default();
    // No trailing character at all after the command name: exercises the
    // `chars.get(j).map(...).unwrap_or(text_len)` end-of-input fallback.
    // The 40-'a' run is itself a command name (`\` followed by ASCII
    // letters) and is excluded in full; "hello" is a known word. So the
    // only possible outcome is zero misspellings -- the point of this test
    // is solely that checking a command that runs off the end of input
    // returns promptly rather than panicking or looping.
    let text = format!("hello \\{}", "a".repeat(40));
    let out = checker.check(&text, &d);
    assert!(
        out.is_empty(),
        "the command name is excluded in full and 'hello' is a known word: {out:?}"
    );
}

// ---------------------------------------------------------------------
// 6. A command name of enormous length.
// ---------------------------------------------------------------------

#[test]
fn enormous_command_name_is_bounded_and_fully_excluded() {
    let d = dict(&["done"]);
    let checker = SpellChecker::default();

    let huge_command = "a".repeat(3_000_000);
    let text = format!("\\{huge_command} done");

    let out = assert_bounded("enormous command name", || checker.check(&text, &d));
    assert!(
        out.is_empty(),
        "the enormous command name is excluded and 'done' is a known word: {out:?}"
    );
}

// ---------------------------------------------------------------------
// 7. A user dictionary exactly at and past capacity.
// ---------------------------------------------------------------------

#[test]
fn user_dictionary_accepts_up_to_exactly_the_default_capacity_then_rejects() {
    let mut ud = UserDictionary::default();

    for i in 0..USER_DICTIONARY_DEFAULT_MAX_ENTRIES {
        ud.add_word(&format!("word{i}")).unwrap_or_else(|e| {
            panic!("entry {i} of {USER_DICTIONARY_DEFAULT_MAX_ENTRIES} must fit: {e}")
        });
    }
    assert_eq!(
        ud.len(),
        USER_DICTIONARY_DEFAULT_MAX_ENTRIES,
        "must be exactly at capacity, not over"
    );

    let err = ud
        .add_word("one_word_past_capacity")
        .expect_err("the entry immediately past capacity must be rejected");
    assert_eq!(
        err,
        UserDictionaryError::CapacityExceeded {
            max_entries: USER_DICTIONARY_DEFAULT_MAX_ENTRIES
        }
    );
    assert_eq!(
        ud.len(),
        USER_DICTIONARY_DEFAULT_MAX_ENTRIES,
        "a rejected entry must not be stored"
    );
}

#[test]
fn user_dictionary_capacity_is_shared_and_exact_at_small_bounds() {
    let mut ud = UserDictionary::new(3);
    ud.add_word("a").unwrap();
    ud.ignore_word("b").unwrap();
    ud.add_word("c").unwrap();
    assert_eq!(ud.len(), 3);

    // Exactly at capacity: further genuinely-new entries of either kind
    // are rejected with the exact same typed error.
    assert_eq!(
        ud.add_word("d").unwrap_err(),
        UserDictionaryError::CapacityExceeded { max_entries: 3 }
    );
    assert_eq!(
        ud.ignore_word("e").unwrap_err(),
        UserDictionaryError::CapacityExceeded { max_entries: 3 }
    );

    // Re-adding/re-ignoring an already-stored entry at capacity is still a
    // documented no-op success, not an error -- capacity governs distinct
    // entries, not calls.
    assert!(ud.add_word("a").is_ok());
    assert!(ud.ignore_word("b").is_ok());
    assert_eq!(ud.len(), 3);
}

#[test]
fn user_dictionary_zero_capacity_rejects_every_new_entry() {
    let mut ud = UserDictionary::new(0);
    assert_eq!(
        ud.add_word("anything").unwrap_err(),
        UserDictionaryError::CapacityExceeded { max_entries: 0 }
    );
    assert_eq!(
        ud.ignore_word("anything").unwrap_err(),
        UserDictionaryError::CapacityExceeded { max_entries: 0 }
    );
    assert!(ud.is_empty());
}

// ---------------------------------------------------------------------
// 8. A dictionary containing empty strings and very long entries.
// ---------------------------------------------------------------------

#[test]
fn dictionary_with_empty_string_entry_does_not_disrupt_checking() {
    let mut words: HashSet<String> = dict(&["hello", "world"]);
    words.insert(String::new());
    let checker = SpellChecker::default();

    // The tokenizer never emits an empty token, so an empty dictionary
    // entry has no visible effect either way -- the assertion is simply
    // that its presence causes no panic and normal behavior is unchanged.
    let out = checker.check("hello wrold", &words);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].word, "wrold");
}

#[test]
fn dictionary_with_very_long_entry_is_bounded_and_still_matches_exactly() {
    let huge_word = "z".repeat(2_000_000);
    let mut words: HashSet<String> = dict(&["hello"]);
    words.insert(huge_word.clone());
    let checker = SpellChecker::default();

    let text = format!("hello {huge_word} wrold");
    let out = assert_bounded("dictionary with a very long entry", || {
        checker.check(&text, &words)
    });

    // The huge word is itself a dictionary entry (direct hash lookup,
    // independent of its length) so it is not flagged; "wrold" still is.
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].word, "wrold");
}

#[test]
fn user_dictionary_accepts_empty_string_and_very_long_entries_without_panicking() {
    let mut ud = UserDictionary::new(10);
    assert!(ud.add_word("").is_ok());
    assert!(ud.is_addition(""));

    let huge = "q".repeat(1_000_000);
    assert!(ud.ignore_word(&huge).is_ok());
    assert!(ud.is_ignored(&huge));
    assert_eq!(ud.len(), 2);
}

// ---------------------------------------------------------------------
// 9. Text of only combining marks, zero-width characters, or RTL overrides.
// ---------------------------------------------------------------------

#[test]
fn text_of_only_combining_marks_is_bounded_and_flags_nothing() {
    let d = dict(&["anything"]);
    let checker = SpellChecker::default();
    // U+0301 COMBINING ACUTE ACCENT: not `Alphabetic`, so it can never
    // start or extend a word token on its own.
    let text = "\u{0301}".repeat(100_000);
    let out = assert_bounded("combining-marks-only text", || checker.check(&text, &d));
    assert!(
        out.is_empty(),
        "combining marks alone contain no word to flag: {out:?}"
    );
}

#[test]
fn text_of_only_zero_width_characters_is_bounded_and_flags_nothing() {
    let d = dict(&["anything"]);
    let checker = SpellChecker::default();
    // U+200B ZERO WIDTH SPACE, U+200D ZERO WIDTH JOINER: not `Alphabetic`.
    let text = "\u{200B}\u{200D}".repeat(100_000);
    let out = assert_bounded("zero-width-only text", || checker.check(&text, &d));
    assert!(
        out.is_empty(),
        "zero-width characters alone contain no word to flag: {out:?}"
    );
}

#[test]
fn text_of_only_rtl_overrides_is_bounded_and_flags_nothing() {
    let d = dict(&["anything"]);
    let checker = SpellChecker::default();
    // U+202E RIGHT-TO-LEFT OVERRIDE, U+202C POP DIRECTIONAL FORMATTING:
    // not `Alphabetic`.
    let text = "\u{202E}\u{202C}".repeat(100_000);
    let out = assert_bounded("RTL-override-only text", || checker.check(&text, &d));
    assert!(
        out.is_empty(),
        "directional-override characters alone contain no word to flag: {out:?}"
    );
}

#[test]
fn combining_marks_zero_width_and_rtl_overrides_mixed_with_words_stay_char_boundary_safe() {
    let d = dict(&["hello"]);
    let checker = SpellChecker::default();
    let text = "hello\u{0301} \u{200B}wrold\u{202E}\u{202C} \u{0301}\u{200B}\u{202E}";
    let out = checker.check(text, &d);
    for m in &out {
        assert!(text.is_char_boundary(m.range.start));
        assert!(text.is_char_boundary(m.range.end));
        assert_eq!(&text[m.range.clone()], m.word);
    }
}

// ---------------------------------------------------------------------
// 10. A cancellation callback that always returns true.
// ---------------------------------------------------------------------

#[test]
fn always_true_cancellation_callback_stops_after_exactly_one_poll_regardless_of_input_size() {
    let d = dict(&["hello"]);
    let checker = SpellChecker::default();
    // Large input: if the bound did not actually bind, an always-true
    // callback that still got polled per-word would be called many times
    // before the (irrelevant, since always true) loop even mattered.
    let words: Vec<String> = (0..50_000).map(|i| format!("wrold{i}")).collect();
    let text = words.join(" ");

    let calls = Cell::new(0usize);
    let is_cancelled = || {
        calls.set(calls.get() + 1);
        true
    };

    let outcome = assert_bounded("always-true cancellation over a large input", || {
        checker.check_cancellable(&text, 1, &d, &is_cancelled)
    });

    assert_eq!(outcome, CheckOutcome::Cancelled { revision: 1 });
    assert_eq!(
        calls.get(),
        1,
        "an always-true callback must short-circuit on the very first poll, before any per-word work, \
         proving the cancellation bound binds independent of input size (polled {} times)",
        calls.get()
    );
}

#[test]
fn always_true_cancellation_callback_never_yields_a_completed_outcome() {
    let d = dict(&["hello"]);
    let checker = SpellChecker::default();
    let long_text = "wrold ".repeat(1000);
    for text in ["", "hello", "hello wrold", long_text.as_str()] {
        let outcome = checker.check_cancellable(text, 7, &d, &|| true);
        assert_eq!(
            outcome,
            CheckOutcome::Cancelled { revision: 7 },
            "input: {text:?}"
        );
    }
}

// ---------------------------------------------------------------------
// Cross-cutting: none of the above configuration surfaces let a caller
// escape the hard suggestion-generation ceiling either.
// ---------------------------------------------------------------------

// ---------------------------------------------------------------------
// 11. A permissive max_word_length_for_suggestions combined with one long
//     misspelled word: edits1's O(alphabet_len * word_len^2) cost, with no
//     hard ceiling on word_len prior to the fix (unlike max_edit_distance's
//     MAX_ALLOWED_EDIT_DISTANCE).
// ---------------------------------------------------------------------

#[test]
fn permissive_max_word_length_for_suggestions_is_still_bounded_for_a_long_word() {
    // Before the fix, `max_word_length_for_suggestions` had no hard
    // ceiling, so a caller configuring it to `usize::MAX` combined with one
    // long misspelled word in the text let `edits1`'s
    // O(alphabet_len * word_len^2) cost run unchecked: a 3,000-character
    // word with this exact config took well over a second pre-fix (and a
    // 10,000-character word took far longer). It must now complete in well
    // under a second, since `SpellChecker::new` clamps
    // `max_word_length_for_suggestions` to
    // `SpellChecker::MAX_ALLOWED_WORD_LENGTH_FOR_SUGGESTIONS`.
    let d = dict(&["hello"]);
    let cfg = SpellCheckerConfig {
        max_edit_distance: 2,
        max_suggestions: 5,
        max_word_length_for_suggestions: usize::MAX,
    };
    let checker = SpellChecker::new(cfg);
    let long_word = "q".repeat(3_000);
    let text = format!("hello {long_word}");
    let counting = CountingDict::new(&d);

    let out = checker.check(&text, &counting);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].word, long_word);
    // Deterministic work bound instead of a wall-clock limit: two lookups
    // per token (as written and lowercased), plus at most one distance-1
    // pass over a word no longer than the hard ceiling
    // (MAX_ALLOWED_WORD_LENGTH_FOR_SUGGESTIONS = 64). Without the clamp the
    // 3,000-char word alone generates ~160,000 candidates.
    let lookups = counting.lookups.get();
    let bound = 2 * 2 + edits1_candidates(SpellChecker::MAX_ALLOWED_WORD_LENGTH_FOR_SUGGESTIONS);
    assert!(
        lookups <= bound,
        "a long word with a permissive max_word_length_for_suggestions made {lookups} dictionary \
         lookups (bound {bound}); max_word_length_for_suggestions must be clamped to a hard ceiling"
    );
}

// ---------------------------------------------------------------------
// 12. Many DISTINCT words that never match the dictionary: MAX_RAW_CANDIDATES
//     bounds any one word's suggestion search, but (prior to the fix)
//     nothing bounded the aggregate cost across many distinct words in one
//     document, since suggestion cost is paid once per distinct word.
// ---------------------------------------------------------------------

/// Deterministic, purely-alphabetic, distinct token for index `i`: a fixed
/// prefix (so it can never coincidentally match the empty dictionary below)
/// plus a base-26 encoding of `i`. Not randomized -- the same 300 words are
/// produced on every run.
fn distinct_never_matching_word(i: usize) -> String {
    let mut n = i;
    let mut suffix = String::new();
    loop {
        let digit = (n % 26) as u8;
        suffix.push((b'a' + digit) as char);
        n /= 26;
        if n == 0 {
            break;
        }
    }
    format!("zzzznever{suffix}")
}

#[test]
fn many_distinct_never_matching_words_do_not_blow_up_aggregate_suggestion_cost() {
    // Before the fix, each of these 300 distinct words independently paid
    // up to MAX_RAW_CANDIDATES worth of distance-2 candidate generation
    // (distance-1 finds nothing against an empty dictionary, and none of
    // them repeat, so the per-word suggestion_cache never helps): total
    // cost scaled linearly in the number of *distinct* misspelled words in
    // the document, with no aggregate ceiling, and took several seconds for
    // just this many. It must now stay fast because suggestion-generation
    // work is budgeted once per `check()` call
    // (MAX_TOTAL_SUGGESTION_WORK_PER_CHECK), not once per word.
    let d: HashSet<String> = HashSet::new();
    let checker = SpellChecker::default();

    // Deterministic work counter instead of a wall-clock limit: count
    // dictionary lookups, one per generated candidate. The whole-check
    // budget is 300,000 raw candidates (MAX_TOTAL_SUGGESTION_WORK_PER_CHECK),
    // plus two lookups per token for the word itself. Per-word budgeting
    // would instead cost up to MAX_RAW_CANDIDATES (50,000) per distinct
    // word: ~15M lookups for 300 words, ~60M for 1,200.
    const AGGREGATE_WORK_CEILING: usize = 300_000;
    let lookups_for = |n: usize| {
        let text = (0..n)
            .map(distinct_never_matching_word)
            .collect::<Vec<_>>()
            .join(" ");
        let counting = CountingDict::new(&d);
        let out = checker.check(&text, &counting);
        assert_eq!(
            out.len(),
            n,
            "every distinct word must still be reported as a misspelling"
        );
        counting.lookups.get()
    };

    for n in [300, 1_200] {
        let lookups = lookups_for(n);
        let bound = AGGREGATE_WORK_CEILING + 2 * n;
        assert!(
            lookups <= bound,
            "{n} distinct never-matching words made {lookups} dictionary lookups (bound {bound}); \
             aggregate suggestion-generation work must be budgeted per check() call, not per word"
        );
    }
}

#[test]
fn max_suggestions_and_edit_distance_extremes_stay_bounded_together() {
    // Every letter except 'x' gets a dictionary word of the form "c_t", so
    // "cxt" itself is deliberately left out of the dictionary: it must
    // stay a genuine misspelling (one substitution away from all 25
    // entries below) rather than accidentally being a dictionary word
    // itself, which checking it against the full a..=z set would make it.
    let mut words: Vec<String> = ('a'..='z')
        .filter(|&c| c != 'x')
        .map(|c| format!("c{c}t"))
        .collect();
    for i in 0..20_000 {
        words.push(format!("unrelated{i}"));
    }
    let d: HashSet<String> = words.into_iter().collect();

    let cfg = SpellCheckerConfig {
        max_edit_distance: usize::MAX,
        max_suggestions: usize::MAX,
        max_word_length_for_suggestions: 20,
    };
    let checker = SpellChecker::new(cfg);
    assert_eq!(
        checker.config().max_edit_distance,
        SpellChecker::MAX_ALLOWED_EDIT_DISTANCE
    );

    let out = assert_bounded("extreme config against a large dictionary", || {
        checker.check("cxt", &d)
    });
    assert_eq!(out.len(), 1);
    // 25 single-substitution candidates exist ("cat".."czt", skipping
    // "cxt" itself); an unbounded `max_suggestions` must still only return
    // what was actually found and ranked, not panic or allocate unbounded
    // output.
    assert!(out[0].suggestions.len() <= 25);
}
