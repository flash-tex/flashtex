import Foundation
import FlashTeXProtocol

/// Talks to a Rust worker process over runtime v1 JSON Lines (stdin/stdout).
/// Decoding happens off the main thread; callbacks are delivered on `queue`.
final class WorkerClient {
    enum Event {
        case result(RuntimeV1.Envelope<RuntimeV1.CompileResult>)
        /// Negotiated `display-list-v2` sibling line (protocol_version 2, type
        /// display_list) for request `id`; decoded off-main by V2Loader (PreviewV2View.swift).
        case displayList(id: String, line: Data)
        case error(id: String, message: String)
        case protocolViolation(String)
        case stderr(String)
        case exited(Int32)
    }

    let executable: URL
    private let process = Process()
    private let stdin = Pipe()
    private let stdout = Pipe()
    private let stderr = Pipe()
    private var splitter = LineSplitter()
    private let queue: DispatchQueue
    private let handler: (Event) -> Void
    private let lock = NSLock()
    private let stateLock = NSLock()
    private var violated = false
    /// Optional verbatim record of every line sent/received (see `RuntimeTranscript`).
    let transcript: RuntimeTranscript?

    init(executable: URL, arguments: [String] = [], queue: DispatchQueue = .main,
         transcript: RuntimeTranscript? = nil, handler: @escaping (Event) -> Void) throws {
        self.executable = executable
        self.queue = queue
        self.handler = handler
        self.transcript = transcript
        process.executableURL = executable
        process.arguments = arguments
        // Direct producer route: the bundled rooted TFM directory is
        // prepended to FLASHTEX_TFM_DIRS (BundledMetrics.swift, GH36).
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
            self.deliver { self.handler(.stderr(s)) }
        }
        process.terminationHandler = { [weak self] p in
            guard let self else { return }
            self.stdout.fileHandleForReading.readabilityHandler = nil
            self.stderr.fileHandleForReading.readabilityHandler = nil
            // Drain whatever is left, then flag an unterminated trailing line:
            // a partial JSON object at EOF is a protocol failure, not silence.
            let rest = self.stdout.fileHandleForReading.readDataToEndOfFile()
            self.consume(rest)
            let pending = self.stateLock.withLock { self.splitter.pendingBytes }
            if pending > 0 {
                self.deliver { self.handler(.protocolViolation("worker exited with \(pending) unterminated trailing bytes")) }
            }
            self.deliver { self.handler(.exited(p.terminationStatus)) }
        }
        try process.run()
    }

    var isRunning: Bool { process.isRunning }

    func send(_ request: RuntimeV1.CompileRequest, id: String) throws {
        let line = try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: id, request))
        lock.lock(); defer { lock.unlock() }
        try stdin.fileHandleForWriting.write(contentsOf: line)
        transcript?.record(line)
    }

    func terminate() {
        try? stdin.fileHandleForWriting.close()
        if process.isRunning { process.terminate() }
    }

    private func consume(_ data: Data) {
        guard !data.isEmpty else { return }
        let (lines, pending, alreadyViolated) = stateLock.withLock {
            (splitter.append(data), splitter.pendingBytes, violated)
        }
        guard !alreadyViolated else { return }
        // Both a complete oversized line and an oversized partial buffer are rejected.
        if let big = lines.first(where: { $0.count > RuntimeV1.maxLineBytes }) {
            violate("line of \(big.count) bytes exceeds the \(RuntimeV1.maxLineBytes)-byte limit")
            return
        }
        if pending > RuntimeV1.maxLineBytes {
            violate("unterminated line exceeds the \(RuntimeV1.maxLineBytes)-byte limit")
            return
        }
        for line in lines where !line.isEmpty {
            let t0 = MonotonicClock.nowNs()
            let event = Self.decode(line)
            let t1 = MonotonicClock.nowNs()
            // The runtime-v1 transcript (check_runtime.py) records v1 lines only; a
            // negotiated v2 display_list line is not part of that contract's record.
            if case .displayList = event {} else { transcript?.record(line) }
            if TypingBench.isBenchActive {
                if case .displayList(let id, _) = event { FlashTeXLog.write("worker: display_list \(id) line \(line.count) B received at \(t1) (header probe \(Double(t1 - t0) / 1e6) ms)") }
                else { FlashTeXLog.write("worker: line \(line.count) B decoded in \(Double(t1 - t0) / 1e6) ms at \(t1)") }
            }
            deliver {
                if TypingBench.isBenchActive { FlashTeXLog.write("worker: event on main at \(MonotonicClock.nowNs())") }
                self.handler(event)
            }
        }
    }

    /// Hands a decoded event to the main thread. `DispatchQueue.main.async` is
    /// only drained when AppKit's run loop gets around to the dispatch port,
    /// which under typing was a full turn (~30 ms) after the result arrived
    /// (typing-bench timeline: decoded → "event on main" 30 ms, worker 0.7 ms).
    /// A run-loop block plus an explicit wake-up runs at the head of the next
    /// iteration: send → applied p50 32 ms → 0.8 ms on a one-page document.
    /// Other queues keep plain dispatch.
    private func deliver(_ block: @escaping @Sendable () -> Void) {
        if queue === DispatchQueue.main {
            CFRunLoopPerformBlock(CFRunLoopGetMain(), CFRunLoopMode.commonModes.rawValue, block)
            CFRunLoopWakeUp(CFRunLoopGetMain())
        } else {
            queue.async(execute: block)
        }
    }

    private func violate(_ message: String) {
        stateLock.withLock { violated = true; splitter = LineSplitter() }
        deliver { self.handler(.protocolViolation(message)) }
        terminate()
    }

    static func decode(_ line: Data) -> Event {
        do {
            // runtime-v1-display-list-v2.md: the sibling line is ~2 MB; probe its
            // header with the payload skipped by a byte scan (JSONDecoder over the
            // whole line cost 11 ms p50 on this thread) and hand the bytes to V2Loader.
            if let probe = RenderingV2Fast.header(line), probe.protocolVersion == 2,
               probe.type == "display_list" || probe.type == DisplayListDelta.messageType {
                return .displayList(id: probe.id, line: line)
            }
            let header = try RuntimeV1.header(of: line)
            guard header.protocolVersion == RuntimeV1.protocolVersion else {
                return .protocolViolation("unsupported protocol_version \(header.protocolVersion)")
            }
            switch header.type {
            case "compile_result":
                return .result(try RuntimeV1.decodeCompileResult(line))
            case "error":
                let env = try JSONDecoder().decode(RuntimeV1.Envelope<RuntimeV1.ErrorPayload>.self, from: line)
                return .error(id: env.id, message: env.payload.message)
            default:
                return .protocolViolation("unexpected message type \(header.type)")
            }
        } catch {
            return .protocolViolation("undecodable line: \(error)")
        }
    }
}
