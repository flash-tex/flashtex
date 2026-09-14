import AppKit
import Combine
import Foundation
import PDFKit
import SwiftUI
import FlashTeXProtocol

/// Shadow compile of a capture proposal in document context (transfer-v1:
/// "Show the proposed edit and resulting diagnostics before approval").
///
/// The review sheet owns one of these. It builds the document set WITH the
/// proposed insertion applied at the pinned anchor (same rules as approval:
/// `Insertion.resolve` + `Insertion.insertionText`), compiles it on a SEPARATE
/// worker process (same compiler executable, request ids `preview-N`, project
/// id `<project>-preview`), and compiles the UNMODIFIED documents once per
/// document revision as a baseline so "new diagnostics introduced by this
/// insertion" is a real diff rather than a guess. Nothing here ever writes to
/// the editor buffer or to `ShellModel.result` / `editorRevision`.
@MainActor
final class ProposalPreview: ObservableObject {
    static let debounceInterval: TimeInterval = 0.3

    /// Snapshot of the model state the preview needs (read-only copy).
    struct Input: Equatable {
        var documents: [RuntimeV1.Document]
        var entryPath: String
        var anchor: InsertionAnchor?
        var editorRevision: Int
        var projectId: String

        @MainActor init(model: ShellModel) {
            documents = model.documents
            entryPath = model.activePath
            anchor = model.anchor
            editorRevision = model.editorRevision
            projectId = model.projectId
        }

        init(documents: [RuntimeV1.Document], entryPath: String, anchor: InsertionAnchor?,
             editorRevision: Int, projectId: String) {
            self.documents = documents; self.entryPath = entryPath; self.anchor = anchor
            self.editorRevision = editorRevision; self.projectId = projectId
        }
    }

    /// The documents with the insertion applied plus where it landed.
    struct Shadow: Equatable {
        var documents: [RuntimeV1.Document]
        var path: String
        /// Byte offset in the ORIGINAL text where the insertion text starts.
        var insertByte: Int
        /// Full insertion text (`Insertion.insertionText`), including any added newlines.
        var insertedText: String
        var insertedLength: Int { insertedText.utf8.count }
        /// Range of the inserted text in the shadow document.
        var insertedRange: Range<Int> { insertByte..<(insertByte + insertedLength) }
        /// Byte offset in the shadow document where the trimmed proposal body starts.
        var bodyStart: Int { insertByte + (insertedText.hasPrefix("\n") ? 1 : 0) }
        /// Range of the trimmed proposal body (the inserted text without the
        /// newlines `Insertion.insertionText` adds) in the shadow document.
        var bodyRange: Range<Int> { bodyStart..<(insertedRange.upperBound - (insertedText.hasSuffix("\n") ? 1 : 0)) }
        var shadowText: String { documents.first { $0.path == path }?.text ?? "" }
    }

    struct Finding: Equatable, Identifiable {
        var id: Int
        var diagnostic: RuntimeV1.Diagnostic
        /// Shadow text under the diagnostic's range (with a little context), for
        /// display: the sheet's TextEditor cannot select a range.
        var fragment: String?
        /// Offset of the diagnostic within the proposal body (UTF-8 bytes) when it
        /// lies inside the inserted text; nil when it is elsewhere.
        var proposalOffset: Int?
    }

    struct Report: Equatable {
        var status: RuntimeV1.Status
        var pageCount: Int
        /// Pages in the shadow result minus pages in the baseline (unmodified)
        /// result compiled by the same worker; nil when no baseline is available.
        var pageDelta: Int?
        /// Diagnostics whose range intersects the inserted text, its enclosing
        /// lines, or one line either side.
        var nearby: [Finding]
        /// Diagnostics not present in the baseline compile (message + range
        /// relative to the insertion).
        var new: [Finding]
        var unrelatedCount: Int
        /// Number of the shadow page whose text items map the inserted text
        /// (nil when nothing maps it, e.g. a failed compile).
        var insertionPage: Int?
        var newErrorCount: Int { new.filter { $0.diagnostic.severity == .error }.count }
    }

    enum State: Equatable {
        case idle
        case noCompiler
        case notPreviewable(String)
        case compiling
        case ready(Report)
        case failed(String)
    }

    @Published private(set) var state: State = .idle
    @Published private(set) var shadow: Shadow?
    @Published var highlightedFragment: String?
    /// Thumbnail of the shadow page containing the insertion, drawn from the
    /// shadow result with `PDFExport.render` (only that page) via PDFKit.
    @Published private(set) var thumbnail: NSImage?
    /// Requests sent so far, by kind (tests assert debounce/coalescing).
    @Published private(set) var shadowCompileCount = 0
    @Published private(set) var baselineCompileCount = 0

    let executable: URL?
    let arguments: [String]
    private var worker: WorkerClient?
    private var nextRequestID = 1
    private var debounce: DispatchWorkItem?
    private var latest: (input: Input, latex: String)?
    private var compiled: (input: Input, latex: String)?
    private enum Kind { case baseline, shadow }
    private struct InFlight { var kind: Kind; var projectId: String; var revision: Int; var shadow: Shadow? }
    private var inFlight: [String: InFlight] = [:]
    private var baselineKey: (documents: [RuntimeV1.Document], entryPath: String)?
    private var baseline: RuntimeV1.CompileResult?
    private var shadowResult: (shadow: Shadow, result: RuntimeV1.CompileResult, requestId: String)?

    /// `executable == nil` means no compiler is attached: the preview reports
    /// that honestly and never launches anything.
    init(executable: URL?, arguments: [String] = []) {
        self.executable = executable
        self.arguments = arguments
        if executable == nil { state = .noCompiler }
    }

    var workerIsRunning: Bool { worker?.isRunning == true }
    var isInFlight: Bool { !inFlight.isEmpty }
    var debounceIdle: Bool { debounce == nil }

    var statusText: String {
        switch state {
        case .idle: return "preview: waiting for edits"
        case .noCompiler: return "no compiler attached — cannot preview"
        case .notPreviewable(let why): return "preview unavailable: \(why)"
        case .compiling: return "compiling preview…"
        case .failed(let why): return "preview failed: \(why)"
        case .ready(let r):
            let delta = r.pageDelta.map { $0 >= 0 ? "+\($0)" : "\($0)" } ?? "?"
            return "preview: \(r.status.rawValue), \(delta) pages, \(r.new.count) new diagnostic\(r.new.count == 1 ? "" : "s")"
        }
    }

    var hasNewErrors: Bool {
        if case .ready(let r) = state { return r.newErrorCount > 0 }
        return false
    }

    // MARK: driving

    /// Records the latest proposal text and model snapshot; compiles after the
    /// debounce interval. Bursts coalesce into one shadow compile (plus one
    /// baseline compile per document revision).
    func update(from model: ShellModel, latex: String) {
        update(input: Input(model: model), latex: latex)
    }

    func update(input: Input, latex: String) {
        latest = (input, latex)
        guard executable != nil else { state = .noCompiler; return }
        switch Self.makeShadow(input: input, latex: latex) {
        case .success(let s):
            shadow = s
            if case .failed = state {} else { state = .compiling }
        case .failure(let why):
            shadow = nil
            state = .notPreviewable(why.message)
            debounce?.cancel()
            return
        }
        debounce?.cancel()
        let item = DispatchWorkItem { [weak self] in self?.compileLatest() }
        debounce = item
        DispatchQueue.main.asyncAfter(deadline: .now() + Self.debounceInterval, execute: item)
    }

    /// Re-sends after a worker fault; no-op when nothing is pending.
    func retry() {
        if case .failed = state { state = .compiling }
        compileLatest()
    }

    /// Terminates the preview worker. The sheet calls this on dismiss.
    func close() {
        debounce?.cancel()
        debounce = nil
        worker?.terminate()
        worker = nil
        inFlight.removeAll()
    }

    private func compileLatest() {
        debounce = nil
        guard let latest, let executable else { return }
        if !inFlight.isEmpty { return } // coalesce: the newest text goes out when the current result lands
        if let compiled, compiled.input == latest.input, compiled.latex == latest.latex, shadowResult != nil { return }
        guard case .success(let s) = Self.makeShadow(input: latest.input, latex: latest.latex) else { return }
        if worker == nil || worker?.isRunning != true {
            do {
                worker = try WorkerClient(executable: executable, arguments: arguments) { [weak self] event in
                    self?.handle(event)
                }
            } catch {
                state = .failed("launch failed: \(error.localizedDescription)")
                return
            }
        }
        guard let worker else { return }
        let projectId = latest.input.projectId + "-preview"
        do {
            if baselineKey?.documents != latest.input.documents || baselineKey?.entryPath != latest.input.entryPath || baseline == nil {
                baseline = nil
                baselineKey = (latest.input.documents, latest.input.entryPath)
                let (id, revision) = nextRequest()
                let req = RuntimeV1.CompileRequest(projectId: projectId, revision: revision,
                                                   entryPath: latest.input.entryPath, documents: latest.input.documents)
                try worker.send(req, id: id)
                inFlight[id] = InFlight(kind: .baseline, projectId: projectId, revision: req.revision, shadow: nil)
                baselineCompileCount += 1
            }
            let (id, revision) = nextRequest()
            let req = RuntimeV1.CompileRequest(projectId: projectId, revision: revision,
                                               entryPath: latest.input.entryPath, documents: s.documents)
            try worker.send(req, id: id)
            inFlight[id] = InFlight(kind: .shadow, projectId: projectId, revision: req.revision, shadow: s)
            shadowCompileCount += 1
            shadowResult = nil
            compiled = latest
            state = .compiling
        } catch {
            state = .failed("send failed: \(error.localizedDescription)")
        }
    }

    /// Request ids are `preview-N`; the revision field carries N too so a
    /// mismatched reply can be rejected like the main worker's.
    private func nextRequest() -> (id: String, revision: Int) {
        defer { nextRequestID += 1 }
        return ("preview-\(nextRequestID)", nextRequestID)
    }

    private func handle(_ event: WorkerClient.Event) {
        switch event {
        case .result(let env):
            guard let sent = inFlight.removeValue(forKey: env.id) else { return } // not ours; never applied
            let r = env.payload
            guard r.projectId == sent.projectId, r.revision == sent.revision else {
                state = .failed("protocol violation: \(env.id) answered project \(r.projectId) revision \(r.revision)")
                return
            }
            switch sent.kind {
            case .baseline: baseline = r
            case .shadow: if let s = sent.shadow { shadowResult = (s, r, env.id) }
            }
            if inFlight.isEmpty { finish() }
        case .error(let id, let message):
            inFlight.removeValue(forKey: id)
            state = .failed("worker error: \(message)")
        case .protocolViolation(let message):
            inFlight.removeAll()
            state = .failed("protocol violation: \(message)")
        case .stderr, .displayList: // the shadow compile never requests display-list-v2
            break
        case .exited(let code):
            let lost = !inFlight.isEmpty
            inFlight.removeAll()
            worker = nil
            // An idle exit after close() changes nothing; a crash mid-compile is
            // reported and Retry relaunches the worker.
            if lost { state = .failed("preview worker exited (\(code))") }
        }
    }

    private func finish() {
        if let (s, r, _) = shadowResult {
            let report = Self.report(shadow: s, result: r, baseline: baseline)
            thumbnail = report.insertionPage.flatMap { Self.thumbnail(of: $0, in: r) }
            state = .ready(report)
        }
        // A newer edit arrived while compiling: go again.
        if let latest, let compiled, latest.input != compiled.input || latest.latex != compiled.latex {
            compileLatest()
        }
    }

    // MARK: pure pieces (tested without a worker)

    struct ShadowError: Error, Equatable { var message: String }

    /// Applies the proposal at the anchor exactly as approval would, in a copy.
    static func makeShadow(input: Input, latex: String) -> Result<Shadow, ShadowError> {
        guard let anchor = input.anchor else { return .failure(.init(message: "no insertion point pinned")) }
        guard let doc = input.documents.first(where: { $0.path == anchor.path }) else {
            return .failure(.init(message: "anchor document \(anchor.path) is not open"))
        }
        let byte: Int
        switch Insertion.resolve(anchor, in: doc.text, revision: input.editorRevision) {
        case .exact(let b), .rebased(let b): byte = b
        case .needsReselection(let why): return .failure(.init(message: why))
        }
        guard !latex.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return .failure(.init(message: "proposal is empty"))
        }
        guard let idx = doc.text.rangeOfUTF8(start: byte, end: byte)?.lowerBound else {
            return .failure(.init(message: "anchor offset is not a valid position"))
        }
        let insert = Insertion.insertionText(latex, into: doc.text, atByte: byte)
        var text = doc.text
        text.insert(contentsOf: insert, at: idx)
        var docs = input.documents
        if let i = docs.firstIndex(where: { $0.path == anchor.path }) { docs[i].text = text }
        return .success(Shadow(documents: docs, path: anchor.path, insertByte: byte, insertedText: insert))
    }

    /// The inserted range widened to its enclosing lines plus one line before
    /// and after, in shadow-document bytes.
    static func neighborhood(of inserted: Range<Int>, in text: String) -> Range<Int> {
        let bytes = Array(text.utf8)
        var start = min(inserted.lowerBound, bytes.count)
        var linesBack = 0
        while start > 0 {
            if bytes[start - 1] == UInt8(ascii: "\n") {
                linesBack += 1
                if linesBack > 1 { break }
            }
            start -= 1
        }
        var end = min(inserted.upperBound, bytes.count)
        var linesForward = 0
        while end < bytes.count {
            if bytes[end] == UInt8(ascii: "\n") {
                linesForward += 1
                if linesForward > 1 { break }
            }
            end += 1
        }
        return start..<end
    }

    private static func intersects(_ a: Range<Int>, _ b: Range<Int>) -> Bool {
        a.lowerBound <= b.upperBound && b.lowerBound <= a.upperBound
    }

    /// Diagnostics in `shadow` that have no counterpart in `baseline`. Ranges
    /// after the insertion are shifted back by the inserted length before
    /// comparing; anything overlapping the inserted text is new by definition.
    static func newDiagnostics(baseline: [RuntimeV1.Diagnostic], shadow diags: [RuntimeV1.Diagnostic],
                               shadow: Shadow) -> [RuntimeV1.Diagnostic] {
        func key(_ d: RuntimeV1.Diagnostic, _ range: Range<Int>?) -> String {
            let loc = range.map { "\(d.source!.path):\($0.lowerBound)..<\($0.upperBound)" } ?? "-"
            return "\(d.severity.rawValue)|\(d.message)|\(loc)"
        }
        var counts: [String: Int] = [:]
        for d in baseline {
            counts[key(d, d.source.map { $0.startByte..<max($0.startByte, $0.endByte) }), default: 0] += 1
        }
        var fresh: [RuntimeV1.Diagnostic] = []
        for d in diags {
            var normalized: Range<Int>? = nil
            if let s = d.source {
                let r = s.startByte..<max(s.startByte, s.endByte)
                if s.path == shadow.path {
                    if r.lowerBound >= shadow.insertedRange.upperBound {
                        normalized = (r.lowerBound - shadow.insertedLength)..<(r.upperBound - shadow.insertedLength)
                    } else if r.upperBound <= shadow.insertedRange.lowerBound {
                        normalized = r
                    } else {
                        fresh.append(d); continue
                    }
                } else {
                    normalized = r
                }
            }
            let k = key(d, normalized)
            if let n = counts[k], n > 0 { counts[k] = n - 1 } else { fresh.append(d) }
        }
        return fresh
    }

    static func report(shadow: Shadow, result: RuntimeV1.CompileResult, baseline: RuntimeV1.CompileResult?) -> Report {
        let text = shadow.shadowText
        let nb = neighborhood(of: shadow.insertedRange, in: text)
        let fresh = newDiagnostics(baseline: baseline?.diagnostics ?? [], shadow: result.diagnostics, shadow: shadow)
        func finding(_ i: Int, _ d: RuntimeV1.Diagnostic) -> Finding {
            var fragment: String? = nil
            var offset: Int? = nil
            if let s = d.source, s.path == shadow.path {
                let lo = max(0, min(s.startByte, text.utf8.count)), hi = max(lo, min(s.endByte, text.utf8.count))
                let ctxLo = max(0, lo - 12), ctxHi = min(text.utf8.count, max(hi, lo + 1) + 12)
                if let r = text.rangeOfUTF8(start: ctxLo, end: ctxHi) {
                    fragment = String(text[r]).replacingOccurrences(of: "\n", with: "⏎")
                }
                if lo >= shadow.bodyStart, lo < shadow.insertedRange.upperBound { offset = lo - shadow.bodyStart }
            }
            return Finding(id: i, diagnostic: d, fragment: fragment, proposalOffset: offset)
        }
        var nearby: [Finding] = []
        var unrelated = 0
        for (i, d) in result.diagnostics.enumerated() {
            if let s = d.source, s.path == shadow.path, intersects(s.startByte..<max(s.startByte, s.endByte), nb) {
                nearby.append(finding(i, d))
            } else if d.source == nil, fresh.contains(d) {
                nearby.append(finding(i, d)) // unlocated but new: show it rather than hide it
            } else {
                unrelated += 1
            }
        }
        let new = fresh.enumerated().map { finding(1000 + $0.offset, $0.element) }
        return Report(status: result.status, pageCount: result.pages.count,
                      pageDelta: baseline.map { result.pages.count - $0.pages.count },
                      nearby: nearby, new: new, unrelatedCount: unrelated,
                      insertionPage: insertionPage(in: result, shadow: shadow))
    }

    /// First page with a text item whose source range intersects the inserted
    /// text (touching counts, so an item ending exactly at the insertion point
    /// still locates the page). Nil when no item maps the insertion.
    static func insertionPage(in result: RuntimeV1.CompileResult, shadow: Shadow) -> Int? {
        for page in result.pages {
            for case .text(let t) in page.items {
                guard let s = t.source, s.path == shadow.path else { continue }
                if intersects(s.startByte..<max(s.startByte, s.endByte), shadow.insertedRange) { return page.number }
            }
        }
        return nil
    }

    /// Renders only `pageNumber` of `result` through `PDFExport` and asks
    /// PDFKit for a thumbnail. Nil when the page is missing or PDFKit refuses.
    static func thumbnail(of pageNumber: Int, in result: RuntimeV1.CompileResult,
                          width: CGFloat = 180) -> NSImage? {
        guard let page = result.pages.first(where: { $0.number == pageNumber }), page.widthPt > 0 else { return nil }
        var single = result
        single.pages = [page]
        let data = PDFExport.render(single)
        guard let doc = PDFDocument(data: data), let pdfPage = doc.page(at: 0) else { return nil }
        let size = NSSize(width: width, height: width * page.heightPt / page.widthPt)
        return pdfPage.thumbnail(of: size, for: .mediaBox)
    }
}

/// Status line + new-diagnostic list rendered under the sheet's LaTeX editor.
/// "Show" highlights the offending fragment here (a shadow buffer; the real
/// editor is never touched, and SwiftUI's TextEditor cannot select a range).
struct ProposalPreviewView: View {
    @ObservedObject var preview: ProposalPreview

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.xs) {
            HStack(spacing: DS.Space.s) {
                statusGlyph
                Text(preview.statusText).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                Spacer()
                if case .failed = preview.state { Button("Retry") { preview.retry() }.controlSize(.mini) }
            }
            .help("Compiled on a separate preview worker with the proposal inserted at the anchor; the editor buffer is untouched.")
            if case .ready(let r) = preview.state {
                ForEach(r.new) { f in
                    HStack(alignment: .top, spacing: DS.Space.s) {
                        Image(systemName: f.diagnostic.severity == .error ? "xmark.octagon.fill" : "exclamationmark.triangle.fill")
                            .foregroundStyle(f.diagnostic.severity == .error ? .red : .orange).font(.caption)
                        VStack(alignment: .leading, spacing: DS.Size.hairline) {
                            Text(f.diagnostic.message).font(.caption)
                            Text(f.diagnostic.recovery.map { "↳ recovery: \($0)" } ?? "↳ no provisional rendering")
                                .font(.caption2).foregroundStyle(.secondary)
                            if let o = f.proposalOffset {
                                Text("at proposal byte \(o)").font(.caption2).foregroundStyle(.tertiary)
                            }
                        }
                        Spacer()
                        if f.fragment != nil { Button("Show") { preview.highlightedFragment = f.fragment }.controlSize(.mini) }
                    }
                }
                if let frag = preview.highlightedFragment {
                    Text(frag).font(.system(.caption, design: .monospaced))
                        .padding(DS.Space.xs).background(DS.Colors.statusModified.opacity(DS.State.selectionTintOpacity)).cornerRadius(DS.Radius.control)
                }
                let pre = r.nearby.count - r.nearby.filter { n in r.new.contains { $0.diagnostic == n.diagnostic } }.count
                if pre > 0 || r.unrelatedCount > 0 {
                    Text("\(pre) pre-existing near the insertion, \(r.unrelatedCount) elsewhere (unchanged)")
                        .font(.caption2).foregroundStyle(.tertiary)
                }
                if let image = preview.thumbnail, let page = r.insertionPage {
                    HStack(alignment: .top, spacing: DS.Space.s) {
                        Image(nsImage: image).resizable().aspectRatio(contentMode: .fit)
                            .frame(maxWidth: DS.Size.imageTile).border(.separator)
                        Text("shadow page \(page) of \(r.pageCount), with the proposal inserted (not the live preview)")
                            .font(.caption2).foregroundStyle(.tertiary)
                    }
                }
            }
        }
    }

    private var statusGlyph: some View {
        Group {
            switch preview.state {
            case .compiling: ProgressView().controlSize(.mini)
            case .ready(let r):
                Image(systemName: r.newErrorCount > 0 ? "xmark.octagon.fill" : (r.status == .ok ? "checkmark.circle.fill" : "exclamationmark.triangle.fill"))
                    .foregroundStyle(r.newErrorCount > 0 ? .red : (r.status == .ok ? .green : .orange))
            case .failed, .noCompiler: Image(systemName: "bolt.slash").foregroundStyle(DS.Colors.severityWarning)
            case .idle, .notPreviewable: Image(systemName: "eye.slash").foregroundStyle(.secondary)
            }
        }
        .font(.caption)
    }
}

/// Warning glyph shown next to Approve when the preview introduced errors.
/// Approval is never blocked: the reviewer decides.
struct ProposalApproveWarning: View {
    @ObservedObject var preview: ProposalPreview
    var body: some View {
        if preview.hasNewErrors, case .ready(let r) = preview.state {
            Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(DS.Colors.severityWarning)
                .help("Preview compile reports \(r.newErrorCount) new error\(r.newErrorCount == 1 ? "" : "s") from this insertion. You may still approve.")
        }
    }
}
