import AppKit
import SwiftUI
import XCTest
import FlashTeXDisplayListV3
import HostedWindows
@testable import FlashTeXMac

/// The engine-v3 pane's page rasters stay bounded (lane APP-PREVIEW-MEMORY).
/// The owner's app reached 9.6 GB on a ~570-page book after browsing and
/// editing: 1,319 dirty full-page IOSurfaces, every one still held by the
/// autorelease pool of the DL3 reader thread, which rasters and installs
/// the pages near the viewport as a compile sends them, and whose pool
/// drained only when the connection ended. This test opens a 500-page
/// document, scrolls through all of it, applies 50 edits to a page on
/// screen, and requires the process's physical footprint to grow by no
/// more than a few screens' worth of page bitmaps. It needs a built
/// `flashtex-host` and runs in the `mac app: engine-v3 host tests` job.
@MainActor
final class EngineV3MemoryBoundTests: XCTestCase {
    struct Host: View {
        let model: ShellModel
        var body: some View { EngineV3ScrollView(session: model.engineV3, zoom: model.previewZoom) }
    }

    static let cache = FileManager.default.temporaryDirectory.appendingPathComponent("engine-v3-memory-tests-\(getpid())")
    override func setUp() { setenv("FLASHTEX_V3_CACHE", Self.cache.path, 1) }
    override func tearDown() { unsetenv("FLASHTEX_V3_CACHE"); try? FileManager.default.removeItem(at: Self.cache) }

    func waitUntil(_ what: String, timeout: TimeInterval = 60, _ cond: @escaping () -> Bool) async throws {
        let start = Date()
        while !cond() {
            if Date().timeIntervalSince(start) > timeout { XCTFail("timeout waiting for \(what)"); throw CancellationError() }
            try await Task.sleep(nanoseconds: 5_000_000)
        }
    }

    static let pageCount = 500
    static let editedPage = 249

    /// One section per page, each ending in a `\newpage`; page `i + 1` has `Marker<i+1>.`
    static let document: String = {
        var s = "\\documentclass{article}\n\\begin{document}\n"
        let filler = String(repeating: "The quick brown fox jumps over the lazy dog while the pages keep their bitmaps bounded. ", count: 14)
        for i in 1 ... pageCount { s += "\\section*{Part \(i)}\nMarker\(i). \(filler)\n\n\(filler)\n\\newpage\n" }
        return s + "\\end{document}\n"
    }()

    /// The IOSurface dirty bytes and regions of this process, as `footprint`
    /// reports them (nil when the tool is missing or cannot read it).
    static func ioSurfaces() -> (bytes: String, regions: Int)? {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/footprint")
        p.arguments = ["\(getpid())"]
        let out = Pipe()
        p.standardOutput = out
        p.standardError = FileHandle.nullDevice
        guard (try? p.run()) != nil else { return nil }
        let data = out.fileHandleForReading.readDataToEndOfFile()
        p.waitUntilExit()
        for line in String(decoding: data, as: UTF8.self).split(separator: "\n") where line.hasSuffix(" IOSurface") {
            // "9319 MB        0 B          0 B       1319    IOSurface"
            let f = line.split(separator: " ", omittingEmptySubsequences: true)
            guard f.count >= 8, let regions = Int(f[6]) else { return nil }
            return ("\(f[0]) \(f[1])", regions)
        }
        return ("0 B", 0)
    }

    func testRastersStayBoundedOverAFullScrollAndFiftyEdits() async throws {
        try EngineV3TestHost.require()
        let model = ShellModel()
        model.replaceProject(entryText: Self.document, named: "main.tex")
        let stored = UserDefaults.standard.object(forKey: EngineV3.enabledKey)
        model.autoCompile = false // the test sends each compile itself
        model.engineV3Enabled = true
        let s = model.engineV3
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 640, height: 820), styleMask: [.titled, .resizable])
        window.isReleasedWhenClosed = false
        window.contentView = NSHostingView(rootView: Host(model: model))
        model.engineV3.start(model: model)
        defer {
            model.engineV3.stop()
            window.contentView = nil
            if let stored { UserDefaults.standard.set(stored, forKey: EngineV3.enabledKey) } else { UserDefaults.standard.removeObject(forKey: EngineV3.enabledKey) }
        }
        try await EngineV3TestHost.awaitReady(s)
        if !s.compiling, s.pageCount < Self.pageCount { s.compile(model: model, reason: "explicit") }
        try await waitUntil("the 500-page compile", timeout: 600) { !s.compiling && s.pageCount == Self.pageCount && (0 ..< Self.pageCount).allSatisfy { s.pages[$0] != nil } }
        window.layoutIfNeeded()
        model.previewZoom = 1
        let pages = try XCTUnwrap(s.view)
        pages.update(revision: s.layoutRevision, zoom: 1)
        pages.relayout()
        XCTAssertFalse(pages.tiled, "fit to width draws whole pages")
        pages.scrollToPage(0)
        try await waitUntil("page 1 on screen") { pages.pageShowsCurrent(0) }
        try await Task.sleep(nanoseconds: 300_000_000)
        let before = EngineV3ScrollBench.footprint()
        let surfacesBefore = Self.ioSurfaces()

        // Browse: every page through the viewport, each one drawn.
        for i in 0 ..< Self.pageCount {
            pages.scrollToPage(i)
            try await waitUntil("page \(i + 1) drawn while scrolling") { pages.pageShowsCurrent(i) }
        }

        // Edit: 50 compiles of the page on screen, each drawn (on the reader thread) before the next.
        let edited = Self.editedPage
        pages.scrollToPage(edited)
        try await waitUntil("the edited page on screen") { pages.pageShowsCurrent(edited) }
        var text = model.activeText
        for k in 0 ..< 50 {
            let marker = "Marker\(edited + 1). "
            let at = try XCTUnwrap(text.range(of: marker)).upperBound
            text.insert(contentsOf: "edit\(k) ", at: at)
            model.updateActiveText(text)
            let hash = s.pages[edited]?.page.hash
            let done = s.lastDone?["id"]?.int
            s.compile(model: model, reason: "explicit")
            try await waitUntil("edit \(k + 1) compiled") { !s.compiling && s.lastDone?["id"]?.int != done && s.pages[edited]?.page.hash != hash }
            try await waitUntil("edit \(k + 1) drawn") { pages.pageShowsCurrent(edited) }
        }
        // Rasters still in flight land, and the main thread's pool drains.
        try await Task.sleep(nanoseconds: 500_000_000)
        let after = EngineV3ScrollBench.footprint()
        let surfacesAfter = Self.ioSurfaces()

        let held = pages.heldPageViews.count
        let drawn = pages.heldPageViews.values.filter { $0.layer?.contents != nil }.count
        let pageBytes = pages.retainedBytes / max(1, drawn)
        let growth = after - before
        // A few screens of page bitmaps (the held pages, ones in flight, the
        // one each install replaces) plus room for the caches scrolling and
        // compiling fill (glyphs, page text): far below one bitmap per page.
        let bound = 3 * (held + 2) * pageBytes + (128 << 20)
        let mb = { (b: Int) in String(format: "%.1f MB", Double(b) / 1_048_576) }
        func describe(_ x: (bytes: String, regions: Int)?) -> String { x.map { "\($0.bytes) in \($0.regions) regions" } ?? "?" }
        var report = "APP-PREVIEW-MEMORY: footprint \(mb(before)) -> \(mb(after)) (growth \(mb(growth)), bound \(mb(bound)));"
        report += " IOSurface \(describe(surfacesBefore)) -> \(describe(surfacesAfter));"
        report += " \(held) pages held, \(mb(pageBytes)) each at \(pages.currentPixelsPerPoint) px/pt"
        print(report)
        XCTAssertLessThanOrEqual(growth, bound, "footprint grew \(mb(growth)) over a full scroll and 50 edits (bound \(mb(bound)))")
        if let a = surfacesAfter {
            // Region count: the held pages' bitmaps and tiles plus few in
            // flight, never one per page or per compile.
            XCTAssertLessThanOrEqual(a.regions, 4 * (held + 2) + 32, "IOSurfaces alive after the scroll and edits: \(a.regions)")
        }
    }
}
