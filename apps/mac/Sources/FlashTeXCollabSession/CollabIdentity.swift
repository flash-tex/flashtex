// FlashTeXCollabSession: live collaboration's session layer (collab-v1
// transport, invites, the hub and guest roles, and the bridge between the
// CRDT and an editor). Design: docs/design/live-collab/PROPOSAL.md §3–§4;
// wire format: docs/contracts/collab-v1.md. MIT. No AppKit or UIKit: the
// iPad links it later, as it links FlashTeXCollabCore.

import CryptoKit
import Foundation
import Network
import Security

/// The hub's TLS identity: a P-256 key made in memory for one session and a
/// self-signed certificate over it (proposal §3.3). Guests pin it by the
/// SHA-256 of its SubjectPublicKeyInfo, which the invite carries, so no
/// certificate authority, keychain item or trust setting is involved, and
/// nothing outlives the process.
public final class CollabIdentity: @unchecked Sendable {
    public let privateKey: SecKey
    public let certificate: SecCertificate
    /// DER of the certificate (tests and diagnostics).
    public let certificateDER: Data
    /// SHA-256 of the DER SubjectPublicKeyInfo.
    public let fingerprint: Data
    let secIdentity: SecIdentity

    public enum Failure: Error, CustomStringConvertible {
        case keyGeneration(String)
        case signing(String)
        case certificate
        case identity

        public var description: String {
            switch self {
            case let .keyGeneration(m): return "could not make a session key: \(m)"
            case let .signing(m): return "could not sign the session certificate: \(m)"
            case .certificate: return "the session certificate was not accepted"
            case .identity: return "the session identity could not be formed"
            }
        }
    }

    /// A new identity. `commonName` is informational only.
    public init(commonName: String = "FlashTeX Live Share") throws {
        let attributes: [CFString: Any] = [
            kSecAttrKeyType: kSecAttrKeyTypeECSECPrimeRandom,
            kSecAttrKeySizeInBits: 256,
            kSecAttrIsPermanent: false,
        ]
        var error: Unmanaged<CFError>?
        guard let key = SecKeyCreateRandomKey(attributes as CFDictionary, &error),
              let pub = SecKeyCopyPublicKey(key),
              let point = SecKeyCopyExternalRepresentation(pub, &error) as Data? else {
            throw Failure.keyGeneration(error.map { String(describing: $0.takeRetainedValue()) } ?? "unknown")
        }
        let spki = Self.subjectPublicKeyInfo(point: point)
        let tbs = Self.tbsCertificate(spki: spki, commonName: commonName)
        guard let sig = SecKeyCreateSignature(key, .ecdsaSignatureMessageX962SHA256, Data(tbs) as CFData, &error) as Data? else {
            throw Failure.signing(error.map { String(describing: $0.takeRetainedValue()) } ?? "unknown")
        }
        let der = DER.sequence(tbs + DER.sequence(DER.oid(DER.ecdsaWithSHA256)) + DER.bitString(Array(sig)))
        guard let cert = SecCertificateCreateWithData(nil, Data(der) as CFData) else { throw Failure.certificate }
        guard let identity = SecIdentityCreate(nil, cert, key) else { throw Failure.identity }
        privateKey = key
        certificate = cert
        certificateDER = Data(der)
        fingerprint = Data(SHA256.hash(data: spki))
        secIdentity = identity
    }

    /// SHA-256 of a certificate's SubjectPublicKeyInfo (P-256 keys only, as
    /// the hub makes; anything else returns nil and fails the pin).
    public static func fingerprint(of certificate: SecCertificate) -> Data? {
        var error: Unmanaged<CFError>?
        guard let key = SecCertificateCopyKey(certificate),
              let attrs = SecKeyCopyAttributes(key) as? [CFString: Any],
              (attrs[kSecAttrKeyType] as? String) == (kSecAttrKeyTypeECSECPrimeRandom as String),
              (attrs[kSecAttrKeySizeInBits] as? Int) == 256,
              let point = SecKeyCopyExternalRepresentation(key, &error) as Data?, point.count == 65 else { return nil }
        return Data(SHA256.hash(data: subjectPublicKeyInfo(point: point)))
    }

    // MARK: DER

    static func subjectPublicKeyInfo(point: Data) -> [UInt8] {
        DER.sequence(DER.sequence(DER.oid(DER.ecPublicKey) + DER.oid(DER.prime256v1)) + DER.bitString(Array(point)))
    }

    static func tbsCertificate(spki: [UInt8], commonName: String) -> [UInt8] {
        var serial = [UInt8](repeating: 0, count: 16)
        _ = SecRandomCopyBytes(kSecRandomDefault, serial.count, &serial)
        serial[0] &= 0x7F // positive
        if serial[0] == 0 { serial[0] = 1 }
        let name = DER.sequence(DER.set(DER.sequence(DER.oid(DER.commonName) + DER.utf8String(commonName))))
        let now = Date()
        let validity = DER.sequence(DER.utcTime(now.addingTimeInterval(-3600)) + DER.utcTime(now.addingTimeInterval(7 * 86_400)))
        return DER.sequence(
            DER.explicit(0, DER.integer([2])) // v3
                + DER.integer(serial)
                + DER.sequence(DER.oid(DER.ecdsaWithSHA256))
                + name + validity + name + spki
        )
    }

    enum DER {
        static let ecPublicKey: [UInt64] = [1, 2, 840, 10045, 2, 1]
        static let prime256v1: [UInt64] = [1, 2, 840, 10045, 3, 1, 7]
        static let ecdsaWithSHA256: [UInt64] = [1, 2, 840, 10045, 4, 3, 2]
        static let commonName: [UInt64] = [2, 5, 4, 3]

        static func length(_ n: Int) -> [UInt8] {
            if n < 0x80 { return [UInt8(n)] }
            var bytes: [UInt8] = []
            var v = n
            while v > 0 { bytes.insert(UInt8(v & 0xFF), at: 0); v >>= 8 }
            return [0x80 | UInt8(bytes.count)] + bytes
        }

        static func tlv(_ tag: UInt8, _ body: [UInt8]) -> [UInt8] { [tag] + length(body.count) + body }
        static func sequence(_ body: [UInt8]) -> [UInt8] { tlv(0x30, body) }
        static func set(_ body: [UInt8]) -> [UInt8] { tlv(0x31, body) }
        static func explicit(_ n: UInt8, _ body: [UInt8]) -> [UInt8] { tlv(0xA0 | n, body) }
        static func utf8String(_ s: String) -> [UInt8] { tlv(0x0C, Array(s.utf8)) }
        static func bitString(_ bytes: [UInt8]) -> [UInt8] { tlv(0x03, [0] + bytes) }

        static func integer(_ magnitude: [UInt8]) -> [UInt8] {
            var m = Array(magnitude.drop { $0 == 0 })
            if m.isEmpty { m = [0] }
            if m[0] & 0x80 != 0 { m.insert(0, at: 0) }
            return tlv(0x02, m)
        }

        static func oid(_ arcs: [UInt64]) -> [UInt8] {
            var body: [UInt8] = [UInt8(arcs[0] * 40 + arcs[1])]
            for arc in arcs.dropFirst(2) {
                var chunk: [UInt8] = [UInt8(arc & 0x7F)]
                var v = arc >> 7
                while v > 0 { chunk.insert(UInt8(v & 0x7F) | 0x80, at: 0); v >>= 7 }
                body += chunk
            }
            return tlv(0x06, body)
        }

        static func utcTime(_ date: Date) -> [UInt8] {
            let f = DateFormatter()
            f.locale = Locale(identifier: "en_US_POSIX")
            f.timeZone = TimeZone(identifier: "UTC")
            f.dateFormat = "yyMMddHHmmss'Z'"
            return tlv(0x17, Array(f.string(from: date).utf8))
        }
    }
}

/// Network.framework parameters for collab-v1 (proposal §3.3): TCP with
/// keepalive, TLS 1.3 only (ECDHE, so forward secrecy, unlike nearby-v1's
/// TLS 1.2 PSK), no resumption. The hub presents its `CollabIdentity`; a
/// guest accepts exactly the certificate whose key the invite pinned.
public enum CollabTLS {
    /// Bonjour service type (separate from nearby-v1's `_flashtex._tcp`: a
    /// capture pairing never grants edit rights).
    public static let serviceType = "_flashtex-collab._tcp"

    static func tcpOptions() -> NWProtocolTCP.Options {
        let tcp = NWProtocolTCP.Options()
        tcp.enableKeepalive = true
        tcp.keepaliveIdle = 15
        tcp.keepaliveInterval = 5
        tcp.keepaliveCount = 3
        tcp.noDelay = true
        return tcp
    }

    static func baseTLS() -> NWProtocolTLS.Options {
        let tls = NWProtocolTLS.Options()
        let sec = tls.securityProtocolOptions
        sec_protocol_options_set_min_tls_protocol_version(sec, .TLSv13)
        sec_protocol_options_set_max_tls_protocol_version(sec, .TLSv13)
        sec_protocol_options_set_tls_resumption_enabled(sec, false)
        sec_protocol_options_set_tls_tickets_enabled(sec, false)
        return tls
    }

    public static func hubParameters(identity: CollabIdentity, loopbackOnly: Bool) -> NWParameters {
        let tls = baseTLS()
        if let id = sec_identity_create(identity.secIdentity) {
            sec_protocol_options_set_local_identity(tls.securityProtocolOptions, id)
        }
        let params = NWParameters(tls: tls, tcp: tcpOptions())
        params.allowLocalEndpointReuse = true
        params.includePeerToPeer = false
        if loopbackOnly { params.requiredInterfaceType = .loopback }
        return params
    }

    /// `pinned`: the invite's fingerprint. `onVerify` reports the outcome
    /// (tests; diagnostics).
    public static func guestParameters(pinned: Data, onVerify: (@Sendable (Bool) -> Void)? = nil) -> NWParameters {
        let tls = baseTLS()
        sec_protocol_options_set_verify_block(tls.securityProtocolOptions, { _, trust, complete in
            let secTrust = sec_trust_copy_ref(trust).takeRetainedValue()
            var ok = false
            if let chain = SecTrustCopyCertificateChain(secTrust) as? [SecCertificate], let leaf = chain.first,
               let fp = CollabIdentity.fingerprint(of: leaf) {
                ok = constantTimeEqual(fp, pinned)
            }
            onVerify?(ok)
            complete(ok)
        }, DispatchQueue.global(qos: .userInitiated))
        let params = NWParameters(tls: tls, tcp: tcpOptions())
        params.includePeerToPeer = false
        return params
    }

    static func constantTimeEqual(_ a: Data, _ b: Data) -> Bool {
        guard a.count == b.count else { return false }
        var diff: UInt8 = 0
        for (x, y) in zip(a, b) { diff |= x ^ y }
        return diff == 0
    }

    /// Negotiated protocol of an established connection.
    public static func negotiatedVersion(_ connection: NWConnection) -> tls_protocol_version_t? {
        guard let meta = connection.metadata(definition: NWProtocolTLS.definition) as? NWProtocolTLS.Metadata else { return nil }
        return sec_protocol_metadata_get_negotiated_tls_protocol_version(meta.securityProtocolMetadata)
    }
}
