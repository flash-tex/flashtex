import Foundation
import FlashTeXProtocol

/// Transport to the FT-007 capture bridge (`flashtex-bridge --store <dir>`):
/// transfer-v1 JSON Lines on stdin/stdout, one reply per request, correlated by
/// the request `id`. Decoding happens off the main thread; completions are
/// delivered on `queue` (main by default) so callers never block the UI.
/// The process plumbing (bounded serial I/O queue, 12 MiB limits, exit and
/// violation handling) lives in `LineProcessClient`.
final class BridgeClient {
    typealias Failure = LineProcessFailure
    typealias Event = LineProcessClient.Event

    let storeDirectory: URL
    private let core: LineProcessClient
    var executable: URL { core.executable }
    var isRunning: Bool { core.isRunning }
    var pendingWriteBytes: Int { core.pendingWriteBytes }

    private struct LenientHeader: Decodable {
        var protocolVersion: Int
        var id: String?
        var type: String
        enum CodingKeys: String, CodingKey { case protocolVersion = "protocol_version", id, type }
    }

    /// `arguments` precede `--store <dir>` so a test double can be
    /// `python3 fake_bridge.py --store <dir>`.
    /// `environment` nil inherits the app's; the shell passes
    /// `ConversionCredential.bridgeEnvironment` (secrets stripped, the
    /// provider's key variable added only when a key resolved). `provider`
    /// selects the bridge's conversion flag (`ConversionProvider.bridgeFlag`;
    /// `.none` passes nothing and `capture_convert` is refused).
    init(executable: URL, arguments: [String] = [], storeDirectory: URL, provider: ConversionProvider = .none,
         environment: [String: String]? = nil,
         queue: DispatchQueue = .main, events: @escaping (Event) -> Void = { _ in }) throws {
        self.storeDirectory = storeDirectory
        core = try LineProcessClient(
            executable: executable,
            arguments: arguments + ["--store", storeDirectory.path] + (provider.bridgeFlag.map { [$0] } ?? []),
            label: "bridge", queue: queue, environment: environment,
            classify: { line in
                guard let header = try? JSONDecoder().decode(LenientHeader.self, from: line) else { return nil }
                guard header.protocolVersion == RuntimeV1.protocolVersion else {
                    return .init(id: header.id, type: "unsupported_version", error: nil)
                }
                var error: TransferV1.ErrorPayload?
                if header.type == "error" {
                    error = (try? JSONDecoder().decode(RuntimeV1.Envelope<TransferV1.ErrorPayload>.self, from: line).payload)
                        ?? .init(code: "undecodable_error", message: String(decoding: line, as: UTF8.self))
                }
                return .init(id: header.id, type: header.type, error: error)
            },
            events: events)
    }

    func terminate() { core.terminate() }

    // MARK: requests

    /// Sends one request and decodes the reply of `request.replyType`. An
    /// `error` envelope becomes `Failure.bridge`; any other type is
    /// `unexpectedReply`. Completion runs on `queue`. The write itself is queued
    /// on the I/O queue (bounded, else `.backpressure`); `timeout` fails the
    /// request if no reply arrives in time.
    func send<Req: Encodable, Rep: Codable>(_ request: TransferV1.Request, _ payload: Req, as replyType: Rep.Type,
                                              timeout: TimeInterval? = nil,
                                              completion: @escaping (Result<Rep, Failure>) -> Void) {
        let id = core.makeID()
        let line: Data
        do {
            let enc = JSONEncoder()
            enc.outputFormatting = [.withoutEscapingSlashes]
            var data = try enc.encode(RequestEnvelope(id: id, type: request.rawValue, payload: payload))
            data.append(0x0A)
            line = data
        } catch {
            completion(.failure(.undecodable("encoding failed: \(error)")))
            return
        }
        core.enqueue(id: id, line: line, expected: request.replyType, timeout: timeout) { result in
            switch result {
            case .failure(let f): completion(.failure(f))
            case .success(let data):
                do {
                    completion(.success(try JSONDecoder().decode(RuntimeV1.Envelope<Rep>.self, from: data).payload))
                } catch {
                    completion(.failure(.undecodable("\(request.replyType): \(error)")))
                }
            }
        }
    }

    /// async/await form of `send`.
    func request<Req: Encodable, Rep: Codable>(_ request: TransferV1.Request, _ payload: Req, as replyType: Rep.Type = Rep.self,
                                                 timeout: TimeInterval? = nil) async throws -> Rep {
        try await withCheckedThrowingContinuation { cont in
            send(request, payload, as: replyType, timeout: timeout) { cont.resume(with: $0) }
        }
    }

    // MARK: discovery

    /// `$FLASHTEX_BRIDGE`, a `flashtex-bridge` next to the executable in a
    /// bundle, then `crates/bridge/target/{release,debug}/flashtex-bridge`.
    @MainActor static func locateBridge() -> URL? {
        locateHelper(named: "flashtex-bridge", environment: "FLASHTEX_BRIDGE", crate: "bridge")
    }

    @MainActor static func locateHelper(named name: String, environment: String, crate: String) -> URL? {
        let fm = FileManager.default
        if let env = ProcessInfo.processInfo.environment[environment], fm.isExecutableFile(atPath: env) {
            return URL(fileURLWithPath: env)
        }
        if let bundled = Bundle.main.executableURL?.deletingLastPathComponent().appendingPathComponent(name),
           fm.isExecutableFile(atPath: bundled.path) {
            return bundled
        }
        guard let root = ShellModel.locateRepoRoot() else { return nil }
        // The workspace builds every member into ./target (Cargo.toml); a
        // crate-local target is what a standalone build left.
        for profile in ["release", "debug"] {
            for dir in ["target/\(profile)", "crates/\(crate)/target/\(profile)"] {
                let url = root.appendingPathComponent("\(dir)/\(name)")
                if fm.isExecutableFile(atPath: url.path) { return url }
            }
        }
        return nil
    }

    /// `$FLASHTEX_BRIDGE_STORE` or `~/Library/Application Support/FlashTeX/captures`.
    static func defaultStoreDirectory() -> URL {
        if let env = ProcessInfo.processInfo.environment["FLASHTEX_BRIDGE_STORE"], !env.isEmpty {
            return URL(fileURLWithPath: env)
        }
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
            ?? FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent("Library/Application Support")
        return base.appendingPathComponent("FlashTeX/captures")
    }
}

/// Outgoing envelope; `RuntimeV1.Envelope` requires a Codable payload.
private struct RequestEnvelope<P: Encodable>: Encodable {
    var protocolVersion = RuntimeV1.protocolVersion
    var id: String
    var type: String
    var payload: P
    enum CodingKeys: String, CodingKey { case protocolVersion = "protocol_version", id, type, payload }
}
