import SwiftUI

/// The one Editor menu. Folding and re-indent item groups join this menu as
/// sections separated by `Divider()`. Go to Line stays in Navigate.
struct EditorMenuCommands: Commands {
    var body: some Commands {
        CommandMenu("Editor") {
            LineCommandMenuItems()
            // Folding and re-indent item groups join this menu as sections
            // separated by Divider().
        }
    }
}

struct LineCommandMenuItems: View {
    var body: some View {
        Button("Duplicate Line/Selection") { EditorLineCommandAction.duplicate() }
            .keyboardShortcut("d")
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
