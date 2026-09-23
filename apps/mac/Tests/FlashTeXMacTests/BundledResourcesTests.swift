import XCTest
@testable import FlashTeXMac

/// The bundled resources must reach the *test* bundle, not just the packaged
/// app. They stopped doing so once SwiftPM started emitting a structured macOS
/// resource bundle (`FlashTeXMac_FlashTeXMac.bundle/Contents/Resources/…`) and
/// copying it inside `FlashTeXMacTests.xctest`: the hand-built candidate paths
/// appended the file name straight to the `.bundle` directory, so every
/// candidate missed. The vocabulary then fell back to empty, which showed up
/// as 86 "not in the compiler inventory" failures *and* as force-unwrap
/// crashes that aborted the whole test process (GH#704).
///
/// These assertions are deliberately about resolution, not about content:
/// `CompletionTests.testBundledInventoryMatchesTheCompiler` owns the content.
final class BundledResourcesTests: XCTestCase {
    func testTheCompletionInventoryResolvesFromTheTestBundle() throws {
        let found = BundledResources.url(forResource: "supported-latex.json", module: Bundle(for: Self.self))
        XCTAssertNotNil(found.url, "supported-latex.json not found; searched \(found.searched.map(\.path))")
        XCTAssertFalse(Completion.Vocabulary.inventory.commands.isEmpty,
                       "the vocabulary fell back to empty -- the bundled inventory did not load")
        XCTAssertFalse(Completion.Vocabulary.entries.isEmpty)
    }

    func testTheBundledEditorFaceRegisters() {
        XCTAssertTrue(EditorFontRegistration.registerIfNeeded(),
                      "JetBrains Mono did not register; the bundled Fonts directory was not found")
    }
}
