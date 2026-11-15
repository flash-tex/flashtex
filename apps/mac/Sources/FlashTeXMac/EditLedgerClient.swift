import Foundation
import FlashTeXProtocol

/// Models for the private edit-ledger helper protocol (`crates/edit-ledger` at
/// commit afb15839f8080f9e86efd6a187e46dfe014ce559, `flashtex-edit-ledger
/// --store <dir>`): one durable `document.json` holding the source and every
/// applied edit ID, committed atomically with fsync before any receipt.
/// Requests are `{id, operation, ...fields}`; replies carry `session_id`,
/// `sequence`, `document_revision`, `document_sha256`, `command_succeeded` and
/// either `payload` or `error: {code, message}`. Prepared edits and receipts
/// are wire-compatible with transfer-v1 (`TransferV1.CaptureEdit` /
/// `CaptureApplied`); the recovery exchange is ledger-local.
enum EditLedgerV1 {
    /// Per-reply service metadata (afb1583): consumers compare the session and
    /// reject older sequence/revision observations before touching UI state.
    struct ServiceMeta: Equatable {
        var sessionId: String
        var sequence: Int
        var documentRevision: Int?
        var documentSha256: String?
        var commandSucceeded: Bool
    }

    struct Document: Codable, Equatable {
        var projectId: String
        var path: String
        var revision: Int
        var text: String
        var sourceSha256: String
        enum CodingKeys: String, CodingKey { case projectId = "project_id", path, revision, text, sourceSha256 = "source_sha256" }
        init(projectId: String, path: String, revision: Int, text: String) {
            self.projectId = projectId; self.path = path; self.revision = revision; self.text = text
            sourceSha256 = SourceDigest.sha256Hex(text)
        }
    }

    /// An applied edit the helper retains until the bridge acknowledged its receipt.
    struct AppliedTransaction: Codable, Equatable {
        var edit: TransferV1.CaptureEdit
        var receipt: TransferV1.CaptureApplied
        var documentBefore: Document?
        var documentAfterSha256: String
        var confirmed: Bool
        enum CodingKeys: String, CodingKey {
            case edit, receipt, documentBefore = "document_before", documentAfterSha256 = "document_after_sha256", confirmed
        }
    }

    struct Status: Codable, Equatable {
        var document: Document?
        var pendingReceipts: [AppliedTransaction]
        enum CodingKeys: String, CodingKey { case document, pendingReceipts = "pending_receipts" }
    }
    struct Applied: Codable, Equatable {
        var receipt: TransferV1.CaptureApplied
        var document: Document
    }
    struct DocumentReply: Codable, Equatable { var document: Document }
    struct Confirmed: Codable, Equatable { var confirmed: TransferV1.CaptureApplied }

    // Recovery exchange (ledger-local, never transfer-v1 wire fields).
    struct RecoveryExport: Codable, Equatable {
        var snapshotToken: String
        var currentDocument: Document
        var pendingReceipts: [AppliedTransaction]
        enum CodingKeys: String, CodingKey {
            case snapshotToken = "snapshot_token", currentDocument = "current_document", pendingReceipts = "pending_receipts"
        }
    }
    /// What the bridge said about one pending capture (`capture_status`).
    enum BridgeObservation: Encodable, Equatable {
        case applied(receipt: TransferV1.CaptureApplied)
        case prepared(edit: TransferV1.CaptureEdit)
        case unavailable(captureId: String, reason: String)
        enum CodingKeys: String, CodingKey { case status, receipt, edit, captureId = "capture_id", reason }
        func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            switch self {
            case .applied(let receipt): try c.encode("applied", forKey: .status); try c.encode(receipt, forKey: .receipt)
            case .prepared(let edit): try c.encode("prepared", forKey: .status); try c.encode(edit, forKey: .edit)
            case .unavailable(let id, let reason):
                try c.encode("unavailable", forKey: .status); try c.encode(id, forKey: .captureId)
                try c.encode(String(decoding: Array(reason.utf8.prefix(4000)), as: UTF8.self), forKey: .reason)
            }
        }
    }
    enum RecoveryAction: Decodable, Equatable {
        case confirmed(receipt: TransferV1.CaptureApplied)
        case replayReceipt(documentBefore: Document, receipt: TransferV1.CaptureApplied)
        case retryStatus(captureId: String, reason: String)
        enum CodingKeys: String, CodingKey { case action, receipt, documentBefore = "document_before", captureId = "capture_id", reason }
        init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            switch try c.decode(String.self, forKey: .action) {
            case "confirmed": self = .confirmed(receipt: try c.decode(TransferV1.CaptureApplied.self, forKey: .receipt))
            case "replay_receipt":
                self = .replayReceipt(documentBefore: try c.decode(Document.self, forKey: .documentBefore),
                                      receipt: try c.decode(TransferV1.CaptureApplied.self, forKey: .receipt))
            case "retry_status":
                self = .retryStatus(captureId: try c.decode(String.self, forKey: .captureId), reason: try c.decode(String.self, forKey: .reason))
            case let other: throw DecodingError.dataCorruptedError(forKey: .action, in: c, debugDescription: "unknown recovery action \(other)")
            }
        }
    }
    struct RecoveryPlan: Decodable, Equatable {
        var actions: [RecoveryAction]
        var recovery: RecoveryExport
    }
    struct RecoveryImport: Encodable {
        var snapshotToken: String
        var observations: [BridgeObservation]
        enum CodingKeys: String, CodingKey { case snapshotToken = "snapshot_token", observations }
    }
    struct RecoveryExportRequest: Encodable { var id: String; var operation = "recovery_export" }
    struct RecoveryImportRequest: Encodable { var id: String; var operation = "recovery_import"; var recovery: RecoveryImport }

    // Requests (id + operation + flattened fields).
    struct StatusRequest: Encodable { var id: String; var operation = "status" }
    struct InitializeRequest: Encodable { var id: String; var operation = "initialize"; var document: Document }
    struct ApplyRequest: Encodable { var id: String; var operation = "apply"; var edit: TransferV1.CaptureEdit }
    struct ReplaceRequest: Encodable {
        var id: String; var operation = "replace_document"
        var expectedRevision: Int; var expectedSha256: String; var text: String
        enum CodingKeys: String, CodingKey { case id, operation, expectedRevision = "expected_revision", expectedSha256 = "expected_sha256", text }
    }
    struct ConfirmRequest: Encodable { var id: String; var operation = "confirm"; var receipt: TransferV1.CaptureApplied }

    static let maxReplacementBytes = 64 * 1024
}

/// Client for one `flashtex-edit-ledger` helper process (one per document
/// store). Same process plumbing as the bridge client; completions on `queue`.
final class EditLedgerClient {
    typealias Failure = LineProcessFailure
    typealias Event = LineProcessClient.Event

    let storeDirectory: URL
    private let core: LineProcessClient
    var executable: URL { core.executable }
    var isRunning: Bool { core.isRunning }
    var pendingWriteBytes: Int { core.pendingWriteBytes }

    private struct Header: Decodable {
        var id: String?
        var error: TransferV1.ErrorPayload?
        var sessionId: String?
        var sequence: Int?
        var documentRevision: Int?
        var documentSha256: String?
        var commandSucceeded: Bool?
        enum CodingKeys: String, CodingKey {
            case id, error, sessionId = "session_id", sequence, documentRevision = "document_revision"
            case documentSha256 = "document_sha256", commandSucceeded = "command_succeeded"
        }
        var meta: EditLedgerV1.ServiceMeta? {
            guard let sessionId, let sequence else { return nil }
            return .init(sessionId: sessionId, sequence: sequence, documentRevision: documentRevision,
                         documentSha256: documentSha256, commandSucceeded: commandSucceeded ?? (error == nil))
        }
    }
    private struct Reply<P: Decodable>: Decodable { var payload: P }

    private let metaLock = NSLock()
    private var sessionId: String?
    private var lastSequence = 0
    /// Ledger session identity as reported by the helper (nil until its first reply
    /// or for helpers that predate the service metadata).
    var ledgerSessionId: String? { metaLock.withLock { sessionId } }
    /// Latest durable revision/hash the helper reported with any reply.
    private(set) var lastObserved: (revision: Int?, sha256: String?) = (nil, nil)

    /// Accepts a reply's metadata: the first session id is pinned; a different
    /// session or a non-increasing sequence is a stale/foreign observation.
    private func admit(_ meta: EditLedgerV1.ServiceMeta?) -> LineProcessFailure? {
        guard let meta else { return nil }
        return metaLock.withLock {
            if let sessionId, sessionId != meta.sessionId {
                return .protocolViolation("reply from ledger session \(meta.sessionId), expected \(sessionId)")
            }
            if sessionId == nil { sessionId = meta.sessionId }
            guard meta.sequence > lastSequence else {
                return .protocolViolation("stale ledger reply sequence \(meta.sequence) ≤ \(lastSequence)")
            }
            lastSequence = meta.sequence
            lastObserved = (meta.documentRevision, meta.documentSha256)
            return nil
        }
    }

    /// `arguments` precede `--store <dir>` (a Python double: `python3 fake_edit_ledger.py --store <dir>`).
    init(executable: URL, arguments: [String] = [], storeDirectory: URL, queue: DispatchQueue = .main,
         events: @escaping (Event) -> Void = { _ in }) throws {
        self.storeDirectory = storeDirectory
        core = try LineProcessClient(
            executable: executable, arguments: arguments + ["--store", storeDirectory.path], label: "ledger", queue: queue,
            classify: { line in
                guard let h = try? JSONDecoder().decode(Header.self, from: line) else { return nil }
                return .init(id: h.id, type: h.error == nil ? "payload" : "error", error: h.error)
            },
            events: events)
    }

    func terminate() { core.terminate() }
    func terminateAndWait(grace: TimeInterval = 2) async { await core.terminateAndWait(grace: grace) }

    func send<Req: Encodable, Rep: Decodable>(_ make: (String) -> Req, as replyType: Rep.Type, timeout: TimeInterval? = nil,
                                               completion: @escaping (Result<Rep, Failure>) -> Void) {
        let id = core.makeID()
        let line: Data
        do {
            let enc = JSONEncoder()
            enc.outputFormatting = [.withoutEscapingSlashes]
            var data = try enc.encode(make(id))
            data.append(0x0A)
            line = data
        } catch {
            completion(.failure(.undecodable("encoding failed: \(error)")))
            return
        }
        core.enqueue(id: id, line: line, expected: "payload", timeout: timeout) { [weak self] result in
            switch result {
            case .failure(let f): completion(.failure(f))
            case .success(let data):
                let meta = (try? JSONDecoder().decode(Header.self, from: data))?.meta
                if let stale = self?.admit(meta) { completion(.failure(stale)); return }
                do { completion(.success(try JSONDecoder().decode(Reply<Rep>.self, from: data).payload)) }
                catch { completion(.failure(.undecodable("\(Rep.self): \(error)"))) }
            }
        }
    }

    func request<Req: Encodable, Rep: Decodable>(_ make: @escaping (String) -> Req, as replyType: Rep.Type = Rep.self,
                                                  timeout: TimeInterval? = nil) async throws -> Rep {
        try await withCheckedThrowingContinuation { cont in
            send(make, as: replyType, timeout: timeout) { cont.resume(with: $0) }
        }
    }

    // MARK: operations

    func status(timeout: TimeInterval? = nil) async throws -> EditLedgerV1.Status {
        try await request({ EditLedgerV1.StatusRequest(id: $0) }, as: EditLedgerV1.Status.self, timeout: timeout)
    }
    func initialize(_ document: EditLedgerV1.Document) async throws -> EditLedgerV1.Document {
        try await request({ EditLedgerV1.InitializeRequest(id: $0, document: document) }, as: EditLedgerV1.DocumentReply.self).document
    }
    func apply(_ edit: TransferV1.CaptureEdit, timeout: TimeInterval? = nil) async throws -> EditLedgerV1.Applied {
        try await request({ EditLedgerV1.ApplyRequest(id: $0, edit: edit) }, as: EditLedgerV1.Applied.self, timeout: timeout)
    }
    func replaceDocument(expectedRevision: Int, expectedSha256: String, text: String) async throws -> EditLedgerV1.Document {
        try await request({ EditLedgerV1.ReplaceRequest(id: $0, expectedRevision: expectedRevision, expectedSha256: expectedSha256, text: text) },
                          as: EditLedgerV1.DocumentReply.self).document
    }
    func confirm(_ receipt: TransferV1.CaptureApplied) async throws -> TransferV1.CaptureApplied {
        try await request({ EditLedgerV1.ConfirmRequest(id: $0, receipt: receipt) }, as: EditLedgerV1.Confirmed.self).confirmed
    }
    func recoveryExport(timeout: TimeInterval? = nil) async throws -> EditLedgerV1.RecoveryExport {
        try await request({ EditLedgerV1.RecoveryExportRequest(id: $0) }, as: EditLedgerV1.RecoveryExport.self, timeout: timeout)
    }
    func recoveryImport(snapshotToken: String, observations: [EditLedgerV1.BridgeObservation], timeout: TimeInterval? = nil) async throws -> EditLedgerV1.RecoveryPlan {
        try await request({ EditLedgerV1.RecoveryImportRequest(id: $0, recovery: .init(snapshotToken: snapshotToken, observations: observations)) },
                          as: EditLedgerV1.RecoveryPlan.self, timeout: timeout)
    }

    /// `$FLASHTEX_EDIT_LEDGER`, a bundled `flashtex-edit-ledger`, then
    /// `crates/edit-ledger/target/{release,debug}/flashtex-edit-ledger`.
    @MainActor static func locate() -> URL? {
        BridgeClient.locateHelper(named: "flashtex-edit-ledger", environment: "FLASHTEX_EDIT_LEDGER", crate: "edit-ledger")
    }

    /// One private store per document under `<captures store>/documents/`:
    /// keyed by the SHA-256 of the file path for file-backed documents. An
    /// unsaved buffer has no stable identity, so it gets a fresh store per
    /// attach (its pending receipts cannot be recovered after a restart).
    static func storeDirectory(under root: URL, documentURL: URL?) -> URL {
        let key = documentURL.map { "file-" + SourceDigest.sha256Hex($0.standardizedFileURL.path).prefix(32) }
            ?? "unsaved-\(ProcessInfo.processInfo.processIdentifier)-\(UUID().uuidString.lowercased())"
        return root.appendingPathComponent("documents").appendingPathComponent(String(key))
    }
}
