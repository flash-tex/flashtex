//! Bookkeeping for TeX's conditional stack (TeXbook ch. 20, "conditional
//! processing"). The actual token-skipping logic lives in `expand.rs`
//! since it needs to pull raw tokens from the input; this module just
//! tracks nesting so `\else`/`\or`/`\fi` know what they're closing and
//! `\ifcase` can count branches.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfBranch {
    /// We are in the branch that was taken; a later `\else` for this level
    /// must be skipped to `\fi`, and a later `\fi` closes normally.
    Taken,
    /// We are actively skipping (this level's condition failed and we have
    /// not yet reached `\else`/the right `\or`).
    Skipping,
    /// The condition itself is still being scanned (tex.web `if_limit =
    /// if_code`): a `\fi`/`\else`/`\or` met now gets `\relax` inserted
    /// before it (§510).
    Testing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfShape {
    /// Plain two-way conditional (`\iftrue` style): `\if...\else...\fi`.
    TwoWay,
    /// `\ifcase` conditionals support `\or` in addition to `\else`/`\fi`.
    Case,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalFrame {
    pub shape: IfShape,
    pub branch: IfBranch,
    /// TeX's `cur_if` as printed by `print_cmd_chr` (e.g. `ifx`), for the
    /// "Incomplete \\if...; all text was ignored" message.
    pub name: &'static str,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConditionalStack {
    frames: Vec<ConditionalFrame>,
}

impl ConditionalStack {
    pub fn push(&mut self, shape: IfShape, branch: IfBranch, name: &'static str) {
        self.frames.push(ConditionalFrame { shape, branch, name });
    }

    pub fn pop(&mut self) -> Option<ConditionalFrame> {
        self.frames.pop()
    }

    pub fn top_mut(&mut self) -> Option<&mut ConditionalFrame> {
        self.frames.last_mut()
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}
