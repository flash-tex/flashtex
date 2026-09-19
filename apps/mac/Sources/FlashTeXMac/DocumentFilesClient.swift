import Foundation
import FlashTeXProtocol

/// Models for the private project-files helper protocol (`crates/project-files`,
/// `flashtex-project-files --root <dir>`, protocol `project-files-v1`): rooted,
/// symlink-refusing `read` / `status` / `save` on one project directory. Requests
/// are `{id, operation, ...fields}`; replies `{id, payload}` or
/// `{id, error: {code, message}}`. A save *conflict* is a payload, never an
/// error: the on-disk file no longer matches what the editor last saw, nothing
/// was written, and the shell must show the conflict and keep its buffer.
enum ProjectFilesV1 {
    static let protocolName = "project-files-v1"

    struct Ping: Decodable, Equatable {
        var `protocol`: String
        var root: String
        var pid: Int
    }

    /// One rooted read. `exists == false` carries only `path`.
    struct Read: Decodable, Equatable {
        var path: String
        var exists: Bool
        var text: String?
        var sha256: String?
        var bytes: Int?
        var mtimeUnixMs: Int?
        enum CodingKeys: String, CodingKey { case path, exists, text, sha256, bytes, mtimeUnixMs = "mtime_unix_ms" }
    }

    /// On-disk state relative to the hash the caller last saw.
    enum DiskState: String, Decodable, Equatable {
        case unchanged, modified, deleted, created
    }

    struct Status: Decodable, Equatable {
        var path: String
        var exists: Bool
        var state: DiskState
        var sha256: String?
        var bytes: Int?
        var mtimeUnixMs: Int?
        enum CodingKeys: String, CodingKey { case path, exists, state, sha256, bytes, mtimeUnixMs = "mtime_unix_ms" }
    }

    struct Receipt: Decodable, Equatable {
        var path: String
        var bytes: Int
        var sha256: String
        var mtimeUnixMs: Int
        enum CodingKeys: String, CodingKey { case path, bytes, sha256, mtimeUnixMs = "mtime_unix_ms" }
    }

    enum ConflictKind: String, Decodable, Equatable {
        /// The file exists with content other than `expected`.
        case modifiedExternally = "modified_externally"
        /// `expected` named a hash but the file is gone.
        case deletedExternally = "deleted_externally"
        /// A new file was expected but something already sits at the path.
        case alreadyExists = "already_exists"
        /// An out-of-contract writer interfered between check and rename.
        case modifiedDuringSave = "modified_during_save"
    }

    struct Conflict: Decodable, Equatable {
        var path: String
        var kind: ConflictKind
        /// The hash the caller expected (nil for a new file); for
        /// `modifiedDuringSave`, the hash of the bytes the helper wrote.
        var ours: String?
        /// The hash actually observed on disk (nil if deleted).
        var theirs: String?
        var mtimeUnixMs: Int?
        var size: Int?
        enum CodingKeys: String, CodingKey { case path, kind, ours, theirs, mtimeUnixMs = "mtime_unix_ms", size }
    }

    enum SaveOutcome: Decodable, Equatable {
        case saved(Receipt)
        case conflict(Conflict)
        enum CodingKeys: String, CodingKey { case outcome, receipt, conflict }
        init(from decoder: Decoder) throws {
            let c = try decoder.container(keyedBy: CodingKeys.self)
            switch try c.decode(String.self, forKey: .outcome) {
            case "saved": self = .saved(try c.decode(Receipt.self, forKey: .receipt))
            case "conflict": self = .conflict(try c.decode(Conflict.self, forKey: .conflict))
            case let other: throw DecodingError.dataCorruptedError(forKey: .outcome, in: c, debugDescription: "unknown save outcome \(other)")
            }
        }
    }

    /// What the caller believes is on disk before a save.
    enum Expected: Equatable {
        case newFile
        case any
        case hash(String)
        var wire: String {
            switch self {
            case .newFile: "new"
            case .any: "any"
            case .hash(let h): h
            }
        }
    }

    /// The `manifest` reply: the `flashtex.toml` governing the root (found
    /// by walking up from it), its warnings, the classified `texinputs`, the
    /// package inputs with their text, and the commented template for
    /// `entry`. One parser for the CLI, the helper and this app
    /// (`crates/project-manifest`); the shell never reads TOML itself.
    struct Manifest: Decodable, Equatable, Sendable {
        struct Project: Decodable, Equatable, Sendable { var entry: String?; var texinputs: [String]; var output: String? }
        struct Fonts: Decodable, Equatable, Sendable { var text: String?; var math: String?; var mono: String?; var sans: String? }
        struct Packages: Decodable, Equatable, Sendable { var source: String; var fetch: String; var pin: [String: String]; var path: [String: String] }
        struct Library: Decodable, Equatable, Sendable { var name: String }
        struct Body: Decodable, Equatable, Sendable { var project: Project; var fonts: Fonts; var packages: Packages; var library: Library? }
        struct Note: Decodable, Equatable, Sendable { var key: String; var message: String }
        /// One `[project] texinputs` entry: `inside` (`dir` under the root),
        /// `outside` (`dir` is the virtual `texinputs/<i>` mount, `path` the
        /// real directory) or `invalid` (`reason`).
        struct TexInput: Decodable, Equatable, Sendable {
            var index: Int; var raw: String; var location: String; var dir: String?; var path: String?; var reason: String?
        }
        /// One package input: a rooted path (or the virtual mount, with
        /// `origin` naming the real file), its kind (`package`, `class`,
        /// `tex`, `bibliography`), the `texinputs` index it came from (nil:
        /// the root's own file) and its text.
        struct File: Decodable, Equatable, Sendable {
            var path: String; var kind: String; var texinput: Int?; var origin: String?; var text: String; var sha256: String; var bytes: Int
        }
        var path: String?
        var exists: Bool
        var manifestDir: String?
        var manifest: Body
        var warnings: [Note]
        var texinputs: [TexInput]
        var files: [File]
        var diagnostics: [Note]
        var template: String
        enum CodingKeys: String, CodingKey {
            case path, exists, manifestDir = "manifest_dir", manifest, warnings, texinputs, files, diagnostics, template
        }
    }

    /// The `set_fonts` reply: the governing manifest's text (or the template
    /// for `entry` when there is none) with its `[fonts]` table replaced --
    /// `crates/project-manifest` `Manifest::with_fonts`, the only TOML
    /// writer -- for the shell to save at `path`. The helper writes nothing.
    /// `changed` false (and no `text`): nothing to write.
    struct SetFonts: Decodable, Equatable, Sendable {
        var path: String
        var exists: Bool
        var changed: Bool
        var text: String?
    }

    struct PingRequest: Encodable { var id: String; var operation = "ping" }
    struct ManifestRequest: Encodable { var id: String; var operation = "manifest"; var entry: String? }
    struct SetFontsRequest: Encodable {
        var id: String; var operation = "set_fonts"; var entry: String?; var fonts: [String: String]
    }
    struct ReadRequest: Encodable { var id: String; var operation = "read"; var path: String }
    struct StatusRequest: Encodable {
        var id: String; var operation = "status"; var path: String; var expectedSha256: String?
        enum CodingKeys: String, CodingKey { case id, operation, path, expectedSha256 = "expected_sha256" }
        func encode(to encoder: Encoder) throws {
            var c = encoder.container(keyedBy: CodingKeys.self)
            try c.encode(id, forKey: .id); try c.encode(operation, forKey: .operation); try c.encode(path, forKey: .path)
            try c.encode(expectedSha256, forKey: .expectedSha256) // explicit null = "I expect no file"
        }
    }
    struct SaveRequest: Encodable {
        var id: String; var operation = "save"; var path: String; var text: String; var expected: String; var force: Bool
    }
}

/// Client for one `flashtex-project-files` helper process bound to one project
/// root directory. Same private-pipe plumbing as the bridge and edit-ledger
/// clients (`LineProcessClient`); completions run on `queue`. Replies are
/// answered in request order by the helper, so one unanswered request blocks
/// every later one: callers that gave up on a reply (`outstanding > 0`) restart
/// the client rather than queueing behind it.
final class ProjectFilesClient {
    typealias Failure = LineProcessFailure
    typealias Event = LineProcessClient.Event

    let root: URL
    /// Arguments placed before `--root` (a Python double's script and flags).
    let arguments: [String]
    private let core: LineProcessClient
    private let counter = NSLock()
    private var inFlight = 0
    var executable: URL { core.executable }
    var isRunning: Bool { core.isRunning }
    /// Requests sent whose reply has not arrived (including ones a caller gave up on).
    var outstanding: Int { counter.withLock { inFlight } }

    private struct Header: Decodable { var id: String?; var error: TransferV1.ErrorPayload? }
    private struct Reply<P: Decodable>: Decodable { var payload: P }

    /// `arguments` precede `--root <dir>` (a Python double: `python3 fake_project_files.py --root <dir>`).
    init(executable: URL, arguments: [String] = [], root: URL, queue: DispatchQueue = .main,
         events: @escaping (Event) -> Void = { _ in }) throws {
        self.root = root
        self.arguments = arguments
        core = try LineProcessClient(
            executable: executable, arguments: arguments + ["--root", root.path], label: "project-files", queue: queue,
            classify: { line in
                guard let h = try? JSONDecoder().decode(Header.self, from: line) else { return nil }
                return .init(id: h.id, type: h.error == nil ? "payload" : "error", error: h.error)
            },
            events: events)
    }

    func terminate() { core.terminate() }

    /// Sends one request; `timeout` (if any) is `LineProcessClient`'s: the reply
    /// is dropped when late. Callers that want to *see* a late reply pass nil and
    /// enforce their own deadline (see `DocumentFilesState`).
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
        counter.withLock { inFlight += 1 }
        core.enqueue(id: id, line: line, expected: "payload", timeout: timeout) { [weak self] result in
            self?.counter.withLock { self?.inFlight -= 1 }
            switch result {
            case .failure(let f): completion(.failure(f))
            case .success(let data):
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

    func ping(timeout: TimeInterval? = nil) async throws -> ProjectFilesV1.Ping {
        try await request({ ProjectFilesV1.PingRequest(id: $0) }, as: ProjectFilesV1.Ping.self, timeout: timeout)
    }
    func read(path: String, timeout: TimeInterval? = nil) async throws -> ProjectFilesV1.Read {
        try await request({ ProjectFilesV1.ReadRequest(id: $0, path: path) }, as: ProjectFilesV1.Read.self, timeout: timeout)
    }
    func status(path: String, expectedSha256: String?, timeout: TimeInterval? = nil) async throws -> ProjectFilesV1.Status {
        try await request({ ProjectFilesV1.StatusRequest(id: $0, path: path, expectedSha256: expectedSha256) },
                          as: ProjectFilesV1.Status.self, timeout: timeout)
    }
    func save(path: String, text: String, expected: ProjectFilesV1.Expected, force: Bool = false,
              timeout: TimeInterval? = nil) async throws -> ProjectFilesV1.SaveOutcome {
        try await request({ ProjectFilesV1.SaveRequest(id: $0, path: path, text: text, expected: expected.wire, force: force) },
                          as: ProjectFilesV1.SaveOutcome.self, timeout: timeout)
    }
    func setFonts(entry: String?, fonts: [String: String], timeout: TimeInterval? = nil) async throws -> ProjectFilesV1.SetFonts {
        try await request({ ProjectFilesV1.SetFontsRequest(id: $0, entry: entry, fonts: fonts) }, timeout: timeout)
    }
    func manifest(entry: String?, timeout: TimeInterval? = nil) async throws -> ProjectFilesV1.Manifest {
        try await request({ ProjectFilesV1.ManifestRequest(id: $0, entry: entry) }, as: ProjectFilesV1.Manifest.self, timeout: timeout)
    }

    /// `$FLASHTEX_PROJECT_FILES`, a bundled `flashtex-project-files`, then
    /// `crates/project-files/target/{release,debug}/flashtex-project-files`.
    @MainActor static func locate() -> URL? {
        BridgeClient.locateHelper(named: "flashtex-project-files", environment: "FLASHTEX_PROJECT_FILES", crate: "project-files")
    }
}
