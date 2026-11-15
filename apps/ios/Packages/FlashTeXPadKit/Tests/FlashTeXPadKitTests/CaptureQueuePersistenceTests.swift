import Foundation
import XCTest
@testable import FlashTeXPadKit

/// Captures are written to disk off the caller's thread: `draft` (called on
/// the main actor by PadModel) used to write the PNG and `captures.json`
/// inline, under the lock every `records` read takes, so a slow disk stalled
/// the main thread both on the tap and on any list refresh during a save.
final class CaptureQueuePersistenceTests: XCTestCase {
    private var dir: URL!

    override func setUpWithError() throws {
        dir = FileManager.default.temporaryDirectory.appendingPathComponent("capture-persist-\(UUID().uuidString)", isDirectory: true)
    }

    override func tearDownWithError() throws { try? FileManager.default.removeItem(at: dir) }

    static let png = Data(base64Encoded: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4//8/AAX+Av4N70a4AAAAAElFTkSuQmCC")!

    func testDraftAndRecordsDoNotWaitForTheDisk() async throws {
        let store = CaptureStore(directory: dir)
        let disk = DispatchSemaphore(value: 0) // the "disk" is stuck until signalled
        store.willSave = { disk.wait() }
        let queue = CaptureQueue(link: MacLink(), store: store)
        let id = "cap-persist-1"

        let returned = expectation(description: "draft and records return while the disk is stuck")
        DispatchQueue.global().async {
            queue.draft(CaptureRecord(id: id, source: .pencil, png: Self.png, instructions: "x"))
            XCTAssertEqual(queue.records.map(\.id), [id])
            returned.fulfill()
        }
        await fulfillment(of: [returned], timeout: 2)
        disk.signal()
        await queue.flush()
        XCTAssertEqual(CaptureStore(directory: dir).load().map(\.id), [id], "on disk once flushed")
    }

    func testSavesLandInMutationOrder() async throws {
        let store = CaptureStore(directory: dir)
        let queue = CaptureQueue(link: MacLink(), store: store)
        let r = CaptureRecord(source: .pencil, png: Self.png, instructions: "x")
        queue.draft(r)
        queue.discard(r.id)
        await queue.flush()
        XCTAssertEqual(CaptureStore(directory: dir).load().first?.status, .discarded, "the later state wins on disk")
    }
}
