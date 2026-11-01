import Foundation

/// Machine-load evidence the editor performance tests record with their
/// numbers. It was the harness that drove input-method compositions against
/// the retired preview controller helper (`IMECompositionTests`, #1236 S6);
/// composition on the editor itself is covered by `SourceEditorViewTests`
/// and `SyntaxPainterIMETests`, which replay it at the `NSTextInputClient`
/// API (`setMarkedText`, `insertText`, `unmarkText`).
enum IMEHarness {
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
