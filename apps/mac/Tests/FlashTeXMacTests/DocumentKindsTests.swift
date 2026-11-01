import XCTest
@testable import FlashTeXMac

/// Explicit document kinds (lane `mac-bibliography-kinds`, gap 3). Kinds are
/// never inferred from an extension; the helper that declared them is
/// retired (RQ7), so no source has a declared kind in this window.
@MainActor
final class DocumentKindsTests: XCTestCase {
    func testKindsAreNeverInferred() {
        let model = ShellModel()
        model.replaceProject(entryText: "\\begin{document}\\cite{knuth84}\\end{document}\n")
        model.documents.append(.init(path: "refs.bib", text: "@book{knuth84, title={The TeXbook}}\n"))
        let kinds = model.documentKinds
        XCTAssertNil(kinds.kind(of: "refs.bib"), "a .bib member has no kind unless one is declared")
        XCTAssertEqual(kinds.kinds, [:])
        XCTAssertEqual(kinds.bibliographyPaths, [])
        XCTAssertEqual(kinds.status, "no document kinds declared")
    }
}
