// name: diagnostics.ts
// purpose: Renders `Diagnostic[]` (as delivered by bridge.ts's `set_diagnostics`
//   dispatch, already converted from wire UTF-8 byte offsets to CodeMirror
//   UTF-16 positions) as CM6 `Decoration.mark` squiggles, styled differently
//   for error/warning/info. A `StateField<DecorationSet>` holding the current
//   diagnostics, updated via `setDiagnosticsEffect`.
// author: Claude Sonnet 5
// date: 2026-09-14

import { EditorState, StateEffect, StateField, type Extension, type Range } from "@codemirror/state";
import { Decoration, EditorView, type DecorationSet } from "@codemirror/view";
import type { Diagnostic } from "./bridge.js";

/** Dispatch this effect to replace the diagnostics currently displayed. */
export const setDiagnosticsEffect = StateEffect.define<readonly Diagnostic[]>();

const severityClass: Record<Diagnostic["severity"], string> = {
  error: "cm-flashtex-diagnostic-error",
  warning: "cm-flashtex-diagnostic-warning",
  info: "cm-flashtex-diagnostic-info",
};

/**
 * Builds one sorted, non-overlapping `Decoration.mark` range per diagnostic.
 * Ranges are clamped to the document's current bounds and dropped if they
 * end up empty or reversed after clamping -- defensive against a diagnostic
 * that was computed against a since-superseded document revision.
 */
function buildDecorations(diagnostics: readonly Diagnostic[], state: EditorState): DecorationSet {
  const docLength = state.doc.length;
  const ranges: Range<Decoration>[] = [];
  for (const diagnostic of diagnostics) {
    const from = Math.max(0, Math.min(diagnostic.range.utf16Start, docLength));
    const to = Math.max(0, Math.min(diagnostic.range.utf16End, docLength));
    if (from >= to) {
      continue;
    }
    const decoration = Decoration.mark({
      class: severityClass[diagnostic.severity],
      attributes: { title: diagnostic.code ? `${diagnostic.message} (${diagnostic.code})` : diagnostic.message },
    });
    ranges.push(decoration.range(from, to));
  }
  ranges.sort((a, b) => a.from - b.from || a.to - b.to);
  return Decoration.set(ranges, true);
}

/** Holds the decoration set for whatever diagnostics were last set via `setDiagnosticsEffect`. */
export const diagnosticsField = StateField.define<DecorationSet>({
  create(): DecorationSet {
    return Decoration.none;
  },
  update(decorations, tr) {
    let next = decorations.map(tr.changes);
    for (const effect of tr.effects) {
      if (effect.is(setDiagnosticsEffect)) {
        next = buildDecorations(effect.value, tr.state);
      }
    }
    return next;
  },
  provide: (field) => EditorView.decorations.from(field),
});

const diagnosticsBaseTheme = EditorView.baseTheme({
  ".cm-flashtex-diagnostic-error": {
    textDecoration: "underline wavy red",
    textDecorationSkipInk: "none",
  },
  ".cm-flashtex-diagnostic-warning": {
    textDecoration: "underline wavy orange",
    textDecorationSkipInk: "none",
  },
  ".cm-flashtex-diagnostic-info": {
    textDecoration: "underline dotted #6699cc",
    textDecorationSkipInk: "none",
  },
});

/** The full extension: install this once in the editor's extensions list. */
export function diagnostics(): Extension {
  return [diagnosticsField, diagnosticsBaseTheme];
}

/** Convenience wrapper: dispatches `setDiagnosticsEffect` against `view`'s current state. */
export function setDiagnostics(view: EditorView, diagnosticsList: readonly Diagnostic[]): void {
  view.dispatch({ effects: setDiagnosticsEffect.of(diagnosticsList) });
}
