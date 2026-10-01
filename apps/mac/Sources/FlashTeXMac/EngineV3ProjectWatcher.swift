import CoreServices
import Foundation

/// Watches a project folder, recursively, for its file set changing: a file
/// created, removed or renamed, or its extended attributes changed (a
/// download's `com.apple.quarantine`). Hidden paths (any component starting
/// with ".", such as `.git` or FlashTeX's own save temps
/// `.<name>.flashtex-tmp-…`) are ignored. `onChange` runs on the main
/// thread with the changed paths.
///
/// The engine-v3 session uses it so that trust (owner decision 9A, #1332)
/// is decided again before the next compile, an edit's included, once the
/// project's files change: a file that appears after the last walk (a
/// downloaded `.bib`, an unpacked archive) never reaches bibtex/biber or
/// `\write18` without the trust check (lane P5-APP-PARITY review of #1344).
final class EngineV3ProjectWatcher {
    private var stream: FSEventStreamRef?
    private let box: Box

    private final class Box {
        let root: String
        let onChange: ([String]) -> Void
        init(root: String, onChange: @escaping ([String]) -> Void) { self.root = root; self.onChange = onChange }
    }

    init?(root: URL, latency: TimeInterval = 0.2, onChange: @escaping ([String]) -> Void) {
        let path = root.standardizedFileURL.resolvingSymlinksInPath().path
        box = Box(root: path, onChange: onChange)
        var context = FSEventStreamContext(version: 0, info: Unmanaged.passUnretained(box).toOpaque(),
                                           retain: nil, release: nil, copyDescription: nil)
        let flags = FSEventStreamCreateFlags(kFSEventStreamCreateFlagUseCFTypes | kFSEventStreamCreateFlagFileEvents
                                             | kFSEventStreamCreateFlagNoDefer | kFSEventStreamCreateFlagWatchRoot)
        guard let s = FSEventStreamCreate(nil, Self.callback, &context, [path] as CFArray,
                                          FSEventStreamEventId(kFSEventStreamEventIdSinceNow), latency, flags) else { return nil }
        FSEventStreamSetDispatchQueue(s, .main)
        guard FSEventStreamStart(s) else {
            FSEventStreamInvalidate(s); FSEventStreamRelease(s)
            return nil
        }
        stream = s
    }

    deinit { stop() }

    func stop() {
        guard let s = stream else { return }
        stream = nil
        FSEventStreamStop(s)
        FSEventStreamInvalidate(s)
        FSEventStreamRelease(s)
    }

    /// The events that change a project's file set or a file's quarantine.
    static let relevantFlags = UInt32(kFSEventStreamEventFlagItemCreated | kFSEventStreamEventFlagItemRemoved
                                      | kFSEventStreamEventFlagItemRenamed | kFSEventStreamEventFlagItemXattrMod
                                      | kFSEventStreamEventFlagMustScanSubDirs | kFSEventStreamEventFlagRootChanged)

    /// Whether `path` (absolute, under `root`) is hidden: a component of it
    /// below the root starts with ".".
    static func isHidden(_ path: String, root: String) -> Bool {
        let rel = path.hasPrefix(root + "/") ? String(path.dropFirst(root.count + 1)) : path
        return rel.split(separator: "/").contains { $0.hasPrefix(".") }
    }

    private static let callback: FSEventStreamCallback = { _, info, count, paths, flags, _ in
        guard let info else { return }
        let box = Unmanaged<Box>.fromOpaque(info).takeUnretainedValue()
        guard let list = unsafeBitCast(paths, to: NSArray.self) as? [String] else { return }
        var out: [String] = []
        for i in 0 ..< count where flags[i] & relevantFlags != 0 {
            let p = list[i]
            if isHidden(p, root: box.root) { continue }
            out.append(p)
        }
        if !out.isEmpty { box.onChange(out) }
    }
}
