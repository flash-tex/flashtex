import AppKit
import SwiftUI

/// Routes classic Find-menu actions to whichever `NSTextView` is first
/// responder — normally the source editor (`SourceEditorView.makeNSView`,
/// `usesFindBar = true`) — the way a literal `NSMenuItem` with `target = nil`
/// and a numeric tag would: AppKit walks the responder chain from the first
/// responder until something implements the selector, exactly as it does for
/// a plain Interface-Builder-wired Find menu.
///
/// `performTextFinderAction(_:)` is used rather than the older
/// `performFindPanelAction(_:)`: a `usesFindBar` text view owns its
/// `NSTextFinder` privately (there is no public accessor to call its actions
/// directly), and only the newer selector's action set (`NSTextFinder.Action`)
/// has the case this feature needs that the legacy `NSFindPanelAction` enum
/// lacks — `showReplaceInterface`, which opens the find bar already showing
/// its Replace row for "Find and Replace…". Both selectors share tags 1–10
/// for the actions they have in common (next, previous, "use selection", …),
/// so every other item here behaves exactly as the classic wiring would.
///
/// Replacements `NSTextFinder` makes go through the client `NSTextView`'s
/// normal `shouldChangeText(in:replacementString:)` / `didChangeText()` path
/// — the same one `SourceEditorView.Coordinator` uses for every other edit —
/// so they land as one undo step and are reported through the editor's usual
/// `textDidChange` delegate hook (the model sees the new text and recompiles)
/// without anything here needing to know about it.
///
/// The find bar's own match highlighting and the editor's temporary-attribute
/// painters key off different `NSAttributedString.Key`s and so never erase
/// each other: syntax colouring is `.foregroundColor`
/// (`SyntaxPainter.key`, `SyntaxHighlighter.swift`), diagnostic underlines are
/// `.underlineStyle` / `.underlineColor` / `.toolTip`
/// (`SourceEditorView.MarkPainter.keys`), and only the brace-match highlight
/// (`SourceEditorView.Coordinator.highlightKey`) shares `.backgroundColor`
/// with the find bar's own highlight — a cosmetic, self-healing overlap
/// (`refreshBraceHighlight` recomputes it on the next caret move) rather than
/// one that can strip syntax colour or diagnostics.
enum EditorFindAction {
    static func send(_ action: NSTextFinder.Action) {
        let sender = NSMenuItem()
        sender.tag = action.rawValue
        NSApp.sendAction(#selector(NSTextView.performTextFinderAction(_:)), to: nil, from: sender)
    }

    /// "Jump to Selection": scrolls the current selection into view and
    /// centers it. Not an `NSTextFinder.Action` — Cocoa's own Find submenu
    /// binds this item straight to `NSTextView.centerSelectionInVisibleArea(_:)`.
    static func centerSelection() {
        NSApp.sendAction(#selector(NSTextView.centerSelectionInVisibleArea(_:)), to: nil, from: nil)
    }
}

/// Edit ▸ Find submenu, added from `FlashTeXMacApp` with one line
/// (`FindCommands()`). Targets the first responder rather than a specific
/// view, so it needs no `ShellModel` or `FocusedValue` plumbing.
///
/// "Find Next"/"Find Previous" have no key equivalent here: the obvious ⌘G /
/// ⇧⌘G are already taken — ⌘G is "Next match" in the Find in Project window
/// (`ProjectSearchPanel.swift`, scoped to that window; project search wording
/// and shortcuts are mac-claude-a's, coordinated separately per GH73) and
/// ⇧⌘G is "Convert Capture" (`FlashTeXMacApp.swift`, `CommandGroup(after:
/// .pasteboard)`) — and `CommandTableTests` requires every documented
/// shortcut to be globally unique, so reusing either would need changing a
/// binding this branch doesn't own. Return / Shift-Return in the find bar's
/// own search field already step to the next/previous match, which covers
/// the common case (search, then repeat); the menu items exist so the action
/// is still discoverable and reachable from the command palette without a
/// key equivalent.
struct FindCommands: Commands {
    var body: some Commands {
        CommandGroup(after: .textEditing) {
            Menu("Find") {
                Button("Find…") { EditorFindAction.send(.showFindInterface) }
                    .keyboardShortcut("f", modifiers: [.command])
                Button("Find and Replace…") { EditorFindAction.send(.showReplaceInterface) }
                    .keyboardShortcut("f", modifiers: [.command, .option])
                Divider()
                Button("Find Next") { EditorFindAction.send(.nextMatch) }
                Button("Find Previous") { EditorFindAction.send(.previousMatch) }
                Divider()
                Button("Use Selection for Find") { EditorFindAction.send(.setSearchString) }
                    .keyboardShortcut("e", modifiers: [.command])
                Button("Jump to Selection") { EditorFindAction.centerSelection() }
                    .keyboardShortcut("j", modifiers: [.command])
            }
        }
    }
}
