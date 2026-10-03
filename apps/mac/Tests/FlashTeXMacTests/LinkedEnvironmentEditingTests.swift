import XCTest
import FlashTeXEditorCore
@testable import FlashTeXMac

/// The shared core of linked `\begin{…}` / `\end{…}` name editing
/// (`FlashTeXEditorCore/LinkedEnvironmentEditing.swift`), which the Mac's
/// `SourceEditorView` and the iPad's `EditorController` both drive. Pure;
/// the hosted editor scenarios are `LinkedEnvironmentRenameTests` (Mac) and
/// the iPad's test of the same name.
final class LinkedEnvironmentEditingTests: XCTestCase {
    typealias L = LinkedEnvironmentEditing

    private static let doc = "\\begin{itemize}\n\\item a\n\\end{itemize}\n" as NSString

    private func name(_ n: String, _ which: String, in text: NSString) -> NSRange {
        let r = text.range(of: "\\\(which){\(n)}")
        return NSRange(location: r.location + which.utf16.count + 2, length: n.utf16.count)
    }

    /// Applies `edit` to `text` the way an editor does, then the partner edit
    /// the session asks for; returns the final text.
    private func edit(_ text: NSString, _ range: NSRange, _ replacement: String) -> String {
        let session = L.session(for: range, in: text, continuing: nil)
        let applied = NSMutableString(string: text)
        applied.replaceCharacters(in: range, with: replacement)
        if let session, let p = L.partnerEdit(for: session, in: applied) {
            applied.replaceCharacters(in: p.range, with: p.replacement)
        }
        return applied as String
    }

    // MARK: pair linking

    func testBothDirectionsLink() {
        let s = Self.doc
        let b = name("itemize", "begin", in: s), e = name("itemize", "end", in: s)
        let fromBegin = try! XCTUnwrap(L.linkedNames(at: b.location + 2, in: s))
        XCTAssertEqual(fromBegin.active, b)
        XCTAssertEqual(fromBegin.partner, e)
        XCTAssertEqual(fromBegin.name, "itemize")
        let fromEnd = try! XCTUnwrap(L.linkedNames(at: NSMaxRange(e), in: s), "the caret right after the name is on it")
        XCTAssertEqual(fromEnd.active, e)
        XCTAssertEqual(fromEnd.partner, b)
    }

    func testThingsThatNeverLink() {
        XCTAssertNil(L.linkedNames(at: 8, in: "\\begin{itemize}\n" as NSString), "unbalanced")
        XCTAssertNil(L.linkedNames(at: 6, in: "\\end{itemize}\n" as NSString), "a stray end")
        XCTAssertNil(L.linkedNames(at: Self.doc.range(of: "\\item").location + 2, in: Self.doc), "the body")
        let commented = "% \\begin{x}\n\\begin{y}\n\\end{y}\n" as NSString
        XCTAssertNil(L.linkedNames(at: 10, in: commented), "a commented-out begin")
        let verbatim = "\\begin{verbatim}\n\\begin{a}\\end{a}\n\\end{verbatim}\n" as NSString
        XCTAssertNil(L.linkedNames(at: name("a", "begin", in: verbatim).location, in: verbatim), "inside a verbatim body")
        XCTAssertEqual(L.linkedNames(at: 10, in: verbatim)?.name, "verbatim", "a verbatim environment's own names link")
        let verb = "\\verb|\\begin{a}| \\begin{a}\\end{a}" as NSString
        XCTAssertNil(L.linkedNames(at: 13, in: verb), "inside \\verb")
    }

    func testNestedSameNameLinksTheInnermostPair() {
        let s = "\\begin{itemize}\n\\begin{itemize}\n\\end{itemize}\n\\end{itemize}\n" as NSString
        let innerBegin = s.range(of: "itemize", range: NSRange(location: 10, length: s.length - 10))
        let link = try! XCTUnwrap(L.linkedNames(at: innerBegin.location, in: s))
        XCTAssertEqual(link.partner, s.range(of: "itemize", range: NSRange(location: 36, length: s.length - 36)))
        let outerEnd = s.range(of: "itemize", options: .backwards)
        XCTAssertEqual(L.linkedNames(at: outerEnd.location, in: s)?.partner, NSRange(location: 7, length: 7))
    }

    func testEmptiedNamesStillLink() {
        let s = "\\begin{}\nx\n\\end{}\n" as NSString
        let link = try! XCTUnwrap(L.linkedNames(at: 7, in: s))
        XCTAssertEqual(link.active, NSRange(location: 7, length: 0))
        XCTAssertEqual(link.name, "")
        XCTAssertEqual(edit(s, NSRange(location: 7, length: 0), "a"), "\\begin{a}\nx\n\\end{a}\n")
    }

    // MARK: edits

    func testTypingDeletingAndReplacingInEitherName() {
        let s = Self.doc
        let b = name("itemize", "begin", in: s), e = name("itemize", "end", in: s)
        XCTAssertEqual(edit(s, NSRange(location: NSMaxRange(b), length: 0), "*"), "\\begin{itemize*}\n\\item a\n\\end{itemize*}\n")
        XCTAssertEqual(edit(s, NSRange(location: b.location, length: 0), "x"), "\\begin{xitemize}\n\\item a\n\\end{xitemize}\n",
                       "an insert at the start of the name is inside it")
        XCTAssertEqual(edit(s, NSRange(location: NSMaxRange(e) - 1, length: 1), ""), "\\begin{itemiz}\n\\item a\n\\end{itemiz}\n")
        XCTAssertEqual(edit(s, b, "description"), "\\begin{description}\n\\item a\n\\end{description}\n")
        XCTAssertEqual(edit(s, e, "enumerate"), "\\begin{enumerate}\n\\item a\n\\end{enumerate}\n")
        XCTAssertEqual(edit(s, e, ""), "\\begin{}\n\\item a\n\\end{}\n")
    }

    func testOptionalArgumentAndStarStayOutsideTheSpans() {
        let s = "\\begin{enumerate}[label=(\\alph*)]\n\\item a\n\\end{enumerate}\n" as NSString
        XCTAssertEqual(edit(s, name("enumerate", "begin", in: s), "itemize"),
                       "\\begin{itemize}[label=(\\alph*)]\n\\item a\n\\end{itemize}\n")
    }

    func testAnEditLeavingTheNameDoesNotLink() {
        let s = Self.doc
        let b = name("itemize", "begin", in: s)
        XCTAssertNil(L.session(for: NSRange(location: b.location, length: b.length + 2), in: s, continuing: nil),
                     "a selection past the `}` is not a name edit")
        XCTAssertNil(L.session(for: NSRange(location: 0, length: s.length), in: s, continuing: nil), "delete-all")
    }

    /// The guard: the partner is rewritten only while it still reads the old
    /// name where the session expects it.
    func testThePartnerIsNotRewrittenOnceSomethingElseChangedIt() {
        let s = Self.doc
        let b = name("itemize", "begin", in: s)
        let session = try! XCTUnwrap(L.session(for: NSRange(location: NSMaxRange(b), length: 0), in: s, continuing: nil))
        let now = NSMutableString(string: s)
        now.replaceCharacters(in: NSRange(location: NSMaxRange(b), length: 0), with: "x")
        XCTAssertNotNil(L.partnerEdit(for: session, in: now))
        let e = now.range(of: "itemize", options: .backwards)
        now.replaceCharacters(in: e, with: "itemizq") // same length, different text
        XCTAssertNil(L.partnerEdit(for: session, in: now), "the partner no longer reads `itemize`")
        let shorter = NSMutableString(string: s)
        shorter.replaceCharacters(in: NSRange(location: 0, length: 1), with: "") // outside the name: spans are wrong
        XCTAssertNil(L.partnerEdit(for: session, in: shorter), "never write into a guessed range")
        XCTAssertNil(L.partnerEdit(for: session, in: s), "unchanged names need no edit")
    }

    /// An IME composition: the first marked character opens the session; the
    /// next replacement of the marked text continues it although the names
    /// already differ (a fresh scan would refuse).
    func testACompositionContinuesItsSession() {
        let s = Self.doc
        let at = NSMaxRange(name("itemize", "begin", in: s))
        let first = try! XCTUnwrap(L.session(for: NSRange(location: at, length: 0), in: s, continuing: nil))
        let marked = NSMutableString(string: s)
        marked.replaceCharacters(in: NSRange(location: at, length: 0), with: "´")
        XCTAssertNil(L.session(for: NSRange(location: at, length: 1), in: marked, continuing: nil), "a fresh scan refuses")
        let next = try! XCTUnwrap(L.session(for: NSRange(location: at, length: 1), in: marked, continuing: first))
        marked.replaceCharacters(in: NSRange(location: at, length: 1), with: "é")
        let p = try! XCTUnwrap(L.partnerEdit(for: next, in: marked))
        marked.replaceCharacters(in: p.range, with: p.replacement)
        XCTAssertEqual(marked as String, "\\begin{itemizeé}\n\\item a\n\\end{itemizeé}\n")
    }

    // MARK: completion acceptance

    func testCompletionRenameSpan() {
        let s = "\\begin{enumer}\n\\item a\n\\end{enumer}\n" as NSString
        let b = name("enumer", "begin", in: s)
        XCTAssertEqual(L.completionRenameSpan(for: NSRange(location: b.location, length: 3), in: s) { _ in false }, b,
                       "a prefix inside the name renames the whole name")
        XCTAssertNil(L.completionRenameSpan(for: b, in: s) { $0 == NSMaxRange(b) }, "an auto-closed `}`: a new environment")
        XCTAssertNil(L.completionRenameSpan(for: NSRange(location: 0, length: 0), in: s) { _ in false })
        let open = "\\begin{enu}\n" as NSString
        XCTAssertNil(L.completionRenameSpan(for: NSRange(location: 7, length: 3), in: open) { _ in false }, "unbalanced")
    }

    // MARK: the O(line) gate

    func testIsOnEnvironmentName() {
        let s = Self.doc
        XCTAssertTrue(L.isOnEnvironmentName(in: s, at: 7))
        XCTAssertTrue(L.isOnEnvironmentName(in: s, at: 14), "right after the name")
        XCTAssertFalse(L.isOnEnvironmentName(in: s, at: 15), "after the `}`")
        XCTAssertFalse(L.isOnEnvironmentName(in: s, at: 0))
        XCTAssertTrue(L.isOnEnvironmentName(in: "\\begin {x}" as NSString, at: 8), "a space before the brace")
        XCTAssertFalse(L.isOnEnvironmentName(in: "\\textbf{x}" as NSString, at: 8))
        XCTAssertFalse(L.isOnEnvironmentName(in: "" as NSString, at: 0))
    }

    // MARK: the Mac forwards to the core

    func testTheMacNamesAreTheCore() {
        let s = Self.doc
        XCTAssertEqual(EditorChangeEnvironment.linkedNames(at: 8, in: s)?.partner, L.linkedNames(at: 8, in: s)?.partner)
        XCTAssertEqual(EditorNavigation.uses(in: s), LaTeXScan.uses(in: s))
        let old = "\\begin{a}\\end{a}" as NSString
        XCTAssertEqual(EditorChangeEnvironment.linkedPartnerEdit(old: old, edit: (NSRange(location: 8, length: 0), "b"))?.replacement, "ab")
    }
}
