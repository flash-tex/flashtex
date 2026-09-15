// name: bridge.test.ts
// purpose: Unit tests for the envelope/HostTransport plumbing in bridge.ts,
//   plus an integration test that drives a real CodeMirror `EditorState`
//   through a `Transaction` over multi-byte UTF-8 content and checks that
//   the resulting `edit_made` message carries correct UTF-8 byte offsets --
//   proving the UTF-16 (CodeMirror) -> UTF-8 byte (wire) conversion holds up
//   through actual CodeMirror machinery, not just byte-offsets.ts in isolation.
// author: Claude Sonnet 5
// date: 2026-09-14

import { describe, expect, it } from "vitest";
import { EditorState } from "@codemirror/state";
import {
  EditorBridge,
  createMockTransport,
  createWebViewTransport,
  type EditMadeWire,
  type Envelope,
  type NativeMessageHandlers,
} from "../src/bridge.js";

function utf8ByteCount(text: string): number {
  return new TextEncoder().encode(text).length;
}

describe("EditorBridge: outgoing edit_made (CodeMirror Transaction integration)", () => {
  it("converts a UTF-16 CodeMirror change range after multi-byte content into correct UTF-8 byte offsets", () => {
    // "é" is 1 UTF-16 unit / 2 UTF-8 bytes; "🎉" is a surrogate pair (2 UTF-16
    // units) / 4 UTF-8 bytes -- so byte offsets diverge from UTF-16 offsets
    // for everything at or after them.
    const initialText = "héllo 🎉 wörld";
    const state = EditorState.create({ doc: initialText });
    const target = "wörld";
    const fromA = initialText.indexOf(target);
    const toA = fromA + target.length;
    expect(fromA).toBeGreaterThan(0);

    const tr = state.update({ changes: { from: fromA, to: toA, insert: "erde" } });

    const transport = createMockTransport();
    const bridge = new EditorBridge(transport, () => tr.state.doc.toString());
    bridge.notifyEdit(tr, 42);

    expect(transport.sent).toHaveLength(1);
    const message = transport.sent[0] as Envelope<"edit_made", EditMadeWire>;
    expect(message.v).toBe(1);
    expect(message.type).toBe("edit_made");

    const expectedStartByte = utf8ByteCount(initialText.slice(0, fromA));
    const expectedEndByte = utf8ByteCount(initialText.slice(0, toA));
    // Sanity check the fixture actually exercises multi-byte divergence.
    expect(expectedStartByte).not.toBe(fromA);

    expect(message.payload).toEqual({
      start_byte: expectedStartByte,
      end_byte: expectedEndByte,
      removed_text: "wörld",
      replacement: "erde",
      new_revision: 42,
    });
  });

  it("converts a change positioned immediately after a surrogate-pair emoji", () => {
    const initialText = "🎉x";
    const state = EditorState.create({ doc: initialText });
    // Insert right between the emoji (2 UTF-16 units) and "x".
    const tr = state.update({ changes: { from: 2, to: 2, insert: "!" } });

    const transport = createMockTransport();
    const bridge = new EditorBridge(transport, () => tr.state.doc.toString());
    bridge.notifyEdit(tr, 1);

    const message = transport.sent[0] as Envelope<"edit_made", EditMadeWire>;
    // "🎉" is 4 UTF-8 bytes, so byte offset 4 (not UTF-16 offset 2) is where "!" lands.
    expect(message.payload.start_byte).toBe(4);
    expect(message.payload.end_byte).toBe(4);
    expect(message.payload.replacement).toBe("!");
  });

  it("sends one edit_made message per change when a transaction has multiple disjoint edits", () => {
    const initialText = "aaa bbb ccc";
    const state = EditorState.create({ doc: initialText });
    const tr = state.update({
      changes: [
        { from: 0, to: 3, insert: "XXX" },
        { from: 8, to: 11, insert: "ZZZ" },
      ],
    });

    const transport = createMockTransport();
    const bridge = new EditorBridge(transport, () => tr.state.doc.toString());
    bridge.notifyEdit(tr, 7);

    expect(transport.sent).toHaveLength(2);
    const [first, second] = transport.sent as [Envelope<"edit_made", EditMadeWire>, Envelope<"edit_made", EditMadeWire>];
    expect(first.payload.removed_text).toBe("aaa");
    expect(second.payload.removed_text).toBe("ccc");
    expect(first.payload.new_revision).toBe(7);
    expect(second.payload.new_revision).toBe(7);
  });

  it("throws a descriptive error rather than sending a message for an invalid UTF-16 range", () => {
    const transport = createMockTransport();
    const bridge = new EditorBridge(transport, () => "abc");
    expect(() => bridge.notifyCaretMoved(-1)).toThrow(/not a valid UTF-16 offset/);
    expect(transport.sent).toHaveLength(0);
  });
});

describe("EditorBridge: other outgoing messages", () => {
  it("notifyCaretMoved sends caret_moved with the byte offset for the current doc text", () => {
    const transport = createMockTransport();
    const bridge = new EditorBridge(transport, () => "héllo");
    bridge.notifyCaretMoved(2); // right after "hé" (1+2 = 3 bytes)
    expect(transport.sent).toEqual([expect.objectContaining({ type: "caret_moved", payload: { byte_offset: 3 } })]);
  });

  it("requestCompletion sends completion_request with position_byte and prefix", () => {
    const transport = createMockTransport();
    const bridge = new EditorBridge(transport, () => "héllo");
    bridge.requestCompletion(3, "l");
    expect(transport.sent).toEqual([
      expect.objectContaining({ type: "completion_request", payload: { position_byte: 4, prefix: "l" } }),
    ]);
  });

  it("notifyCaretScreenRect passes screen-space pixels through unchanged", () => {
    const transport = createMockTransport();
    const bridge = new EditorBridge(transport, () => "");
    bridge.notifyCaretScreenRect({ x: 10, y: 20, width: 2, height: 14 });
    expect(transport.sent).toEqual([
      expect.objectContaining({ type: "set_caret_screen_rect", payload: { x: 10, y: 20, width: 2, height: 14 } }),
    ]);
  });
});

describe("EditorBridge: incoming native -> JS messages", () => {
  interface RecordedCalls {
    onSetDocument: unknown[][];
    onApplyEdit: unknown[][];
    onSetDiagnostics: unknown[][];
    onSetTheme: unknown[][];
    onSetFontSize: unknown[][];
    onCompletionReply: unknown[][];
    onRevealRange: unknown[][];
    onProtocolError: unknown[][];
  }

  function listenAndCapture(getDocText: () => string, transport = createMockTransport()) {
    const bridge = new EditorBridge(transport, getDocText);
    const calls: RecordedCalls = {
      onSetDocument: [],
      onApplyEdit: [],
      onSetDiagnostics: [],
      onSetTheme: [],
      onSetFontSize: [],
      onCompletionReply: [],
      onRevealRange: [],
      onProtocolError: [],
    };
    const handlers: NativeMessageHandlers = {
      onSetDocument: (...args) => calls.onSetDocument.push(args),
      onApplyEdit: (...args) => calls.onApplyEdit.push(args),
      onSetDiagnostics: (...args) => calls.onSetDiagnostics.push(args),
      onSetTheme: (...args) => calls.onSetTheme.push(args),
      onSetFontSize: (...args) => calls.onSetFontSize.push(args),
      onCompletionReply: (...args) => calls.onCompletionReply.push(args),
      onRevealRange: (...args) => calls.onRevealRange.push(args),
      onProtocolError: (...args) => calls.onProtocolError.push(args),
    };
    bridge.listen(handlers);
    return { transport, calls };
  }

  it("dispatches set_document with text and revision untouched", () => {
    const { transport, calls } = listenAndCapture(() => "");
    transport.emitFromHost({ v: 1, id: "n-1", type: "set_document", payload: { text: "hello", revision: 3 } });
    expect(calls.onSetDocument).toEqual([["hello", 3]]);
  });

  it("dispatches apply_edit converting byte offsets to a UTF-16 range against the current doc", () => {
    const { transport, calls } = listenAndCapture(() => "héllo world");
    // "héllo " is 7 bytes (h,é=2,l,l,o,space) but 6 UTF-16 units.
    transport.emitFromHost({
      v: 1,
      id: "n-2",
      type: "apply_edit",
      payload: { start_byte: 7, end_byte: 12, replacement: "earth", revision: 9 },
    });
    expect(calls.onApplyEdit).toEqual([[{ utf16Start: 6, utf16End: 11 }, "earth", 9]]);
  });

  it("dispatches set_diagnostics converting every diagnostic's byte range", () => {
    const { transport, calls } = listenAndCapture(() => "héllo");
    transport.emitFromHost({
      v: 1,
      id: "n-3",
      type: "set_diagnostics",
      payload: [
        { start_byte: 0, end_byte: 1, severity: "error", message: "bad h" },
        { start_byte: 1, end_byte: 3, severity: "warning", message: "odd é", code: "W001" },
      ],
    });
    expect(calls.onSetDiagnostics).toEqual([
      [
        [
          { range: { utf16Start: 0, utf16End: 1 }, severity: "error", message: "bad h", code: undefined },
          { range: { utf16Start: 1, utf16End: 2 }, severity: "warning", message: "odd é", code: "W001" },
        ],
      ],
    ]);
  });

  it("dispatches set_theme and set_font_size unchanged", () => {
    const { transport, calls } = listenAndCapture(() => "");
    transport.emitFromHost({ v: 1, id: "n-4", type: "set_theme", payload: { theme: "dark" } });
    transport.emitFromHost({ v: 1, id: "n-5", type: "set_font_size", payload: { font_size_px: 16 } });
    expect(calls.onSetTheme).toEqual([["dark"]]);
    expect(calls.onSetFontSize).toEqual([[16]]);
  });

  it("dispatches completion_reply mapping insert_text to insertText", () => {
    const { transport, calls } = listenAndCapture(() => "");
    transport.emitFromHost({
      v: 1,
      id: "n-6",
      type: "completion_reply",
      payload: { request_id: "req-1", items: [{ label: "\\alpha", detail: "Greek alpha", insert_text: "\\alpha" }] },
    });
    expect(calls.onCompletionReply).toEqual([
      ["req-1", [{ label: "\\alpha", detail: "Greek alpha", insertText: "\\alpha" }]],
    ]);
  });

  it("dispatches reveal_range converting its byte range to UTF-16", () => {
    const { transport, calls } = listenAndCapture(() => "héllo");
    transport.emitFromHost({ v: 1, id: "n-7", type: "reveal_range", payload: { start_byte: 1, end_byte: 3 } });
    expect(calls.onRevealRange).toEqual([[{ utf16Start: 1, utf16End: 2 }]]);
  });

  it("routes a malformed envelope to onProtocolError instead of throwing", () => {
    const { transport, calls } = listenAndCapture(() => "");
    expect(() => transport.emitFromHost({ not: "an envelope" })).not.toThrow();
    expect(() => transport.emitFromHost({ v: 2, id: "x", type: "set_document", payload: {} })).not.toThrow();
    expect(() => transport.emitFromHost({ v: 1, id: "x", type: "not_a_real_type", payload: {} })).not.toThrow();
    expect(calls.onProtocolError).toHaveLength(3);
  });

  it("routes an out-of-range byte offset to onProtocolError instead of throwing", () => {
    const { transport, calls } = listenAndCapture(() => "abc");
    expect(() =>
      transport.emitFromHost({ v: 1, id: "n-8", type: "reveal_range", payload: { start_byte: 999, end_byte: 1000 } }),
    ).not.toThrow();
    expect(calls.onProtocolError).toHaveLength(1);
    expect(calls.onRevealRange).toHaveLength(0);
  });
});

describe("createWebViewTransport", () => {
  it("throws when window.chrome.webview is unavailable", () => {
    expect(() => createWebViewTransport().postMessage({})).toThrow(/window\.chrome\.webview/);
  });

  it("wraps window.chrome.webview.postMessage and the 'message' event when available", () => {
    const posted: unknown[] = [];
    let listener: ((event: { data: unknown }) => void) | null = null;
    const fakeWindow = {
      chrome: {
        webview: {
          postMessage(json: unknown) {
            posted.push(json);
          },
          addEventListener(type: string, cb: (event: { data: unknown }) => void) {
            expect(type).toBe("message");
            listener = cb;
          },
        },
      },
    };
    const globalWithWindow = globalThis as typeof globalThis & { window?: unknown };
    const original = globalWithWindow.window;
    globalWithWindow.window = fakeWindow as unknown as Window & typeof globalThis;
    try {
      const transport = createWebViewTransport();
      transport.postMessage({ hello: "native" });
      expect(posted).toEqual([{ hello: "native" }]);

      const received: unknown[] = [];
      transport.onMessage((json) => received.push(json));
      expect(listener).not.toBeNull();
      listener!({ data: { hello: "js" } });
      expect(received).toEqual([{ hello: "js" }]);
    } finally {
      globalWithWindow.window = original;
    }
  });
});
