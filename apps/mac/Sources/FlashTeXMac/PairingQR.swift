import AppKit
import CoreImage
import Foundation

/// What the pairing QR image carries: exactly the bootstrap inputs the
/// reference client's `pair` command accepts (`apps/mac/tools/nearby-client`,
/// `NearbyBootstrapPayload`), so scanning is equivalent to typing the code and
/// picking the Mac by fingerprint:
///
///     flashtex-nearby://pair?v=2&code=<52 base32>&salt=<32 hex>&fp=<16 hex>&name=<Mac name>
///
/// `salt` is the Mac's TXT salt (the companion derives the bootstrap key from
/// code + salt); `fp` is derived from it and lets the companion match the
/// Bonjour result without connecting; `name` is display only. Nothing here is
/// secret beyond the code itself, which is already shown next to the image.
/// The nearby `protocol_version` stays 1: no wire message changes. Payload
/// `v=2` carries the 256-bit code (`Pairing.generateCode`); `v=1` carried six
/// digits and is refused, as is any code that is not full strength.
struct PairingBootstrapPayload: Equatable {
    static let scheme = "flashtex-nearby"
    static let host = "pair"
    static let version = "2"

    var code: String
    var salt: Data
    var macName: String

    var fingerprint: String { Pairing.fingerprint(salt: salt) }

    /// The exact string encoded in the QR image (query items in this order).
    var urlString: String {
        var c = URLComponents()
        c.scheme = Self.scheme
        c.host = Self.host
        c.queryItems = [
            .init(name: "v", value: Self.version),
            .init(name: "code", value: code),
            .init(name: "salt", value: Pairing.hex(salt)),
            .init(name: "fp", value: fingerprint),
            .init(name: "name", value: macName),
        ]
        return c.string ?? ""
    }

    struct ParseError: Error, Equatable, CustomStringConvertible {
        let reason: String
        var description: String { reason }
    }

    /// Strict inverse of `urlString`: the Mac's own tests decode the rendered
    /// QR through this before handing it to the reference client.
    static func parse(_ text: String) -> Result<PairingBootstrapPayload, ParseError> {
        guard let c = URLComponents(string: text), c.scheme == scheme, c.host == host else {
            return .failure(.init(reason: "not a \(scheme)://\(host) payload"))
        }
        var q: [String: String] = [:]
        for item in c.queryItems ?? [] { q[item.name] = item.value ?? "" }
        guard q["v"] == version else { return .failure(.init(reason: "payload version \(q["v"] ?? "missing") is not \(version)")) }
        guard let code = q["code"].flatMap(Pairing.canonicalCode) else {
            return .failure(.init(reason: "code must be the \(Pairing.codeLength)-character code"))
        }
        guard let saltHex = q["salt"], let salt = Pairing.data(hex: saltHex), salt.count == Pairing.saltLength else {
            return .failure(.init(reason: "salt must be \(Pairing.saltLength) hex bytes"))
        }
        let fp = Pairing.fingerprint(salt: salt)
        guard q["fp"] == fp else { return .failure(.init(reason: "fp does not match the salt")) }
        return .success(.init(code: code, salt: salt, macName: q["name"] ?? ""))
    }
}

/// QR rendering with CoreImage's built-in generator (no third-party code).
enum PairingQR {
    /// Error-correction level: M (15 %) keeps the module count low for a
    /// ~170-character payload while surviving a smudged screen.
    static let correctionLevel = "M"

    /// The payload as a crisp, `side`-pixel square (nearest-neighbour scaled
    /// so modules stay sharp), with the generator's own quiet zone. nil only
    /// if CoreImage cannot encode (it always can for this payload size).
    static func image(for payload: PairingBootstrapPayload, side: Int = 240) -> CGImage? {
        image(text: payload.urlString, side: side)
    }

    static func image(text: String, side: Int) -> CGImage? {
        guard let filter = CIFilter(name: "CIQRCodeGenerator") else { return nil }
        filter.setValue(Data(text.utf8), forKey: "inputMessage")
        filter.setValue(correctionLevel, forKey: "inputCorrectionLevel")
        guard let small = filter.outputImage else { return nil }
        let scale = max(1, CGFloat(side) / small.extent.width)
        let scaled = small.transformed(by: CGAffineTransform(scaleX: scale, y: scale))
        // Software renderer: deterministic off-screen, no GPU needed in tests.
        let context = CIContext(options: [.useSoftwareRenderer: true])
        return context.createCGImage(scaled, from: scaled.extent)
    }

    static func nsImage(for payload: PairingBootstrapPayload, side: Int = 240) -> NSImage? {
        guard let cg = image(for: payload, side: side) else { return nil }
        return NSImage(cgImage: cg, size: NSSize(width: side, height: side))
    }
}
