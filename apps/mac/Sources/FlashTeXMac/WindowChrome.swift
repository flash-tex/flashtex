import AppKit
import SwiftUI

/// The IDE title bar (context/PROMPT-appearance-overhaul.md §6, reshaped by
/// the owner's #653 feedback): transparent title bar over a full-size
/// content view, hidden title, no separator hairline, and an EMPTY
/// `NSToolbar` kept solely so AppKit gives the title bar its unified-compact
/// height and centres the traffic lights in it. The controls themselves are
/// `TitleBarRow` (TitleBar.swift), drawn as content in that region,
/// IntelliJ-fashion — macOS 26 floats real toolbar items on Liquid Glass
/// platters, which is exactly the look the owner rejected. The row carries
/// its own drag surface, one step further toward Ghostty than before.
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
            // Height-only toolbar: no delegate, no items — TitleBarRow in the
            // content draws the controls (TitleBar.swift).
            if window.toolbar?.identifier != "FlashTeX.titlebar-height" {
                window.toolbar = NSToolbar(identifier: "FlashTeX.titlebar-height")
            }
            window.backgroundColor = DS.NSColors.windowChrome
        }
    }
}
