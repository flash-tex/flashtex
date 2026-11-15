import Foundation

/// When to tell the host which page the view shows (`VIEWPORT`,
/// `viewport-v1`, docs/protocol/display-list-v3.md §6.3.1): at once after a
/// jump, so the host hears of it before the next key, and otherwise once
/// scrolling settles. A move of `jumpPages` pages or more from the page last
/// sent is a jump (a search result, an outline click, a SyncTeX reveal);
/// smaller moves are scrolling, sent `settle` after the last one. The same
/// page twice in a row is sent once.
@MainActor
public final class DL3ViewportHint {
    public static let jumpPages = 3
    public static let settle: Duration = .milliseconds(150)

    private let send: (Int) -> Void
    private var lastSent: Int?
    private var pending: Task<Void, Never>?

    public init(send: @escaping (Int) -> Void) {
        self.send = send
    }

    /// The view now shows `page` (0-based).
    public func note(_ page: Int) {
        pending?.cancel()
        pending = nil
        guard page != lastSent else { return }
        if let last = lastSent, abs(page - last) < Self.jumpPages {
            pending = Task { @MainActor [weak self] in
                try? await Task.sleep(for: Self.settle)
                guard !Task.isCancelled else { return }
                self?.fire(page)
            }
        } else {
            fire(page)
        }
    }

    private func fire(_ page: Int) {
        lastSent = page
        send(page)
    }
}
