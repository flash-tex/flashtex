import AppKit
import CryptoKit
import Foundation
import Observation
import FlashTeXDisplayListV3
import FlashTeXPreviewV3

/// The engine-v3 preview for the open document (flag-gated, EngineV3Host.swift):
/// one `flashtex-host` per session, one incremental connection, the editor's
/// changes sent as byte `edits` on every keystroke (the host's preemption is
/// the only debounce: a newer COMPILE supersedes the running one), pages
/// replaced in place as they arrive (the edited page first), `PAGES` stale
/// ranges shown as stale.
///
/// Threading: the socket's reader thread decodes every frame, binds
/// resources and prepares pages (font programs load there); rasterising is
/// on `EngineV3PagesView`'s queue; the main thread only swaps layer contents
/// and updates this model. Nothing draws on the main thread.
@MainActor
@Observable
final class EngineV3Session {
    enum Phase: Equatable {
        case idle
        /// Host starting: preparing the format (the first use builds it from
        /// the user's TeX Live), warming up.
        case starting(since: Date)
        case ready
        case failed(String)
    }

    private(set) var phase: Phase = .idle
    /// What the host prepared: which TeX Live, the format's status.
    private(set) var environmentNote = ""
    /// Last compile outcome, for the pane's status line.
    private(set) var statusNote = ""
    private(set) var pageCount = 0
    private(set) var errorCount = 0
    private(set) var warningCount = 0
    /// Bumped whenever the page list or a page size changes (the view re-lays out).
    private(set) var layoutRevision = 0

    @ObservationIgnored private(set) var pages: [Int: DL3PreparedPage] = [:]
    @ObservationIgnored private(set) var stale: Set<Int> = []
    @ObservationIgnored private(set) var forms: [UInt32: DL3PreparedPage] = [:]
    /// Pages whose PDF rendering replaces the display list (INCOMPLETE, or a
    /// resource that did not resolve), rendered from `DONE.pdf`.
    @ObservationIgnored private(set) var pdfFallback: [Int: CGPDFPage] = [:]
    @ObservationIgnored weak var view: EngineV3PagesView?
    @ObservationIgnored weak var model: ShellModel?

    @ObservationIgnored private var host: EngineV3HostProcess?
    @ObservationIgnored private var connection: DL3Connection?
    @ObservationIgnored private var nextID = 1
    @ObservationIgnored private var root: URL?
    @ObservationIgnored private var main = "main.tex"
    @ObservationIgnored private var outputDir: URL?
    /// The text of each document as the host's copy of the project holds it.
    @ObservationIgnored private var sentTexts: [String: String] = [:]
    @ObservationIgnored private var project: EngineV3Mirror?
    /// The page the view shows (sent as `viewport`).
    @ObservationIgnored var visiblePage = 0
    @ObservationIgnored let latency = EngineV3Latency()
    @ObservationIgnored private var keyMonitor: Any?
    @ObservationIgnored private var lastKeyNs: UInt64 = 0
    /// Set by a scripted bench just before it edits the text view.
    @ObservationIgnored var nextKeystrokeNs: UInt64?
    @ObservationIgnored let logDone = ProcessInfo.processInfo.environment["FLASHTEX_V3_LOG_DONE"] == "1"
    @ObservationIgnored var log: (String) -> Void = { FlashTeXLog.write("engine-v3: " + $0) }

    init() {}

    // MARK: lifecycle

    /// Starts the host (once) and compiles the model's documents.
    func start(model: ShellModel) {
        self.model = model
        if keyMonitor == nil {
            keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] e in
                self?.lastKeyNs = MonotonicClock.ns(fromUptimeSeconds: e.timestamp)
                return e
            }
        }
        switch phase {
        case .starting: return
        case .ready: compile(model: model, reason: "start"); return
        case .idle, .failed: break
        }
        guard let exe = EngineV3.locateHost() else {
            phase = .failed("flashtex-host not found. Build it (cargo build --release -p flashtex-engine --bin flashtex-host) or set FLASHTEX_HOST / the \(EngineV3.hostPathKey) default.")
            return
        }
        phase = .starting(since: Date())
        environmentNote = "Preparing the pdfLaTeX format from your TeX Live (the first use builds it; a few seconds)…"
        log("starting \(exe.path)")
        do {
            host = try EngineV3HostProcess(executable: exe) { [weak self] event in
                EngineV3Session.onMain { self?.hostEvent(event) }
            }
        } catch {
            phase = .failed("could not start \(exe.lastPathComponent): \(error.localizedDescription)")
        }
    }

    func stop() {
        connection?.bye()
        connection = nil
        host?.terminate()
        host = nil
        phase = .idle
        sentTexts = [:]
        pages = [:]; stale = []; forms = [:]; pdfFallback = [:]
        pageCount = 0
        layoutRevision &+= 1
        if let keyMonitor { NSEvent.removeMonitor(keyMonitor) }
        keyMonitor = nil
    }

    private func hostEvent(_ e: EngineV3HostProcess.Event) {
        switch e {
        case .line(let l):
            log(l)
        case .prepared(let j):
            let texlive = j["texlive"]?.string ?? "no TeX Live found"
            let formats = (j["formats"]?.array ?? []).map { f in
                "\(f["name"]?.string ?? "?") \(f["status"]?.string ?? "?")\(f["error"]?.string.map { ": " + $0 } ?? "")"
            }.joined(separator: ", ")
            environmentNote = "TeX Live: \(texlive) — format \(formats)"
            if (j["formats"]?.array ?? []).contains(where: { $0["status"]?.string == "failed" }) {
                phase = .failed("The pdfLaTeX format could not be prepared: \(formats)")
            }
        case .listening(let socket):
            connect(socket: socket)
        case .exited(let status):
            log("host exited with status \(status)")
            connection = nil
            if phase != .idle { phase = .failed("The preview engine stopped (status \(status)). Toggle the preview to restart it.") }
        }
    }

    private func connect(socket: String) {
        let ref = EngineV3WeakRef(self)
        DispatchQueue.global(qos: .userInitiated).async {
            do {
                let c = try DL3Connection(socketPath: socket, client: "FlashTeX (engine-v3 preview)")
                let reader = EngineV3Reader(cache: .shared)
                c.start(onEvent: { ev in
                    guard let out = reader.handle(ev) else { return }
                    EngineV3Session.onMain { ref.value?.apply(out) }
                }, onClose: { err in
                    EngineV3Session.onMain { ref.value?.closed(err) }
                })
                EngineV3Session.onMain {
                    guard let self = ref.value else { return }
                    self.connection = c
                    if case .failed = self.phase {} else { self.phase = .ready }
                    self.log("connected: \(c.hello["server"]?.string ?? "?"), \(c.hello["engine"]?.string ?? "?")")
                    if let model = self.model { self.compile(model: model, reason: "open") }
                }
            } catch {
                EngineV3Session.onMain { ref.value?.phase = .failed("could not connect to the preview engine: \(error)") }
            }
        }
    }

    private func closed(_ err: DL3Error?) {
        log("connection closed\(err.map { ": \($0)" } ?? "")")
        connection = nil
        if let err, phase != .idle { phase = .failed("The preview connection broke (\(err)). Toggle the preview to restart it.") }
    }

    // MARK: edits → COMPILE

    /// The editor changed (a keystroke): sends what changed as byte edits.
    /// `keystrokeNs`: when the change was typed (nil: the last key event).
    func textChanged(model: ShellModel, activeText: String? = nil, keystrokeNs: UInt64? = nil) {
        guard phase == .ready, connection != nil else { return }
        let now = MonotonicClock.nowNs()
        let key = keystrokeNs ?? nextKeystrokeNs ?? (now &- lastKeyNs < 200_000_000 ? lastKeyNs : now)
        nextKeystrokeNs = nil
        latency.noteHook(at: now)
        compile(model: model, reason: "edit", keystrokeNs: key, activeText: activeText)
    }

    /// `activeText`: the active document's new text when the model has not
    /// stored it yet (the edit hook runs first).
    func compile(model: ShellModel, reason: String, keystrokeNs: UInt64? = nil, activeText: String? = nil) {
        guard let connection else { return }
        var docs = model.documents
        if let activeText, let i = docs.firstIndex(where: { $0.path == model.activePath }) { docs[i].text = activeText }
        let entry = model.project.entryPath
        // The host compiles a copy of the project (EngineV3Mirror): it writes
        // the editor's text to its files, and must never write the user's.
        let projectRoot = model.project.projectRoot
        if project == nil || project?.source != projectRoot {
            project = EngineV3Mirror(source: projectRoot)
            sentTexts = [:]
            pages = [:]; stale = []; forms = [:]; pdfFallback = [:]; pageCount = 0; layoutRevision &+= 1
        }
        guard let project else { return }
        // Linking the project's other files walks its directory: on open and
        // explicit compiles, not per keystroke.
        if reason != "edit" { project.sync(except: Set(docs.map(\.path))) }
        root = project.root
        main = entry
        outputDir = project.output
        var req = DL3CompileRequest(id: nextID, root: project.root.path, main: entry)
        nextID += 1
        req.outputDir = project.output.path
        req.jobname = (entry as NSString).lastPathComponent.replacingOccurrences(of: ".tex", with: "")
        req.haveFonts = DL3ResourceCache.shared.heldFontKeys
        req.viewport = visiblePage
        for doc in docs {
            if let old = sentTexts[doc.path] {
                if old == doc.text { continue }
                var a = old, b = doc.text
                a.makeContiguousUTF8(); b.makeContiguousUTF8()
                let edit: DL3CompileRequest.Edit? = a.utf8.withContiguousStorageIfAvailable { ab in
                    b.utf8.withContiguousStorageIfAvailable { bb in
                        let e = EngineV3Edits.splice(old: ab, new: bb)
                        return DL3CompileRequest.Edit(path: doc.path, offset: e.offset, delete: e.delete,
                                                      insert: String(decoding: UnsafeBufferPointer(rebasing: bb[e.insertRange]), as: UTF8.self))
                    }
                } ?? nil
                if let edit { req.edits.append(edit) } else { req.buffers.append((doc.path, doc.text)) }
            } else {
                let new = Array(doc.text.utf8)
                // First sight of this document: its file in the copy is the
                // editor's text (the host checks `main` exists before it
                // applies buffers), and the buffer says so again.
                let dst = project.root.appendingPathComponent(doc.path)
                try? FileManager.default.createDirectory(at: dst.deletingLastPathComponent(), withIntermediateDirectories: true)
                if (try? FileManager.default.destinationOfSymbolicLink(atPath: dst.path)) != nil { try? FileManager.default.removeItem(at: dst) }
                try? Data(new).write(to: dst)
                req.buffers.append((doc.path, doc.text))
            }
            sentTexts[doc.path] = doc.text
        }
        if reason == "edit", req.edits.isEmpty, req.buffers.isEmpty { return }
        if let keystrokeNs { latency.sent(compile: req.id, keystrokeNs: keystrokeNs) }
        do {
            try connection.compile(req)
            latency.noteSent(compile: req.id, at: MonotonicClock.nowNs())
        } catch {
            phase = .failed("could not send to the preview engine: \(error)")
        }
    }

    // MARK: events from the reader

    private func apply(_ out: EngineV3Reader.Output) {
        switch out {
        case .started(let j):
            errorCount = 0; warningCount = 0
            if j["keep"]?.bool == false {
                pages = [:]; stale = []; forms = [:]; pdfFallback = [:]
                layoutRevision &+= 1
            }
        case .page(let p, let compileID):
            let index = Int(p.page.index)
            let changed = pages[index]?.page.hash != p.page.hash
            let sizeChanged = pages[index].map { $0.widthPt != p.widthPt || $0.heightPt != p.heightPt } ?? true
            pages[index] = p
            stale.remove(index)
            pdfFallback[index] = nil
            if index >= pageCount { pageCount = index + 1; layoutRevision &+= 1 } else if sizeChanged { layoutRevision &+= 1 }
            if changed { latency.expect(compile: compileID); latency.noteArrived(compile: compileID, at: MonotonicClock.nowNs()) }
            if view?.pageArrived(index, changed: changed, compileID: compileID) != true, changed { latency.offscreen(compile: compileID) }
        case .form(let f):
            forms[f.page.index] = f
            view?.formArrived(f.page.index)
        case .pages(let j):
            if let n = j["count"]?.int { setCount(Int(n), complete: j["complete"]?.bool ?? false) }
            var newStale = Set<Int>()
            for r in j["stale"]?.array ?? [] {
                if let a = r.array, a.count == 2, let lo = a[0].int, let hi = a[1].int, lo <= hi { newStale.formUnion(Int(lo) ... Int(hi)) }
            }
            for r in j["current"]?.array ?? [] {
                if let a = r.array, a.count == 2, let lo = a[0].int, let hi = a[1].int, lo <= hi { stale.subtract(Int(lo) ... Int(hi)) }
            }
            stale.formUnion(newStale)
            view?.staleChanged()
        case .diagnostic(let j):
            if j["severity"]?.string == "error" { errorCount += 1 } else { warningCount += 1 }
        case .done(let j, let compileID):
            let status = j["status"]?.string ?? "?"
            if logDone { log("DONE \(j)") }
            if status != "cancelled" {
                if let n = j["pages"]?.int { setCount(Int(n), complete: true) }
                stale = []
                view?.staleChanged()
                statusNote = "\(status) · \(j["mode"]?.string ?? "") · \(pageCount) page\(pageCount == 1 ? "" : "s") · \(String(format: "%.0f", j["elapsed_ms"]?.double ?? 0)) ms"
                loadFallbacks(pdf: j["pdf"]?.string)
            }
            latency.done(compile: compileID, cancelled: status == "cancelled")
        case .error(let j):
            statusNote = "error: \(j["code"]?.string ?? "?") \(j["message"]?.string ?? "")"
            log(statusNote)
        }
    }

    private func setCount(_ n: Int, complete: Bool) {
        guard n != pageCount || complete else { return }
        if complete, n < pageCount {
            for i in n ..< pageCount { pages[i] = nil; stale.remove(i); pdfFallback[i] = nil }
        }
        if n != pageCount { pageCount = n; layoutRevision &+= 1 }
    }

    /// Pages the display list cannot draw exactly render from the compile's PDF.
    private func loadFallbacks(pdf: String?) {
        let need = pages.filter { $0.value.needsPDFFallback }.map(\.key)
        guard !need.isEmpty, let pdf, let doc = CGPDFDocument(URL(fileURLWithPath: pdf) as CFURL) else { return }
        for i in need { if let p = doc.page(at: i + 1) { pdfFallback[i] = p } }
        view?.fallbacksChanged(need)
    }

    nonisolated static func onMain(_ block: @escaping @MainActor () -> Void) {
        CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.commonModes.rawValue) { MainActor.assumeIsolated { block() } }
        CFRunLoopWakeUp(CFRunLoopGetMain())
    }
}

/// A weak reference that may cross threads (it is only dereferenced on main).
final class EngineV3WeakRef: @unchecked Sendable {
    weak var value: EngineV3Session?
    init(_ v: EngineV3Session) { value = v }
}

/// Reader-thread state: resource bindings of the connection and the
/// compile each frame belongs to. Pages are prepared here, off the main thread.
final class EngineV3Reader: @unchecked Sendable {
    enum Output {
        case started(DL3JSON)
        case page(DL3PreparedPage, compileID: Int)
        case form(DL3PreparedPage)
        case pages(DL3JSON)
        case diagnostic(DL3JSON)
        case done(DL3JSON, compileID: Int)
        case error(DL3JSON)
    }
    private var bindings = DL3Bindings()
    private let cache: DL3ResourceCache
    private var compileID = 0

    init(cache: DL3ResourceCache) { self.cache = cache }

    func handle(_ ev: DL3Event) -> Output? {
        switch ev {
        case .started(let j):
            compileID = Int(j["id"]?.int ?? 0)
            if j["keep"]?.bool == false { bindings.reset() }
            return .started(j)
        case .font(let f): bindings.bind(font: f, cache: cache); return nil
        case .image(let j): bindings.bind(image: j, cache: cache); return nil
        case .page(let p): return .page(bindings.prepare(p), compileID: compileID)
        case .form(let p): return .form(bindings.prepare(p))
        case .pages(let j): return .pages(j)
        case .diagnostic(let j): return .diagnostic(j)
        case .done(let j): return .done(j, compileID: Int(j["id"]?.int ?? Int64(compileID)))
        case .error(let j): return .error(j)
        case .hello, .sources, .other: return nil
        }
    }
}

/// Byte splices between what the host holds and the editor's text.
enum EngineV3Edits {
    struct Splice: Equatable { var offset: Int; var delete: Int; var insertRange: Range<Int> }

    /// The shortest single splice turning `old` into `new` (common prefix
    /// and suffix; never splitting a UTF-8 sequence, so `insert` is text).
    static func splice(old: [UInt8], new: [UInt8]) -> Splice {
        old.withUnsafeBufferPointer { a in new.withUnsafeBufferPointer { b in splice(old: a, new: b) } }
    }

    static func splice(old a: UnsafeBufferPointer<UInt8>, new b: UnsafeBufferPointer<UInt8>) -> Splice {
        var p = 0
        let n = min(a.count, b.count)
        // Word-at-a-time prefix scan, then bytes.
        if let pa = a.baseAddress, let pb = b.baseAddress {
            while p + 8 <= n, UnsafeRawPointer(pa + p).loadUnaligned(as: UInt64.self) == UnsafeRawPointer(pb + p).loadUnaligned(as: UInt64.self) { p += 8 }
        }
        while p < n, a[p] == b[p] { p += 1 }
        while p > 0, p < b.count, b[p] & 0xC0 == 0x80 { p -= 1 } // back to a character boundary
        var s = 0
        let m = n - p
        if let pa = a.baseAddress, let pb = b.baseAddress {
            while s + 8 <= m, UnsafeRawPointer(pa + a.count - s - 8).loadUnaligned(as: UInt64.self) == UnsafeRawPointer(pb + b.count - s - 8).loadUnaligned(as: UInt64.self) { s += 8 }
        }
        while s < m, a[a.count - 1 - s] == b[b.count - 1 - s] { s += 1 }
        while s > 0, b.count - s < b.count, b[b.count - s] & 0xC0 == 0x80 { s -= 1 }
        return Splice(offset: p, delete: a.count - s - p, insertRange: p ..< b.count - s)
    }
}

/// The host's copy of the project: the user's files are never written. Text
/// the editor holds is sent as `buffers`/`edits` (the host writes it here);
/// every other file of the project is linked in (read-only use: images,
/// bibliographies, included files the editor has not opened). An untitled
/// document gets an empty directory.
final class EngineV3Mirror {
    let source: URL?
    let root: URL
    let output: URL

    init(source: URL?) {
        self.source = source
        let key = SHA256.hash(data: Data((source?.path ?? "untitled-\(getpid())").utf8)).prefix(8).map { String(format: "%02x", $0) }.joined()
        let base = EngineV3.cacheDirectory.appendingPathComponent("projects/\(key)", isDirectory: true)
        root = base.appendingPathComponent("src", isDirectory: true)
        output = base.appendingPathComponent("out", isDirectory: true)
        try? FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
        try? FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    }

    /// Links every project file not in `editorPaths` (which the host writes)
    /// into the copy. Bounded: 20,000 entries, hidden directories skipped.
    func sync(except editorPaths: Set<String>) {
        guard let source else { return }
        let fm = FileManager.default
        guard let e = fm.enumerator(at: source, includingPropertiesForKeys: [.isDirectoryKey], options: [.skipsHiddenFiles, .skipsPackageDescendants]) else { return }
        var n = 0
        let prefix = source.standardizedFileURL.path + "/"
        for case let url as URL in e {
            n += 1
            if n > 20_000 { break }
            let path = url.standardizedFileURL.path
            guard path.hasPrefix(prefix) else { continue }
            let rel = String(path.dropFirst(prefix.count))
            let dst = root.appendingPathComponent(rel)
            if (try? url.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory == true {
                try? fm.createDirectory(at: dst, withIntermediateDirectories: true)
                continue
            }
            if editorPaths.contains(rel) {
                // A link here would let the host write through to the user's file.
                if (try? fm.destinationOfSymbolicLink(atPath: dst.path)) != nil { try? fm.removeItem(at: dst) }
                continue
            }
            if (try? fm.destinationOfSymbolicLink(atPath: dst.path)) == nil, !fm.fileExists(atPath: dst.path) {
                try? fm.createSymbolicLink(at: dst, withDestinationURL: url)
            }
        }
    }
}

/// Keystroke → pixels: from the key event's timestamp to the Core Animation
/// commit of the first page the resulting compile changed. A keystroke whose
/// compile was superseded is covered by the next compile that paints.
@MainActor
final class EngineV3Latency {
    struct Sample: Codable {
        var compile: Int; var keystrokeNs: UInt64; var commitNs: UInt64; var page: Int
        /// Phases of the compile that painted it: COMPILE written to the
        /// socket, its first changed page decoded and on the main thread,
        /// that page rasterised (off-main), committed.
        var sentNs: UInt64?, arrivedNs: UInt64?, rasterMs: Double?
        /// When the editor's change reached the engine-v3 hook (ShellModel.updateActiveText).
        var hookNs: UInt64?
        var ms: Double { Double(commitNs &- keystrokeNs) / 1e6 }
    }
    private var sentAt: [Int: UInt64] = [:], arrivedAt: [Int: UInt64] = [:], rasterAt: [Int: Double] = [:]
    private var lastHookNs: UInt64 = 0
    private var hookAt: [Int: UInt64] = [:]
    func noteHook(at ns: UInt64) { lastHookNs = ns }
    func noteSent(compile: Int, at ns: UInt64) { sentAt[compile] = ns; hookAt[compile] = lastHookNs }
    func noteArrived(compile: Int, at ns: UInt64) { if arrivedAt[compile] == nil { arrivedAt[compile] = ns } }
    func noteRaster(compile: Int, ms: Double) { if rasterAt[compile] == nil { rasterAt[compile] = ms } }
    private var pending: [(compile: Int, ns: UInt64)] = []
    private(set) var samples: [Sample] = []
    /// Keystrokes whose compile changed no page (e.g. a comment).
    private(set) var unchanged = 0
    private var painted: Set<Int> = []
    private var expected: Set<Int> = []
    /// Keystrokes whose first changed page was not on screen.
    private(set) var offscreenCount = 0

    func sent(compile: Int, keystrokeNs: UInt64) { pending.append((compile, keystrokeNs)) }
    func expect(compile: Int) { expected.insert(compile) }
    func offscreen(compile: Int) {
        guard expected.contains(compile), !painted.contains(compile) else { return }
        painted.insert(compile)
        offscreenCount += pending.filter { $0.compile <= compile }.count
        pending.removeAll { $0.compile <= compile }
    }

    /// The first changed page of `compile` was committed.
    func committed(compile: Int, page: Int, at ns: UInt64) {
        guard !painted.contains(compile) else { return }
        painted.insert(compile)
        let covered = pending.filter { $0.compile <= compile }
        pending.removeAll { $0.compile <= compile }
        for k in covered {
            samples.append(Sample(compile: compile, keystrokeNs: k.ns, commitNs: ns, page: page,
                                  sentNs: sentAt[compile], arrivedNs: arrivedAt[compile], rasterMs: rasterAt[compile], hookNs: hookAt[compile]))
        }
    }

    func done(compile: Int, cancelled: Bool) {
        guard !cancelled, !expected.contains(compile) else { return }
        // This compile (and any it superseded) changed no page.
        unchanged += pending.filter { $0.compile <= compile }.count
        pending.removeAll { $0.compile <= compile }
    }

    var pendingCount: Int { pending.count }
    func reset() { pending = []; samples = []; unchanged = 0; painted = []; expected = []; offscreenCount = 0; sentAt = [:]; arrivedAt = [:]; rasterAt = [:] }
}
