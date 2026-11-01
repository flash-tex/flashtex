import Foundation

/// Appends every runtime-v1 JSON line the shell sends to or receives from the
/// worker, in observed transport order, so `scripts/check_runtime.py` can
/// validate the shell's own request/reply behavior (ids, revisions, negotiated
/// capabilities, stale classification). Enabled with `FLASHTEX_TRANSCRIPT=<path>`
/// or injected by tests. Lines are written verbatim (no re-encoding of received
/// bytes) and the file is opened in append mode.
final class RuntimeTranscript {
    let url: URL
    private let handle: FileHandle
    private let lock = NSLock()
    private(set) var lineCount = 0

    init(url: URL) throws {
        self.url = url
        let fm = FileManager.default
        if !fm.fileExists(atPath: url.path) { fm.createFile(atPath: url.path, contents: nil) }
        handle = try FileHandle(forWritingTo: url)
        try handle.seekToEnd()
    }

    /// From `FLASHTEX_TRANSCRIPT`, when set and creatable.
    static func fromEnvironment(_ env: [String: String] = ProcessInfo.processInfo.environment) -> RuntimeTranscript? {
        guard let path = env["FLASHTEX_TRANSCRIPT"], !path.isEmpty else { return nil }
        return try? RuntimeTranscript(url: URL(fileURLWithPath: path))
    }

    /// Records one complete JSON line (with or without its trailing LF).
    func record(_ line: Data) {
        var bytes = line
        if bytes.last != UInt8(ascii: "\n") { bytes.append(UInt8(ascii: "\n")) }
        lock.lock(); defer { lock.unlock() }
        try? handle.write(contentsOf: bytes)
        try? handle.synchronize()
        lineCount += 1
    }

    deinit { try? handle.close() }
}
