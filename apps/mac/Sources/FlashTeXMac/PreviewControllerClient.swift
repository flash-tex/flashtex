import Foundation
import FlashTeXProtocol

/// Client for `flashtex-preview-controller` (crates/preview-controller,
/// "Native helper protocol v1", STDIO.md): one durable-source helper per
/// project that owns the edit ledger, the lexical index and the original
/// compiler behind it. Frames are JSON Lines with `protocol_version:1`, the
/// configured `session_id`, a request `id` (null on asynchronous `update`
/// frames), `type` and `payload`.
///
/// Threading: stdin writes are serialized under a lock and never wait for a
/// reply; stdout is drained by the pipe's readability handler, decoded there,
/// and delivered to the main run loop with an explicit wake-up (same path as
/// `WorkerClient`, so a preview update is applied on the next iteration, not
/// the next dispatch-queue drain). Nothing here blocks the UI thread.
final class PreviewControllerClient {
    /// A decoded frame. `update` carries the helper's asynchronous events
    /// (`kind: preview | stale | discarded | …`); `result`/`error` correlate
    /// to a request id.
    enum Event {
        case ready(compilerError: String?, compilerMaxFrameBytes: Int, helperMaxOutputBytes: Int)
        case result(id: String, payload: JSONObject)
        case error(id: String?, message: String)
        case preview(PreviewUpdate)
        /// `kind: completed_snapshot` (HistoricalPreview.swift): a validated
        /// compile result for an OLDER source than the helper's current one.
        case completedSnapshot(HistoricalFrame)
        /// `kind: display_candidate` (ShellModel+DisplayCandidates.swift): the
        /// producer's untrusted rendering-v2 sibling for a current request.
        case displayCandidate(DisplayCandidateFrame)
        case update(kind: String, payload: JSONObject)
        case protocolViolation(String)
        case stderr(String)
        case exited(Int32)
    }

    /// `type:update, kind:preview`: a compile result for exact source versions.
    struct PreviewUpdate {
        var requestID: String
        var compileRevision: Int
        /// Durable revision per document path the preview was compiled from.
        var sourceVersions: [String: Int]
        var missingLayoutCapabilities: [String]
        var controllerTotalMs: Double?
        var runtimeTotalMs: Double?
        var result: RuntimeV1.Envelope<RuntimeV1.CompileResult>
    }

    typealias JSONObject = [String: Any]

    /// Startup configuration written to the JSON file the helper is launched with.
    struct Config {
        var sessionID: String
        var projectID: String
        var entryPath: String
        /// File-backed mode: the rooted project directory and an application
        /// owned private ledger root (mutually exclusive with `storePaths`).
        var projectRoot: URL?
        var privateLedgerRoot: URL?
        /// Store-backed mode: initialized ledger directories.
        var storePaths: [URL] = []
        var compilerPath: URL?
        var compilerMaxFrameBytes: Int?
        /// Explicit bibliography declarations (rooted non-entry paths, no
        /// duplicates) the helper indexes as `bibliography` at startup; the
        /// helper never infers a kind from an extension, and declarations
        /// must be supplied on every launch (DocumentKinds.swift persists them).
        var bibliographyPaths: [String] = []

        func json() -> JSONObject {
            var o: JSONObject = ["session_id": sessionID, "project_id": projectID, "entry_path": entryPath]
            if let projectRoot { o["project_root"] = projectRoot.path }
            if let privateLedgerRoot { o["private_ledger_root"] = privateLedgerRoot.path }
            if !storePaths.isEmpty { o["store_paths"] = storePaths.map(\.path) }
            if let compilerPath { o["compiler_path"] = compilerPath.path }
            if let compilerMaxFrameBytes { o["compiler_max_frame_bytes"] = compilerMaxFrameBytes }
            if !bibliographyPaths.isEmpty { o["bibliography_paths"] = bibliographyPaths }
            // Evidence only: the helper's own per-phase stderr timings (display
            // transport profile, optional-output serialization), logged as
            // `controller: {...}` lines. Off unless FLASHTEX_CONTROLLER_DIAGNOSTIC_TIMINGS=1.
            if ProcessInfo.processInfo.environment["FLASHTEX_CONTROLLER_DIAGNOSTIC_TIMINGS"] == "1" { o["diagnostic_timings"] = true }
            return o
        }
    }

    /// Helper output frames are bounded at 16 MiB each (STDIO.md).
    static let maxFrameBytes = 16 * 1024 * 1024

    /// The helper reads each stdin line through a 1 MiB bound
    /// (`crates/preview-controller/src/main.rs` `MAX_FRAME`; draft contract
    /// docs/contracts/runtime-v1-display-list-v2.md "Current helper stdin is
    /// bounded at 1 MiB"). Measured against the real helper (2026-09-12,
    /// docs/evidence/mac-v2-conformance-2026-09-12/helper-stdin-measurement.jsonl):
    /// an `edit` line of exactly 1,048,576 bytes including the newline is
    /// admitted as a durable revision; 1,048,577 and 2,097,156 bytes answer
    /// `error {id:null, message:"truncated or oversized input"}` and the
    /// helper then CLOSES stdout and exits (status 0) — every later request
    /// is a broken pipe. So the client refuses such a line locally, typed,
    /// before any byte reaches the helper (`RequestTooLarge`); nothing is
    /// split or truncated.
    static let maxRequestLineBytes = 1024 * 1024

    /// A request line the helper would refuse (and die on): refused here instead.
    struct RequestTooLarge: LocalizedError, Equatable {
        var type: String
        var lineBytes: Int
        var errorDescription: String? {
            "\(type) request of \(lineBytes) bytes exceeds the helper's \(PreviewControllerClient.maxRequestLineBytes)-byte stdin line limit; not sent (the helper would answer \"truncated or oversized input\" and exit)"
        }
    }

    /// The exact bytes `send` writes for a request (newline included).
    static func requestLine(sessionID: String, id: String, type: String, payload: JSONObject) throws -> Data {
        let frame: JSONObject = ["protocol_version": 1, "session_id": sessionID, "id": id, "type": type, "payload": payload]
        var line = try JSONSerialization.data(withJSONObject: frame, options: [.sortedKeys, .withoutEscapingSlashes])
        line.append(0x0A)
        return line
    }

    private static let refusedLock = NSLock()
    /// Requests refused locally by `maxRequestLineBytes` (tests/evidence).
    private(set) static var requestsRefusedTooLarge = 0

    let executable: URL
    let config: Config
    let configURL: URL
    private let process = Process()
    private let stdin = Pipe()
    private let stdout = Pipe()
    private let stderr = Pipe()
    private var splitter = LineSplitter()
    private let handler: (Event) -> Void
    private let writeLock = NSLock()
    private let stateLock = NSLock()
    private var violated = false
    private var nextID = 1
    /// The tail of what the helper wrote to stderr, so an exit can be reported
    /// with its reason instead of a bare status code. Bounded: a helper that
    /// logs steadily for an hour must not grow this without limit.
    private var stderrTail = ""
    private static let maxStderrTailBytes = 4 * 1024

    init(executable: URL, config: Config, handler: @escaping (Event) -> Void) throws {
        self.executable = executable
        self.config = config
        self.handler = handler
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent("flashtex-controller-\(config.sessionID)")
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        configURL = dir.appendingPathComponent("config.json")
        try JSONSerialization.data(withJSONObject: config.json(), options: [.sortedKeys]).write(to: configURL)
        process.executableURL = executable
        process.arguments = [configURL.path]
        // A write into a pipe whose helper died (measured: it exits after an
        // oversized stdin line) must surface as a thrown error, not SIGPIPE
        // (same rule as LineProcessClient).
        signal(SIGPIPE, SIG_IGN)
        // Helper-spawned producer route: the helper's compiler child inherits
        // this environment, so the bundled rooted TFM directory is prepended
        // to FLASHTEX_TFM_DIRS here too (BundledMetrics.swift, GH36).
        process.environment = BundledMetrics.producerEnvironment()
        process.standardInput = stdin
        process.standardOutput = stdout
        process.standardError = stderr
        stdout.fileHandleForReading.readabilityHandler = { [weak self] fh in
            self?.consume(fh.availableData)
        }
        stderr.fileHandleForReading.readabilityHandler = { [weak self] fh in
            let d = fh.availableData
            guard let self, !d.isEmpty else { return }
            let s = String(decoding: d, as: UTF8.self)
            self.noteStderr(s)
            self.deliver { self.handler(.stderr(s)) }
        }
        process.terminationHandler = { [weak self] p in
            guard let self else { return }
            self.stdout.fileHandleForReading.readabilityHandler = nil
            self.stderr.fileHandleForReading.readabilityHandler = nil
            self.consume(self.stdout.fileHandleForReading.readDataToEndOfFile())
            // Drain stderr too, exactly as stdout is drained above. Clearing
            // the readability handler stops delivery, so without this the last
            // burst -- which is where a helper says WHY it is exiting (a Rust
            // panic, "no such file") -- was dropped, and an exit arrived with
            // no reason attached. That cost a CI investigation: run
            // 35022802823 reported only `helper exited (1)`.
            let trailing = self.stderr.fileHandleForReading.readDataToEndOfFile()
            if !trailing.isEmpty {
                let s = String(decoding: trailing, as: UTF8.self)
                self.noteStderr(s)
                self.deliver { self.handler(.stderr(s)) }
            }
            let pending = self.stateLock.withLock { self.splitter.pendingBytes }
            if pending > 0 {
                self.deliver { self.handler(.protocolViolation("helper exited with \(pending) unterminated trailing bytes")) }
            }
            self.deliver { self.handler(.exited(p.terminationStatus)) }
        }
        try process.run()
    }

    var isRunning: Bool { process.isRunning }
    var processIdentifier: Int32 { process.processIdentifier }

    // MARK: requests

    /// Sends one operation; returns its request id. Never waits for the reply.
    @discardableResult
    func send(_ type: String, _ payload: JSONObject, id explicitID: String? = nil) throws -> String {
        let id = explicitID ?? stateLock.withLock { defer { nextID += 1 }; return "pc-\(nextID)" }
        let line = try Self.requestLine(sessionID: config.sessionID, id: id, type: type, payload: payload)
        guard line.count <= Self.maxRequestLineBytes else {
            Self.refusedLock.lock(); Self.requestsRefusedTooLarge += 1; Self.refusedLock.unlock()
            throw RequestTooLarge(type: type, lineBytes: line.count)
        }
        writeLock.lock(); defer { writeLock.unlock() }
        try stdin.fileHandleForWriting.write(contentsOf: line)
        return id
    }

    func document(path: String) throws -> String { try send("document", ["path": path]) }

    /// `sourceBindingToken` (≤ 128 bytes, opaque to the helper) is echoed on a
    /// `completed_snapshot` for this submission; nil sends the unchanged wire.
    func edit(path: String, expectedRevision: Int, expectedSHA256: String, text: String, sourceBindingToken: String? = nil) throws -> String {
        var payload: JSONObject = ["path": path, "expected_revision": expectedRevision, "expected_sha256": expectedSHA256, "text": text]
        if let sourceBindingToken { payload["source_binding_token"] = sourceBindingToken }
        return try send("edit", payload)
    }

    func compile(sourceBindingToken: String? = nil) throws -> String {
        try send("compile", sourceBindingToken.map { ["source_binding_token": $0] } ?? [:])
    }

    /// `export`: writes exactly the durable source at (`expectedRevision`,
    /// `expectedSHA256`) through the rooted project-files lock. The disk
    /// expectation is mandatory: the hash of the file we last saw, or `nil`
    /// (JSON null) for a file that must not exist yet.
    func export(path: String, expectedRevision: Int, expectedSHA256: String, expectedDiskSHA256: String?) throws -> String {
        try send("export", ["path": path, "expected_revision": expectedRevision, "expected_sha256": expectedSHA256,
                            "expected_disk_sha256": expectedDiskSHA256 ?? NSNull()])
    }

    func fileStatus(path: String) throws -> String { try send("file_status", ["path": path]) }
    func restart() throws -> String { try send("restart", [:]) }
    func snapshot() throws -> String { try send("snapshot", [:]) }

    func configureLayout(capabilities: [String]) throws -> String {
        try send("configure_layout", ["layout_capabilities": capabilities, "renderer_support_confirmed": true])
    }

    /// `configure_display_candidates` (crates/preview-controller/docs/display-forwarding.md):
    /// opts this session into (or out of) the producer's `display_list`
    /// sibling forwarded as `update {kind: display_candidate}`. Enabling
    /// carries the explicit renderer confirmation the helper requires; the
    /// reply is `result {capability, enabled, preview_error}`.
    func configureDisplayCandidates(enabled: Bool) throws -> String {
        var payload: JSONObject = ["capability": DisplayCandidates.capability, "enabled": enabled]
        if enabled { payload["renderer_support_confirmed"] = true }
        return try send(DisplayCandidates.operation, payload)
    }

    func close() {
        _ = try? send("close", [:])
        try? stdin.fileHandleForWriting.close()
    }

    /// Same lifetime-tying rule as `LineProcessClient` (#687): a dropped client
    /// still kills and reaps its helper, instead of relying on every call site
    /// to remember `terminate()`/`detachController()`.
    deinit {
        if process.isRunning { process.terminate() }
    }

    func terminate() {
        try? stdin.fileHandleForWriting.close()
        if process.isRunning { process.terminate() }
    }

    // MARK: decoding

    private func consume(_ data: Data) {
        guard !data.isEmpty else { return }
        let (lines, pending, alreadyViolated) = stateLock.withLock {
            (splitter.append(data), splitter.pendingBytes, violated)
        }
        guard !alreadyViolated else { return }
        if let big = lines.first(where: { $0.count > Self.maxFrameBytes }) {
            violate("frame of \(big.count) bytes exceeds the \(Self.maxFrameBytes)-byte limit")
            return
        }
        if pending > Self.maxFrameBytes {
            violate("unterminated frame exceeds the \(Self.maxFrameBytes)-byte limit")
            return
        }
        for line in lines where !line.isEmpty {
            let event = Self.decode(line, sessionID: config.sessionID)
            deliver { self.handler(event) }
        }
    }

    private func violate(_ message: String) {
        stateLock.withLock { violated = true; splitter = LineSplitter() }
        deliver { self.handler(.protocolViolation(message)) }
        terminate()
    }

    /// The last `maxStderrTailBytes` of the helper's stderr, trimmed, or nil
    /// when it said nothing. Evidence only — no control flow reads this.
    var recentStderr: String? {
        let tail = stateLock.withLock { stderrTail }.trimmingCharacters(in: .whitespacesAndNewlines)
        return tail.isEmpty ? nil : tail
    }

    private func noteStderr(_ s: String) {
        stateLock.withLock {
            stderrTail += s
            if stderrTail.utf8.count > Self.maxStderrTailBytes {
                // Trim on UTF-8, not Characters: `suffix(n)` counts grapheme
                // clusters, so a tail of multi-byte scalars would hold several
                // times the stated bound. Decoding repairs a scalar split at
                // the new start, which is fine for an evidence tail.
                stderrTail = String(decoding: Array(stderrTail.utf8.suffix(Self.maxStderrTailBytes)), as: UTF8.self)
            }
        }
    }

    /// Main run-loop delivery with an explicit wake-up (see `WorkerClient.deliver`).
    private func deliver(_ block: @escaping @Sendable () -> Void) {
        CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.commonModes.rawValue, block)
        CFRunLoopWakeUp(CFRunLoopGetMain())
    }

    /// Frames are read with FastJSON; the (large) `result` value of a preview
    /// update is kept as a byte range and parsed by the typed compile_result
    /// reader in place, so a 1.6 MB result is decoded once, not re-serialized.
    static func decode(_ line: Data, sessionID: String) -> Event {
        let frame: FastJSON.Value
        do { frame = try FastJSON.parse(line, rawKeys: ["result", "display_list"]) }
        catch { return .protocolViolation("frame is not valid JSON: \(error)") }
        guard let obj = frame.object else { return .protocolViolation("frame is not a JSON object") }
        guard obj["protocol_version"]?.int == 1 else {
            return .protocolViolation("unsupported protocol_version \(obj["protocol_version"].map { "\($0)" } ?? "missing")")
        }
        guard obj["session_id"]?.string == sessionID else {
            return .protocolViolation("frame for session \(obj["session_id"]?.string ?? "missing"), expected \(sessionID)")
        }
        let type = obj["type"]?.string ?? ""
        let id = obj["id"]?.string
        let payload = obj["payload"]?.object ?? [:]
        switch type {
        case "ready":
            return .ready(compilerError: payload["compiler_error"]?.string,
                          compilerMaxFrameBytes: payload["compiler_max_frame_bytes"]?.int ?? 0,
                          helperMaxOutputBytes: payload["helper_max_output_bytes"]?.int ?? 0)
        case "result":
            guard let id else { return .protocolViolation("result without id") }
            return .result(id: id, payload: Self.bridged(payload))
        case "error":
            return .error(id: id, message: payload["message"]?.string ?? "unspecified helper error")
        case "update":
            let kind = payload["kind"]?.string ?? ""
            if kind == CompletedSnapshots.updateKind { return HistoricalFrame.decode(line, payload: payload, frameSessionID: sessionID) }
            if kind == DisplayCandidates.updateKind { return DisplayCandidateFrame.decode(line, payload: payload, frameSessionID: sessionID) }
            guard kind == "preview" else { return .update(kind: kind, payload: Self.bridged(payload)) }
            do {
                let env: RuntimeV1.Envelope<RuntimeV1.CompileResult>
                switch payload["result"] {
                case .raw(let range)?:
                    do { env = try FastJSON.compileResultEnvelope(line, range: range) }
                    catch { env = try RuntimeV1.decodeCompileResultReference(line.subdata(in: range)) }
                case nil:
                    return .protocolViolation("preview update without result")
                default:
                    return .protocolViolation("preview update result is not an object")
                }
                guard env.protocolVersion == RuntimeV1.protocolVersion, env.type == "compile_result" else {
                    return .protocolViolation("preview update carries \(env.type) v\(env.protocolVersion)")
                }
                let versions = (payload["source_versions"]?.object ?? [:]).compactMapValues(\.int)
                return .preview(PreviewUpdate(
                    requestID: payload["request_id"]?.string ?? "",
                    compileRevision: payload["compile_revision"]?.int ?? 0,
                    sourceVersions: versions,
                    missingLayoutCapabilities: (payload["missing_layout_capabilities"]?.array ?? []).compactMap(\.string),
                    controllerTotalMs: payload["controller_total_ms"]?.double,
                    runtimeTotalMs: payload["runtime_total_ms"]?.double,
                    result: env))
            } catch {
                return .protocolViolation("preview update: \(error)")
            }
        default:
            return .protocolViolation("unknown frame type \(type)")
        }
    }

    /// Generic values as Foundation objects for the small `result`/`update`
    /// payloads the model reads by key.
    static func bridged(_ object: [String: FastJSON.Value]) -> JSONObject {
        object.mapValues(bridge)
    }

    private static func bridge(_ v: FastJSON.Value) -> Any {
        switch v {
        case .null: return NSNull()
        case .bool(let b): return b
        case .number(let d, let isInt): return isInt && d >= Double(Int.min) && d <= Double(Int.max) ? Int(d) : d
        case .string(let s): return s
        case .array(let a): return a.map(bridge)
        case .object(let o): return o.mapValues(bridge)
        case .raw: return NSNull()
        }
    }
}
