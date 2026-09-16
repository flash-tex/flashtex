import Foundation
import FlashTeXProtocol

/// Failure of one request to a JSON Lines helper process (bridge or edit ledger).
enum LineProcessFailure: Error, Equatable {
    /// The process answered with a structured `{code, message}` error.
    case bridge(TransferV1.ErrorPayload)
    case unexpectedReply(expected: String, actual: String)
    case undecodable(String)
    case requestTooLarge(Int)
    case notRunning
    case exited(Int32)
    case protocolViolation(String)
    /// No reply within the caller's deadline (the process may be busy).
    case timeout(TimeInterval)
    /// The outbound queue already holds `pendingBytes`; the process is not reading.
    case backpressure(pendingBytes: Int)

    var code: String? { if case .bridge(let e) = self { return e.code }; return nil }
    var text: String {
        switch self {
        case .bridge(let e): "\(e.code): \(e.message)"
        case .unexpectedReply(let expected, let actual): "expected \(expected) reply, got \(actual)"
        case .undecodable(let s): "undecodable reply: \(s)"
        case .requestTooLarge(let n): "request line of \(n) bytes exceeds \(TransferV1.maxLineBytes)"
        case .notRunning: "process is not running"
        case .exited(let code): "process exited (\(code))"
        case .protocolViolation(let s): "protocol violation: \(s)"
        case .timeout(let s): "no reply within \(Int(s)) s"
        case .backpressure(let n): "process is not reading; \(n) bytes still queued — try again later"
        }
    }

    /// Transport-level failures: the process did not answer the question, so
    /// nothing can be concluded about its durable records.
    var isTransient: Bool {
        if case .bridge = self { return false }
        return true
    }
}

/// One local helper process speaking newline-delimited JSON on private pipes.
/// Replies are correlated to requests by `id`; each protocol supplies a
/// `classify` function that extracts `(id, type, error)` from a reply line.
/// All stdin writes happen on a serial I/O queue bounded by `maxQueuedBytes`,
/// never on the caller's thread: a process that stops reading (a 90 s
/// conversion, a stalled fsync) blocks that queue only. Lines over 12 MiB in
/// either direction are refused; an oversized reply terminates the process.
final class LineProcessClient {
    typealias Failure = LineProcessFailure
    struct Classified { var id: String?; var type: String; var error: TransferV1.ErrorPayload? }
    enum Event {
        case stderr(String)
        case protocolViolation(String)
        case unsolicited(id: String?, type: String, code: String?)
        case exited(Int32)
    }

    let executable: URL
    let label: String
    private let process = Process()
    private let stdin = Pipe()
    private let stdout = Pipe()
    private let stderr = Pipe()
    private var splitter = LineSplitter()
    private let queue: DispatchQueue
    private let ioQueue: DispatchQueue
    private let events: (Event) -> Void
    private let classify: (Data) -> Classified?
    private let stateLock = NSLock()
    private var violated = false
    private var nextID = 1
    private var queuedBytes = 0
    private var stdinClosed = false
    private var pending: [String: (expected: String, done: (Result<Data, Failure>) -> Void)] = [:]
    static let maxQueuedBytes = 32 * 1024 * 1024

    /// Bytes accepted for writing but not yet handed to the pipe.
    var pendingWriteBytes: Int { stateLock.withLock { queuedBytes } }
    var isRunning: Bool { process.isRunning }
    var processIdentifier: Int32 { process.processIdentifier }

    /// `environment` nil inherits the app's environment; a helper that must
    /// see (or must not see) a credential gets an explicit dictionary.
    init(executable: URL, arguments: [String], label: String, queue: DispatchQueue = .main,
         environment: [String: String]? = nil,
         classify: @escaping (Data) -> Classified?, events: @escaping (Event) -> Void) throws {
        self.executable = executable
        self.label = label
        self.queue = queue
        self.ioQueue = DispatchQueue(label: "flashtex.\(label).io")
        self.events = events
        self.classify = classify
        // A write into a pipe whose reader died must surface as an error, not SIGPIPE.
        signal(SIGPIPE, SIG_IGN)
        process.executableURL = executable
        process.arguments = arguments
        if let environment { process.environment = environment }
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
            self.queue.async { self.events(.stderr(s)) }
        }
        process.terminationHandler = { [weak self] p in
            guard let self else { return }
            self.stdout.fileHandleForReading.readabilityHandler = nil
            self.stderr.fileHandleForReading.readabilityHandler = nil
            self.consume(self.stdout.fileHandleForReading.readDataToEndOfFile())
            // Drain stderr as well as stdout: clearing the readability handler
            // stops delivery, so the final burst -- the one that says why the
            // process is exiting -- was being dropped (same fix as
            // PreviewControllerClient).
            let trailing = self.stderr.fileHandleForReading.readDataToEndOfFile()
            if !trailing.isEmpty {
                let s = String(decoding: trailing, as: UTF8.self)
                self.queue.async { self.events(.stderr(s)) }
            }
            let pendingBytes = self.stateLock.withLock { self.splitter.pendingBytes }
            if pendingBytes > 0 {
                self.queue.async { self.events(.protocolViolation("\(self.label) exited with \(pendingBytes) unterminated trailing bytes")) }
            }
            self.failAll(.exited(p.terminationStatus))
            self.queue.async { self.events(.exited(p.terminationStatus)) }
        }
        try process.run()
    }

    func terminate() {
        stateLock.withLock { stdinClosed = true }
        // Close on the I/O queue so a write in progress finishes (or fails) first.
        ioQueue.async { [stdin] in try? stdin.fileHandleForWriting.close() }
        if process.isRunning { process.terminate() }
    }

    /// A fresh request id for this process.
    func makeID() -> String { stateLock.withLock { defer { nextID += 1 }; return "\(label)-\(nextID)" } }

    /// Queues `line` (already newline-terminated) and delivers the raw reply line
    /// whose classified id equals `id`. `expected` is the reply type required;
    /// an error reply becomes `.bridge`, another type `.unexpectedReply`.
    func enqueue(id: String, line: Data, expected: String, timeout: TimeInterval?,
                 completion: @escaping (Result<Data, Failure>) -> Void) {
        guard line.count <= TransferV1.maxLineBytes else {
            queue.async { completion(.failure(.requestTooLarge(line.count))) }
            return
        }
        guard process.isRunning, !stateLock.withLock({ stdinClosed }) else {
            queue.async { completion(.failure(.notRunning)) }
            return
        }
        let admitted: Bool = stateLock.withLock {
            guard queuedBytes + line.count <= Self.maxQueuedBytes else { return false }
            queuedBytes += line.count
            return true
        }
        guard admitted else {
            queue.async { completion(.failure(.backpressure(pendingBytes: self.pendingWriteBytes))) }
            return
        }
        stateLock.withLock { pending[id] = (expected, completion) }
        if let timeout {
            queue.asyncAfter(deadline: .now() + timeout) { [weak self] in
                guard let self, let entry = self.stateLock.withLock({ self.pending.removeValue(forKey: id) }) else { return }
                entry.done(.failure(.timeout(timeout)))
            }
        }
        ioQueue.async { [weak self] in
            guard let self else { return }
            defer { self.stateLock.withLock { self.queuedBytes -= line.count } }
            let closed = self.stateLock.withLock { self.stdinClosed }
            do {
                guard !closed else { throw Failure.notRunning }
                try self.stdin.fileHandleForWriting.write(contentsOf: line)
            } catch {
                let entry = self.stateLock.withLock { self.pending.removeValue(forKey: id) }
                self.queue.async { entry?.done(.failure(.notRunning)) }
            }
        }
    }

    // MARK: reading

    private func consume(_ data: Data) {
        guard !data.isEmpty else { return }
        let (lines, pendingBytes, alreadyViolated) = stateLock.withLock {
            (splitter.append(data), splitter.pendingBytes, violated)
        }
        guard !alreadyViolated else { return }
        if let big = lines.first(where: { $0.count + 1 > TransferV1.maxLineBytes }) {
            violate("line of \(big.count) bytes exceeds the \(TransferV1.maxLineBytes)-byte limit")
            return
        }
        if pendingBytes >= TransferV1.maxLineBytes {
            violate("unterminated line exceeds the \(TransferV1.maxLineBytes)-byte limit")
            return
        }
        for line in lines where !line.isEmpty { deliver(line) }
    }

    private func deliver(_ line: Data) {
        guard let reply = classify(line) else {
            queue.async { self.events(.protocolViolation("undecodable line from \(self.label): \(String(decoding: line.prefix(200), as: UTF8.self))")) }
            return
        }
        if reply.type == "unsupported_version" {
            violate("unsupported protocol version from \(label)")
            return
        }
        guard let id = reply.id, let entry = stateLock.withLock({ pending.removeValue(forKey: id) }) else {
            queue.async { self.events(.unsolicited(id: reply.id, type: reply.type, code: reply.error?.code)) }
            return
        }
        let result: Result<Data, Failure>
        if let error = reply.error {
            result = .failure(.bridge(error))
        } else if reply.type != entry.expected {
            result = .failure(.unexpectedReply(expected: entry.expected, actual: reply.type))
        } else {
            result = .success(line)
        }
        queue.async { entry.done(result) }
    }

    private func violate(_ message: String) {
        stateLock.withLock { violated = true; splitter = LineSplitter() }
        queue.async { self.events(.protocolViolation(message)) }
        failAll(.protocolViolation(message))
        terminate()
    }

    private func failAll(_ failure: Failure) {
        let entries = stateLock.withLock { defer { pending.removeAll() }; return Array(pending.values) }
        for entry in entries { queue.async { entry.done(.failure(failure)) } }
    }
}
