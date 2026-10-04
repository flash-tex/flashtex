import FlashTeXCollabCore
import Foundation
import Network

/// One collab-v1 stream over TLS: frames in (`CollabWire.decode`), frames
/// out, and a single close report. Network.framework runs it on the main
/// queue, so every callback is on the main actor; collaboration traffic is
/// keystroke-sized, and the CRDT and the editor live there anyway.
@MainActor
final class CollabConnection {
    let nw: NWConnection
    var onReady: () -> Void = {}
    var onMessage: (CollabMessage) -> Void = { _ in }
    /// Called once, whatever ended the connection.
    var onClosed: (String) -> Void = { _ in }
    private var buffer: [UInt8] = []
    private(set) var isReady = false
    private(set) var isClosed = false
    private var deadline: DispatchWorkItem?
    /// Largest frame accepted. The hub keeps it small (`CollabHub.Limits.
    /// preJoinFrameBytes`) until a connection has joined, so an
    /// unauthenticated peer cannot make it buffer megabytes.
    var maxFrameBytes = CollabWire.maxFrame
    /// The peer's address (per-address connection caps).
    var remoteAddress: String {
        if case let .hostPort(host, _) = nw.endpoint { return "\(host)" }
        return "\(nw.endpoint)"
    }
    /// Bytes handed to the stack and not yet reported sent.
    private(set) var unsentBytes = 0
    static let maxUnsentBytes = 64 * 1024 * 1024

    init(_ nw: NWConnection) { self.nw = nw }

    func start(handshakeTimeout: TimeInterval = 10) {
        nw.stateUpdateHandler = { [weak self] state in
            MainActor.assumeIsolated {
                guard let self else { return }
                switch state {
                case .ready:
                    guard !self.isReady else { return }
                    self.isReady = true
                    self.disarm()
                    self.onReady()
                    self.receive()
                case let .failed(error): self.finish("failed: \(error)")
                case let .waiting(error): self.finish("unreachable: \(error)")
                case .cancelled: self.finish("closed")
                default: break
                }
            }
        }
        arm(after: handshakeTimeout) { [weak self] in self?.close(reason: "the TLS handshake timed out") }
        nw.start(queue: .main)
    }

    /// Runs `fire` after `seconds` unless re-armed or disarmed first.
    func arm(after seconds: TimeInterval, _ fire: @escaping @MainActor () -> Void) {
        deadline?.cancel()
        let item = DispatchWorkItem { [weak self] in
            MainActor.assumeIsolated {
                guard let self, !self.isClosed else { return }
                self.deadline = nil
                fire()
            }
        }
        deadline = item
        DispatchQueue.main.asyncAfter(deadline: .now() + seconds, execute: item)
    }

    func disarm() {
        deadline?.cancel()
        deadline = nil
    }

    private func receive() {
        nw.receive(minimumIncompleteLength: 1, maximumLength: 256 * 1024) { [weak self] data, _, complete, error in
            MainActor.assumeIsolated {
                guard let self, !self.isClosed else { return }
                if let data, !data.isEmpty { self.consume(data) }
                if self.isClosed { return }
                if let error { self.finish("receive error: \(error)"); return }
                if complete { self.close(reason: "the peer closed the connection"); return }
                self.receive()
            }
        }
    }

    private func consume(_ data: Data) {
        buffer.append(contentsOf: data)
        while !isClosed, buffer.count >= 4 {
            let len = Int(buffer[0]) | Int(buffer[1]) << 8 | Int(buffer[2]) << 16 | Int(buffer[3]) << 24
            if len == 0 || len > maxFrameBytes {
                refuse(code: "bad_frame", message: "frame length \(len) is outside 1…\(maxFrameBytes)")
                return
            }
            guard buffer.count >= 4 + len else { return }
            let frame = Array(buffer[0..<(4 + len)])
            buffer.removeFirst(4 + len)
            do {
                guard let (msg, _) = try CollabWire.decode(frame) else { return }
                onMessage(msg)
            } catch {
                refuse(code: "bad_frame", message: "undecodable frame: \(error)")
                return
            }
        }
    }

    func send(_ msg: CollabMessage) {
        guard !isClosed else { return }
        let bytes = CollabWire.encode(msg)
        if unsentBytes + bytes.count > Self.maxUnsentBytes {
            close(reason: "the peer is not reading (more than \(Self.maxUnsentBytes) bytes queued)")
            return
        }
        unsentBytes += bytes.count
        nw.send(content: Data(bytes), completion: .contentProcessed { [weak self] _ in
            MainActor.assumeIsolated { self?.unsentBytes -= bytes.count }
        })
    }

    /// Sends an `error` and closes behind it.
    func refuse(code: String, message: String) {
        send(.error(.init(code: code, message: message)))
        closeAfterFlush(reason: "\(code): \(message)")
    }

    func closeAfterFlush(reason: String) {
        guard !isClosed else { return }
        nw.send(content: nil, contentContext: .finalMessage, isComplete: true, completion: .contentProcessed { [weak self] _ in
            MainActor.assumeIsolated { self?.nw.cancel() }
        })
        // A peer that keeps the window full (it is still sending what we
        // refused) can stall the final send forever: cancel regardless.
        let nw = self.nw
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) { nw.cancel() }
        report(reason)
    }

    func close(reason: String) {
        guard !isClosed else { return }
        report(reason)
        nw.cancel()
    }

    private func finish(_ reason: String) {
        if !isClosed { report(reason) }
        nw.cancel()
    }

    private func report(_ reason: String) {
        isClosed = true
        disarm()
        onClosed(reason)
    }
}

/// Lowercase 16-digit hex, as collab-v1 JSON writes participant ids.
public func collabHex(_ v: UInt64) -> String {
    let s = String(v, radix: 16)
    return String(repeating: "0", count: max(0, 16 - s.count)) + s
}

public func collabParseHex(_ s: String) -> UInt64? {
    guard s.count == 16 else { return nil }
    return UInt64(s, radix: 16)
}
