import Foundation
import Network

/// Command logic behind the `nearby-client` executable, kept in the library so
/// tests (including apps/mac's, against the real listener) can run the exact
/// CLI paths in-process. Output goes through `emit` one line at a time.
public enum NearbyCLI {
    public static let usage = """
    usage: nearby-client <command> [options]

      pair   --code <52-character code> --name "iPad Sim" [--mac <name|fp>] [--seconds 5]
             [--host 127.0.0.1 --port N --salt <32 hex>]      (skip Bonjour)
             [--qr "flashtex-nearby://pair?v=2&code=…&salt=…&fp=…&name=…"]
             (the Mac's QR image, decoded: supplies --code, --mac fp and --salt)
      send   --image file.png|jpg [--instructions "..."] [--capture-id ID]
             [--mac <name|fp>] [--host H --port N]
             [--destination-id ID --base-revision N]          (override the Mac's pin)
             [--attempts 5] [--retry-delay 0.25] [--max-delay 4] [--deadline 60]
             [--request-timeout 30]
      status [--seconds 2]        stored pairings and which Macs are visible now
      browse [--seconds 2]        every _flashtex._tcp service and its TXT record
      forget --mac <name|fp>
      doctor [--mac <name|fp>] [--host H --port N] [--seconds 5] [--json]
             validate one stored pairing against the running listener: store,
             discovery, TLS-PSK connect, TLS params, hello, destination; each
             line "check <name>: ok|warn|FAIL code=<exact code>"; no capture sent

    common: --store <path> (default $NEARBY_CLIENT_STORE or
            ~/Library/Application Support/FlashTeX/nearby-client-pairs.json),
            --timeout <s> (TLS connect, default 10), -v (print every JSON line)

    send retries only when the Mac was unreachable, the connection dropped or a
    reply timed out — bounded by --attempts and --deadline, exponential backoff
    from --retry-delay up to --max-delay (±20 % jitter) — and always re-sends the
    same capture_id and payload (the Mac de-duplicates). It never retries a
    refused key, an error reply or a changed destination.

    exit codes: 0 ok · 1 nothing stored / not found · 2 error (see message)
                3 pairing refused: re-pair · 4 retry budget exhausted: try later
                5 the Mac's pinned destination changed: reselect there, send again
                6 this companion is view-only on the Mac (capture_not_permitted):
                  the pairing is intact; change its permission in Nearby Companion
                64 usage · 65 not a PNG/JPEG · 66 unreadable file
    """

    public struct Failure: Error { public let code: Int32; public let message: String }

    /// Returns the process exit code. `emit` receives output lines (stdout).
    public static func run(_ arguments: [String], emit: @escaping (String) -> Void) async -> Int32 {
        do {
            if arguments.first == "doctor" { return try await doctor(parse(arguments), emit: emit) }
            try await main(arguments, emit: emit)
            return 0
        } catch let f as Failure {
            emit("error: \(f.message)")
            return f.code
        } catch let e as NearbyError {
            emit("error: \(e.description)" + (e.isClosing ? " (the Mac closes after this code: re-pair or fix the input, do not retry)" : ""))
            return exitCode(for: e, emit: emit)
        } catch {
            emit("error: \(error)")
            return 2
        }
    }

    /// Exit code for a `NearbyError`, with a one-line hint for the terminal ones.
    static func exitCode(for e: NearbyError, emit: (String) -> Void) -> Int32 {
        if e.needsRepair {
            emit("hint: the pairing was refused — the pairing code expired/was used, the Mac forgot this pairing, or this is not a FlashTeX nearby listener; run `nearby-client pair` again (exit 3)")
            return 3
        }
        switch e {
        case .attemptsExhausted(_, let last):
            if last.contains("too_many_sessions") {
                emit("hint: the Mac already holds its maximum sessions for this pairing; close the companion's other connections to it (or wait for them to time out) and try again (exit 4)")
            } else if last.contains("too_many_in_flight") || last.contains("inbox_full") {
                emit("hint: the Mac is still working through earlier captures; wait for their acknowledgements and try again (exit 4)")
            } else {
                emit("hint: the Mac stayed unreachable for the whole retry budget; check it is advertising and try again (exit 4)")
            }
            return 4
        case .destinationChanged:
            emit("hint: the insertion point on the Mac was unpinned or moved since this capture was built; reselect it there (Edit > Pin Insertion Point) and send again (exit 5)")
            return 5
        case .remote where e.needsPermission:
            emit("hint: this companion is view-only on the Mac; the pairing is intact (no re-pair) — change its permission to “Captures allowed” in Nearby Companion and send the same capture again (exit 6)")
            return 6
        case .remote(let code, _) where e.needsNewCapture:
            switch code {
            case "image_too_large": emit("hint: the Mac accepts images up to 8 MiB encoded and 8192×8192; shrink the image and send again with a new --capture-id (exit 2)")
            case "invalid_image": emit("hint: the Mac could not parse the image as the declared PNG/JPEG; re-export it and send again with a new --capture-id (exit 2)")
            case "revision_mismatch": emit("hint: this capture_id was already accepted at another base_revision; use a new --capture-id for the new revision (exit 2)")
            case "capture_id_conflict": emit("hint: this capture_id was already accepted with different content; use a new --capture-id (exit 2)")
            case "destination_reselection_required": emit("hint: the Mac's bridge no longer holds the insertion point this capture targets (unpinned, or an edit overlapped it); reselect it on the Mac (Edit > Pin Insertion Point), read the new hello_ack destination and send again with a new --capture-id (exit 2)")
            case "revision_conflict": emit("hint: the Mac's bridge says base_revision is not the pinned destination's revision; read the current hello_ack destination and send again with a new --capture-id (exit 2)")
            default: emit("hint: the Mac refused this capture as sent; fix the input and send again with a new --capture-id (exit 2)")
            }
            return 2
        default:
            return 2
        }
    }

    // MARK: argument parsing

    struct Options {
        var command = ""
        var values: [String: String] = [:]
        var verbose = false
        func string(_ k: String) -> String? { values[k] }
        func require(_ k: String) throws -> String {
            guard let v = values[k], !v.isEmpty else { throw Failure(code: 64, message: "missing --\(k)\n\(usage)") }
            return v
        }
        func double(_ k: String, default d: Double) throws -> Double {
            guard let s = values[k] else { return d }
            guard let v = Double(s) else { throw Failure(code: 64, message: "--\(k) must be a number") }
            return v
        }
        func int(_ k: String) throws -> Int? {
            guard let s = values[k] else { return nil }
            guard let v = Int(s) else { throw Failure(code: 64, message: "--\(k) must be an integer") }
            return v
        }
    }

    static func parse(_ args: [String]) throws -> Options {
        var o = Options()
        var it = args.makeIterator()
        guard let cmd = it.next() else { throw Failure(code: 64, message: usage) }
        o.command = cmd
        while let a = it.next() {
            if a == "-v" || a == "--verbose" { o.verbose = true; continue }
            if a == "--json" { o.values["json"] = "1"; continue }
            guard a.hasPrefix("--") else { throw Failure(code: 64, message: "unexpected argument \(a)\n\(usage)") }
            let key = String(a.dropFirst(2))
            if let eq = key.firstIndex(of: "=") {
                o.values[String(key[..<eq])] = String(key[key.index(after: eq)...])
            } else {
                guard let v = it.next() else { throw Failure(code: 64, message: "--\(key) needs a value") }
                o.values[key] = v
            }
        }
        return o
    }

    // MARK: commands

    static func main(_ args: [String], emit: @escaping (String) -> Void) async throws {
        let o = try parse(args)
        switch o.command {
        case "pair": try await pair(o, emit: emit)
        case "send": try await send(o, emit: emit)
        case "status": try await status(o, emit: emit)
        case "browse": try await browse(o, emit: emit)
        case "forget": try forget(o, emit: emit)
        case "help", "--help", "-h": emit(usage)
        default: throw Failure(code: 64, message: "unknown command \(o.command)\n\(usage)")
        }
    }

    static func store(_ o: Options) throws -> PairFile {
        try PairFile(url: o.string("store").map { URL(fileURLWithPath: $0) } ?? PairFile.defaultURL())
    }

    static func lineLogger(_ o: Options, emit: @escaping (String) -> Void) -> ((NearbyConnection.Direction, Data) -> Void)? {
        guard o.verbose else { return nil }
        return { dir, line in emit((dir == .sent ? ">> " : "<< ") + abbreviate(line)) }
    }

    /// Keeps transcripts readable: the base64 image is replaced by its size.
    static func abbreviate(_ line: Data) -> String {
        var s = String(decoding: line, as: UTF8.self)
        while s.last == "\n" { s.removeLast() }
        if let r = s.range(of: "\"data_base64\":\""), let end = s[r.upperBound...].firstIndex(of: "\"") {
            let n = s.distance(from: r.upperBound, to: end)
            s.replaceSubrange(r.upperBound..<end, with: "<\(n) base64 chars>")
        }
        return s
    }

    /// Picks the Mac to talk to: `--host/--port` directly, else Bonjour by
    /// `--mac` (name or fp) or the only/first supported service.
    static func resolveEndpoint(_ o: Options, emit: @escaping (String) -> Void, wantFP: String? = nil) async throws -> (NWEndpoint, DiscoveredMac?) {
        if let host = o.string("host") {
            guard let p = try o.int("port"), let port = NWEndpoint.Port(rawValue: UInt16(clamping: p)) else {
                throw Failure(code: 64, message: "--host needs --port")
            }
            return (.hostPort(host: NWEndpoint.Host(host), port: port), nil)
        }
        let seconds = try o.double("seconds", default: 5)
        let key = o.string("mac") ?? wantFP
        emit("browsing \(NearbyWire.serviceType) for \(key.map { "\"\($0)\"" } ?? "any Mac") (up to \(Int(seconds))s)…")
        let results = try await NearbyBrowser.discover(seconds: seconds) { m in
            guard let key else { return m.isSupported }
            return m.isSupported && (m.fingerprint == key || m.name == key || m.macName == key)
        }
        let candidates = results.filter { m in
            guard let key else { return true }
            return m.fingerprint == key || m.name == key || m.macName == key
        }
        guard let mac = candidates.first(where: \.isSupported) else {
            let seen = results.map { "\($0.name) fp=\($0.fingerprint ?? "?")\($0.unsupportedReason.map { " (\($0))" } ?? "")" }
            throw NearbyError.noMatchingMac("\(key ?? "no supported service") (seen: \(seen.isEmpty ? "none" : seen.joined(separator: "; ")))")
        }
        if candidates.count > 1 { emit("note: \(candidates.count) matching services; using \(mac.name) fp=\(mac.fingerprint ?? "?")") }
        emit("found \(mac.name) (fp \(mac.fingerprint ?? "?"), v=\(mac.version ?? "?"))")
        return (mac.endpoint, mac)
    }

    static func pair(_ o: Options, emit: @escaping (String) -> Void) async throws {
        // `--qr <payload>` (the Mac's QR image, decoded) supplies code, salt
        // and fingerprint; an explicit --code/--mac must agree with it.
        let scanned: NearbyBootstrapPayload? = try o.string("qr").map { text in
            do { return try NearbyBootstrapPayload.parse(text) } catch let e as NearbyError {
                throw Failure(code: 64, message: "--qr: \(e.description)")
            }
        }
        if let scanned {
            if let c = o.string("code"), (NearbyCrypto.canonicalCode(c) ?? c) != scanned.code { throw Failure(code: 64, message: "--code \(c) differs from the QR payload's code") }
            if let m = o.string("mac"), m != scanned.fingerprint, m != scanned.macName {
                throw Failure(code: 64, message: "--mac \(m) is neither the QR payload's fp \(scanned.fingerprint) nor its name")
            }
            emit("qr payload: fp \(scanned.fingerprint) name \"\(scanned.macName)\"")
        }
        let code = try scanned?.code ?? o.require("code")
        let name = try o.require("name")
        let file = try store(o)
        let timeout = try o.double("timeout", default: 10)
        let (endpoint, mac) = try await resolveEndpoint(o, emit: emit, wantFP: scanned?.fingerprint)
        let salt: Data, fp: String, macName: String
        if let mac {
            salt = mac.salt!; fp = mac.fingerprint!; macName = mac.macName
            if let scanned, scanned.salt != salt {
                throw NearbyError.unsupportedService("the discovered Mac's TXT salt differs from the QR payload's; scan again")
            }
        } else if let scanned {
            salt = scanned.salt; fp = scanned.fingerprint; macName = scanned.macName.isEmpty ? o.string("host")! : scanned.macName
        } else {
            guard let s = NearbyCrypto.data(hex: try o.require("salt")), s.count == NearbyCrypto.saltLength else {
                throw Failure(code: 64, message: "--salt must be the Mac's 32-hex TXT salt")
            }
            salt = s; fp = NearbyCrypto.fingerprint(salt: s); macName = o.string("host")!
        }
        let derived = NearbyCrypto.derive(code: code, salt: salt)
        emit("pairing as \"\(name)\" with pair_id \(derived.pairId) (bootstrap key from code + salt \(NearbyCrypto.hex(salt)))")
        let (pair, session) = try await NearbyClient.pair(endpoint: endpoint, salt: salt, fingerprint: fp, macName: macName,
                                                          code: code, companionName: name, connectTimeout: timeout,
                                                          onLine: lineLogger(o, emit: emit))
        defer { session.close() }
        if let n = session.connection.negotiated { emit("tls: \(n.tlsv12 ? "1.2" : "NOT 1.2") suite 0x\(String(n.suite, radix: 16))") }
        emit("hello_ack from \"\(session.macName)\": received long-term pair_psk (\(pair.psk?.count ?? 0) bytes)")
        emit("destination: \(describe(session.destination))")
        try file.upsert(pair)
        emit("stored pairing for fp \(fp) in \(file.url.path)")
        // The bootstrap connection stays usable; prove it with a query.
        let d = try await session.destinationQuery()
        emit("destination_query on the same connection: \(describe(d))")
        emit("paired")
    }

    static func send(_ o: Options, emit: @escaping (String) -> Void) async throws {
        let file = try store(o)
        let imagePath = try o.require("image")
        let timeout = try o.double("timeout", default: 10)
        guard let image = FileManager.default.contents(atPath: imagePath) else {
            throw Failure(code: 66, message: "cannot read \(imagePath)")
        }
        guard let mime = sniffMime(image) else {
            throw Failure(code: 65, message: "\(imagePath) is not a PNG or JPEG (the Mac accepts image/png and image/jpeg)")
        }
        if let why = NearbyWire.checkImage(image, mimeType: mime) {
            throw Failure(code: 65, message: "\(imagePath): \(why)")
        }
        let pair: PairedMac
        if let key = o.string("mac") {
            guard let p = file.pair(matching: key) else { throw Failure(code: 1, message: "no stored pairing matches \(key); run `nearby-client pair`") }
            pair = p
        } else if file.pairs.count == 1 {
            pair = file.pairs[0]
        } else if file.pairs.isEmpty {
            throw Failure(code: 1, message: "no pairings stored in \(file.url.path); run `nearby-client pair`")
        } else {
            throw Failure(code: 64, message: "several pairings stored; pick one with --mac <name|fp>")
        }
        var policy = ReconnectPolicy(maxAttempts: try o.int("attempts") ?? 5,
                                     initialDelay: try o.double("retry-delay", default: 0.25),
                                     maxDelay: try o.double("max-delay", default: 4),
                                     overallDeadline: try o.double("deadline", default: 60),
                                     connectTimeout: timeout)
        policy.requestTimeout = try o.double("request-timeout", default: 30)
        // Endpoint: fixed --host/--port, else re-browse by fp before every attempt
        // (a restarted Mac may be back on another port).
        let fixed: NWEndpoint? = o.string("host") != nil ? try await resolveEndpoint(o, emit: emit, wantFP: pair.fingerprint).0 : nil
        let browseSeconds = try o.double("seconds", default: 5)
        let key = pair.fingerprint
        let endpoints: NearbyReconnector.EndpointSource = {
            if let fixed { return fixed }
            emit("browsing \(NearbyWire.serviceType) for fp \(key) (up to \(Int(browseSeconds))s)…")
            let mac = try await NearbyClient.find(pair: pair, seconds: browseSeconds)
            emit("found \(mac.name) (fp \(mac.fingerprint ?? "?"), v=\(mac.version ?? "?"))")
            return mac.endpoint
        }
        emit("connecting to \"\(pair.macName)\" as pair_id \(pair.pairId) with the stored pair_psk (up to \(policy.maxAttempts) attempt\(policy.maxAttempts == 1 ? "" : "s"), \(Int(policy.overallDeadline))s)")
        let reconnector = NearbyReconnector(pair: pair, policy: policy, endpoints: endpoints, onEvent: { e in
            switch e {
            case .attempt(let n, let of): if n > 1 { emit("attempt \(n) of \(of)") }
            case .connected(let mac, let n, let d): emit("hello_ack from \"\(mac)\" (attempt \(n)); destination: \(describe(d))")
            case .failed(let n, let why, let wait): emit("attempt \(n) failed: \(why); retrying in \(String(format: "%.2f", wait ?? 0))s")
            case .disconnected(let why): emit("disconnected: \(why)")
            case .waitingForAcks(let n): emit("the Mac asked to wait: \(n) request\(n == 1 ? "" : "s") still awaiting acknowledgement")
            case .gaveUp: break // the thrown error is reported by `run`
            }
        }, onLine: lineLogger(o, emit: emit))
        let session = try await reconnector.connect()
        var dest = session.destination
        var checkDestination = true
        if let id = o.string("destination-id") {
            guard let rev = try o.int("base-revision") else { throw Failure(code: 64, message: "--destination-id needs --base-revision") }
            dest = .init(destinationId: id, projectId: dest?.projectId ?? "", path: dest?.path ?? "", baseRevision: rev)
            checkDestination = false // the user overrode the Mac's pin on purpose
        }
        let captureId = o.string("capture-id") ?? "cli-" + UUID().uuidString.lowercased()
        let capture = try session.makeCapture(captureId: captureId, image: image, mimeType: mime,
                                              instructions: o.string("instructions") ?? "", destination: dest)
        emit("capture_submit \(captureId): \(mime), \(image.count) bytes → destination \(capture.destinationId) @ rev \(capture.baseRevision)")
        let ack: NearbyWire.CaptureReceived
        do {
            ack = try await reconnector.submit(capture, requireCurrentDestination: checkDestination, requestID: o.string("request-id"))
        } catch { await reconnector.shutdown(); throw error }
        let attempts = await reconnector.attemptsMade
        await reconnector.shutdown()
        emit("capture_received \(ack.captureId): durable=\(ack.durable) has_proposal=\(ack.hasProposal) applied=\(ack.applied)"
             + (attempts > 1 ? " after \(attempts) connection attempts (same capture_id re-sent)" : ""))
        if !ack.durable { emit("note: durable=false — the Mac's in-memory inbox took it (no capture bridge attached)") }
        var updated = pair
        updated.lastSeenAt = Date()
        updated.lastDestination = session.destination
        try file.upsert(updated)
        emit("sent")
    }

    static func status(_ o: Options, emit: @escaping (String) -> Void) async throws {
        let file = try store(o)
        emit("store: \(file.url.path)")
        if file.pairs.isEmpty { emit("no pairings stored") }
        let seconds = try o.double("seconds", default: 2)
        let visible = (try? await NearbyBrowser.discover(seconds: seconds)) ?? []
        for p in file.pairs {
            let seen = visible.first { $0.fingerprint == p.fingerprint }
            let iso = ISO8601DateFormatter()
            emit("\(p.macName) fp=\(p.fingerprint) pair_id=\(p.pairId) as \"\(p.companionName)\" paired \(iso.string(from: p.pairedAt))"
                 + (p.lastSeenAt.map { " last seen \(iso.string(from: $0))" } ?? "")
                 + (seen.map { " — visible now as \"\($0.name)\"" } ?? " — not visible"))
            if let d = p.lastDestination { emit("  last destination: \(describe(d))") }
        }
        let unpaired = visible.filter { m in !file.pairs.contains { $0.fingerprint == m.fingerprint } }
        for m in unpaired {
            emit("unpaired: \(m.name) fp=\(m.fingerprint ?? "?") v=\(m.version ?? "?")\(m.unsupportedReason.map { " (\($0))" } ?? "")")
        }
    }

    static func browse(_ o: Options, emit: @escaping (String) -> Void) async throws {
        let seconds = try o.double("seconds", default: 2)
        let results = try await NearbyBrowser.discover(seconds: seconds)
        if results.isEmpty { emit("no \(NearbyWire.serviceType) services within \(Int(seconds))s") }
        for m in results {
            let txt = m.txt.keys.sorted().map { "\($0)=\(m.txt[$0]!)" }.joined(separator: " ")
            emit("\(m.name): \(txt)\(m.unsupportedReason.map { " — unsupported: \($0)" } ?? "")")
        }
    }

    /// `doctor`: see `NearbyDoctor`. Exit code is the report's; `--json` adds
    /// the whole report as one JSON line after the summary.
    static func doctor(_ o: Options, emit: @escaping (String) -> Void) async throws -> Int32 {
        let file = try store(o)
        var fixed: NWEndpoint?
        if o.string("host") != nil { fixed = try await resolveEndpoint(o, emit: emit).0 }
        let options = NearbyDoctor.Options(store: file, macKey: o.string("mac"), fixedEndpoint: fixed,
                                           browseSeconds: try o.double("seconds", default: 5),
                                           connectTimeout: try o.double("timeout", default: 10), onLine: lineLogger(o, emit: emit))
        let report = await NearbyDoctor.run(options, emit: emit)
        if o.values["json"] != nil { emit(report.json) }
        return report.exit
    }

    static func forget(_ o: Options, emit: @escaping (String) -> Void) throws {
        let file = try store(o)
        let key = try o.require("mac")
        guard let p = file.pair(matching: key) else { throw Failure(code: 1, message: "no stored pairing matches \(key)") }
        try file.remove(fingerprint: p.fingerprint)
        emit("forgot \(p.macName) (fp \(p.fingerprint)); the Mac still lists it until you Forget there too")
    }

    // MARK: helpers

    static func describe(_ d: NearbyWire.Destination?) -> String {
        guard let d else { return "null (nothing pinned on the Mac — Edit > Pin Insertion Point)" }
        return "\(d.destinationId) (\(d.projectId)/\(d.path) @ rev \(d.baseRevision))"
    }

    public static func sniffMime(_ data: Data) -> String? {
        if data.starts(with: [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) { return "image/png" }
        if data.starts(with: [0xFF, 0xD8, 0xFF]) { return "image/jpeg" }
        return nil
    }
}
