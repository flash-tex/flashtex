import Foundation

/// The bootstrap payload the Mac shows as a QR image beside its pairing code
/// (`apps/mac` `PairingBootstrapPayload`), accepted by `pair --qr <text>`:
///
///     flashtex-nearby://pair?v=2&code=<52 base32>&salt=<32 hex>&fp=<16 hex>&name=<Mac name>
///
/// Scanning it is equivalent to typing `--code` and picking the Mac by
/// `--mac <fp>`; `salt` also serves direct `--host/--port` mode in place of
/// `--salt`. `fp` must equal `NearbyCrypto.fingerprint(salt:)` — a payload
/// whose fingerprint does not match its salt is refused before any connection.
public struct NearbyBootstrapPayload: Equatable {
    public static let scheme = "flashtex-nearby"
    public static let host = "pair"
    public static let version = "2"

    public var code: String
    public var salt: Data
    public var fingerprint: String
    public var macName: String

    public init(code: String, salt: Data, macName: String) {
        self.code = code
        self.salt = salt
        self.fingerprint = NearbyCrypto.fingerprint(salt: salt)
        self.macName = macName
    }

    /// Same string the Mac encodes (query items in the same order).
    public var urlString: String {
        var c = URLComponents()
        c.scheme = Self.scheme
        c.host = Self.host
        c.queryItems = [
            .init(name: "v", value: Self.version),
            .init(name: "code", value: code),
            .init(name: "salt", value: NearbyCrypto.hex(salt)),
            .init(name: "fp", value: fingerprint),
            .init(name: "name", value: macName),
        ]
        return c.string ?? ""
    }

    public static func parse(_ text: String) throws -> NearbyBootstrapPayload {
        guard let c = URLComponents(string: text.trimmingCharacters(in: .whitespacesAndNewlines)),
              c.scheme == scheme, c.host == host else {
            throw NearbyError.invalidInput("not a \(scheme)://\(host) payload")
        }
        var q: [String: String] = [:]
        for item in c.queryItems ?? [] { q[item.name] = item.value ?? "" }
        guard q["v"] == version else { throw NearbyError.invalidInput("payload version \(q["v"] ?? "missing") is not \(version)") }
        guard let code = q["code"].flatMap(NearbyCrypto.canonicalCode) else {
            throw NearbyError.invalidInput("payload code must be the \(NearbyCrypto.codeLength)-character code")
        }
        guard let saltHex = q["salt"], let salt = NearbyCrypto.data(hex: saltHex), salt.count == NearbyCrypto.saltLength else {
            throw NearbyError.invalidInput("payload salt must be \(NearbyCrypto.saltLength) hex bytes")
        }
        guard q["fp"] == NearbyCrypto.fingerprint(salt: salt) else {
            throw NearbyError.invalidInput("payload fp does not match its salt")
        }
        return .init(code: code, salt: salt, macName: q["name"] ?? "")
    }
}
