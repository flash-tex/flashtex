// name: main.ts
// purpose: Browser-side entry point for the WebView2 editor pane host page.
//   Builds the one active CodeMirror 6 `EditorView` (LaTeX grammar highlighting
//   + diagnostics squiggles from diagnostics.ts) and wires it to the native
//   <-> JS bridge (bridge.ts). This process shows exactly one document at a
//   time: switching tabs makes FlashTeX.App's EditorHost resend a full
//   `set_document` for the newly active path rather than this page holding a
//   `path -> EditorState` map itself, because the wire protocol (bridge.ts)
//   has no document/path identifier on any message -- see EditorHost.xaml.cs
//   and HANDOFF.md for why per-tab CM6 undo history is not preserved across a
//   tab switch yet (a deliberately deferred rough edge, not an oversight).
//
//   Completion and signature help are deliberately NOT rendered here: per the
//   Windows port's design (already reflected in FlashTeX.Editor/Completion.cs
//   and SignatureHelp.cs), those popups are native WinUI3 Flyouts positioned
//   from `set_caret_screen_rect`. This module only requests completions
//   (`completion_request`) and reports caret screen geometry; it never shows
//   its own popup UI for either.
// author: Claude Sonnet 5
// date: 2026-09-14

import { Annotation, Compartment, EditorState, type Extension } from "@codemirror/state";
import {
  EditorView,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightSpecialChars,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { defaultHighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { closeBrackets, closeBracketsKeymap } from "@codemirror/autocomplete";
import { latex } from "./latex-lang/index.js";
import { diagnostics, setDiagnostics } from "./diagnostics.js";
import { EditorBridge, createWebViewTransport } from "./bridge.js";

/** Marks a transaction as applying a native-originated change (`apply_edit`/`set_document`) so the update listener never echoes it back as `edit_made` -- otherwise every native edit would bounce back and forth forever. */
const fromNativeAnnotation = Annotation.define<boolean>();

const themeCompartment = new Compartment();
const fontSizeCompartment = new Compartment();

const DEFAULT_FONT_SIZE_PX = 13;

/** This page's own wire-message counter for `edit_made`'s `new_revision` field; independent of (and not required to match) ShellModel's shell-local `ShellDocument.Revision` counter on the native side -- that one is recomputed from the applied text, not read back from this field. */
let revisionCounter = 1;

function themeExtension(theme: "light" | "dark"): Extension {
  const dark = theme === "dark";
  return EditorView.theme(
    {
      "&": {
        color: dark ? "#d4d4d4" : "#1e1e1e",
        backgroundColor: dark ? "#1e1e1e" : "#ffffff",
        height: "100%",
      },
      ".cm-scroller": { fontFamily: "Cascadia Code, Consolas, ui-monospace, monospace" },
      ".cm-gutters": {
        backgroundColor: dark ? "#1e1e1e" : "#f3f3f3",
        color: dark ? "#6e7681" : "#237893",
        border: "none",
      },
    },
    { dark },
  );
}

function fontSizeExtension(fontSizePx: number): Extension {
  return EditorView.theme({ "&": { fontSize: `${fontSizePx}px` } });
}

function updateListener(bridge: EditorBridge) {
  return EditorView.updateListener.of((update) => {
    if (update.transactions.some((tr) => tr.annotation(fromNativeAnnotation))) {
      return;
    }
    for (const tr of update.transactions) {
      if (tr.docChanged) {
        revisionCounter += 1;
        bridge.notifyEdit(tr, revisionCounter);
      }
    }
    if (update.docChanged || update.selectionSet) {
      bridge.notifySelectionChanged(update.view);
      bridge.notifyCaretMoved(update.state.selection.main.head);
      sendCaretScreenRect(bridge, update.view);
      maybeRequestCompletion(bridge, update.view);
    }
  });
}

/** Sends the caret's client-area screen rect so the native host can anchor a completion/signature-help Flyout; the host adds its own control's screen offset (see EditorHost.xaml.cs). */
function sendCaretScreenRect(bridge: EditorBridge, view: EditorView): void {
  const pos = view.state.selection.main.head;
  const coords = view.coordsAtPos(pos);
  if (!coords) {
    return;
  }
  bridge.notifyCaretScreenRect({
    x: Math.round(coords.left),
    y: Math.round(coords.top),
    width: 2,
    height: Math.round(coords.bottom - coords.top),
  });
}

/** A control word being typed (`\sec`, `\|`) triggers a completion request with the letters typed so far as the prefix; anywhere else, no request is sent. Mirrors the trigger the native `Completion.CommandSuggestions` ranking expects: a plain (possibly empty) command-name prefix. */
function maybeRequestCompletion(bridge: EditorBridge, view: EditorView): void {
  const pos = view.state.selection.main.head;
  const line = view.state.doc.lineAt(pos);
  const textBefore = line.text.slice(0, pos - line.from);
  const match = /\\([A-Za-z]*)$/.exec(textBefore);
  if (match) {
    bridge.requestCompletion(pos, match[1] ?? "");
  }
}

function createState(text: string, bridge: EditorBridge): EditorState {
  return EditorState.create({
    doc: text,
    extensions: [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightActiveLine(),
      highlightSpecialChars(),
      drawSelection(),
      history(),
      closeBrackets(),
      syntaxHighlighting(defaultHighlightStyle),
      latex(),
      diagnostics(),
      themeCompartment.of(themeExtension("light")),
      fontSizeCompartment.of(fontSizeExtension(DEFAULT_FONT_SIZE_PX)),
      keymap.of([...closeBracketsKeymap, ...defaultKeymap, ...historyKeymap, indentWithTab]),
      EditorView.lineWrapping,
      updateListener(bridge),
    ],
  });
}

function mount(): void {
  const parent = document.getElementById("editor");
  if (!parent) {
    throw new Error("main.ts: #editor host element is missing from index.html");
  }

  // `getDocText` is read lazily on every dispatch (see bridge.ts's doc comment), so it is safe
  // to close over `view` here even though `view` itself is assigned only a few lines below.
  const bridge = new EditorBridge(createWebViewTransport(), () => view.state.doc.toString());
  const view = new EditorView({ state: createState("", bridge), parent });

  bridge.listen({
    onSetDocument(text, revision) {
      revisionCounter = revision;
      view.setState(createState(text, bridge));
    },
    onApplyEdit(range, replacement, revision) {
      revisionCounter = revision;
      view.dispatch({
        changes: { from: range.utf16Start, to: range.utf16End, insert: replacement },
        annotations: fromNativeAnnotation.of(true),
      });
    },
    onSetDiagnostics(items) {
      setDiagnostics(view, items);
    },
    onSetTheme(theme) {
      document.documentElement.dataset["theme"] = theme;
      view.dispatch({ effects: themeCompartment.reconfigure(themeExtension(theme)) });
    },
    onSetFontSize(fontSizePx) {
      view.dispatch({ effects: fontSizeCompartment.reconfigure(fontSizeExtension(fontSizePx)) });
    },
    onCompletionReply() {
      // Deliberate no-op: completion is rendered as a native WinUI3 Flyout (see this
      // file's header comment and FlashTeX.App/EditorHost.xaml.cs), never by this page.
    },
    onRevealRange(range) {
      view.dispatch({
        selection: { anchor: range.utf16Start, head: range.utf16End },
        effects: EditorView.scrollIntoView(range.utf16Start, { y: "center" }),
        annotations: fromNativeAnnotation.of(true),
      });
    },
    onProtocolError(error, rawMessage) {
      console.error("EditorBridge: protocol error", error, rawMessage);
    },
  });

  view.focus();
}

mount();
