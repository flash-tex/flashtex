import XCTest
@testable import FlashTeXMac

/// biber with no TeX Live (#1782): the lock's pin for this Mac, the consent
/// for exactly that pin, and the host's skip that asks for it.
final class EngineV3BiberTests: XCTestCase {
    let lock = """
    url = "https://example.org/texlive2026.ttb"
    digest = "\(String(repeating: "0", count: 64))"
    biber_url_universal_darwin = "https://example.org/biber-universal-darwin.gz"
    biber_sha256_universal_darwin = "\(String(repeating: "AB", count: 32))"
    biber_size_universal_darwin = "72119597"
    biber_url_x86_64_linux = "https://example.org/biber-x86_64-linux.gz"
    """

    func testThePinIsThisMacs() {
        let p = EngineV3Biber.pin(lockText: lock)
        XCTAssertEqual(p, .init(url: "https://example.org/biber-universal-darwin.gz", sha256: String(repeating: "ab", count: 32)))
        XCTAssertEqual(EngineV3Biber.allowValue(p!), "\(String(repeating: "ab", count: 32))@https://example.org/biber-universal-darwin.gz")
        XCTAssertNil(EngineV3Biber.pin(lockText: "url = \"x\"\n"), "no biber pinned")
    }

    func testTheHostsSkipAsks() {
        XCTAssertTrue(EngineV3Biber.isConsentRequest("biber is not installed: FlashTeX will download biber (~70 MB) to build this bibliography"))
        XCTAssertFalse(EngineV3Biber.isConsentRequest("biber is not installed"))
    }
}
