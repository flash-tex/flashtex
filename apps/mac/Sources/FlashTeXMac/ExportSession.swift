import Darwin
import Foundation
import Observation
import FlashTeXProtocol

/// One exact-PDF export (`flashtex-pdf-exact from-v2`) as an isolated,
/// observable state machine so the UI can show progress, offer Cancel, and
/// report refusals without ever leaving a damaged or partial file at the
/// destination:
///
/// - The tool never writes the destination itself: it is pointed at a sibling
///   temp file (`.<name>.flashtex-export-<uuid>.tmp`, same directory, so the
///   final `rename(2)` is atomic on the same volume). The destination is
///   replaced only after the tool exited 0 and produced a non-empty file.
/// - Cancel and timeout terminate only the `Process` this session launched
///   (the pid recorded in `.running` is re-checked before signalling), then the
///   temp file is removed. The destination is untouched.
/// - Overwrite conflicts: the destination's disk SHA-256 recorded when the
///   user chose it (`Destination.expectedDiskSHA256`; nil = "did not exist")
///   is compared before launching and again just before the rename. Any
///   difference is a `DocumentConflict`-style refusal; the existing file is kept.
/// - Exit ≠ 0 or an empty output is a refusal naming the tool's message; the
///   destination is untouched.
///
/// The session is single-use per export: `start` refuses while `.running`.
@MainActor
@Observable
final class ExportSession {
    enum State: Equatable {
        case idle
        case running(pid: Int32, started: Date)
        case cancelled
        case failed(reason: String)
        case succeeded(bytes: Int, sha256: String)

        var isRunning: Bool { if case .running = self { true } else { false } }
    }

    /// Where the export goes and what the user saw there when choosing it.
    struct Destination: Equatable {
        var url: URL
        /// SHA-256 of the file at `url` when the user chose it (they confirmed
        /// overwriting it in the panel); nil when no file existed then.
        var expectedDiskSHA256: String?

        /// Records the destination as it is on disk right now.
        static func recordingCurrentDisk(_ url: URL) -> Destination {
            Destination(url: url, expectedDiskSHA256: ExportSession.diskSHA256(url))
        }
    }

    /// Everything the run produced, for the reporting layer (`captureNote`).
    struct Report: Equatable {
        var state: State
        var exitCode: Int32?
        var stdout: String
        var stderr: String
        /// Set when the refusal was an overwrite conflict.
        var conflict: DocumentConflict?
    }

    private(set) var state: State = .idle
    /// The last overwrite conflict (kept until the next `start`).
    private(set) var conflict: DocumentConflict?
    /// The process of the running export; only ever signalled by `cancel()`.
    @ObservationIgnored private var process: Process?
    @ObservationIgnored private var cancelRequested = false
    /// An external producer's export (`startExternal`): how to cancel it,
    /// where it goes and whom to tell.
    @ObservationIgnored private var externalCancel: (@MainActor () -> Void)?
    @ObservationIgnored private var externalDestination: Destination?
    @ObservationIgnored private var externalCompletion: (@MainActor (Report) -> Void)?

    /// An export whose PDF another component produces (the engine-v3 host's
    /// `export: true` run): this session holds the Cancel, then publishes the
    /// bytes exactly as the exact route does (a sibling temp file, the
    /// overwrite-conflict check again, an atomic rename). Returns false, after
    /// calling `completion` with the refusal, when it cannot start.
    @discardableResult
    func startExternal(destination: Destination, cancel: @escaping @MainActor () -> Void,
                       completion: (@MainActor (Report) -> Void)? = nil) -> Bool {
        if state.isRunning {
            completion?(Report(state: .failed(reason: "an export is already running"), exitCode: nil, stdout: "", stderr: "", conflict: nil))
            return false
        }
        conflict = nil
        cancelRequested = false
        if let c = Self.conflict(at: destination, phase: .beforeLaunch) {
            conflict = c
            state = .failed(reason: c.exportSummary)
            completion?(Report(state: state, exitCode: nil, stdout: "", stderr: "", conflict: c))
            return false
        }
        externalCancel = cancel
        externalDestination = destination
        externalCompletion = completion
        state = .running(pid: 0, started: Date())
        return true
    }

    /// The external producer finished: `pdf` on success, else `failure`
    /// (nil with no PDF: cancelled). Publishes or refuses; never leaves a
    /// partial file at the destination.
    func finishExternal(pdf: Data?, failure: String? = nil) {
        guard state.isRunning, let destination = externalDestination else { return }
        let completion = externalCompletion
        externalCancel = nil; externalDestination = nil; externalCompletion = nil
        let outcome: (State, DocumentConflict?)
        if let pdf, !pdf.isEmpty {
            let temp = Self.temporarySibling(of: destination.url)
            do {
                try pdf.write(to: temp)
                outcome = Self.publish(temp: temp, to: destination)
            } catch {
                Self.discard(temp)
                outcome = (.failed(reason: "could not write the PDF beside \(destination.url.lastPathComponent): \(error.localizedDescription); nothing was written there"), nil)
            }
        } else if let failure {
            outcome = (.failed(reason: "Export failed: \(failure). Nothing was written to \(destination.url.lastPathComponent)."), nil)
        } else {
            outcome = (.cancelled, nil)
        }
        conflict = outcome.1
        state = outcome.0
        completion?(Report(state: outcome.0, exitCode: nil, stdout: "", stderr: "", conflict: outcome.1))
    }

    /// Launches the export off the main actor; `completion` runs on the main
    /// actor with the final state. Refused (state unchanged, completion with
    /// `.failed`) while another export of this session is running.
    func start(tool: URL, list: URL, destination: Destination, fontDirs: [String],
               timeout: TimeInterval = 60, completion: (@MainActor (Report) -> Void)? = nil) {
        if state.isRunning {
            completion?(Report(state: .failed(reason: "an export is already running"), exitCode: nil, stdout: "", stderr: "", conflict: nil))
            return
        }
        conflict = nil
        cancelRequested = false
        // Overwrite conflict before doing any work.
        if let c = Self.conflict(at: destination, phase: .beforeLaunch) {
            conflict = c
            state = .failed(reason: c.exportSummary)
            completion?(Report(state: state, exitCode: nil, stdout: "", stderr: "", conflict: c))
            return
        }
        let temp = Self.temporarySibling(of: destination.url)
        let p = Process()
        p.executableURL = tool
        var args = ["from-v2", list.path, "--out", temp.path]
        for d in fontDirs { args += ["--font-dir", d] }
        p.arguments = args
        let stdoutPipe = Pipe(), stderrPipe = Pipe()
        p.standardOutput = stdoutPipe; p.standardError = stderrPipe
        do { try p.run() } catch {
            state = .failed(reason: "could not launch \(tool.lastPathComponent): \(error.localizedDescription)")
            completion?(Report(state: state, exitCode: nil, stdout: "", stderr: "", conflict: nil))
            return
        }
        process = p
        let pid = p.processIdentifier
        state = .running(pid: pid, started: Date())

        let session = self
        Task.detached {
            let run = Self.waitForExit(p, pid: pid, timeout: timeout, stdout: stdoutPipe, stderr: stderrPipe)
            let cancelled = await MainActor.run { session.cancelRequested }
            let outcome: (State, DocumentConflict?)
            if cancelled {
                Self.discard(temp)
                outcome = (.cancelled, nil)
            } else if run.timedOut {
                Self.discard(temp)
                outcome = (.failed(reason: "\(tool.lastPathComponent) (pid \(pid)) did not finish within \(Int(timeout)) s; terminated. Nothing was written to \(destination.url.lastPathComponent)."), nil)
            } else if run.status != 0 || run.signalled {
                Self.discard(temp)
                let why = (run.stderr + run.stdout).trimmingCharacters(in: .whitespacesAndNewlines)
                let how = run.signalled ? "signal \(run.status)" : "exit \(run.status)"
                outcome = (.failed(reason: "Exact export refused (\(how)): \(why.isEmpty ? "no message" : why)"), nil)
            } else {
                outcome = Self.publish(temp: temp, to: destination)
            }
            await MainActor.run {
                session.process = nil
                session.conflict = outcome.1
                session.state = outcome.0
                completion?(Report(state: outcome.0, exitCode: run.timedOut || cancelled ? nil : run.status,
                                   stdout: run.stdout, stderr: run.stderr, conflict: outcome.1))
            }
        }
    }

    /// Terminates the export this session launched (and nothing else). No-op
    /// unless `.running`. The state becomes `.cancelled` once the tool exited
    /// and the temp file is gone; the destination is never touched.
    func cancel() {
        guard case .running(let pid, _) = state else { return }
        if let externalCancel {
            cancelRequested = true
            externalCancel()
            return
        }
        guard let p = process else { return }
        cancelRequested = true
        Self.terminate(p, expectedPid: pid)
    }

    // MARK: - Steps (nonisolated, off the main actor)

    struct Run {
        var timedOut: Bool
        var status: Int32
        var signalled: Bool
        var stdout: String
        var stderr: String
    }

    /// Drains both pipes concurrently and waits for exit; after `timeout`
    /// seconds the launched process is terminated (by its own `Process`).
    nonisolated private static func waitForExit(_ p: Process, pid: Int32, timeout: TimeInterval, stdout: Pipe, stderr: Pipe) -> Run {
        var outData = Data(), errData = Data()
        let drained = DispatchGroup()
        drained.enter(); DispatchQueue.global().async { outData = stdout.fileHandleForReading.readDataToEndOfFile(); drained.leave() }
        drained.enter(); DispatchQueue.global().async { errData = stderr.fileHandleForReading.readDataToEndOfFile(); drained.leave() }
        let exited = DispatchGroup()
        exited.enter(); DispatchQueue.global().async { p.waitUntilExit(); exited.leave() }
        var timedOut = false
        if exited.wait(timeout: .now() + timeout) == .timedOut {
            timedOut = true
            terminate(p, expectedPid: pid)
            if exited.wait(timeout: .now() + 5) == .timedOut, p.isRunning, p.processIdentifier == pid {
                kill(pid, SIGKILL) // still ours: the pid was re-checked on the live Process
                _ = exited.wait(timeout: .now() + 5)
            }
        }
        drained.wait()
        return Run(timedOut: timedOut, status: p.terminationStatus, signalled: p.terminationReason == .uncaughtSignal,
                   stdout: String(decoding: outData, as: UTF8.self), stderr: String(decoding: errData, as: UTF8.self))
    }

    /// SIGTERM to the launched process only: `Process.terminate()` addresses
    /// that process object, and the recorded pid must still be its pid.
    nonisolated private static func terminate(_ p: Process, expectedPid: Int32) {
        guard p.isRunning, p.processIdentifier == expectedPid else { return }
        p.terminate()
    }

    nonisolated static func temporarySibling(of url: URL) -> URL {
        url.deletingLastPathComponent().appendingPathComponent(".\(url.lastPathComponent).flashtex-export-\(UUID().uuidString).tmp")
    }

    nonisolated private static func discard(_ temp: URL) {
        try? FileManager.default.removeItem(at: temp)
    }

    enum Phase { case beforeLaunch, beforeReplace }

    /// Compares the destination on disk with what the user chose.
    nonisolated static func conflict(at destination: Destination, phase: Phase) -> DocumentConflict? {
        let url = destination.url
        let current = diskSHA256(url)
        let kind: ProjectFilesV1.ConflictKind
        switch (destination.expectedDiskSHA256, current) {
        case (nil, nil): return nil
        case (let e?, let c?) where e == c: return nil
        case (nil, .some): kind = .alreadyExists
        case (.some, nil): kind = .deletedExternally
        case (.some, .some): kind = phase == .beforeReplace ? .modifiedDuringSave : .modifiedExternally
        }
        let attrs = try? FileManager.default.attributesOfItem(atPath: url.path)
        let mtime = (attrs?[.modificationDate] as? Date).map { Int($0.timeIntervalSince1970 * 1000) }
        return DocumentConflict(url: url, kind: kind, ours: destination.expectedDiskSHA256, theirs: current,
                                size: attrs?[.size] as? Int, mtimeUnixMs: mtime, viaHelper: false)
    }

    /// Moves the finished temp file over the destination atomically, unless the
    /// destination changed meanwhile or the tool left nothing usable.
    nonisolated private static func publish(temp: URL, to destination: Destination) -> (State, DocumentConflict?) {
        guard let data = try? Data(contentsOf: temp), !data.isEmpty else {
            discard(temp)
            return (.failed(reason: "Exact export produced no output (exit 0 but \(temp.lastPathComponent) is missing or empty); nothing was written to \(destination.url.lastPathComponent)."), nil)
        }
        if let c = conflict(at: destination, phase: .beforeReplace) {
            discard(temp)
            return (.failed(reason: c.exportSummary), c)
        }
        let sha = SourceDigest.sha256Hex(data)
        // rename(2) is atomic on the same volume: the destination is either the
        // old file or the complete new one, never a partial write.
        if rename(temp.path, destination.url.path) != 0 {
            let err = String(cString: strerror(errno))
            discard(temp)
            return (.failed(reason: "could not replace \(destination.url.lastPathComponent): \(err); the existing file is kept"), nil)
        }
        return (.succeeded(bytes: data.count, sha256: sha), nil)
    }

    nonisolated static func diskSHA256(_ url: URL) -> String? {
        guard let data = try? Data(contentsOf: url) else { return nil }
        return SourceDigest.sha256Hex(data)
    }
}

extension DocumentConflict {
    /// The export wording of a destination conflict (the file was chosen in a
    /// save panel, not opened in the editor).
    var exportSummary: String {
        let what: String = switch kind {
        case .modifiedExternally: "was modified on disk since you chose it"
        case .deletedExternally: "was deleted on disk since you chose it"
        case .alreadyExists: "appeared on disk since you chose it"
        case .modifiedDuringSave: "changed while the PDF was being produced"
        }
        let hashes = [ours.map { "expected \($0.prefix(12))" }, theirs.map { "found \($0.prefix(12))" }].compactMap { $0 }.joined(separator: ", ")
        return "Export refused: \(url.lastPathComponent) \(what) (\(hashes)); the existing file is kept. Choose the destination again to overwrite."
    }
}
