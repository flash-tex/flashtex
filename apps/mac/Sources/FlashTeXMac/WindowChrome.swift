import AppKit
import SwiftUI

/// The IDE title bar (context/PROMPT-appearance-overhaul.md §6): transparent
/// title bar over a full-size content view, hidden title, no separator
/// hairline, compact toolbar — the window reads as one chrome surface with
/// the toolbar sitting on it, VS Code/JetBrains-fashion, instead of a stock
/// Aqua band. The macOS 14 path is `NSWindow` configuration (below); the
/// macOS 15 SwiftUI equivalents are applied where available in ContentView.
///
/// Deliberately *not* chased to the Ghostty level (drag strips, hidden
/// traffic lights): transparent + hidden + unified is enough for the look.
struct WindowChromeConfigurator: NSViewRepresentable {
    func makeNSView(context: Context) -> ChromeApplyingView { ChromeApplyingView() }
    func updateNSView(_ view: ChromeApplyingView, context: Context) {}

    final class ChromeApplyingView: NSView {
        private var reapplyObservers: [NSObjectProtocol] = []

        deinit { for o in reapplyObservers { NotificationCenter.default.removeObserver(o) } }

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            guard let window else { return }
            Self.apply(to: window)
            for o in reapplyObservers { NotificationCenter.default.removeObserver(o) }
            reapplyObservers = [
                // AppKit re-reveals native title chrome on exiting
                // fullscreen; setting a title can do the same on 15+.
                NotificationCenter.default.addObserver(forName: NSWindow.didExitFullScreenNotification,
                                                       object: window, queue: .main) { _ in
                    MainActor.assumeIsolated { Self.apply(to: window) }
                },
                NotificationCenter.default.addObserver(forName: NSWindow.didBecomeKeyNotification,
                                                       object: window, queue: .main) { _ in
                    MainActor.assumeIsolated { Self.apply(to: window) }
                },
            ]
        }

        @MainActor static func apply(to window: NSWindow) {
            // Only real titled windows: hosted borderless snapshot surfaces
            // and panels keep their own chrome.
            guard window.styleMask.contains(.titled) else { return }
            window.styleMask.insert(.fullSizeContentView)
            window.titlebarAppearsTransparent = true
            window.titleVisibility = .hidden
            window.titlebarSeparatorStyle = .none
            window.toolbarStyle = .unifiedCompact
            window.toolbar?.displayMode = .iconOnly
            window.backgroundColor = DS.NSColors.windowChrome
        }
    }
}
