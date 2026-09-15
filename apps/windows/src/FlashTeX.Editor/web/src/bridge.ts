// name: bridge.ts
// purpose: The native<->JS message protocol for the FlashTeX Windows editor
//   WebView2 pane. Defines the `{v, id, type, payload}` envelope and every
//   message shape in both directions, an injectable `HostTransport` seam
//   (a real WebView2-backed implementation plus a mock for tests), and the
//   `EditorBridge` that is the *only* place UTF-16 CodeMirror positions are
//   converted to/from the wire's UTF-8 byte offsets (via byte-offsets.ts) --
//   every byte offset that crosses the native<->JS boundary must go through
//   this module.
// author: Claude Sonnet 5
// date: 2026-09-14

import type { EditorView } from "@codemirror/view";
import type { Transaction } from "@codemirror/state";
import { utf16RangeForUtf8Bytes, utf8ByteRangeForUtf16Range, type Utf16Range } from "./byte-offsets.js";

// ---------------------------------------------------------------------------
// Envelope
// ---------------------------------------------------------------------------

/** The wire envelope every message (either direction) is wrapped in. */
export interface Envelope<Type extends string = string, Payload = unknown> {
  readonly v: 1;
  readonly id: string;
  readonly type: Type;
  readonly payload: Payload;
}

const PROTOCOL_VERSION = 1;

let nextMessageId = 0;

/** A per-process-unique id for an outgoing envelope (native replies, e.g. `completion_reply`, echo it back). */
function nextId(): string {
  nextMessageId += 1;
  return `js-${nextMessageId}`;
}

// ---------------------------------------------------------------------------
// Native -> JS payloads (wire shape: snake_case fields, UTF-8 byte offsets)
// ---------------------------------------------------------------------------

export type DiagnosticSeverity = "error" | "warning" | "info";

export interface SetDocumentWire {
  readonly text: string;
  readonly revision: number;
}

export interface ApplyEditWire {
  readonly start_byte: number;
  readonly end_byte: number;
  readonly replacement: string;
  readonly revision: number;
}

export interface DiagnosticWire {
  readonly start_byte: number;
  readonly end_byte: number;
  readonly severity: DiagnosticSeverity;
  readonly message: string;
  readonly code?: string;
}

export type SetDiagnosticsWire = readonly DiagnosticWire[];

export interface SetThemeWire {
  readonly theme: "light" | "dark";
}

export interface SetFontSizeWire {
  readonly font_size_px: number;
}

export interface CompletionItemWire {
  readonly label: string;
  readonly detail?: string;
  readonly insert_text?: string;
}

export interface CompletionReplyWire {
  readonly request_id: string;
  readonly items: readonly CompletionItemWire[];
}

export interface RevealRangeWire {
  readonly start_byte: number;
  readonly end_byte: number;
}

export type NativeToJsMessage =
  | Envelope<"set_document", SetDocumentWire>
  | Envelope<"apply_edit", ApplyEditWire>
  | Envelope<"set_diagnostics", SetDiagnosticsWire>
  | Envelope<"set_theme", SetThemeWire>
  | Envelope<"set_font_size", SetFontSizeWire>
  | Envelope<"completion_reply", CompletionReplyWire>
  | Envelope<"reveal_range", RevealRangeWire>;

// ---------------------------------------------------------------------------
// JS -> native payloads (wire shape: snake_case fields, UTF-8 byte offsets)
// ---------------------------------------------------------------------------

export interface EditMadeWire {
  readonly start_byte: number;
  readonly end_byte: number;
  readonly removed_text: string;
  readonly replacement: string;
  readonly new_revision: number;
}

export interface SelectionChangedWire {
  readonly anchor_byte: number;
  readonly head_byte: number;
}

export interface CaretMovedWire {
  readonly byte_offset: number;
}

export interface CompletionRequestWire {
  readonly position_byte: number;
  readonly prefix: string;
}

export interface SetCaretScreenRectWire {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

export type JsToNativeMessage =
  | Envelope<"edit_made", EditMadeWire>
  | Envelope<"selection_changed", SelectionChangedWire>
  | Envelope<"caret_moved", CaretMovedWire>
  | Envelope<"completion_request", CompletionRequestWire>
  | Envelope<"set_caret_screen_rect", SetCaretScreenRectWire>;

// ---------------------------------------------------------------------------
// Application-facing (UTF-16, camelCase) shapes -- what callers of
// EditorBridge actually work with; conversion to/from the wire shapes above
// happens only inside this module.
// ---------------------------------------------------------------------------

export interface Diagnostic {
  readonly range: Utf16Range;
  readonly severity: DiagnosticSeverity;
  readonly message: string;
  readonly code: string | undefined;
}

export interface CompletionItem {
  readonly label: string;
  readonly detail: string | undefined;
  readonly insertText: string | undefined;
}

/** Callbacks invoked for each native -> JS message `EditorBridge.listen` receives, with byte offsets already converted to UTF-16. */
export interface NativeMessageHandlers {
  onSetDocument?(text: string, revision: number): void;
  onApplyEdit?(range: Utf16Range, replacement: string, revision: number): void;
  onSetDiagnostics?(diagnostics: readonly Diagnostic[]): void;
  onSetTheme?(theme: "light" | "dark"): void;
  onSetFontSize?(fontSizePx: number): void;
  onCompletionReply?(requestId: string, items: readonly CompletionItem[]): void;
  onRevealRange?(range: Utf16Range): void;
  /** Called when a message's byte offsets don't land on a UTF-8/UTF-16 boundary of `getDocText()`, or the envelope is malformed. Never thrown past the transport's message handler. */
  onProtocolError?(error: Error, rawMessage: unknown): void;
}

// ---------------------------------------------------------------------------
// HostTransport: the injectable seam between this module and the actual host
// ---------------------------------------------------------------------------

/** The minimal surface EditorBridge needs from its host -- implemented for real by WebView2, and by a mock in tests. */
export interface HostTransport {
  postMessage(json: unknown): void;
  onMessage(handler: (json: unknown) => void): void;
}

interface WebViewLike {
  postMessage(json: unknown): void;
  addEventListener(type: "message", listener: (event: { data: unknown }) => void): void;
}

function getChromeWebview(): WebViewLike {
  if (typeof window === "undefined") {
    throw new Error("HostTransport: window.chrome.webview is unavailable -- not running inside a WebView2 host");
  }
  const host = window as unknown as { chrome?: { webview?: WebViewLike } };
  const webview = host.chrome?.webview;
  if (!webview) {
    throw new Error("HostTransport: window.chrome.webview is unavailable -- not running inside a WebView2 host");
  }
  return webview;
}

/** The real `HostTransport`, wrapping WebView2's `window.chrome.webview` JS API. */
export function createWebViewTransport(): HostTransport {
  return {
    postMessage(json: unknown): void {
      getChromeWebview().postMessage(json);
    },
    onMessage(handler: (json: unknown) => void): void {
      getChromeWebview().addEventListener("message", (event) => handler(event.data));
    },
  };
}

/** A `HostTransport` test double: records every outgoing message and lets a test inject an incoming one via `emitFromHost`. */
export interface MockTransport extends HostTransport {
  readonly sent: readonly unknown[];
  emitFromHost(json: unknown): void;
}

export function createMockTransport(): MockTransport {
  const sent: unknown[] = [];
  let handler: ((json: unknown) => void) | null = null;
  return {
    sent,
    postMessage(json: unknown): void {
      sent.push(json);
    },
    onMessage(nextHandler: (json: unknown) => void): void {
      handler = nextHandler;
    },
    emitFromHost(json: unknown): void {
      handler?.(json);
    },
  };
}

// ---------------------------------------------------------------------------
// EditorBridge
// ---------------------------------------------------------------------------

/**
 * Owns one `HostTransport` and is the sole boundary where CodeMirror's
 * UTF-16 code-unit positions are converted to/from the wire's UTF-8 byte
 * offsets. `getDocText` is called fresh on every incoming message (rather
 * than cached) since it must reflect whatever CodeMirror document state is
 * current at the moment the message actually arrives.
 */
export class EditorBridge {
  constructor(
    private readonly transport: HostTransport,
    private readonly getDocText: () => string,
  ) {}

  /**
   * Sends one `edit_made` message per change in `tr.changes`, converting
   * each change's `[fromA, toA)` UTF-16 range against the transaction's
   * *pre-edit* document (`tr.startState.doc`) into UTF-8 byte offsets.
   */
  notifyEdit(tr: Transaction, newRevision: number): void {
    const beforeText = tr.startState.doc.toString();
    tr.changes.iterChanges((fromA, toA, _fromB, _toB, inserted) => {
      const byteRange = utf8ByteRangeForUtf16Range(beforeText, fromA, toA);
      if (!byteRange) {
        throw new Error(`EditorBridge.notifyEdit: [${fromA}, ${toA}) is not a valid UTF-16 range of the pre-edit document`);
      }
      this.postJsToNative("edit_made", {
        start_byte: byteRange.startByte,
        end_byte: byteRange.endByte,
        removed_text: beforeText.slice(fromA, toA),
        replacement: inserted.toString(),
        new_revision: newRevision,
      });
    });
  }

  /** Sends `selection_changed` for the view's main selection range. */
  notifySelectionChanged(view: EditorView): void {
    const text = view.state.doc.toString();
    const { anchor, head } = view.state.selection.main;
    this.postJsToNative("selection_changed", {
      anchor_byte: this.requireByteOffset(text, anchor, "selection anchor"),
      head_byte: this.requireByteOffset(text, head, "selection head"),
    });
  }

  /** Sends `caret_moved` for a single UTF-16 caret position in the current document. */
  notifyCaretMoved(pos: number): void {
    const text = this.getDocText();
    this.postJsToNative("caret_moved", { byte_offset: this.requireByteOffset(text, pos, "caret position") });
  }

  /** Sends `completion_request` for a UTF-16 position plus the prefix already typed there. */
  requestCompletion(pos: number, prefix: string): void {
    const text = this.getDocText();
    this.postJsToNative("completion_request", {
      position_byte: this.requireByteOffset(text, pos, "completion position"),
      prefix,
    });
  }

  /** Sends `set_caret_screen_rect` (screen-space pixels; no byte offsets involved). */
  notifyCaretScreenRect(rect: SetCaretScreenRectWire): void {
    this.postJsToNative("set_caret_screen_rect", rect);
  }

  /** Registers the single handler for every native -> JS message on this bridge's transport. */
  listen(handlers: NativeMessageHandlers): void {
    this.transport.onMessage((raw) => {
      try {
        this.dispatch(raw, handlers);
      } catch (error) {
        handlers.onProtocolError?.(error instanceof Error ? error : new Error(String(error)), raw);
      }
    });
  }

  private dispatch(raw: unknown, handlers: NativeMessageHandlers): void {
    const envelope = asNativeEnvelope(raw);
    const text = this.getDocText();
    switch (envelope.type) {
      case "set_document":
        handlers.onSetDocument?.(envelope.payload.text, envelope.payload.revision);
        return;
      case "apply_edit": {
        const { start_byte, end_byte, replacement, revision } = envelope.payload;
        handlers.onApplyEdit?.(this.requireUtf16Range(text, start_byte, end_byte, "apply_edit"), replacement, revision);
        return;
      }
      case "set_diagnostics":
        handlers.onSetDiagnostics?.(
          envelope.payload.map((d) => ({
            range: this.requireUtf16Range(text, d.start_byte, d.end_byte, "set_diagnostics"),
            severity: d.severity,
            message: d.message,
            code: d.code,
          })),
        );
        return;
      case "set_theme":
        handlers.onSetTheme?.(envelope.payload.theme);
        return;
      case "set_font_size":
        handlers.onSetFontSize?.(envelope.payload.font_size_px);
        return;
      case "completion_reply":
        handlers.onCompletionReply?.(
          envelope.payload.request_id,
          envelope.payload.items.map((item) => ({
            label: item.label,
            detail: item.detail,
            insertText: item.insert_text,
          })),
        );
        return;
      case "reveal_range":
        handlers.onRevealRange?.(this.requireUtf16Range(text, envelope.payload.start_byte, envelope.payload.end_byte, "reveal_range"));
        return;
    }
  }

  private postJsToNative<M extends JsToNativeMessage>(type: M["type"], payload: M["payload"]): void {
    const envelope: Envelope<M["type"], M["payload"]> = { v: PROTOCOL_VERSION, id: nextId(), type, payload };
    this.transport.postMessage(envelope);
  }

  private requireByteOffset(text: string, utf16Pos: number, what: string): number {
    const range = utf8ByteRangeForUtf16Range(text, utf16Pos, utf16Pos);
    if (!range) {
      throw new Error(`EditorBridge: ${what} ${utf16Pos} is not a valid UTF-16 offset in the current document`);
    }
    return range.startByte;
  }

  private requireUtf16Range(text: string, startByte: number, endByte: number, what: string): Utf16Range {
    const range = utf16RangeForUtf8Bytes(text, startByte, endByte);
    if (!range) {
      throw new Error(`EditorBridge: ${what}'s byte range [${startByte}, ${endByte}) is not valid for the current document`);
    }
    return range;
  }
}

function asNativeEnvelope(raw: unknown): NativeToJsMessage {
  if (typeof raw !== "object" || raw === null) {
    throw new Error("EditorBridge: expected a JSON object envelope");
  }
  const candidate = raw as Partial<Envelope>;
  if (candidate.v !== PROTOCOL_VERSION) {
    throw new Error(`EditorBridge: unsupported envelope version ${String(candidate.v)}`);
  }
  if (typeof candidate.type !== "string") {
    throw new Error("EditorBridge: envelope is missing a string 'type'");
  }
  if (!NATIVE_TO_JS_TYPES.has(candidate.type)) {
    throw new Error(`EditorBridge: unknown native -> JS message type '${candidate.type}'`);
  }
  return raw as NativeToJsMessage;
}

const NATIVE_TO_JS_TYPES: ReadonlySet<string> = new Set<NativeToJsMessage["type"]>([
  "set_document",
  "apply_edit",
  "set_diagnostics",
  "set_theme",
  "set_font_size",
  "completion_reply",
  "reveal_range",
]);
