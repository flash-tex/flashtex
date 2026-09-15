import XCTest

/// SwiftUI builds a separate top-level menu for every `CommandMenu("Editor")`,
/// so two editor lanes that each declare one merge cleanly in git and still
/// show two "Editor" menus. Folding, line commands and re-indent share the one
/// in `EditorMenu.swift`.
final class EditorMenuTests: XCTestCase {

    func testOnlyOneEditorCommandMenuIsDeclared() throws {
        let sourcesDir = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent()
            .appendingPathComponent("Sources/FlashTeXMac")
        let files = FileManager.default.enumerator(at: sourcesDir, includingPropertiesForKeys: nil)?
            .compactMap { $0 as? URL }
            .filter { $0.pathExtension == "swift" } ?? []
        XCTAssertFalse(files.isEmpty, "no sources found under \(sourcesDir.path)")

        var declaring: [String] = []
        for file in files {
            let text = try String(contentsOf: file, encoding: .utf8)
            let uncommented = text.split(separator: "\n", omittingEmptySubsequences: false)
                .filter { !$0.trimmingCharacters(in: .whitespaces).hasPrefix("//") }
                .joined(separator: "\n")
            let count = uncommented.components(separatedBy: "CommandMenu(\"Editor\")").count - 1
            declaring += Array(repeating: file.lastPathComponent, count: count)
        }
        XCTAssertEqual(declaring, ["EditorMenu.swift"],
                       "add editor items as a section of EditorMenuCommands instead of a second CommandMenu(\"Editor\")")
    }
}
