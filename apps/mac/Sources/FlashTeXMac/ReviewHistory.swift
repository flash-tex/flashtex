import Foundation
import Observation
import SwiftUI
import FlashTeXProtocol

/// Review history: what happened to every capture proposal the reviewer
/// decided on (AI review follow-up 1). One entry per decision, newest last,
/// bounded, kept in memory and persisted as JSON under Application Support
/// (`FlashTeX/review-history/<project>.json`) so the list survives a
/// relaunch. Entries describe outcomes the model already produced
/// (`ShellModel.ApproveOutcome`, `rejectProposal`, `editRefused`,
/// `editApplied`); nothing here changes a proposal, the queue or the buffer.
struct ReviewHistoryEntry: Codable, Equatable, Identifiable {
    enum Outcome: String, Codable, CaseIterable {
        /// Approved: the insertion was staged for the editor (`.inserted`).
        case inserted
        /// The editor applied the staged insertion (undo registered).
        case applied
        /// The editor refused the staged insertion (buffer moved on, range no
        /// longer fits); the proposal went back to the review queue.
        case refused
        case rejected
        /// Approval refused before staging: the anchor could not be resolved.
        case needsReselection
        case noAnchor
        /// Already inserted earlier (capture id known).
        case duplicate
        /// Approval refused for another reason (bridge capture, ledger, …).
        case declined

        var label: String {
            switch self {
            case .inserted: return "approved (staged)"
            case .applied: return "inserted"
            case .refused: return "not inserted — back to review"
            case .rejected: return "rejected"
            case .needsReselection: return "needs a new insertion point"
            case .noAnchor: return "no insertion point"
            case .duplicate: return "duplicate (already inserted)"
            case .declined: return "declined"
            }
        }
    }

    var id: String
    var captureId: String
    /// The reviewer's LaTeX at decision time (clipped to `ReviewHistory.latexLimit` bytes).
    var latex: String
    var latexClipped: Bool
    var outcome: Outcome
    /// The exact reason for a refusal/reselection/decline; nil otherwise.
    var reason: String?
    var path: String?
    var byteOffset: Int?
    var editorRevision: Int
    var recordedAt: Date
}

struct ReviewHistory: Codable, Equatable {
    static let schemaVersion = 1
    static let capacity = 200
    static let latexLimit = 4 * 1024

    var version = ReviewHistory.schemaVersion
    private(set) var entries: [ReviewHistoryEntry] = []
    private var nextNumber = 1

    var count: Int { entries.count }
    var last: ReviewHistoryEntry? { entries.last }

    func entries(for captureId: String) -> [ReviewHistoryEntry] { entries.filter { $0.captureId == captureId } }

    /// Newest first, for display.
    var newestFirst: [ReviewHistoryEntry] { entries.reversed() }

    /// Appends one decision; the oldest entries beyond `capacity` are dropped.
    mutating func record(captureId: String, latex: String, outcome: ReviewHistoryEntry.Outcome, reason: String? = nil,
                         path: String? = nil, byteOffset: Int? = nil, editorRevision: Int, at date: Date = Date()) -> ReviewHistoryEntry {
        let (clipped, wasClipped) = Self.clip(latex)
        let entry = ReviewHistoryEntry(id: "\(captureId)#\(nextNumber)", captureId: captureId, latex: clipped, latexClipped: wasClipped,
                                       outcome: outcome, reason: reason, path: path, byteOffset: byteOffset,
                                       editorRevision: editorRevision,
                                       // Whole seconds: what the ISO-8601 file keeps, so a reloaded history equals the recorded one.
                                       recordedAt: Date(timeIntervalSince1970: date.timeIntervalSince1970.rounded(.down)))
        nextNumber += 1
        entries.append(entry)
        if entries.count > Self.capacity { entries.removeFirst(entries.count - Self.capacity) }
        return entry
    }

    /// The first `latexLimit` bytes, cut back to a scalar boundary.
    static func clip(_ latex: String) -> (String, Bool) {
        let utf8 = latex.utf8
        guard utf8.count > latexLimit else { return (latex, false) }
        var end = utf8.index(utf8.startIndex, offsetBy: latexLimit)
        while end > utf8.startIndex, utf8[end] & 0xC0 == 0x80 { end = utf8.index(before: end) }
        return (String(decoding: utf8[..<end], as: UTF8.self), true)
    }

    // MARK: persistence

    /// `~/Library/Application Support/FlashTeX/review-history/<project>.json`
    /// (`project` sanitized to a file name). `FLASHTEX_REVIEW_HISTORY_DIR`
    /// replaces the Application Support base (test runs, sandboxes); the
    /// value `off` keeps the history in memory only. nil when no base exists.
    static func defaultURL(projectId: String,
                           applicationSupport: URL? = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first,
                           environment: [String: String] = ProcessInfo.processInfo.environment) -> URL? {
        let base: URL?
        if let dir = environment["FLASHTEX_REVIEW_HISTORY_DIR"] {
            base = dir == "off" ? nil : URL(fileURLWithPath: dir)
        } else {
            base = applicationSupport
        }
        guard let base else { return nil }
        let safe = projectId.map { $0.isLetter || $0.isNumber || $0 == "-" || $0 == "_" || $0 == "." ? $0 : "_" }
        let name = String(safe).isEmpty ? "project" : String(safe)
        return base.appendingPathComponent("FlashTeX/review-history/\(name).json")
    }

    /// Loads a history; a missing file is an empty history, an unreadable or
    /// mismatched file is reported (and left in place) rather than guessed at.
    static func load(from url: URL) -> (history: ReviewHistory, problem: String?) {
        guard FileManager.default.fileExists(atPath: url.path) else { return (ReviewHistory(), nil) }
        do {
            let data = try Data(contentsOf: url)
            let decoder = JSONDecoder()
            decoder.dateDecodingStrategy = .iso8601
            let h = try decoder.decode(ReviewHistory.self, from: data)
            guard h.version == schemaVersion else {
                return (ReviewHistory(), "review history at \(url.lastPathComponent) has schema \(h.version), expected \(schemaVersion); starting empty (file kept)")
            }
            return (h, nil)
        } catch {
            return (ReviewHistory(), "review history at \(url.lastPathComponent) is unreadable: \(error.localizedDescription); starting empty (file kept)")
        }
    }

    /// Atomic write (temp file + rename) with ISO-8601 dates.
    func save(to url: URL) throws {
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.sortedKeys]
        try encoder.encode(self).write(to: url, options: .atomic)
    }
}

/// Owns one project's history, persists every recorded decision, and turns
/// the model's outcomes into entries. `ShellModel` holds one (parent hook:
/// `let reviewHistory = ReviewHistoryRecorder(projectId:)`) and calls the
/// `record…` methods from `approveProposal` / `rejectProposal` /
/// `editRefused` / `editApplied`; the panel observes it.
@MainActor
@Observable
final class ReviewHistoryRecorder {
    private(set) var history: ReviewHistory
    /// Set when loading or saving failed; the panel shows it. Never blocks a decision.
    private(set) var persistenceProblem: String?
    private(set) var saveCount = 0
    let url: URL?

    /// `url` nil keeps the history in memory only (and for previews/tests).
    init(url: URL?) {
        self.url = url
        if let url {
            let loaded = ReviewHistory.load(from: url)
            history = loaded.history
            persistenceProblem = loaded.problem
        } else {
            history = ReviewHistory()
        }
    }

    convenience init(projectId: String) {
        self.init(url: ReviewHistory.defaultURL(projectId: projectId))
    }

    var entries: [ReviewHistoryEntry] { history.entries }

    /// Maps an `approveProposal` outcome onto an entry.
    @discardableResult
    func record(_ outcome: ShellModel.ApproveOutcome, proposal: RuntimeV1.CaptureProposal, latex: String,
                path: String?, editorRevision: Int) -> ReviewHistoryEntry {
        switch outcome {
        case .inserted(let byte):
            return append(proposal.captureId, latex, .inserted, nil, path, byte, editorRevision)
        case .needsReselection(let why):
            return append(proposal.captureId, latex, .needsReselection, why, path, nil, editorRevision)
        case .duplicate:
            return append(proposal.captureId, latex, .duplicate, nil, path, nil, editorRevision)
        case .noAnchor:
            return append(proposal.captureId, latex, .noAnchor, nil, path, nil, editorRevision)
        case .refused(let why):
            return append(proposal.captureId, latex, .declined, why, path, nil, editorRevision)
        }
    }

    @discardableResult
    func recordRejected(_ proposal: RuntimeV1.CaptureProposal, editorRevision: Int) -> ReviewHistoryEntry {
        append(proposal.captureId, proposal.latex, .rejected, nil, nil, nil, editorRevision)
    }

    /// The editor refused the staged insertion (`ShellModel.editRefused`).
    @discardableResult
    func recordRefused(_ refund: ShellModel.CaptureRefund, reason: String, editorRevision: Int) -> ReviewHistoryEntry {
        append(refund.proposal.captureId, refund.proposal.latex, .refused, reason, refund.anchorBefore.path, refund.anchorBefore.byteOffset, editorRevision)
    }

    /// The editor applied the staged insertion (`ShellModel.editApplied`).
    @discardableResult
    func recordApplied(_ edit: ShellModel.PendingEdit, editorRevision: Int) -> ReviewHistoryEntry? {
        guard let refund = edit.captureRefund else { return nil }
        return append(refund.proposal.captureId, refund.proposal.latex, .applied, nil, edit.path, refund.anchorBefore.byteOffset, editorRevision)
    }

    private func append(_ captureId: String, _ latex: String, _ outcome: ReviewHistoryEntry.Outcome, _ reason: String?,
                        _ path: String?, _ byte: Int?, _ revision: Int) -> ReviewHistoryEntry {
        let entry = history.record(captureId: captureId, latex: latex, outcome: outcome, reason: reason,
                                   path: path, byteOffset: byte, editorRevision: revision)
        persist()
        return entry
    }

    private func persist() {
        guard let url else { return }
        do {
            try history.save(to: url)
            saveCount += 1
        } catch {
            persistenceProblem = "could not save review history to \(url.lastPathComponent): \(error.localizedDescription)"
        }
    }
}

/// Read-only list of past decisions (newest first). Nothing here re-applies,
/// re-queues or edits a proposal.
struct ReviewHistoryView: View {
    var recorder: ReviewHistoryRecorder

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.xs) {
            HStack {
                Text("Review history").font(.caption.bold())
                Spacer()
                Text("\(recorder.entries.count) decision\(recorder.entries.count == 1 ? "" : "s")").font(.caption2).foregroundStyle(.secondary)
            }
            if let problem = recorder.persistenceProblem {
                Text(problem).font(.caption2).foregroundStyle(DS.Colors.severityWarning).lineLimit(2)
            }
            if recorder.entries.isEmpty {
                Text("No proposals reviewed yet.").font(.caption2).foregroundStyle(.secondary)
            }
            ForEach(recorder.history.newestFirst) { e in
                HStack(alignment: .top, spacing: DS.Space.s) {
                    Image(systemName: Self.glyph(for: e.outcome)).font(.caption).foregroundStyle(Self.tint(for: e.outcome))
                    VStack(alignment: .leading, spacing: DS.Size.hairline) {
                        Text("\(e.captureId) — \(e.outcome.label)" + (e.byteOffset.map { " at byte \($0)" } ?? "") + " (revision \(e.editorRevision))")
                            .font(.caption)
                        Text(e.latex.replacingOccurrences(of: "\n", with: "⏎") + (e.latexClipped ? "…" : ""))
                            .font(.system(.caption2, design: .monospaced)).foregroundStyle(.secondary).lineLimit(2)
                        if let why = e.reason {
                            Text(why).font(.caption2).foregroundStyle(.tertiary).lineLimit(2)
                        }
                    }
                    Spacer()
                    Text(e.recordedAt, style: .time).font(.caption2).foregroundStyle(.tertiary)
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel("\(e.captureId), \(e.outcome.label)" + (e.reason.map { ", \($0)" } ?? ""))
            }
        }
    }

    static func glyph(for outcome: ReviewHistoryEntry.Outcome) -> String {
        switch outcome {
        case .applied: return "checkmark.circle.fill"
        case .inserted: return "arrow.down.doc"
        case .refused, .needsReselection, .noAnchor: return "arrow.uturn.backward.circle"
        case .rejected: return "xmark.circle"
        case .duplicate, .declined: return "minus.circle"
        }
    }

    static func tint(for outcome: ReviewHistoryEntry.Outcome) -> Color {
        switch outcome {
        case .applied: return .green
        case .inserted: return .blue
        case .refused, .needsReselection, .noAnchor: return .orange
        case .rejected, .duplicate, .declined: return .secondary
        }
    }
}
