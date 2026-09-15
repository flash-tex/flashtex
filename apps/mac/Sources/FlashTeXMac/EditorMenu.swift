import SwiftUI

/// The one Editor menu. Folding and re-indent item groups join this menu as
/// sections separated by `Divider()`. Go to Line stays in Navigate.
struct EditorMenuCommands: Commands {
    var model: ShellModel
    var body: some Commands {
        CommandMenu("Editor") {
            LineCommandMenuItems()
            Divider()
            ChangeEnvironmentMenuItems(model: model)
            Divider()
            FoldMenuItems()
            // Re-indent (EditorIndentation.swift) still lives under Edit; it
            // can join as another Divider()-separated section.
        }
    }
}

struct LineCommandMenuItems: View {
    var body: some View {
        Button("Duplicate Line") { EditorLineCommandAction.duplicateBelow() }
            .keyboardShortcut(.downArrow, modifiers: [.option, .shift])
        Button("Duplicate Line Up") { EditorLineCommandAction.duplicateAbove() }
            .keyboardShortcut(.upArrow, modifiers: [.option, .shift])
        Button("Move Line Up") { EditorLineCommandAction.moveUp() }
            .keyboardShortcut(.upArrow, modifiers: [.command, .option])
        Button("Move Line Down") { EditorLineCommandAction.moveDown() }
            .keyboardShortcut(.downArrow, modifiers: [.command, .option])
        Button("Delete Line") { EditorLineCommandAction.deleteLines() }
            .keyboardShortcut("k", modifiers: [.control, .command])
        Button("Join Lines") { EditorLineCommandAction.joinLines() }
            .keyboardShortcut("j", modifiers: [.control])
        Divider()
        Button("Sort Lines Ascending") { EditorLineCommandAction.sortAscending() }
        Button("Sort Lines Descending") { EditorLineCommandAction.sortDescending() }
        Button("Trim Trailing Whitespace") { EditorLineCommandAction.trimTrailingWhitespace() }
    }
}

struct ChangeEnvironmentMenuItems: View {
    var model: ShellModel
    var body: some View {
        Button("Change Environment…") { model.presentChangeEnvironment() }
            .keyboardShortcut("e", modifiers: [.control, .command])
    }
}

/// Editor ▸ Fold / Unfold / Fold All / Unfold All (⌥⌘← / ⌥⌘→ / ⌥⇧⌘← / ⌥⇧⌘→),
/// a section of the one Editor menu above; actions in EditorFolding.swift.
struct FoldMenuItems: View {
    var body: some View {
        Button("Fold") { EditorFoldAction.fold() }
            .keyboardShortcut(.leftArrow, modifiers: [.command, .option])
        Button("Unfold") { EditorFoldAction.unfold() }
            .keyboardShortcut(.rightArrow, modifiers: [.command, .option])
        Button("Fold All") { EditorFoldAction.foldAll() }
            .keyboardShortcut(.leftArrow, modifiers: [.command, .option, .shift])
        Button("Unfold All") { EditorFoldAction.unfoldAll() }
            .keyboardShortcut(.rightArrow, modifiers: [.command, .option, .shift])
    }
}
