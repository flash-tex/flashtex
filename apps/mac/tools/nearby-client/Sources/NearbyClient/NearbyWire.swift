import CryptoKit
import Foundation

/// Wire types of the nearby transport (apps/mac/docs/nearby-v1-proposal.md §4).
///
/// KEEP IN SYNC — these are copies of the Mac-side definitions so this package
/// has no dependency on the app:
///   `Envelope`, `CaptureSubmit`, `CaptureImage`, `LineSplitter`
///       ← apps/mac/Sources/FlashTeXProtocol/{RuntimeV1,Capture,JSONLines}.swift
///   `Hello`, `HelloAck`, `Destination`, `CaptureReceived`, `ErrorPayload`
///       ← apps/mac/Sources/FlashTeXMac/NearbyProtocol.swift
/// `NearbyReferenceClientTests` in apps/mac round-trips every one of them
/// through the real listener, so a drift shows up there.
public enum NearbyWire {
    /// Nearby protocol version carried in `hello.payload.protocol_version`.
    public static let version = 1
    /// runtime-v1 envelope version carried in every line.
    public static let envelopeVersion = 1
    public static let serviceType = "_flashtex._tcp"
    /// A line including its newline may not exceed this (transfer-v1 bound).
    public static let maxLineBytes = 12 * 1024 * 1024
    public static let maxIDBytes = 128
    public static let acceptedMimeTypes: Set<String> = ["image/png", "image/jpeg"]
    /// Encoded PNG/JPEG bytes the Mac accepts (transfer-v1 / proposal §4); the
    /// Mac also caps width and height at `maxImageSide` and refuses images
    /// that are not structurally valid (`invalid_image`).
    public static let maxImageBytes = 8 * 1024 * 1024
    public static let maxImageSide = 8192

    /// runtime-v1 envelope: `{protocol_version, id, type, payload}`.
    public struct Envelope<Payload: Codable>: Codable {
        public var protocolVersion: Int
        public var id: String
        public var type: String
        public var payload: Payload
        enum CodingKeys: String, CodingKey { case protocolVersion = "protocol_version", id, type, payload }
        public init(id: String, type: String, payload: Payload) {
            self.protocolVersion = NearbyWire.envelopeVersion; self.id = id; self.type = type; self.payload = payload
        }
    }

    /// Header only, for dispatching on `type` before decoding a payload. `id`
    /// is optional because `error` lines for unidentifiable requests carry `null`.
    public struct Header: Decodable {
        public var protocolVersion: Int
        public var id: String?
        public var type: String
        enum CodingKeys: String, CodingKey { case protocolVersion = "protocol_version", id, type }
    }

    /// First line on every connection. `role` is what the pre-TLS companion
    /// already sent; the Mac ignores it. Everything else is the nearby-v1 delta.
    public struct Hello: Codable, Equatable {
        public var role: String = "companion"
        public var pairId: String
        public var companionName: String
        public var protocolVersion: Int = NearbyWire.version
        public var nonce: String
        public var proof: String
        enum CodingKeys: String, CodingKey {
            case role, pairId = "pair_id", companionName = "companion_name"
            case protocolVersion = "protocol_version", nonce, proof
        }
        public init(pairId: String, companionName: String, nonce: String, proof: String) {
            self.pairId = pairId; self.companionName = companionName; self.nonce = nonce; self.proof = proof
        }
    }

    /// The Mac's pinned insertion anchor; copied verbatim into `capture_submit`.
    public struct Destination: Codable, Equatable {
        public var destinationId: String
        public var projectId: String
        public var path: String
        public var baseRevision: Int
        /// What kind of place the caret is in — text, inline/display math, a
        /// tabular cell, verbatim, a comment — and the wrapping a capture
        /// landing there needs. Additive and optional: a Mac that predates it
        /// omits the key and a companion that predates it ignores it.
        /// See protocol/proposals/transfer-v1-caret-context.md.
        public var caretContext: CaretContext?
        enum CodingKeys: String, CodingKey {
            case destinationId = "destination_id", projectId = "project_id", path, baseRevision = "base_revision"
            case caretContext = "caret_context"
        }
        public init(destinationId: String, projectId: String, path: String, baseRevision: Int,
                    caretContext: CaretContext? = nil) {
            self.destinationId = destinationId; self.projectId = projectId; self.path = path; self.baseRevision = baseRevision
            self.caretContext = caretContext
        }
    }

    /// What kind of place the Mac's caret is in, so the companion can say where
    /// a capture will land and how it will be wrapped. The Mac derives it; the
    /// companion only displays it. Mirrors `CaretContext` in `apps/mac` and
    /// `crates/bridge/src/caret.rs`.
    public struct CaretContext: Codable, Equatable {
        public var mode: String
        public var delimiter: String?
        public var environment: String?
        public var environments: [String]
        public var amsmath: Bool
        public var wrap: String
        enum CodingKeys: String, CodingKey { case mode, delimiter, environment, environments, amsmath, wrap }
        public init(mode: String, delimiter: String? = nil, environment: String? = nil,
                    environments: [String] = [], amsmath: Bool = false, wrap: String) {
            self.mode = mode; self.delimiter = delimiter; self.environment = environment
            self.environments = environments; self.amsmath = amsmath; self.wrap = wrap
        }

        /// One short line for the companion's destination row.
        public var label: String {
            switch wrap {
            case "display": return "text — formulas wrapped in \\[ … \\] or $ … $"
            case "inline":
                if let environment, !environment.isEmpty, mode == "text", environment.hasPrefix("tabular") || environment == "longtable" {
                    return "\(environment) cell — inline math only"
                }
                return "text — inline math only"
            case "already_math":
                if let environment, mode == "display_math" { return "\(environment) — already math, no delimiters added" }
                return "\(delimiter ?? "$") math — already math, no delimiters added"
            case "literal": return mode == "verbatim" ? "verbatim — inserted literally" : "comment — inserted literally"
            default: return wrap
            }
        }
    }

    /// `destination` is always present (explicit `null` when nothing is pinned);
    /// `pair_psk` only on the bootstrap (pairing-code) connection.
    public struct HelloAck: Codable, Equatable {
        public var macName: String
        public var nonce: String
        public var destination: Destination?
        public var pairPsk: String?
        enum CodingKeys: String, CodingKey { case macName = "mac_name", nonce, destination, pairPsk = "pair_psk" }
    }

    public struct DestinationReply: Codable, Equatable {
        public var destination: Destination?
    }

    public struct CaptureImage: Codable, Equatable {
        public var mimeType: String
        public var dataBase64: String
        enum CodingKeys: String, CodingKey { case mimeType = "mime_type", dataBase64 = "data_base64" }
        public init(mimeType: String, dataBase64: String) { self.mimeType = mimeType; self.dataBase64 = dataBase64 }
    }

    /// Unchanged runtime-v1 `capture_submit` payload.
    public struct CaptureSubmit: Codable, Equatable {
        public var captureId: String
        public var destinationId: String
        public var baseRevision: Int
        public var image: CaptureImage
        public var instructions: String
        enum CodingKeys: String, CodingKey {
            case captureId = "capture_id", destinationId = "destination_id", baseRevision = "base_revision", image, instructions
        }
        public init(captureId: String, destinationId: String, baseRevision: Int, image: CaptureImage, instructions: String) {
            self.captureId = captureId; self.destinationId = destinationId; self.baseRevision = baseRevision
            self.image = image; self.instructions = instructions
        }
    }

    /// transfer-v1 acknowledgement. `durable` is true only when the Mac's bridge
    /// journaled the capture; the in-memory inbox answers false.
    public struct CaptureReceived: Codable, Equatable {
        public var captureId: String
        public var durable: Bool
        public var hasProposal: Bool
        public var applied: Bool
        enum CodingKeys: String, CodingKey { case captureId = "capture_id", durable, hasProposal = "has_proposal", applied }
    }

    public struct ErrorPayload: Codable, Equatable {
        public var code: String
        public var message: String
    }
    /// Additive `capture_status` request (nearby-v1 §4, closes the §6 gap):
    /// what became of a capture this pairing submitted. Answered only for a
    /// `capture_id` the Mac acknowledged on this pairing (`unknown_capture`
    /// otherwise); a Mac that predates the message answers `unknown_type`.
    public struct CaptureStatusRequest: Codable, Equatable {
        public var captureId: String
        enum CodingKeys: String, CodingKey { case captureId = "capture_id" }
        public init(captureId: String) { self.captureId = captureId }
    }

    /// `capture_status_ack`. `state`: `received` (Mac inbox, no bridge) ·
    /// `journaled` · `converting` · `proposal_ready` · `inserted` ·
    /// `rejected` · `failed` · `uncertain`; show anything else verbatim
    /// (additive vocabulary). `latex` is the proposal text once one exists,
    /// read-only on the companion; `note` the Mac's detail; `new_revision`
    /// after an insertion.
    public struct CaptureStatus: Codable, Equatable {
        public var captureId: String
        public var state: String
        public var durable: Bool
        public var latex: String?
        public var note: String?
        public var newRevision: Int?
        enum CodingKeys: String, CodingKey {
            case captureId = "capture_id", state, durable, latex, note, newRevision = "new_revision"
        }
        public static let knownStates: [String] = ["received", "journaled", "converting", "proposal_ready", "inserted", "rejected", "failed", "uncertain"]
        /// A state after which polling can stop.
        public var isFinal: Bool { ["inserted", "rejected", "failed"].contains(state) }
        public var hasProposal: Bool { latex != nil }
        /// Normally decoded from the wire; constructed directly only to
        /// converge a row on a `capture_insert_ack` without waiting for the
        /// next poll (the synthesized memberwise init is internal to this
        /// package, so a companion could not).
        public init(captureId: String, state: String, durable: Bool, latex: String? = nil,
                    note: String? = nil, newRevision: Int? = nil) {
            self.captureId = captureId; self.state = state; self.durable = durable
            self.latex = latex; self.note = note; self.newRevision = newRevision
        }
    }

    /// Additive `capture_insert` request: the companion approves the proposal
    /// `capture_status_ack.latex` showed it and asks the Mac to apply it.
    ///
    /// `approvedLatexSha256` is `NearbyWire.proposalDigest` of exactly the text
    /// that was displayed. It is the approval token, not a checksum: a Mac
    /// whose proposal has changed since answers `proposal_changed` rather than
    /// inserting something the person never read.
    public struct CaptureInsertRequest: Codable, Equatable {
        public var captureId: String
        public var approvedLatexSha256: String
        enum CodingKeys: String, CodingKey {
            case captureId = "capture_id", approvedLatexSha256 = "approved_latex_sha256"
        }
        public init(captureId: String, approvedLatexSha256: String) {
            self.captureId = captureId; self.approvedLatexSha256 = approvedLatexSha256
        }
    }

    /// `capture_insert_ack`. `state` is a `CaptureStatus` state string —
    /// `inserted` on success, otherwise what the capture actually is now.
    public struct CaptureInsertAck: Codable, Equatable {
        public var captureId: String
        public var state: String
        public var newRevision: Int?
        public var note: String?
        enum CodingKeys: String, CodingKey {
            case captureId = "capture_id", state, newRevision = "new_revision", note
        }
        public init(captureId: String, state: String, newRevision: Int? = nil, note: String? = nil) {
            self.captureId = captureId; self.state = state; self.newRevision = newRevision; self.note = note
        }
    }

    /// Lowercase hex SHA-256 of a proposal's UTF-8 bytes. Mirrors
    /// `NearbyV1.proposalDigest` on the Mac; both ends must agree exactly, so
    /// `NearbyReferenceClientTests` pins them against each other.
    public static func proposalDigest(_ latex: String) -> String {
        SHA256.hash(data: Data(latex.utf8)).map { String(format: "%02x", $0) }.joined()
    }

    /// `error` envelope; `id` is `null` when the request could not be identified.
    public struct ErrorLine: Decodable {
        public var id: String?
        public var payload: ErrorPayload
    }

    public struct Empty: Codable, Equatable { public init() {} }

    /// Error codes after which the Mac closes the connection: fix the client or
    /// re-pair, never retry blindly (§8 "Parse these reply lines").
    /// `too_many_sessions` also closes, but is retryable once the companion has
    /// closed its older connections to that Mac.
    public static let closingErrorCodes: Set<String> = [
        "pair_mismatch", "pairing_expired", "hello_required", "unsupported_version", "line_too_long", "bad_request",
        "too_many_sessions",
    ]
    /// Backpressure (proposal §4 receive caps): the session stays open; wait
    /// for outstanding acknowledgements, then retry the *same* capture.
    public static let backpressureErrorCodes: Set<String> = ["too_many_in_flight", "inbox_full"]
    /// The Mac's user set this companion to view-only (pairs.json v3
    /// `permission`): session stays open, pairing intact, no re-pair and no
    /// new capture — the same capture is accepted once the permission changes.
    public static let permissionErrorCodes: Set<String> = ["capture_not_permitted"]
    /// The Mac refused this capture's content or identity; retrying the same
    /// bytes can only repeat the refusal. Build a new capture (new id, valid
    /// image, current destination) instead. The first line is the listener's
    /// own codes; the second line is what the Mac's bridge answers verbatim
    /// when one is attached (crates/bridge `validate`/`capture_anchor`):
    /// `destination_reselection_required` — the pinned target was unpinned or
    /// an edit overlapped it (reselect on the Mac; `hello_ack`/`destination`
    /// then report `null` or a new id), `revision_conflict` — `base_revision`
    /// is not the pin's revision, `instructions_too_large`, `invalid_id`.
    public static let captureInputErrorCodes: Set<String> = [
        "image_too_large", "invalid_image", "unsupported_image", "revision_mismatch", "capture_id_conflict", "bad_request",
        "destination_reselection_required", "revision_conflict", "instructions_too_large", "invalid_id",
    ]
    /// Subset of `captureInputErrorCodes` meaning the *destination* the capture
    /// was built against is gone: reselect the insertion point on the Mac
    /// (re-read `hello_ack.destination`) before building the new capture.
    public static let destinationErrorCodes: Set<String> = ["destination_reselection_required", "revision_conflict"]

    /// Client-side mirror of the Mac's cheap image checks: declared MIME must
    /// match the file signature and the encoded size must be within
    /// `maxImageBytes`. Structural validation stays the Mac's (`invalid_image`).
    public static func checkImage(_ bytes: Data, mimeType: String) -> String? {
        guard acceptedMimeTypes.contains(mimeType) else { return "mime_type must be image/png or image/jpeg" }
        guard !bytes.isEmpty else { return "image is empty" }
        guard bytes.count <= maxImageBytes else { return "image is \(bytes.count) bytes; the Mac accepts at most \(maxImageBytes)" }
        let png = bytes.starts(with: [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A])
        let jpeg = bytes.starts(with: [0xFF, 0xD8, 0xFF])
        if mimeType == "image/png", !png { return "mime_type image/png but the bytes are not a PNG" }
        if mimeType == "image/jpeg", !jpeg { return "mime_type image/jpeg but the bytes are not a JPEG" }
        return nil
    }

    // MARK: encoding

    public static func line<P: Codable>(id: String, type: String, _ payload: P) throws -> Data {
        let enc = JSONEncoder()
        enc.outputFormatting = [.withoutEscapingSlashes, .sortedKeys] // deterministic lines (order is not significant)
        var data = try enc.encode(Envelope(id: id, type: type, payload: payload))
        data.append(0x0A)
        return data
    }

    public static func header(of line: Data) throws -> Header {
        try JSONDecoder().decode(Header.self, from: line)
    }

    public static func decode<P: Codable>(_ line: Data, as: P.Type = P.self) throws -> Envelope<P> {
        try JSONDecoder().decode(Envelope<P>.self, from: line)
    }

    /// `capture_id`/`destination_id` rule the Mac enforces: 1–128 ASCII `[A-Za-z0-9_-]`.
    public static func isValidID(_ id: String) -> Bool {
        !id.isEmpty && id.utf8.count <= maxIDBytes
            && id.unicodeScalars.allSatisfy { $0.isASCII && (CharacterSet.alphanumerics.contains($0) || $0 == "-" || $0 == "_") }
    }
}

/// Splits a byte stream into complete lines, keeping a partial trailing line
/// (copy of FlashTeXProtocol.LineSplitter).
public struct LineSplitter {
    private var buffer = Data()
    private var scanned = 0
    public init() {}

    public mutating func append(_ data: Data) -> [Data] {
        buffer.append(data)
        var lines: [Data] = []
        var start = 0
        buffer.withUnsafeBytes { (raw: UnsafeRawBufferPointer) in
            var i = scanned
            let n = raw.count
            while i < n {
                if raw[i] == 0x0A {
                    lines.append(Data(raw[start..<i]))
                    start = i + 1
                }
                i += 1
            }
        }
        if start > 0 { buffer.removeSubrange(0..<start) }
        scanned = buffer.count
        return lines
    }

    public var pendingBytes: Int { buffer.count }
}
