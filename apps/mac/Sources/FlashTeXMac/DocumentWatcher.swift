import Foundation
import ObjectiveC

/// Watches the open document's file with a `DispatchSource` so an external
/// change is noticed while the app stays frontmost (lane mac-document-files-2,
/// follow-up 2): `applicationDidBecomeActive` remains the fallback, this is
/// the live path. Events are coalesced (`debounce`) and delivered on the main
/// actor to `onChange`, which the shell binds to `refreshDiskStatus()` — the
/// same explicit-conflict path as before: nothing is ever reloaded or written
/// by an event, the status check decides (`unchanged` clears a stale conflict,
/// `modified`/`created` surface one, `deleted` is a notice).
///
/// A vnode source follows the *inode*: an atomic replace (rename over the
/// path, which is how every cooperative writer here saves) and a delete both
/// end the watched inode, so the watcher re-opens the path after each such
/// event and keeps going while the path exists; a missing file is retried on
/// the next `watch`/`rearm`. Editors that write in place (`.write`/`.extend`)
/// keep the inode and just fire.
@MainActor
final class DocumentWatcher {
    private(set) var url: URL?
    /// Raw vnode events seen (tests).
    private(set) var events = 0
    /// Coalesced deliveries to `onChange` (tests).
    private(set) var deliveries = 0
    var debounce: TimeInterval = 0.25
    var onChange: (@MainActor () -> Void)?
    private(set) var status = "not watching"

    private var source: DispatchSourceFileSystemObject?
    private var fd: Int32 = -1
    private var pending: DispatchWorkItem?

    var isWatching: Bool { source != nil }

    /// Watches `url` (replacing any previous watch). False when the file
    /// cannot be opened (missing, unreadable); the watcher then waits for the
    /// next `watch`/`rearm` call — it never polls.
    @discardableResult
    func watch(_ url: URL) -> Bool {
        stop(keepURL: true)
        self.url = url
        return arm()
    }

    /// Re-opens the current URL after its inode went away (or `watch` failed).
    @discardableResult
    func rearm() -> Bool {
        guard url != nil else { return false }
        stop(keepURL: true)
        return arm()
    }

    private func arm() -> Bool {
        guard let url else { return false }
        let fd = open(url.path, O_EVTONLY)
        guard fd >= 0 else {
            status = "not watching \(url.lastPathComponent): \(String(cString: strerror(errno)))"
            FlashTeXLog.write("watch: " + status)
            return false
        }
        self.fd = fd
        let source = DispatchSource.makeFileSystemObjectSource(fileDescriptor: fd, eventMask: [.write, .extend, .attrib, .delete, .rename, .revoke], queue: .main)
        source.setEventHandler { [weak self] in
            MainActor.assumeIsolated { self?.handle(source.data) }
        }
        source.setCancelHandler { close(fd) }
        self.source = source
        source.resume()
        status = "watching \(url.lastPathComponent)"
        return true
    }

    private func handle(_ flags: DispatchSource.FileSystemEvent) {
        events += 1
        let inodeGone = !flags.intersection([.delete, .rename, .revoke]).isEmpty
        if inodeGone {
            // Atomic replace or delete: the fd now names an orphaned inode.
            // Re-open the path (the new file, if any) before reporting, so a
            // change to the *replacement* is watched too.
            stop(keepURL: true)
            if !arm() { status = "file went away: \(url?.lastPathComponent ?? "-") (rearm when it returns)" }
        }
        pending?.cancel()
        let item = DispatchWorkItem { [weak self] in
            MainActor.assumeIsolated {
                guard let self else { return }
                self.pending = nil
                self.deliveries += 1
                self.onChange?()
            }
        }
        pending = item
        DispatchQueue.main.asyncAfter(deadline: .now() + debounce, execute: item)
    }

    /// Stops watching; the URL is forgotten unless `keepURL`.
    func stop(keepURL: Bool = false) {
        pending?.cancel()
        pending = nil
        source?.cancel() // the cancel handler closes the fd
        source = nil
        fd = -1
        if !keepURL { url = nil; status = "not watching" }
    }
}

extension ShellModel {
    private static var watcherKey = 0
    /// Live file watcher for the open document (see `DocumentWatcher`).
    var documentWatcher: DocumentWatcher {
        if let existing = objc_getAssociatedObject(self, &Self.watcherKey) as? DocumentWatcher { return existing }
        let watcher = DocumentWatcher()
        watcher.onChange = { [weak self] in self?.watcherFired(retry: true) }
        // A check dropped while a save was in flight runs once the save's late
        // reply has settled (#831): by then nothing is outstanding to lose.
        files.onSaveSettledLate = { [weak self] in
            guard let self, pendingWatcherRecheck else { return }
            pendingWatcherRecheck = false
            watcherFired(retry: false)
        }
        objc_setAssociatedObject(self, &Self.watcherKey, watcher, .OBJC_ASSOCIATION_RETAIN_NONATOMIC)
        return watcher
    }

    /// A status check must not race a save whose reply is still outstanding:
    /// the file layer restarts a helper with an unanswered request, which would
    /// lose the late receipt the reconciliation waits for (the event is often
    /// that very save landing). Retry once after the helper wait; a reply that
    /// arrives meanwhile settles the state itself. If the reply is *still*
    /// outstanding at the retry, the event is not dropped (#831): the check is
    /// re-armed exactly once for when the late reply has been reconciled
    /// (`onSaveSettledLate`), so a genuine external change made while the save
    /// was in flight is seen without waiting for the next filesystem event.
    private func watcherFired(retry: Bool) {
        if files.helperBusy {
            guard retry else { pendingWatcherRecheck = true; return }
            let wait = max(files.helperTimeout, 0.5)
            DispatchQueue.main.asyncAfter(deadline: .now() + wait) { [weak self] in
                MainActor.assumeIsolated { self?.watcherFired(retry: false) }
            }
            return
        }
        Task { @MainActor [weak self] in await self?.refreshDiskStatus() }
    }

    /// (Re)binds the watcher to `documentURL`; called after open, reload and
    /// save. Without a URL the watcher stops. Opt out with
    /// `FLASHTEX_NO_FILE_WATCH=1` (benches, automation that hammers the file).
    func watchOpenDocument() {
        if ProcessInfo.processInfo.environment["FLASHTEX_NO_FILE_WATCH"] == "1" { return }
        guard let documentURL else { documentWatcher.stop(); return }
        if documentWatcher.url != documentURL || !documentWatcher.isWatching { documentWatcher.watch(documentURL) }
    }
}
