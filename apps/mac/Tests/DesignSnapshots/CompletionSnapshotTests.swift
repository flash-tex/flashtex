//  The completion popup (Completion.swift, an NSPanel on purpose): row
//  anatomy is typed icon → monospace candidate → right-aligned dimmed
//  origin, with the fixed-height documentation pane and keyboard hints
//  under the list.

import AppKit
import HostedWindows
import SwiftUI
import XCTest
@testable import FlashTeXMac

@MainActor
final class CompletionSnapshotTests: XCTestCase {

    private var suggestions: [Completion.Suggestion] {
        [
            .init(label: "\\section", insertText: "\\section", kind: .command, detail: "supported by this compiler"),
            .init(label: "\\subsection", insertText: "\\subsection", kind: .command, detail: "supported by this compiler"),
            .init(label: "figure", insertText: "figure", kind: .environment, detail: "supported by this compiler"),
            .init(label: "fig:results", insertText: "fig:results", kind: .reference, detail: "defined in chapters/results.tex"),
            .init(label: "knuth84", insertText: "knuth84", kind: .citation, detail: "refs.bib · Knuth 1984"),
            .init(label: "sectional", insertText: "sectional", kind: .word, detail: "used in this document"),
        ]
    }

    func testCompletionPopup() {
        for appearance in Appearance.allCases {
            let parent = HostedWindowSupport.window(
                contentRect: NSRect(x: 0, y: 0, width: 600, height: 400), styleMask: [.borderless])
            parent.appearance = appearance.nsAppearance
            parent.makeKeyAndOrderFront(nil)
            let popup = CompletionPopup()
            popup.show(items: suggestions, selected: 0,
                       below: NSRect(x: 40, y: 320, width: 1, height: 16), parent: parent)
            assertWindowCapture(popup, named: "completion", appearance: appearance)
            popup.hide()
            parent.orderOut(nil)
        }
    }
}
