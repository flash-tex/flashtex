import Foundation

/// Counts body evaluations of the window's large views while a test records
/// (P5-KEYSTROKE-MAIN): `KeystrokeInvalidationTests` types into the hosted
/// window and checks that a keystroke re-evaluates the editor, not the
/// window, the sidebar or the preview. Off, a call is one Bool test.
///
/// A view that knows its window's model passes it (`of:`), so a test with
/// several windows open (ProjectWindowsHostedTests) can tell which window's
/// views were evaluated, not only how many: `counts(of:)`.
@MainActor
enum ViewBodyProbe {
    static var isRecording = false
    private(set) static var counts: [String: Int] = [:]
    private(set) static var countsByModel: [ObjectIdentifier: [String: Int]] = [:]
    /// Each evaluation while recording, in order: the view, its model (if
    /// given) and when (`MonotonicClock.nowNs()`).
    private(set) static var events: [(view: String, model: ObjectIdentifier?, ns: UInt64)] = []
    /// Also keep the call stack of each evaluation (diagnostics only).
    static var recordsStacks = false
    private(set) static var stacks: [[String]] = []

    static func start() { counts = [:]; countsByModel = [:]; events = []; stacks = []; isRecording = true }
    static func stop() { isRecording = false }

    /// `let _ = ViewBodyProbe.note("Name", of: model)` at the top of a body.
    @discardableResult
    static func note(_ view: StaticString, of model: AnyObject? = nil) -> Bool {
        guard isRecording else { return false }
        let name = "\(view)"
        counts[name, default: 0] += 1
        let id = model.map(ObjectIdentifier.init)
        if let id { countsByModel[id, default: [:]][name, default: 0] += 1 }
        events.append((name, id, MonotonicClock.nowNs()))
        if recordsStacks { stacks.append(Thread.callStackSymbols) }
        return true
    }

    /// The evaluations of the views showing `model`, since `start()`.
    static func counts(of model: AnyObject) -> [String: Int] { countsByModel[ObjectIdentifier(model)] ?? [:] }
}
