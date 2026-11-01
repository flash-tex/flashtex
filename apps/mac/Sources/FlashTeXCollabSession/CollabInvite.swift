import CryptoKit
import Foundation

/// A single-use invitation to one session (proposal §3.2):
///
/// `flashtex-collab://join?v=1&s=<session>&k=<secret>&fp=<pin>&name=<project>&port=<port>&addr=<ip,…>&host=<name>`
///
/// `k` (32 random bytes, base64url) is the invite secret: whoever presents a
/// proof made with it may ask to join, once, within ten minutes, and the hub
/// user still approves. `fp` pins the hub's TLS key (`CollabIdentity`). The
/// rest says where to connect: `port` with the hub's addresses (`addr`), or
/// loopback when `host` names this machine (two app instances on one Mac),
/// or the Bonjour service named `s` (`_flashtex-collab._tcp`). There is no
/// short numeric code: nearby-v1 §5 calls its 20-bit code its weakest point,
/// and here the joiner is another person.
public struct CollabInvite: Equatable, Sendable {
    public static let scheme = "flashtex-collab"
    public static let lifetime: TimeInterval = 600

    public var sessionID: String
    public var secret: Data
    public var fingerprint: Data
    public var projectName: String
    public var port: UInt16
    public var addresses: [String]
    public var hostName: String?

    public init(sessionID: String, secret: Data, fingerprint: Data, projectName: String, port: UInt16,
                addresses: [String], hostName: String?) {
        self.sessionID = sessionID
        self.secret = secret
        self.fingerprint = fingerprint
        self.projectName = projectName
        self.port = port
        self.addresses = addresses
        self.hostName = hostName
    }

    public var link: String {
        var c = URLComponents()
        c.scheme = Self.scheme
        c.host = "join"
        var items = [
            URLQueryItem(name: "v", value: "1"),
            URLQueryItem(name: "s", value: sessionID),
            URLQueryItem(name: "k", value: Base64URL.encode(secret)),
            URLQueryItem(name: "fp", value: Base64URL.encode(fingerprint)),
            URLQueryItem(name: "name", value: projectName),
            URLQueryItem(name: "port", value: String(port)),
        ]
        if !addresses.isEmpty { items.append(URLQueryItem(name: "addr", value: addresses.joined(separator: ","))) }
        if let hostName { items.append(URLQueryItem(name: "host", value: hostName)) }
        c.queryItems = items
        return c.string ?? ""
    }

    public enum ParseError: Error, Equatable, CustomStringConvertible {
        case notAnInvite, unsupportedVersion(String), missing(String), invalid(String)

        public var description: String {
            switch self {
            case .notAnInvite: return "This is not a FlashTeX Live Share invitation."
            case let .unsupportedVersion(v): return "This invitation is version \(v), which this FlashTeX cannot read."
            case let .missing(k): return "The invitation is incomplete (no \(k))."
            case let .invalid(k): return "The invitation is damaged (\(k))."
            }
        }
    }

    /// Parses a pasted link (surrounding whitespace and angle brackets are
    /// ignored, as mail clients add them).
    public init(link raw: String) throws {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines.union(CharacterSet(charactersIn: "<>")))
        guard let c = URLComponents(string: trimmed), c.scheme == Self.scheme, c.host == "join" else { throw ParseError.notAnInvite }
        var q: [String: String] = [:]
        for item in c.queryItems ?? [] { q[item.name] = item.value ?? "" }
        guard let v = q["v"] else { throw ParseError.missing("version") }
        guard v == "1" else { throw ParseError.unsupportedVersion(v) }
        guard let s = q["s"], !s.isEmpty else { throw ParseError.missing("session") }
        guard let k = q["k"] else { throw ParseError.missing("secret") }
        guard let secret = Base64URL.decode(k), secret.count == 32 else { throw ParseError.invalid("secret") }
        guard let f = q["fp"] else { throw ParseError.missing("fingerprint") }
        guard let fp = Base64URL.decode(f), fp.count == 32 else { throw ParseError.invalid("fingerprint") }
        guard let p = q["port"], let port = UInt16(p), port != 0 else { throw ParseError.missing("port") }
        sessionID = s
        self.secret = secret
        fingerprint = fp
        projectName = q["name"] ?? "Untitled"
        self.port = port
        addresses = (q["addr"] ?? "").split(separator: ",").map(String.init).filter { !$0.isEmpty }
        hostName = q["host"].flatMap { $0.isEmpty ? nil : $0 }
    }

    /// The `invite_proof` of `join`: base64url(HMAC-SHA256(k, nonce ‖ guest public key)).
    public static func proof(secret: Data, nonce: Data, guestPublicKey: Data) -> String {
        let mac = HMAC<SHA256>.authenticationCode(for: nonce + guestPublicKey, using: SymmetricKey(data: secret))
        return Base64URL.encode(Data(mac))
    }

    public static func randomBytes(_ n: Int) -> Data {
        var g = SystemRandomNumberGenerator()
        return Data((0..<n).map { _ in UInt8.random(in: 0...255, using: &g) })
    }
}

public enum Base64URL {
    public static func encode(_ d: Data) -> String {
        d.base64EncodedString().replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }

    public static func decode(_ s: String) -> Data? {
        var b = s.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/")
        while b.count % 4 != 0 { b += "=" }
        return Data(base64Encoded: b)
    }
}
