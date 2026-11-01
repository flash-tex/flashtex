import AppKit
import SwiftUI
import XCTest
import FlashTeXProtocol
import HostedWindows
@testable import FlashTeXMac

/// Drives an input method's `NSTextInputClient` calls on the real editor
/// (`CompletingTextView` inside `SourceEditorView`, hosted like `ContentView`
/// does) while a `ShellModel` is attached to the REAL preview controller and
/// compiler. The window is ordered front but never made key: no focus is
/// stolen (`FLASHTEX_NO_ACTIVATE` semantics for a test process).
///
/// A synthesized key event cannot drive a real input source in a test
/// process (no candidate window, no dead-key state), so the composition is
/// replayed at the client API — `setMarkedText(_:selectedRange:replacementRange:)`,
/// `insertText(_:replacementRange:)`, `unmarkText()` — which is exactly the
/// path the view takes when `NSTextInputContext` calls it for a Japanese,
/// Chinese or Korean input method.
///
/// Helper observation goes through the shell's own client
/// (`model.controller`) with `controllerState.awaiting`, so a `document` or
/// `history_status` reply is observed without being applied to the model.
@MainActor
final class IMEHarness {
    static var helper: URL? {
        ProcessInfo.processInfo.environment["FLASHTEX_PREVIEW_CONTROLLER"].map { URL(fileURLWithPath: $0) }
    }
    static let noReplacement = NSRange(location: NSNotFound, length: 0)

    final class Probe {
        var editApplied: [(ShellModel.PendingEdit, String)] = []
        var coordinator: SourceEditorView.Coordinator?
    }

    private struct Host: View {
        var model: ShellModel
        var probe: Probe
        var body: some View {
            SourceEditorView(
                text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                selection: model.selection,
                pendingEdit: model.pendingEdit,
                marks: model.editorMarks,
                result: model.result,
                editorRevision: model.editorRevision,
                onCaretChange: { model.caretUTF16 = $0 },
                onSelectionChange: { model.caretLengthUTF16 = $0.length },
                onEditApplied: { edit, text in
                    probe.editApplied.append((edit, text))
                    model.editApplied(edit, newText: text)
                },
                onEditRefused: { edit, reason in model.editRefused(edit, reason: reason) }
            )
        }
    }

    let model: ShellModel
    let probe = Probe()
    let root: URL
    let tex: URL
    let helper: URL
    private(set) var window: NSWindow!
    private(set) var textView: NSTextView!
    var coordinator: SourceEditorView.Coordinator { probe.coordinator! }
    /// Every preview-controller pid this harness launched (the only pids it may kill).
    private(set) var launchedPIDs: [Int32] = []

    private init(model: ShellModel, root: URL, tex: URL, helper: URL) {
        self.model = model; self.root = root; self.tex = tex; self.helper = helper
    }

    /// Skips unless `FLASHTEX_PREVIEW_CONTROLLER` and `FLASHTEX_COMPILER` name
    /// executables. Opens a temporary project holding `text`, attaches the
    /// helper (private ledger under the project's temp root), hosts the editor
    /// and waits for durable r1 and the first preview bound to the buffer.
    static func attached(_ name: String, text: String) async throws -> IMEHarness {
        guard let helper, FileManager.default.isExecutableFile(atPath: helper.path),
              ShellModel.locateCompiler() != nil else {
            throw XCTSkip("set FLASHTEX_PREVIEW_CONTROLLER and FLASHTEX_COMPILER to built binaries")
        }
        let root = FileManager.default.temporaryDirectory.appendingPathComponent("ime-\(name)-\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: root.appendingPathComponent("project"), withIntermediateDirectories: true)
        let tex = root.appendingPathComponent("project/main.tex")
        try text.write(to: tex, atomically: true, encoding: .utf8)
        setenv("FLASHTEX_CONTROLLER_LEDGER_ROOT", root.appendingPathComponent("ledger").path, 1)
        let model = ShellModel()
        model.autoCompile = true
        XCTAssertEqual(model.openTex(at: tex), .opened)
        let h = IMEHarness(model: model, root: root, tex: tex, helper: helper)
        try await h.host()
        h.attach()
        try await h.waitUntil("durable r1 and first preview") {
            model.controllerState.durable["main.tex"]?.revision == 1 && model.controllerState.inFlight == nil
                && model.result?.revision == model.editorRevision
        }
        return h
    }

    private func host() async throws {
        HostedWindowSupport.prepare() // non-activating: hosted windows must never pull the app forward
        let window = HostedWindowSupport.window(contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.titled],
                                                backing: .buffered, defer: false)
        window.contentView = NSHostingView(rootView: Host(model: model, probe: probe))
        window.orderFrontRegardless() // never makeKey
        self.window = window
        var found: NSTextView?
        try await waitUntil("editor text view") {
            found = TypingBenchDriver.findTextView(in: [window.contentView!]); return found != nil
        }
        let tv = try XCTUnwrap(found)
        probe.coordinator = tv.delegate as? SourceEditorView.Coordinator
        XCTAssertNotNil(probe.coordinator)
        XCTAssertTrue(window.makeFirstResponder(tv))
        textView = tv
    }

    /// Launches (or relaunches) the helper for the project and records its pid.
    func attach() {
        model.attachController(at: helper)
        XCTAssertTrue(model.controllerAttached)
        if let pid = model.controller?.processIdentifier { launchedPIDs.append(pid) }
    }

    func close() {
        model.detachController()
        window?.orderOut(nil)
        unsetenv("FLASHTEX_CONTROLLER_LEDGER_ROOT")
        try? FileManager.default.removeItem(at: root)
    }

    // MARK: composition (what the input context does)

    /// One composition step: the marked text is replaced by `text` with the
    /// insertion point at its end (Japanese/Chinese/Korean IMEs keep the
    /// caret at the end of the marked run while converting).
    func compose(_ text: String) {
        textView.setMarkedText(text, selectedRange: NSRange(location: (text as NSString).length, length: 0),
                               replacementRange: Self.noReplacement)
    }

    /// The IME confirms the candidate: `insertText` replaces the marked text.
    func commit(_ text: String) {
        textView.insertText(text, replacementRange: Self.noReplacement)
    }

    /// The IME cancels (Esc): empty marked text, then `unmarkText`.
    func cancel() {
        compose("")
        textView.unmarkText()
    }

    var markedRange: NSRange { textView.markedRange() }
    var selectedRange: NSRange { textView.selectedRange() }
    var hasMarkedText: Bool { textView.hasMarkedText() }
    var string: String { textView.string }

    // MARK: helper observation (never applied to the model)

    struct HelperDocument: Equatable {
        var revision: Int
        var sha256: String
        var text: String
    }

    /// `document {path}` answered by the running helper.
    func helperDocument(path: String = "main.tex") async throws -> HelperDocument {
        guard let controller = model.controller, controller.isRunning, model.controllerState.ready else {
            throw XCTSkip("helper not running")
        }
        let id = try controller.document(path: path)
        let reply: Result<[String: Any], ControllerError> = await withCheckedContinuation { cont in
            model.controllerState.awaiting[id] = { cont.resume(returning: $0) }
        }
        let payload = try reply.get()
        let doc = try XCTUnwrap(payload["document"] as? [String: Any], "document reply")
        return HelperDocument(revision: try XCTUnwrap(doc["revision"] as? Int),
                              sha256: try XCTUnwrap(doc["source_sha256"] as? String),
                              text: try XCTUnwrap(doc["text"] as? String))
    }

    /// `history_status {path}` answered by the running helper: (undo labels, redo labels).
    func historyLabels(path: String = "main.tex") async throws -> (undo: [String], redo: [String]) {
        guard let controller = model.controller, controller.isRunning, model.controllerState.ready else {
            throw XCTSkip("helper not running")
        }
        let id = try controller.send("history_status", ["path": path])
        let reply: Result<[String: Any], ControllerError> = await withCheckedContinuation { cont in
            model.controllerState.awaiting[id] = { cont.resume(returning: $0) }
        }
        let status = try XCTUnwrap(EditHistory.Status.decode(try reply.get()), "history_status reply")
        return (status.undoLabels, status.redoLabels)
    }

    /// `file_status {path}` through the shell (disk vs durable source).
    func fileState(path: String = "main.tex") async -> String? {
        await model.controllerFileStatus(path: path)?.state
    }

    /// SIGKILL to the helper pid THIS harness launched (never a name-based
    /// kill), then waits for the shell to observe the exit. Returns the pid.
    func killHelper() async throws -> Int32 {
        let pid = try XCTUnwrap(model.controller?.processIdentifier, "no helper to kill")
        XCTAssertTrue(launchedPIDs.contains(pid), "only pids launched by this harness are killed")
        XCTAssertEqual(kill(pid, SIGKILL), 0, "kill(\(pid), SIGKILL)")
        try await waitUntil("helper exit observed by the shell") { [model] in model.controller == nil && !model.controllerAttached }
        return pid
    }

    // MARK: waiting

    func waitUntil(_ what: String, timeout: TimeInterval = 20, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(timeout)
        while Date() < deadline {
            if cond() { return }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        XCTFail("timed out waiting for \(what)")
        throw Timeout(what: what)
    }

    struct Timeout: Error { var what: String }

    /// Lets the current run-loop turn end (SwiftUI updates, coalesced announcements).
    func turn() async throws { try await Task.sleep(nanoseconds: 30_000_000) }

    /// Settles for `seconds` and fails if `cond` stops holding at any sample.
    func holds(_ what: String, for seconds: TimeInterval = 0.5, _ cond: @escaping @MainActor () -> Bool) async throws {
        let deadline = Date().addingTimeInterval(seconds)
        while Date() < deadline {
            XCTAssertTrue(cond(), "\(what) stopped holding")
            try await Task.sleep(nanoseconds: 20_000_000)
        }
    }

    /// 1-minute load average (nil if unavailable) — recorded, and a bound for tests that need a calm machine.
    static func loadAverage1() -> Double? {
        var loads = [Double](repeating: 0, count: 3)
        return getloadavg(&loads, 3) >= 1 ? loads[0] : nil
    }

    /// `uptime` output for evidence.
    static func uptime() -> String {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/uptime")
        let out = Pipe()
        p.standardOutput = out
        guard (try? p.run()) != nil else { return "uptime unavailable" }
        p.waitUntilExit()
        return String(decoding: out.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self).trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
