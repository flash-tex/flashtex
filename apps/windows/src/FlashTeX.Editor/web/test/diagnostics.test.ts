// name: diagnostics.test.ts
// purpose: Tests for diagnostics.ts's StateField, verifying diagnostics
//   addressed by UTF-8 byte range (as bridge.ts converts a `set_diagnostics`
//   message) land as CM6 decorations at the correct UTF-16 position in a
//   multi-byte-UTF-8 document, and that severities are styled distinctly.
// author: Claude Sonnet 5
// date: 2026-09-14

import { describe, expect, it } from "vitest";
import { EditorState } from "@codemirror/state";
import type { Diagnostic } from "../src/bridge.js";
import { utf16RangeForUtf8Bytes } from "../src/byte-offsets.js";
import { diagnostics, diagnosticsField, setDiagnosticsEffect } from "../src/diagnostics.js";

function utf8ByteCount(text: string): number {
  return new TextEncoder().encode(text).length;
}

interface FoundDecoration {
  readonly from: number;
  readonly to: number;
  readonly className: string | null;
  readonly title: string | null;
}

function collectDecorations(state: EditorState): FoundDecoration[] {
  const found: FoundDecoration[] = [];
  state.field(diagnosticsField).between(0, state.doc.length, (from, to, deco) => {
    const spec = deco.spec as { class?: string; attributes?: { title?: string } };
    found.push({ from, to, className: spec.class ?? null, title: spec.attributes?.title ?? null });
  });
  return found;
}

function diagnosticForUtf16Range(text: string, utf16Start: number, utf16End: number, rest: Omit<Diagnostic, "range">): Diagnostic {
  const startByte = utf8ByteCount(text.slice(0, utf16Start));
  const endByte = utf8ByteCount(text.slice(0, utf16End));
  const range = utf16RangeForUtf8Bytes(text, startByte, endByte);
  if (!range) {
    throw new Error("test fixture produced an invalid byte range");
  }
  return { range, ...rest };
}

describe("diagnostics: UTF-8 byte range -> UTF-16 decoration position", () => {
  it("places a squiggle at the correct UTF-16 offsets for a diagnostic addressed by UTF-8 byte range in multi-byte content", () => {
    const text = "héllo 🎉 wörld";
    const target = "wörld";
    const utf16Start = text.indexOf(target);
    const utf16End = utf16Start + target.length;
    // Sanity: this fixture must actually exercise UTF-8/UTF-16 divergence,
    // otherwise the test wouldn't prove the byte->UTF-16 conversion at all.
    expect(utf8ByteCount(text.slice(0, utf16Start))).not.toBe(utf16Start);

    const diagnostic = diagnosticForUtf16Range(text, utf16Start, utf16End, {
      severity: "error",
      message: "bad word",
      code: undefined,
    });

    const state0 = EditorState.create({ doc: text, extensions: [diagnostics()] });
    const state1 = state0.update({ effects: setDiagnosticsEffect.of([diagnostic]) }).state;

    expect(collectDecorations(state1)).toEqual([
      { from: utf16Start, to: utf16End, className: "cm-flashtex-diagnostic-error", title: "bad word" },
    ]);
  });

  it("styles error/warning/info diagnostics with distinct classes and includes the code in the tooltip", () => {
    const text = "héllo 🎉 wörld";
    const errorAt = diagnosticForUtf16Range(text, 0, 1, { severity: "error", message: "e", code: undefined });
    const warningAt = diagnosticForUtf16Range(text, 6, 8, { severity: "warning", message: "w", code: "W1" });
    const infoAt = diagnosticForUtf16Range(text, 9, 10, { severity: "info", message: "i", code: undefined });

    const state = EditorState.create({ doc: text, extensions: [diagnostics()] }).update({
      effects: setDiagnosticsEffect.of([warningAt, errorAt, infoAt]), // deliberately out of position order
    }).state;

    const found = collectDecorations(state);
    expect(found.map((d) => d.className)).toEqual([
      "cm-flashtex-diagnostic-error",
      "cm-flashtex-diagnostic-warning",
      "cm-flashtex-diagnostic-info",
    ]);
    // Sorted by position even though the effect's array wasn't.
    expect(found.map((d) => d.from)).toEqual([0, 6, 9]);
    expect(found.find((d) => d.className === "cm-flashtex-diagnostic-warning")?.title).toBe("w (W1)");
  });

  it("drops a zero-length or reversed range instead of throwing", () => {
    const text = "abc";
    const zeroLength: Diagnostic = { range: { utf16Start: 1, utf16End: 1 }, severity: "error", message: "x", code: undefined };
    const state = EditorState.create({ doc: text, extensions: [diagnostics()] }).update({
      effects: setDiagnosticsEffect.of([zeroLength]),
    }).state;
    expect(collectDecorations(state)).toEqual([]);
  });

  it("clamps a diagnostic range that runs past the current document length", () => {
    const text = "abc";
    const tooLong: Diagnostic = { range: { utf16Start: 1, utf16End: 999 }, severity: "warning", message: "x", code: undefined };
    const state = EditorState.create({ doc: text, extensions: [diagnostics()] }).update({
      effects: setDiagnosticsEffect.of([tooLong]),
    }).state;
    expect(collectDecorations(state)).toEqual([
      { from: 1, to: 3, className: "cm-flashtex-diagnostic-warning", title: "x" },
    ]);
  });

  it("replaces the previous diagnostic set entirely on a new setDiagnosticsEffect", () => {
    const text = "abcdef";
    const first: Diagnostic = { range: { utf16Start: 0, utf16End: 1 }, severity: "error", message: "first", code: undefined };
    const second: Diagnostic = { range: { utf16Start: 3, utf16End: 4 }, severity: "info", message: "second", code: undefined };

    let state = EditorState.create({ doc: text, extensions: [diagnostics()] });
    state = state.update({ effects: setDiagnosticsEffect.of([first]) }).state;
    expect(collectDecorations(state)).toHaveLength(1);

    state = state.update({ effects: setDiagnosticsEffect.of([second]) }).state;
    expect(collectDecorations(state)).toEqual([{ from: 3, to: 4, className: "cm-flashtex-diagnostic-info", title: "second" }]);
  });

  it("remaps decoration positions across an unrelated document edit", () => {
    const text = "abcdef";
    const diagnostic: Diagnostic = { range: { utf16Start: 3, utf16End: 5 }, severity: "error", message: "de", code: undefined };
    let state = EditorState.create({ doc: text, extensions: [diagnostics()] });
    state = state.update({ effects: setDiagnosticsEffect.of([diagnostic]) }).state;
    // Insert "XY" at the very start; the diagnostic's range should shift by 2.
    state = state.update({ changes: { from: 0, to: 0, insert: "XY" } }).state;
    expect(collectDecorations(state)).toEqual([{ from: 5, to: 7, className: "cm-flashtex-diagnostic-error", title: "de" }]);
  });
});
