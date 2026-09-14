import XCTest
@testable import FlashTeXProtocol
@testable import FlashTeXAccessibility
@testable import FlashTeXMac

/// Partial output on a real result: a generated homework-shaped document
/// (`PartialOutputFixture`) compiles (live `flashtex-compiler`,
/// `FLASHTEX_COMPILER`) to `recovered` with pages AND ~115 diagnostics, many
/// in regions the compiler skipped. Every mark must slice to the command its
/// message names, survive an edit sequence only by rebasing onto the same
/// bytes or being withheld, be kept and flagged (never cleared or duplicated)
/// while a later result fails with no output, and group with its identical
/// siblings for the panel. Skips loudly without the compiler.
///
/// The fixture used to be `fixtures/real-world/hw1/HW1.tex`, which measured
/// 119 diagnostics on 2026-09-12 and ten a day later once the compiler grew
/// `\in`, `\hfill`, `\setlength` and friends. The generated document keeps
/// the same shape (a `\problem` macro whose expansion carries three
/// diagnostics per call site, `\Z`/`\R` macros, a repeated math command, a
/// skipped preamble command, a package warning first) but spells every
/// unsupported command with an `hw` prefix that no compiler version will
/// implement, so the diagnostic count is a property of the fixture, not of
/// the compiler's current coverage.
final class EditorDiagnosticsPartialOutputTests: XCTestCase {
    /// A multi-byte first line so UTF-16 offsets differ from byte offsets for
    /// every span of the (otherwise ASCII) fixture: "naïve", curly quotes,
    /// an em dash and a ZWJ emoji sequence. Comments are skipped by the
    /// compiler, so the diagnostics are the fixture's own.
    static let prefix = "% naïve “HW1” — 👩‍💻 partial-output fixture\n"

    /// The generated partial-output document and what it is built from.
    enum PartialOutputFixture {
        /// Number of `\problem` sections; every per-section count below scales with it.
        static let problems = 8
        /// The repeated unsupported math command (`\in` in the original HW1).
        static let mathCommand = "\\hwin"
        /// `\hwin` occurrences: three per section (two in the first sentence, one in the list).
        static var mathCommandCount: Int { 3 * problems }
        /// `\problem` macro body: `\subsection*` is supported, the other three commands are not.
        static let problemBody = "[2]{\\subsection*{Problem #1} \\hwfill \\hwpoints{#2 points} \\hwrule}"
        /// The first diagnosed span (a package warning on line 4 with the prefix).
        static let firstMark = "\\usepackage{microtype}"

        /// The document without the prefix. Line 4 with the prefix is the
        /// `microtype` warning; `\hwpreamble` is an unsupported preamble
        /// command the compiler skips; `\Z` and `\R` expand to an unsupported
        /// math command so their call sites carry a diagnostic; `\hwskip{0.6em}`
        /// and `\hwstrike{nothing}` have arguments the compiler treats as
        /// parameters (skipped), `\hwbox` and `\hwnote` carry prose (typeset).
        static let text: String = {
            var lines: [String] = [
                "\\documentclass[11pt]{article}",
                "",
                firstMark,
                "\\usepackage{amsmath,amssymb,amsthm}",
                "\\hwpreamble{secnumdepth}",
                "\\newcommand{\\Z}{\\mathbf{Z}\\hwbolt}",
                "\\newcommand{\\R}{\\mathbf{R}\\hwbolt}",
                "\\newcommand{\\problem}" + problemBody,
                "\\begin{document}",
                "\\pagestyle{empty}",
                "",
            ]
            for k in 1...problems {
                lines += [
                    "\\problem{\(k)}{5}",
                    "",
                    "Let $x \(mathCommand) \\Z$ and $y \(mathCommand) \\R$. Then \\hwbox{$x + y$} is \\hwskip{0.6em} fine.",
                    "\\begin{enumerate}",
                    "    \\item Show $x \(mathCommand) \\Z$ using \\hwnote{a lemma from lecture}.",
                    "    \\item Prove \\hwstrike{nothing} about $\\R$.",
                    "\\end{enumerate}",
                    "",
                ]
            }
            lines.append("\\end{document}")
            return lines.joined(separator: "\n") + "\n"
        }()

        /// Marks whose message names the command under the mark: `\hwin`
        /// (3 per section), the four body commands (4 per section) and
        /// `\hwpreamble` once.
        static var checkedByName: Int { 7 * problems + 1 }
        /// Marks on a macro call site whose expansion contains the named
        /// command: `\Z`/`\R` (4 per section) and `\problem` (3 per section).
        static var viaMacro: Int { 7 * problems }
        /// Diagnostics whose recovery says the compiler skipped something:
        /// the body and macro commands (7 per section) and the preamble one.
        static var skippedRegion: Int { 7 * problems + 1 }
        /// Every diagnostic the fixture yields: the above plus 4 `\hwbolt`
        /// per section and two package warnings.
        static var diagnostics: Int { checkedByName + viaMacro + 2 }
    }

    /// The fixture with the multi-byte prefix, as compiled by every test here.
    static var fixtureText: String { prefix + PartialOutputFixture.text }

    private func compiler() throws -> URL {
        guard let path = ProcessInfo.processInfo.environment["FLASHTEX_COMPILER"],
              FileManager.default.isExecutableFile(atPath: path) else {
            throw XCTSkip("set FLASHTEX_COMPILER=crates/compiler/target/release/flashtex-compiler (cargo build --release in crates/compiler)")
        }
        return URL(fileURLWithPath: path)
    }

    /// One compile request over the live compiler (one line in, first line out).
    static func compile(_ documents: [RuntimeV1.Document], entry: String = "main.tex", revision: Int, id: String,
                        with compiler: URL) throws -> RuntimeV1.Envelope<RuntimeV1.CompileResult> {
        let req = RuntimeV1.CompileRequest(projectId: "hw1", revision: revision, entryPath: entry, documents: documents)
        let line = try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: id, req))
        let p = Process(); p.executableURL = compiler
        let stdin = Pipe(), stdout = Pipe()
        p.standardInput = stdin; p.standardOutput = stdout; p.standardError = FileHandle.nullDevice
        try p.run()
        try stdin.fileHandleForWriting.write(contentsOf: line)
        try stdin.fileHandleForWriting.close()
        let out = stdout.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        let first = try XCTUnwrap(out.split(separator: 0x0A).first, "the compiler answered nothing")
        return try RuntimeV1.decodeCompileResult(Data(first))
    }

    /// The premise of every test here is partial output: pages AND many
    /// diagnostics to place. The fixture only uses commands the compiler
    /// never implements, so a clean or failed compile is a real regression
    /// (reported with the result's actual shape), never a skip.
    private func fixtureResult(_ compiler: URL, text: String) throws -> RuntimeV1.Envelope<RuntimeV1.CompileResult> {
        let env = try Self.compile([.init(path: "main.tex", text: text)], revision: 1, id: "hw1-1", with: compiler)
        let r = env.payload
        XCTAssertEqual(r.status, .recovered, "the fixture is partial output: pages AND diagnostics (\(r.pages.count) pages, \(r.diagnostics.count) diagnostics)")
        XCTAssertGreaterThanOrEqual(r.pages.count, 1)
        XCTAssertEqual(r.diagnostics.count, PartialOutputFixture.diagnostics,
                       "the fixture yields exactly its own diagnostics: \(r.diagnostics.map(\.message))")
        XCTAssertGreaterThanOrEqual(r.diagnostics.count, 100, "the partial-output premise needs many marks")
        return env
    }

    /// The command a message names ("\hwin is not supported…", "\subsection
    /// requires a braced argument"), nil for messages that start otherwise.
    private static func namedCommand(_ message: String) -> String? {
        guard message.hasPrefix("\\") else { return nil }
        var name = "\\"
        for c in message.dropFirst() {
            if c.isLetter { name.append(c) } else { break }
        }
        return name.count > 1 ? name : nil
    }

    /// `\newcommand{\name}…{body}` definitions of `text`: the diagnostics the
    /// compiler raises while expanding a user macro are reported at the call
    /// site, so a mark on `\problem` may name `\hwfill`, `\hwpoints` or
    /// `\hwrule` and a mark on `\Z` may name `\hwbolt`.
    private static func macroBodies(in text: String) -> [String: String] {
        var out: [String: String] = [:]
        for line in text.split(separator: "\n") where line.hasPrefix("\\newcommand{") {
            let rest = line.dropFirst("\\newcommand{".count)
            guard let close = rest.firstIndex(of: "}") else { continue }
            out[String(rest[..<close])] = String(rest[rest.index(after: close)...])
        }
        return out
    }

    // MARK: partial output: every mark slices to the diagnosed command (or its macro)

    func testPartialOutputMarksSliceToTheNamedCommand() throws {
        let compiler = try compiler()
        let text = Self.fixtureText
        let env = try fixtureResult(compiler, text: text)
        let result = env.payload
        XCTAssertEqual(result.diagnostics.filter { $0.source == nil }.count, 0, "every fixture diagnostic is sourced")
        let macros = Self.macroBodies(in: text)
        XCTAssertEqual(macros["\\problem"], PartialOutputFixture.problemBody)

        let report = EditorDiagnostics.report(for: result, resultID: env.id, path: "main.tex", compiledText: text, currentText: text)
        XCTAssertEqual(report.marks.count, result.diagnostics.count, "one mark per sourced diagnostic, none refused")
        XCTAssertEqual(report.stale, [])
        XCTAssertNil(report.edit)
        XCTAssertNil(report.staleNote)

        let ns = text as NSString
        let bytes = Array(text.utf8)
        var checkedByName = 0, viaMacro = 0, skippedRegion = 0
        for mark in report.marks {
            let d = result.diagnostics[mark.diagnosticIndex]
            let src = try XCTUnwrap(d.source)
            // Byte-exact: the UTF-16 range covers exactly the reported bytes
            // (the prefix shifts every UTF-16 offset below its byte offset).
            let under = ns.substring(with: mark.nsRange)
            let reported = String(decoding: bytes[src.startByte..<src.endByte], as: UTF8.self)
            XCTAssertTrue(under.sameBytes(as: reported), "\(mark.id): “\(under)” vs reported “\(reported)”")
            XCTAssertLessThan(mark.nsRange.location, src.startByte, "UTF-16 offset must be below the byte offset after the multi-byte prefix")
            XCTAssertEqual(mark.resultStatus, .recovered)
            XCTAssertNotNil(mark.recoveryLine, "a recovered result always shows a recovery line")
            XCTAssertNil(mark.carried)
            if let name = Self.namedCommand(d.message) {
                if under.hasPrefix(name) {
                    checkedByName += 1
                } else if let body = macros[under], body.contains(name) {
                    viaMacro += 1 // reported at the macro's call site; the expansion contains the command
                } else {
                    XCTFail("\(mark.id): mark “\(under)” is neither the command the message names (\(d.message)) nor a macro expanding to it")
                }
            }
            if d.recovery?.contains("skipped") == true { skippedRegion += 1 }
        }
        XCTAssertEqual(checkedByName, PartialOutputFixture.checkedByName, "messages naming the command under the mark")
        XCTAssertEqual(viaMacro, PartialOutputFixture.viaMacro, "\\problem/\\Z/\\R call sites carry their expansion's diagnostics")
        XCTAssertEqual(skippedRegion, PartialOutputFixture.skippedRegion, "marks inside regions the compiler skipped")
        // Three diagnostics of one \problem expansion share one span and stay three distinct marks.
        let problemMarks = report.marks.filter { ns.substring(with: $0.nsRange) == "\\problem" }
        XCTAssertEqual(problemMarks.count, 3 * PartialOutputFixture.problems)
        XCTAssertEqual(Set(problemMarks.map(\.id)).count, problemMarks.count)
        XCTAssertEqual(Set(problemMarks.map(\.nsRange)).count, problemMarks.count / 3, "each call site carries the three expansion diagnostics")
        // The preamble region the compiler skipped entirely still maps exactly.
        let preamble = report.marks.first { result.diagnostics[$0.diagnosticIndex].message.hasPrefix("\\hwpreamble") }
        XCTAssertEqual(preamble.map { ns.substring(with: $0.nsRange) }, "\\hwpreamble")
        XCTAssertTrue(result.diagnostics[try XCTUnwrap(preamble).diagnosticIndex].recovery?.contains("skipped") == true)
        // Document order for keyboard navigation: the first stop is the first byte.
        let items = EditorDiagnostics.navigationItems(report.marks)
        XCTAssertEqual(items.count, report.marks.count)
        XCTAssertEqual(items.first?.nsRange.location, report.marks.map(\.nsRange.location).min())
        let step = try XCTUnwrap(EditorDiagnostics.step(report.marks, fromUTF16: 0, forward: true, in: text))
        let firstMark = try XCTUnwrap(report.marks.min { $0.nsRange.location < $1.nsRange.location })
        let firstLine = try XCTUnwrap(EditorDiagnostics.lineNumber(ofByte: firstMark.originalSource.startByte, in: text))
        XCTAssertEqual(ns.substring(with: firstMark.nsRange), PartialOutputFixture.firstMark)
        XCTAssertEqual(step.announcement, "Warning 1 of \(report.marks.count), line \(firstLine): \(firstMark.message) — \(firstMark.recoveryLine!)")
        XCTAssertEqual(firstLine, 4, "prefix comment, \\documentclass, blank, \\usepackage")
    }

    // MARK: stale invalidation through an edit sequence (no result in between)

    /// Invariant at every revision N+1, N+2, …: a mark still drawn covers the
    /// same bytes it covered at revision N (never an old offset over new
    /// text), a withheld one overlaps the edited region, and together they
    /// account for every diagnostic.
    func testMarksRebaseOntoTheSameBytesOrAreWithheldThroughEdits() throws {
        let compiler = try compiler()
        let compiled = Self.fixtureText
        let env = try fixtureResult(compiler, text: compiled)
        let result = env.payload
        let base = EditorDiagnostics.report(for: result, resultID: env.id, path: "main.tex", compiledText: compiled, currentText: compiled)
        let baseText = Dictionary(uniqueKeysWithValues: base.marks.map { ($0.id, (compiled as NSString).substring(with: $0.nsRange)) })

        func check(_ current: String, _ label: String, expectStale: ClosedRange<Int>) -> EditorDiagnostics.Report {
            let r = EditorDiagnostics.report(for: result, resultID: env.id, path: "main.tex", compiledText: compiled, currentText: current)
            let ns = current as NSString
            XCTAssertEqual(r.marks.count + r.stale.count, result.diagnostics.count, "\(label): every diagnostic is a mark or withheld")
            XCTAssertNotNil(r.edit, label)
            for m in r.marks {
                XCTAssertEqual(ns.substring(with: m.nsRange), baseText[m.id], "\(label): \(m.id) moved onto different text")
            }
            if let edit = r.edit {
                for s in r.stale {
                    XCTAssertTrue(s.identity.source.startByte < edit.oldEndByte && s.identity.source.endByte > edit.startByte
                                  || (s.identity.source.startByte == s.identity.source.endByte && s.identity.source.startByte == edit.startByte),
                                  "\(label): \(s.identity.key) withheld although it does not overlap the edit \(edit.startByte)..<\(edit.oldEndByte)")
                }
            }
            XCTAssertTrue(expectStale.contains(r.stale.count), "\(label): \(r.stale.count) withheld, expected \(expectStale)")
            XCTAssertEqual(Set(r.marks.map(\.id)).count, r.marks.count, "\(label): no duplicate marks")
            return r
        }

        // N+1: insertion before everything — all marks shift, none withheld.
        let n1 = "% revision 2\n" + compiled
        let r1 = check(n1, "insert before all", expectStale: 0...0)
        XCTAssertEqual(r1.marks.count, base.marks.count)
        XCTAssertEqual(try XCTUnwrap(r1.marks.first).nsRange.location, try XCTUnwrap(base.marks.first).nsRange.location + "% revision 2\n".utf16.count)

        // N+2 (on top of N+1, no compile between): replace the 5th "\hwin" with
        // "\hwnotin" — that occurrence is withheld, every other mark keeps its text.
        let math = PartialOutputFixture.mathCommand
        let inGroup = EditorDiagnostics.groups(of: result).first { $0.message.hasPrefix(math + " ") }
        let fifth = try XCTUnwrap(EditorDiagnostics.occurrence(4, of: try XCTUnwrap(inGroup), in: result))
        let fifthNS = try XCTUnwrap(n1.nsRange(utf8Bytes: .init(path: "main.tex", startByte: fifth.startByte + "% revision 2\n".utf8.count,
                                                                  endByte: fifth.endByte + "% revision 2\n".utf8.count)))
        XCTAssertEqual((n1 as NSString).substring(with: fifthNS), math)
        let n2 = (n1 as NSString).replacingCharacters(in: fifthNS, with: "\\hwnotin")
        // Two separate edits since the compile: the single covering region
        // spans from the first inserted byte to the replaced command, so the
        // marks between them are withheld (conservative, never misplaced).
        let r2 = check(n2, "prefix + replace 5th \(math)", expectStale: 1...result.diagnostics.count)
        XCTAssertTrue(r2.stale.contains { $0.identity.source == fifth }, "the edited occurrence is withheld")
        XCTAssertTrue(r2.marks.count >= 1, "marks after the last edit are still drawn")
        XCTAssertNotNil(r2.staleNote)

        // N+3: a block deletion in the middle of the compiled text.
        let bytes = Array(compiled.utf8)
        let mid = bytes.count / 2
        var lo = mid, hi = mid
        while lo > 0, bytes[lo - 1] != 0x0A { lo -= 1 }
        while hi < bytes.count, bytes[hi] != 0x0A { hi += 1 }
        let midRange = try XCTUnwrap(compiled.rangeOfUTF8(start: lo, end: hi))
        var n3 = compiled; n3.removeSubrange(midRange)
        let r3 = check(n3, "delete a middle line", expectStale: 0...result.diagnostics.count)
        let inside = result.diagnostics.filter { $0.source.map { $0.startByte < hi && $0.endByte > lo } ?? false }.count
        XCTAssertEqual(r3.stale.count, inside, "exactly the diagnostics on the deleted line are withheld")
        XCTAssertEqual(r3.marks.count, result.diagnostics.count - inside)

        // N+4: an edit after every span — nothing moves, nothing withheld.
        let r4 = check(compiled + "\n% trailing\n", "append", expectStale: 0...0)
        XCTAssertEqual(r4.marks.map(\.nsRange), base.marks.map(\.nsRange))
    }

    // MARK: a failed follow-up keeps the last marks, flagged, exactly once

    func testFailedFollowUpKeepsMarksFlaggedNotClearedNorDuplicated() throws {
        let compiler = try compiler()
        let text = Self.fixtureText
        let env = try fixtureResult(compiler, text: text)
        let good = env.payload
        let retainedAfterGood = EditorDiagnostics.retained(after: good, resultID: env.id, compiledDocuments: ["main.tex": text], previous: nil)
        XCTAssertEqual(retainedAfterGood?.result.revision, 1)
        XCTAssertFalse(EditorDiagnostics.keepsPreviousMarks(good))

        // A real failed result with no output: the compiler refuses an empty
        // project (revision 2) with one unsourced error and no pages.
        let failedEnv = try Self.compile([], entry: "", revision: 2, id: "hw1-2", with: compiler)
        let failed = failedEnv.payload
        XCTAssertEqual(failed.status, .failed)
        XCTAssertEqual(failed.pages, [])
        XCTAssertEqual(failed.diagnostics.count, 1)
        XCTAssertNil(try XCTUnwrap(failed.diagnostics.first).source)
        XCTAssertTrue(EditorDiagnostics.keepsPreviousMarks(failed))
        let retained = EditorDiagnostics.retained(after: failed, resultID: failedEnv.id, compiledDocuments: [:], previous: retainedAfterGood)
        XCTAssertEqual(retained, retainedAfterGood, "a failure never replaces the retained result")

        // Without retention the marks vanish; with it they are kept and flagged.
        let cleared = EditorDiagnostics.report(for: failed, resultID: failedEnv.id, path: "main.tex", compiledText: nil, currentText: text)
        XCTAssertEqual(cleared.marks, [])
        let kept = EditorDiagnostics.report(for: failed, resultID: failedEnv.id, retained: retained, path: "main.tex",
                                            compiledText: nil, currentText: text)
        XCTAssertEqual(kept.marks.count, good.diagnostics.count)
        XCTAssertEqual(kept.stale, [])
        let carried = EditorDiagnostics.Carried(revision: 1, failedRevision: 2)
        XCTAssertEqual(kept.carried, carried)
        XCTAssertTrue(kept.marks.allSatisfy { $0.carried == carried })
        XCTAssertTrue(kept.marks.allSatisfy { $0.identity.resultID == env.id }, "identities stay those of the retained result")
        XCTAssertEqual(kept.staleNote, "\(good.diagnostics.count) underlines kept from revision 1: revision 2 failed with no output")
        let first = try XCTUnwrap(kept.marks.first, "the retained marks are kept: \(kept.staleNote ?? "no stale note")")
        XCTAssertTrue(first.toolTip.hasSuffix("\n↳ kept from revision 1: revision 2 failed with no output"), first.toolTip)
        XCTAssertTrue(first.spokenDescription.hasSuffix(" — kept from revision 1: revision 2 failed with no output"), first.spokenDescription)
        let fresh = EditorDiagnostics.report(for: good, resultID: env.id, path: "main.tex", compiledText: text, currentText: text)
        XCTAssertEqual(kept.marks.map(\.nsRange), fresh.marks.map(\.nsRange), "same ranges as the good result drew")

        // Edits while the failure stands rebase the kept marks from the
        // retained text (the failed request carried no text at all).
        let edited = "% edited\n" + text
        let keptEdited = EditorDiagnostics.report(for: failed, resultID: failedEnv.id, retained: retained, path: "main.tex",
                                                  compiledText: nil, currentText: edited)
        XCTAssertEqual(keptEdited.marks.count, good.diagnostics.count)
        XCTAssertEqual(try XCTUnwrap(keptEdited.marks.first).nsRange.location, first.nsRange.location + "% edited\n".utf16.count)
        XCTAssertNotNil(keptEdited.edit)

        // A second failure (revision 3) still shows the revision-1 marks exactly once.
        let failed3Env = try Self.compile([], entry: "", revision: 3, id: "hw1-3", with: compiler)
        let retained3 = EditorDiagnostics.retained(after: failed3Env.payload, resultID: failed3Env.id, compiledDocuments: [:], previous: retained)
        XCTAssertEqual(retained3, retainedAfterGood)
        let kept3 = EditorDiagnostics.report(for: failed3Env.payload, resultID: failed3Env.id, retained: retained3, path: "main.tex",
                                             compiledText: nil, currentText: text)
        XCTAssertEqual(kept3.marks.count, good.diagnostics.count, "not duplicated across consecutive failures")
        XCTAssertEqual(Set(kept3.marks.map(\.id)).count, kept3.marks.count)
        XCTAssertEqual(kept3.carried, .init(revision: 1, failedRevision: 3))

        // A failed result that carries its own sourced diagnostic: that one is
        // a fresh mark, the kept ones are flagged, no duplicate.
        var failedWithSpan = failed
        failedWithSpan.revision = 4
        failedWithSpan.diagnostics = [.init(severity: .error, message: "refused span", source: .init(path: "main.tex", startByte: 0, endByte: 1), recovery: nil)]
        let mixed = EditorDiagnostics.report(for: failedWithSpan, resultID: "hw1-4", retained: retained3, path: "main.tex",
                                             compiledText: text, currentText: text)
        XCTAssertEqual(mixed.marks.count, good.diagnostics.count + 1)
        let own = try XCTUnwrap(mixed.marks.first)
        XCTAssertNil(own.carried, "the failed result's own mark is current")
        XCTAssertEqual(own.identity.resultID, "hw1-4")
        XCTAssertEqual(mixed.marks.dropFirst().filter { $0.carried == nil }.count, 0)
        XCTAssertEqual(mixed.staleNote, "\(good.diagnostics.count + 1) underlines kept from revision 1: revision 4 failed with no output")

        // A later result with output replaces the retained one; an `ok` result
        // with no pages (empty document) clears rather than keeps.
        let again = try Self.compile([.init(path: "main.tex", text: text)], revision: 5, id: "hw1-5", with: compiler)
        let retained5 = EditorDiagnostics.retained(after: again.payload, resultID: again.id, compiledDocuments: ["main.tex": text], previous: retained3)
        XCTAssertEqual(retained5?.result.revision, 5)
        let current = EditorDiagnostics.report(for: again.payload, resultID: again.id, retained: retained5, path: "main.tex",
                                               compiledText: text, currentText: text)
        XCTAssertNil(current.carried)
        XCTAssertTrue(current.marks.allSatisfy { $0.carried == nil })
        XCTAssertEqual(current.marks.count, good.diagnostics.count)
        let empty = try Self.compile([.init(path: "main.tex", text: "")], revision: 6, id: "hw1-6", with: compiler)
        XCTAssertNotEqual(empty.payload.status, .failed, "an empty document is not a failure: \(empty.payload.status)")
        XCTAssertFalse(EditorDiagnostics.keepsPreviousMarks(empty.payload))
        let afterEmpty = EditorDiagnostics.report(for: empty.payload, resultID: empty.id,
                                                  retained: EditorDiagnostics.retained(after: empty.payload, resultID: empty.id, compiledDocuments: ["main.tex": ""], previous: retained5),
                                                  path: "main.tex", compiledText: "", currentText: "")
        XCTAssertEqual(afterEmpty.marks, [])
    }

    /// The retention rule without a compiler: a failed result with pages
    /// does not keep (it has its own output), retention never chains.
    func testRetentionRuleIsFailedWithNoPagesOnly() {
        func result(_ status: RuntimeV1.Status, pages: Int, revision: Int) -> RuntimeV1.CompileResult {
            .init(projectId: "p", revision: revision, status: status,
                  pages: (0..<pages).map { .init(number: $0 + 1, widthPt: 100, heightPt: 100, items: []) }, diagnostics: [], pdfPath: nil)
        }
        XCTAssertTrue(EditorDiagnostics.keepsPreviousMarks(result(.failed, pages: 0, revision: 2)))
        XCTAssertFalse(EditorDiagnostics.keepsPreviousMarks(result(.failed, pages: 1, revision: 2)))
        XCTAssertFalse(EditorDiagnostics.keepsPreviousMarks(result(.recovered, pages: 0, revision: 2)))
        XCTAssertFalse(EditorDiagnostics.keepsPreviousMarks(result(.ok, pages: 0, revision: 2)))
        // A first result that is itself a failure retains nothing; the report is the plain one.
        XCTAssertNil(EditorDiagnostics.retained(after: result(.failed, pages: 0, revision: 1), resultID: "a", compiledDocuments: [:], previous: nil))
        let plain = EditorDiagnostics.report(for: result(.failed, pages: 0, revision: 1), resultID: "a", retained: nil, path: "main.tex",
                                             compiledText: nil, currentText: "x")
        XCTAssertEqual(plain, .empty)
        XCTAssertEqual(EditorDiagnostics.Report.empty.staleNote, nil)
    }

    // MARK: follow-up 1: identical diagnostics grouped with a per-occurrence jump

    func testGroupsIdenticalDiagnosticsWithCountAndPerOccurrenceJump() throws {
        let compiler = try compiler()
        let text = Self.fixtureText
        let env = try fixtureResult(compiler, text: text)
        let result = env.payload
        let groups = EditorDiagnostics.groups(of: result, documentOrder: ["main.tex"])
        XCTAssertEqual(groups.reduce(0) { $0 + $1.count }, result.diagnostics.count, "every diagnostic is in exactly one group")
        XCTAssertEqual(Set(groups.flatMap(\.occurrences)).count, result.diagnostics.count)
        XCTAssertLessThan(groups.count, result.diagnostics.count / 2, "the fixture repeats itself: \(groups.count) groups for \(result.diagnostics.count) diagnostics")
        XCTAssertEqual(groups.map(\.id).count, Set(groups.map(\.id)).count)

        let math = PartialOutputFixture.mathCommand
        let n = PartialOutputFixture.mathCommandCount
        let inGroup = try XCTUnwrap(groups.first { $0.message == "\(math) is not supported in math mode" })
        XCTAssertEqual(inGroup.count, n, "\(math) three times per section")
        XCTAssertEqual(inGroup.severity, .error)
        XCTAssertEqual(inGroup.title, "\(n)× \(math) is not supported in math mode")
        XCTAssertNotNil(inGroup.recovery, "all \(n) share the compiler's recovery note")
        // Occurrences are in document order and each jumps to its own "\hwin".
        let starts = try inGroup.occurrences.map { try XCTUnwrap(result.diagnostics[$0].source, "occurrence \($0) is sourced").startByte }
        XCTAssertEqual(starts, starts.sorted())
        XCTAssertEqual(Set(starts).count, n)
        let bytes = Array(text.utf8)
        for k in 0..<n {
            let src = try XCTUnwrap(EditorDiagnostics.occurrence(k, of: inGroup, in: result))
            XCTAssertEqual(String(decoding: bytes[src.startByte..<src.endByte], as: UTF8.self), math, "occurrence \(k + 1)")
            let label = EditorDiagnostics.occurrenceLabel(k, of: inGroup, in: result, texts: ["main.tex": text])
            let line = try XCTUnwrap(EditorDiagnostics.lineNumber(ofByte: src.startByte, in: text))
            XCTAssertEqual(label, "\(k + 1) of \(n): main.tex line \(line)")
            XCTAssertGreaterThan(line, 1, "the multi-byte comment is line 1")
        }
        XCTAssertNil(EditorDiagnostics.occurrence(n, of: inGroup, in: result))
        let fourth = try XCTUnwrap(starts.count > 3 ? starts[3] : nil, "a fourth \(math) occurrence")
        XCTAssertEqual(EditorDiagnostics.occurrenceLabel(3, of: inGroup, in: result), "4 of \(n): main.tex bytes \(fourth)..<\(fourth + math.utf8.count)")
        // Groups list errors, then warnings, then gaps; within a bucket, first
        // occurrence stays in document order. Singles keep the plain message.
        func bucket(_ g: EditorDiagnostics.Group) -> Int { EditorDiagnostics.listBucket(result.diagnostics[g.first]) }
        XCTAssertEqual(groups.map(bucket), groups.map(bucket).sorted(), "errors, then warnings, then gaps")
        for b in 0...2 {
            let starts = try groups.filter { bucket($0) == b }.map {
                try XCTUnwrap(result.diagnostics[$0.first].source, "group \($0.id) is sourced").startByte
            }
            XCTAssertEqual(starts, starts.sorted(), "bucket \(b) stays in document order")
        }
        XCTAssertNotNil(groups.first { $0.message.contains("microtype") }, "the package warning is still grouped: \(groups.first?.message ?? "nil")")
        if let single = groups.first(where: { $0.count == 1 }) { XCTAssertEqual(single.title, single.message) }
        // A group's first occurrence is what the row's explanation/quick fix use.
        XCTAssertEqual(inGroup.first, inGroup.occurrences.first)
        XCTAssertEqual(result.diagnostics[inGroup.first].source?.startByte, starts.first)
    }

    /// Grouping without a compiler: severity separates groups with one
    /// message, unsourced occurrences sort last, mixed recoveries drop the
    /// shared note, documents follow the project order.
    func testGroupingRulesAcrossSeverityDocumentsAndUnsourced() {
        let diags: [RuntimeV1.Diagnostic] = [
            .init(severity: .warning, message: "m", source: .init(path: "chapter.tex", startByte: 5, endByte: 6), recovery: "a"),
            .init(severity: .error, message: "m", source: .init(path: "main.tex", startByte: 9, endByte: 10), recovery: nil),
            .init(severity: .warning, message: "m", source: nil, recovery: "b"),
            .init(severity: .warning, message: "m", source: .init(path: "main.tex", startByte: 2, endByte: 3), recovery: "a"),
            .init(severity: .warning, message: "m", source: .init(path: "main.tex", startByte: 0, endByte: 1), recovery: "a"),
            .init(severity: .error, message: "z", source: .init(path: "main.tex", startByte: 1, endByte: 2), recovery: nil),
        ]
        let result = RuntimeV1.CompileResult(projectId: "p", revision: 1, status: .recovered, pages: [], diagnostics: diags, pdfPath: nil)
        let groups = EditorDiagnostics.groups(of: result, documentOrder: ["main.tex", "chapter.tex"])
        XCTAssertEqual(groups.map(\.id), ["error:z", "error:m", "warning:m"])
        let warnings = groups[2]
        XCTAssertEqual(warnings.occurrences, [4, 3, 0, 2], "main.tex by start, then chapter.tex, unsourced last")
        XCTAssertNil(warnings.recovery, "mixed recovery notes: none shared")
        XCTAssertEqual(warnings.title, "4× m")
        XCTAssertEqual(groups[1].title, "m")
        XCTAssertEqual(EditorDiagnostics.occurrenceLabel(3, of: warnings, in: result), "4 of 4: no source")
        XCTAssertEqual(EditorDiagnostics.occurrenceLabel(2, of: warnings, in: result, texts: ["chapter.tex": "ab\ncd\nef"]), "3 of 4: chapter.tex line 2")
        XCTAssertEqual(EditorDiagnostics.lineNumber(ofByte: 99, in: "ab"), nil)
        XCTAssertEqual(EditorDiagnostics.lineNumber(ofByte: 2, in: "ab"), 1)
        XCTAssertEqual(EditorDiagnostics.lineNumber(ofByte: 3, in: "ab\n"), 2)
        XCTAssertEqual(EditorDiagnostics.groups(of: .init(projectId: "p", revision: 1, status: .ok, pages: [], diagnostics: [], pdfPath: nil)), [])
    }
}
