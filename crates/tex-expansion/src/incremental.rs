//! Incremental re-expansion for the IDE.
//!
//! `Engine` snapshots its whole assignment state (`expand::State`: the
//! save stack with every macro/catcode/register, the conditional stack,
//! counter reset lists, ...) at *safe points* -- moments where the input
//! stack holds nothing but the base lexer, which has just consumed an
//! end-of-line, and no prefix/argument/definition scan is in flight. A
//! snapshot taken with the lexer at byte `P` depends only on source bytes
//! `<= P`, so after an edit at byte `E > P` expansion can resume from it
//! over the edited buffer and produce exactly what a from-scratch
//! expansion would.
//!
//! Two things keep a keystroke cheap:
//!
//! 1. **Restart from the nearest earlier checkpoint** (taken every
//!    `checkpoint_interval` bytes of source at safe points), so the
//!    prefix of the output before it is reused verbatim.
//! 2. **Convergence with the old run.** While re-expanding, whenever the
//!    engine reaches a safe point at the shifted position of one of the
//!    previous run's checkpoints (past the edit), the two states are
//!    compared modulo the span shift. If they are equivalent, everything
//!    the previous run produced after that checkpoint is reused with its
//!    spans shifted, and re-expansion stops. In the common case (an edit
//!    inside one paragraph that does not change any macro) the work is
//!    bounded by the distance between two checkpoints, not by the size of
//!    the document.
//!
//! `tests/incremental_tests.rs` proves `edit()` == `expand_str()` over
//! random edits of the oracle corpus and the HW1/HW2 fixtures;
//! `examples/bench_incremental.rs` measures keystroke latency on a 500 KB
//! synthetic document.

use std::rc::Rc;

use crate::error::{Diagnostic, Limits};

/// One edit's span mapping (see `Converge::shift_span`), kept on checkpoints
/// that were carried past an edit so their states are shifted only when used.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shift {
    edit_start: usize,
    old_edit_end: usize,
    delta: isize,
}

impl Shift {
    fn apply(&self, s: Span) -> Option<Span> {
        if s.is_synthetic() || s.source_id != 0 {
            return Some(s);
        }
        let (start, end) = (s.start as usize, s.end as usize);
        if end <= self.edit_start && start < self.edit_start {
            Some(s)
        } else if start >= self.old_edit_end {
            Some(Span::new(0, (start as isize + self.delta) as u32, (end as isize + self.delta) as u32))
        } else if start == end && start <= self.edit_start {
            Some(s)
        } else {
            None
        }
    }
}

/// Apply every pending shift, oldest first.
fn apply_all(s: Span, pending: &[Shift]) -> Option<Span> {
    pending.iter().try_fold(s, |s, shift| shift.apply(s))
}

/// A shift leaves every span with `end <= edit_start` unchanged, so a chain
/// of shifts is the identity on spans ending at or before the smallest edit
/// start among them (`u32::MAX` for no shifts).
fn identity_bound(pending: &[Shift]) -> u32 {
    pending.iter().map(|s| s.edit_start.min(u32::MAX as usize) as u32).min().unwrap_or(u32::MAX)
}
use crate::expand::{Checkpoint, Engine, LabelRecord, State};
use crate::span::Span;
use crate::token::Token;

/// One text edit: replace bytes `[start, end)` of the current source with
/// `replacement`. Offsets are byte offsets on UTF-8 boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

/// What one `edit()` call had to do (for tests/benchmarks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EditStats {
    /// Byte position of the checkpoint expansion restarted from (0 = the
    /// document start).
    pub restarted_from: usize,
    /// Byte position (in the new source) where the re-run converged with
    /// the previous run and its suffix was reused, if it did.
    pub converged_at: Option<usize>,
    /// Output tokens reused from before the restart checkpoint.
    pub prefix_reused: usize,
    /// Output tokens reused from the previous run after convergence.
    pub suffix_reused: usize,
    /// Output tokens produced by actually running the engine.
    pub tokens_expanded: usize,
    /// Expansion steps the engine performed for this edit.
    pub steps: u64,
}

pub struct IncrementalExpander {
    source: String,
    tokens: Vec<Token>,
    /// Invocation origin of each output token
    /// (`Engine::next_content_token_with_origin`), parallel to `tokens`.
    origins: Vec<Option<Span>>,
    /// Host configuration applied to the fresh engine before checkpoint 0
    /// (host commands, host prelude, state options). Everything it sets must
    /// live in the checkpointed assignment state.
    init: Option<Rc<dyn Fn(&mut Engine)>>,
    diagnostics: Vec<Diagnostic>,
    labels: Vec<LabelRecord>,
    checkpoints: Vec<Checkpoint>,
    /// Pending span shifts per checkpoint (parallel to `checkpoints`): the
    /// checkpoint's state is valid for the current buffer once these are
    /// applied to it, oldest first.
    pending: Vec<Vec<Shift>>,
    limits: Limits,
    checkpoint_interval: usize,
    /// How the last run ended (see [`RunEnd`]).
    end: RunEnd,
}

/// How a run ended. The step limit and the output token limit count from
/// the document start, unlike everything else in a checkpoint, so whether
/// a reused suffix still ends the same way depends on these totals.
#[derive(Debug, Clone, Copy, Default)]
struct RunEnd {
    /// `Engine::steps` when the run ended.
    steps: u64,
    /// The run stopped on the step limit.
    step_limit: bool,
    /// The run stopped on the output token limit.
    output_limit: bool,
    /// The run stopped on the main-memory limit (`max_output_tokens`).
    memory_limit: bool,
    /// At least `Engine::peak_memory` when the run ended (an upper bound
    /// once suffixes were reused).
    peak_memory: u64,
    /// `Engine::input_position` when the run ended.
    input: Option<(u32, usize)>,
}

impl RunEnd {
    fn of(engine: &Engine, output_limit: bool) -> Self {
        RunEnd {
            steps: engine.steps(),
            step_limit: engine.hit_step_limit(),
            output_limit,
            memory_limit: engine.hit_memory_limit(),
            peak_memory: engine.peak_memory(),
            input: engine.input_position(),
        }
    }
}

impl IncrementalExpander {
    pub fn new(source: &str) -> Self {
        Self::with_options(source, Limits::default(), 2048)
    }

    /// `checkpoint_interval`: minimum number of source bytes between two
    /// checkpoints (a checkpoint costs one clone of the assignment state).
    pub fn with_options(source: &str, limits: Limits, checkpoint_interval: usize) -> Self {
        Self::build(source, limits, checkpoint_interval, None)
    }

    /// [`IncrementalExpander::with_options`], with `init` run on the fresh
    /// engine before anything is read (and before checkpoint 0). `init` must
    /// only change checkpointed state (`declare_host_command`,
    /// `run_host_prelude`, `set_emit_unbalanced_close`, `declare_font_switch`,
    /// `declare_host_assignment` or `set_font_metrics`),
    /// since restored engines never see it again.
    pub fn with_host(source: &str, limits: Limits, checkpoint_interval: usize, init: Rc<dyn Fn(&mut Engine)>) -> Self {
        Self::build(source, limits, checkpoint_interval, Some(init))
    }

    fn build(source: &str, limits: Limits, checkpoint_interval: usize, init: Option<Rc<dyn Fn(&mut Engine)>>) -> Self {
        let mut me = IncrementalExpander {
            source: source.to_string(),
            tokens: Vec::new(),
            origins: Vec::new(),
            init,
            diagnostics: Vec::new(),
            labels: Vec::new(),
            checkpoints: Vec::new(),
            pending: Vec::new(),
            limits,
            checkpoint_interval: checkpoint_interval.max(1),
            end: RunEnd::default(),
        };
        me.full_run();
        me
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Invocation origins, parallel to [`IncrementalExpander::tokens`].
    pub fn origins(&self) -> &[Option<Span>] {
        &self.origins
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn labels(&self) -> &[LabelRecord] {
        &self.labels
    }

    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// `Engine::input_position` where the last run ended: where a full run
    /// of the current source stands when it stops.
    pub fn input_position(&self) -> Option<(u32, usize)> {
        self.end.input
    }

    fn full_run(&mut self) {
        let src: Rc<str> = Rc::from(self.source.as_str());
        let mut engine = Engine::with_limits(&self.source, self.limits);
        if let Some(init) = self.init.clone() {
            init(&mut engine);
        }
        let _ = src;
        self.tokens.clear();
        self.origins.clear();
        self.diagnostics.clear();
        self.labels.clear();
        self.checkpoints.clear();
        self.pending.clear();
        self.checkpoints.push(engine.snapshot(0));
        self.pending.push(Vec::new());
        let mut last_cp = 0usize;
        let output_limit = self.drive(&mut engine, &mut last_cp, None).is_err();
        self.end = RunEnd::of(&engine, output_limit);
        self.diagnostics = engine.take_diagnostics();
        self.labels = engine.take_labels();
    }

    /// Run `engine` to the end (or until it converges with an old
    /// checkpoint, when `converge` is given), appending output tokens and
    /// recording new checkpoints. Returns the convergence point if any, or
    /// `Err(())` if the run stopped on the output token limit.
    fn drive(&mut self, engine: &mut Engine, last_cp: &mut usize, converge: Option<&mut Converge>) -> Result<Option<usize>, ()> {
        // (The old checkpoints' pending shifts travel inside `Converge`.)
        let mut converge = converge;
        // The engine's diagnostics/labels vectors start empty after a
        // restore, so checkpoint counts must be offset to absolute
        // positions in `self.diagnostics`/`self.labels`.
        let diag_base = self.diagnostics.len();
        let label_base = self.labels.len();
        loop {
            let (tok, origin) = match engine.next_content_token_with_origin() {
                Some(t) => t,
                None => return Ok(None),
            };
            self.tokens.push(tok);
            self.origins.push(origin);
            if self.tokens.len() as u64 > self.limits.max_output_tokens {
                engine.push_diagnostic(Diagnostic::error(
                    crate::error::output_limit_message(self.limits.max_output_tokens),
                    Span::synthetic(),
                ));
                return Err(());
            }
            if let Some(pos) = engine.safe_point() {
                if let Some(c) = converge.as_deref_mut() {
                    if pos >= c.new_edit_end {
                        if let Some(old_idx) = c.candidate_at(pos) {
                            let old = &c.old_checkpoints[old_idx];
                            if old.lex_state == engine.lex_state()
                                && c.same_end(old, engine.steps(), self.tokens.len(), &self.limits)
                                && c.map_old(old.last_origin, old_idx) == Some(engine.last_origin())
                                && states_equivalent(&old.state, &c.old_pending[old_idx], engine.state(), c)
                            {
                                return Ok(Some(old_idx));
                            }
                        }
                    }
                }
                if pos - *last_cp >= self.checkpoint_interval {
                    let mut cp = engine.snapshot(self.tokens.len());
                    cp.diag_len += diag_base;
                    cp.label_len += label_base;
                    self.checkpoints.push(cp);
                    self.pending.push(Vec::new());
                    *last_cp = pos;
                }
            }
        }
    }

    /// Apply one edit and bring `tokens()`/`diagnostics()`/`labels()` up
    /// to date, re-expanding as little as the checkpoints allow.
    pub fn edit(&mut self, edit: &Edit) -> EditStats {
        self.edit_with_limits(edit, self.limits)
    }

    /// [`IncrementalExpander::edit`], with the edited source expanded under
    /// `limits` (a host whose limits grow with the document).
    ///
    /// Every use of the step and output token limits is a stop once a count
    /// goes past one, so a checkpoint is reused only if its step count,
    /// output count and peak main-memory size are within the new limits (the
    /// run under them got there without stopping), and a suffix only if the
    /// new run ends it as `Converge::same_end` requires. A change to the
    /// other limits re-expands the document.
    pub fn edit_with_limits(&mut self, edit: &Edit, limits: Limits) -> EditStats {
        let Edit { start, end, replacement } = edit;
        let (start, end) = (*start, *end);
        assert!(start <= end && end <= self.source.len(), "edit range out of bounds");
        assert!(self.source.is_char_boundary(start) && self.source.is_char_boundary(end), "edit not on char boundary");
        let old_limits = std::mem::replace(&mut self.limits, limits);
        let usable = |cp: &Checkpoint| {
            (cp.pos == 0 || cp.pos < start)
                && cp.steps <= limits.max_expansion_steps
                && cp.out_len as u64 <= limits.max_output_tokens
                && cp.peak_memory <= limits.max_output_tokens
        };
        let same_depth_limits = (limits.max_conditional_depth, limits.max_group_depth)
            == (old_limits.max_conditional_depth, old_limits.max_group_depth);
        let cp_idx = match self.checkpoints.iter().rposition(usable) {
            Some(index) if same_depth_limits => index,
            _ => {
                self.source.replace_range(start..end, replacement);
                self.full_run();
                return EditStats { tokens_expanded: self.tokens.len(), steps: self.end.steps, ..EditStats::default() };
            }
        };
        let old_len = self.source.len();
        let delta = replacement.len() as isize - (end - start) as isize;
        // Some TeX messages embed a source line number ("... after line N");
        // reusing old diagnostics after convergence only shifts their spans,
        // so an edit that changes the line count must not converge while any
        // such diagnostic would be reused.
        let lines_changed = self.source[start..end].matches('\n').count() != replacement.matches('\n').count();
        self.source.replace_range(start..end, replacement);
        let new_edit_end = start + replacement.len();

        // 1. Nearest checkpoint strictly before the edit (the lexer at
        //    P has looked at byte P to end the previous token, so P must
        //    itself be unchanged: P < start). The initial checkpoint at
        //    0 has looked at nothing and is always usable.
        let mut cp = self.checkpoints[cp_idx].clone();
        if !self.pending[cp_idx].is_empty() {
            // Spans in a checkpoint's state all precede its position, which
            // precedes this edit; earlier edits may still need applying.
            let pending = std::mem::take(&mut self.pending[cp_idx]);
            match cp.state.map_spans(&|sp| apply_all(sp, &pending), identity_bound(&pending)) {
                Some(st) => {
                    cp.state = st;
                    cp.last_origin = cp.last_origin.map(|o| apply_all(o, &pending).expect("an origin before the checkpoint precedes the edit"));
                    self.checkpoints[cp_idx].state = cp.state.clone();
                    self.checkpoints[cp_idx].last_origin = cp.last_origin;
                }
                None => unreachable!("a checkpoint before an edit cannot overlap an earlier edit it survived"),
            }
        }
        let old_checkpoints: Vec<Checkpoint> = self.checkpoints.drain(cp_idx + 1..).collect();
        let old_pending: Vec<Vec<Shift>> = self.pending.drain(cp_idx + 1..).collect();
        // Moved, not cloned: the reused suffix is spliced back below.
        let mut old_tokens: Vec<Token> = self.tokens.split_off(cp.out_len);
        let mut old_origins: Vec<Option<Span>> = self.origins.split_off(cp.out_len);
        let old_diags: Vec<Diagnostic> = self.diagnostics.drain(cp.diag_len..).collect();
        let old_labels: Vec<LabelRecord> = self.labels.drain(cp.label_len..).collect();
        let prefix_reused = self.tokens.len();

        // 2. Re-expand from it over the new buffer.
        let mut engine = Engine::restore(Rc::from(self.source.as_str()), &cp, self.limits);
        let steps_before = engine.steps();
        let mut last_cp = cp.pos;
        let mut conv = Converge {
            old_checkpoints,
            old_pending,
            edit_start: start,
            old_edit_end: end,
            new_edit_end,
            delta,
            old_len,
            old_end: self.end,
            old_limits,
            old_out_len: prefix_reused + old_tokens.len(),
        };
        let line_sensitive = lines_changed && old_diags.iter().any(|d| d.message.contains("line "));
        let run = if line_sensitive {
            self.drive(&mut engine, &mut last_cp, None)
        } else {
            self.drive(&mut engine, &mut last_cp, Some(&mut conv))
        };
        let converged = run.unwrap_or(None);
        self.end = RunEnd::of(&engine, run.is_err());
        let tokens_expanded = self.tokens.len() - prefix_reused;
        let mut stats = EditStats {
            restarted_from: cp.pos,
            converged_at: None,
            prefix_reused,
            suffix_reused: 0,
            tokens_expanded,
            steps: engine.steps() - steps_before,
        };
        let new_diags = engine.take_diagnostics();
        self.diagnostics.extend(new_diags);
        self.labels.extend(engine.take_labels());

        // 3. Splice the old suffix back in if we converged.
        if let Some(old_idx) = converged {
            let old_cp = &conv.old_checkpoints[old_idx];
            let base_out = old_cp.out_len - cp.out_len;
            let base_diag = old_cp.diag_len - cp.diag_len;
            let base_label = old_cp.label_len - cp.label_len;
            // The runs are equivalent from here on, so they take the same
            // number of steps to the end (see `Converge::same_end`).
            let step_offset = engine.steps() as i128 - old_cp.steps as i128;
            // The old run's end lies past the convergence point, so after
            // the edit.
            let input = conv.old_end.input.map(|(source, pos)| match source {
                0 if pos >= end => (source, (pos as isize + delta) as usize),
                _ => (source, pos),
            });
            let engine_peak = engine.peak_memory();
            self.end = RunEnd {
                steps: (conv.old_end.steps as i128 + step_offset) as u64,
                peak_memory: conv.old_end.peak_memory.max(engine_peak),
                input,
                ..conv.old_end
            };
            stats.converged_at = Some((old_cp.pos as isize + delta) as usize);
            stats.suffix_reused = old_tokens.len() - base_out;
            let shift = conv.as_shift();
            self.tokens.extend(old_tokens.drain(base_out..).map(|mut t| {
                t.span = shift.apply(t.span).unwrap_or(t.span);
                t
            }));
            self.origins.extend(old_origins.drain(base_out..).map(|o| o.map(|o| shift.apply(o).unwrap_or(o))));
            for d in &old_diags[base_diag..] {
                let mut d = d.clone();
                d.span = conv.shift_span(d.span).unwrap_or(d.span);
                self.diagnostics.push(d);
            }
            for l in &old_labels[base_label..] {
                let mut l = l.clone();
                l.span = conv.shift_span(l.span).unwrap_or(l.span);
                self.labels.push(l);
            }
            // The new run's own checkpoints up to the convergence point
            // were already recorded by `drive`; keep the old run's
            // checkpoints after it, shifted (their states are valid for
            // the new buffer since the runs are equivalent from here on).
            let out_offset = self.tokens.len() as isize - (old_cp.out_len as isize + stats.suffix_reused as isize);
            let diag_offset = self.diagnostics.len() as isize - (old_diags.len() as isize - base_diag as isize) - old_cp.diag_len as isize;
            let label_offset = self.labels.len() as isize - (old_labels.len() as isize - base_label as isize) - old_cp.label_len as isize;
            let shift = conv.as_shift();
            let old_cps = std::mem::take(&mut conv.old_checkpoints);
            let old_pending = std::mem::take(&mut conv.old_pending);
            for (old, mut pending) in old_cps.into_iter().zip(old_pending).skip(old_idx) {
                let pos = (old.pos as isize + delta) as usize;
                if let Some(last) = self.checkpoints.last() {
                    if pos <= last.pos {
                        continue;
                    }
                }
                // The state is shifted when the checkpoint is next used
                // (restart or convergence), not here.
                pending.push(shift);
                self.checkpoints.push(Checkpoint {
                    pos,
                    lex_state: old.lex_state,
                    state: old.state,
                    steps: (old.steps as i128 + step_offset) as u64,
                    last_origin: old.last_origin,
                    peak_memory: old.peak_memory.max(engine_peak),
                    metrics: old.metrics,
                    out_len: (old.out_len as isize + out_offset) as usize,
                    diag_len: (old.diag_len as isize + diag_offset) as usize,
                    label_len: (old.label_len as isize + label_offset) as usize,
                });
                self.pending.push(pending);
            }
        }
        stats
    }
}

/// Bookkeeping for the convergence search during one edit.
pub(crate) struct Converge {
    old_checkpoints: Vec<Checkpoint>,
    old_pending: Vec<Vec<Shift>>,
    pub edit_start: usize,
    pub old_edit_end: usize,
    pub new_edit_end: usize,
    pub delta: isize,
    old_len: usize,
    /// How the previous run ended, under which limits, and how many tokens
    /// it output.
    old_end: RunEnd,
    old_limits: Limits,
    old_out_len: usize,
}

impl Converge {
    /// Index of the old checkpoint whose shifted position is exactly
    /// `new_pos`, if any.
    fn candidate_at(&self, new_pos: usize) -> Option<usize> {
        let old_pos = new_pos as isize - self.delta;
        if old_pos < 0 || old_pos as usize <= self.old_edit_end.min(self.old_len) && old_pos as usize <= self.old_edit_end {
            // Old checkpoints inside/before the edited region cannot be
            // convergence points.
            if old_pos < self.old_edit_end as isize {
                return None;
            }
        }
        self.old_checkpoints.iter().position(|cp| cp.pos as isize == old_pos)
    }

    /// Would the new run, in a state equivalent to old checkpoint `old` at
    /// `steps` steps and `out_len` output tokens, end as the old run did?
    /// Both limits count from the document start, so the old suffix is
    /// reused only if the new totals stay within them, or, if the old run
    /// stopped on one, the new run reaches that stop at the same count.
    ///
    /// A limit stops the run on the first count past it (`steps` and the
    /// output count go up one at a time), so a stop is reached at the same
    /// place when the new total there is the new limit plus one. Main-memory
    /// sizes are not counts: a memory stop is reused only under the same
    /// limit, and a suffix that did not stop only if its peak size fits.
    fn same_end(&self, old: &Checkpoint, steps: u64, out_len: usize, limits: &Limits) -> bool {
        let step_offset = steps as i128 - old.steps as i128;
        let out_offset = out_len as i128 - old.out_len as i128;
        let new_steps = self.old_end.steps as i128 + step_offset;
        let steps_ok = if self.old_end.step_limit {
            new_steps == limits.max_expansion_steps as i128 + 1
        } else {
            new_steps <= limits.max_expansion_steps as i128
        };
        let new_out = self.old_out_len as i128 + out_offset;
        let out_ok = if self.old_end.output_limit {
            new_out == limits.max_output_tokens as i128 + 1
        } else {
            new_out <= limits.max_output_tokens as i128
        };
        let memory_ok = if self.old_end.memory_limit {
            limits.max_output_tokens == self.old_limits.max_output_tokens
        } else {
            self.old_end.peak_memory <= limits.max_output_tokens
        };
        steps_ok && out_ok && memory_ok
    }

    /// An old checkpoint's span with its pending shifts and this edit's
    /// shift applied (`None` if it touches an edit).
    fn map_old(&self, s: Option<Span>, old_idx: usize) -> Option<Option<Span>> {
        match s {
            None => Some(None),
            Some(s) => apply_all(s, &self.old_pending[old_idx]).and_then(|s| self.shift_span(s)).map(Some),
        }
    }

    /// Map a span from the old buffer to the new one: unchanged before
    /// the edit, shifted after it, `None` if it touches the edited region.
    pub fn shift_span(&self, s: Span) -> Option<Span> {
        if s.is_synthetic() || s.source_id != 0 {
            return Some(s);
        }
        let (start, end) = (s.start as usize, s.end as usize);
        if end <= self.edit_start && start < self.edit_start {
            Some(s)
        } else if start >= self.old_edit_end {
            Some(Span::new(0, (start as isize + self.delta) as u32, (end as isize + self.delta) as u32))
        } else if start == end && start <= self.edit_start {
            Some(s)
        } else {
            None
        }
    }

    pub fn shift_state(&self, st: &State) -> Option<State> {
        st.map_spans(&|s| self.shift_span(s), identity_bound(&[self.as_shift()]))
    }

    fn as_shift(&self) -> Shift {
        Shift { edit_start: self.edit_start, old_edit_end: self.old_edit_end, delta: self.delta }
    }
}

/// Is `old` (an old run's checkpoint state, with its pending shifts and this
/// edit's shift applied) equal to the new run's `new`? Tables the two runs
/// still share and that hold no span the shifts move are skipped, so the cost
/// follows what the runs assigned since they diverged, not the state's size.
fn states_equivalent(old: &State, pending: &[Shift], new: &State, c: &Converge) -> bool {
    let bound = identity_bound(pending).min(identity_bound(&[c.as_shift()]));
    old.eq_mapped(new, &|s| apply_all(s, pending).and_then(|s| c.shift_span(s)), bound)
}
