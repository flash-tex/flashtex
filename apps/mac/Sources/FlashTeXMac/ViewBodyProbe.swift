/// Counts body evaluations of the window's large views while a test records
/// (P5-KEYSTROKE-MAIN): `KeystrokeInvalidationTests` types into the hosted
/// window and checks that a keystroke re-evaluates the editor, not the
/// window, the sidebar or the preview. Off, a call is one Bool test.
@MainActor
enum ViewBodyProbe {
    static var isRecording = false
    private(set) static var counts: [String: Int] = [:]

    static func start() { counts = [:]; isRecording = true }
    static func stop() { isRecording = false }

    /// `let _ = ViewBodyProbe.note("Name")` at the top of a body.
    @discardableResult
    static func note(_ view: StaticString) -> Bool {
        guard isRecording else { return false }
        counts["\(view)", default: 0] += 1
        return true
    }
}
