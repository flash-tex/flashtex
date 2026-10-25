import Foundation
import FlashTeXDisplayListV3
import FlashTeXProtocol

/// Makes the Problems panel's model from a compile's DIAGs and the tools'
/// DIAGNOSTICs (APP-MAIN-THREAD): the rows (`EngineV3Session.problems`),
/// then their groups and counts (`ProblemsModel`).
///
/// Typing in Infinite Descent ×2 (1,152 pages, thousands of box warnings)
/// answers each key with a compile, and making its rows cost the main thread
/// ~25 ms at every DONE. Here they are made on `queue` from inputs the main
/// thread copied at the DONE (value types: the compile's texts, the editor's
/// texts, the copy's roots), and the main thread only assigns the result.
///
/// A small job (a short document's few rows) runs where it is issued, when
/// no job is on the queue: the hop would cost more than the work, and the
/// rows are then published with the DONE as before. Jobs run in the order
/// issued, so the rows of a later DONE never give way to an earlier one's.
final class EngineV3ProblemsPipeline: @unchecked Sendable {
    nonisolated static let queue = DispatchQueue(label: "flashtex.engine-v3.problems", qos: .userInitiated)

    /// A DONE's rows to make: its DIAGs (diag-v1) or, without them, its DIAGNOSTIC messages.
    struct TeXInput {
        var diags: [DL3Diag]
        var diagnostics: [DL3JSON]
        /// The texts the compile read (`textsCompiled`).
        var texts: [String: String]
        /// The editor's texts by path, for a file the compile's texts lack.
        var documents: [String: String]
        var roots: [String]
        var mode: EngineV3ErrorPolicy.Mode
        var engine: String
    }

    struct Job {
        /// A DONE's rows; nil keeps the last DONE's (a tool finished).
        var tex: TeXInput?
        /// Drop the last DONE's rows first (another project, the session stopped).
        var clearTeX: Bool
        /// The tool cycle's DIAGNOSTICs (bibtex, biber, makeindex).
        var tools: [DL3JSON]
        var documents: [String: String]
        var roots: [String]
        /// The texts of the compile the rows are of (`ShellModel.compiledDocuments`): their line labels.
        var texts: [String: String]
        /// The listing's paths, which the groups are sorted by.
        var documentOrder: [String]
        /// Once published: VoiceOver says the counts; the package scan runs (a DONE's).
        var announce = false
        var scanPackages = false

        /// This job and a later one, as one: the later one's inputs, the
        /// last DONE's rows of either, and what either waits to do once published.
        func merged(with later: Job) -> Job {
            var m = later
            // The later DONE's rows replace this one's; a later clear drops them.
            if later.tex == nil, !later.clearTeX { m.tex = tex }
            m.clearTeX = clearTeX || later.clearTeX
            m.announce = announce || later.announce
            m.scanPackages = scanPackages || later.scanPackages
            return m
        }

        /// Whether the job is cheap enough to run where it is issued: few
        /// rows, and texts short enough that a search of them for a macro's
        /// definition or a label (`EngineV3Session.problems`) is quick.
        var isSmall: Bool {
            let rows = (tex.map { $0.diags.count + $0.diagnostics.count } ?? 0) + tools.count
            guard rows <= Self.smallRows else { return false }
            guard let tex, rows > 0 else { return true }
            return tex.texts.values.reduce(0) { $0 + $1.utf8.count } <= Self.smallTextBytes
        }
        static let smallRows = 64
        static let smallTextBytes = 256 * 1024
    }

    private let lock = NSLock()
    /// The last DONE's rows (`Job.tex` nil keeps them).
    private var texRows: [RuntimeV1.Diagnostic] = []
    /// The last model made: returned again when a job's rows are the same,
    /// so the main thread sees the same object and publishes nothing.
    private var last = ProblemsModel.empty

    /// Jobs run on the main thread and off it (tests: a long book's are off it).
    var runs: (onMain: Int, offMain: Int) { lock.lock(); defer { lock.unlock() }; return counted }
    private var counted = (onMain: 0, offMain: 0)

    /// Makes the job's model (any thread; jobs are run one at a time, in order).
    func run(_ job: Job) -> ProblemsModel {
        let main = Thread.isMainThread
        lock.lock(); if main { counted.onMain += 1 } else { counted.offMain += 1 }; lock.unlock()
        var tex: [RuntimeV1.Diagnostic]?
        if let t = job.tex {
            tex = t.diags.isEmpty
                ? EngineV3Session.problems(t.diagnostics, documents: t.documents, roots: t.roots, texts: t.texts, mode: t.mode, engine: t.engine)
                : EngineV3Session.problems(diags: t.diags, documents: t.documents, roots: t.roots, texts: t.texts, mode: t.mode, engine: t.engine)
        }
        let tools = job.tools.isEmpty ? [] : EngineV3Session.problems(job.tools, documents: job.documents, roots: job.roots)
        lock.lock()
        if job.clearTeX { texRows = [] }
        if let tex { texRows = tex }
        let rows = texRows + tools
        let previous = last
        lock.unlock()
        if previous.diagnostics == rows, previous.documentOrder == job.documentOrder, previous.texts == job.texts { return previous }
        let made = ProblemsModel(diagnostics: rows, texts: job.texts, documentOrder: job.documentOrder)
        lock.lock(); last = made; lock.unlock()
        return made
    }
}
