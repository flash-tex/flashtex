import XCTest
@testable import FlashTeXProtocol

/// `payload.date` — the civil date `\today` renders
/// (protocol/proposals/runtime-v1-request-date.md).
///
/// The compiler must never read the wall clock: runtime-v1 requires
/// byte-identical output for byte-identical input, and a compiler that reads a
/// clock is not a function of its inputs at all. So the *app* reads it and
/// sends the answer as an ordinary request field. That is how `\date{\today}`
/// can print today's date without anything downstream becoming
/// non-deterministic.
final class RequestDateTests: XCTestCase {

    // MARK: the wire field

    /// Absent means "no date supplied", which the worker compiles as the Unix
    /// epoch — exactly its behaviour before the field existed. Every older
    /// request stays valid and byte-identical, which is what lets the committed
    /// fixtures stay untouched.
    func testDateIsOmittedFromTheWireWhenNil() throws {
        let plain = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "m", documents: [])
        let line = String(decoding: try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "a", plain)), as: UTF8.self)
        XCTAssertFalse(line.contains("\"date\""), line)
    }

    func testDateEncodesAndRoundTrips() throws {
        let request = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "m", documents: [],
                                               date: "2026-09-13")
        let line = try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "b", request))
        XCTAssertTrue(String(decoding: line, as: UTF8.self).contains(#""date":"2026-09-13""#),
                      String(decoding: line, as: UTF8.self))
        let back = try RuntimeV1.decodeCompileRequest(line)
        XCTAssertEqual(back.payload.date, "2026-09-13")
        XCTAssertEqual(back.payload, request)
    }

    // MARK: validation

    /// The worker refuses a malformed date rather than guessing at another one,
    /// so sending one would only turn into a failed compile. Catching it here
    /// makes it a programming error at the call site instead.
    func testMalformedDatesAreRejectedRatherThanSent() {
        for bad in ["2026-9-13", "13-09-2026", "2026/09/13", "2026-09-13T12:00:00Z",
                    "", "yesterday", "20260913", " 2026-09-13"] {
            XCTAssertThrowsError(try RuntimeV1.validateDate(bad), bad)
        }
    }

    func testImpossibleDaysAreRejected() {
        for bad in ["2026-02-29", "1900-02-29", "2026-13-01", "2026-04-31", "2026-00-10", "0000-01-01"] {
            XCTAssertThrowsError(try RuntimeV1.validateDate(bad), bad)
        }
        // Both Gregorian leap-rule exceptions.
        XCTAssertNoThrow(try RuntimeV1.validateDate("2024-02-29"))
        XCTAssertNoThrow(try RuntimeV1.validateDate("2000-02-29"))
        XCTAssertNoThrow(try RuntimeV1.validateDate("1970-01-01"))
    }

    func testEncodingAMalformedDateThrows() {
        let bad = RuntimeV1.CompileRequest(projectId: "p", revision: 1, entryPath: "m", documents: [],
                                           date: "2026-02-29")
        XCTAssertThrowsError(try RuntimeV1.encodeLine(RuntimeV1.compileEnvelope(id: "c", bad)))
    }

    // MARK: reading the clock

    /// `\today` is a *local* calendar date. Resolving it from a UTC instant
    /// would print the neighbouring day for much of the world near midnight,
    /// so the app asks the user's own timezone.
    func testLocalDateUsesTheGivenTimeZoneNotUTC() {
        // 2026-09-13T00:30:00Z — already the 13th in UTC and in Tokyo, still
        // the 12th in New York. Exactly the near-midnight case a UTC-only
        // answer would get wrong.
        let instant = Date(timeIntervalSince1970: 1_789_259_400)
        XCTAssertEqual(RuntimeV1.localDate(instant, timeZone: TimeZone(identifier: "UTC")!), "2026-09-13")
        XCTAssertEqual(RuntimeV1.localDate(instant, timeZone: TimeZone(identifier: "America/New_York")!), "2026-09-12")
        XCTAssertEqual(RuntimeV1.localDate(instant, timeZone: TimeZone(identifier: "Asia/Tokyo")!), "2026-09-13")
    }

    /// Whatever the clock says, the app must produce something the worker will
    /// accept — the two validators have to agree.
    func testLocalDateAlwaysProducesAValidWireValue() throws {
        for offset in stride(from: 0, to: 400 * 86_400, by: 86_400 / 2) {
            let value = RuntimeV1.localDate(Date(timeIntervalSince1970: TimeInterval(offset)))
            XCTAssertNoThrow(try RuntimeV1.validateDate(value), value)
        }
        XCTAssertEqual(RuntimeV1.localDate(Date(timeIntervalSince1970: 0),
                                           timeZone: TimeZone(identifier: "UTC")!), "1970-01-01")
    }
}
