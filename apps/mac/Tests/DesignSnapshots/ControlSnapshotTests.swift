//  The shared control vocabulary, on its own so a change to it is reviewed
//  as one picture rather than hunted across screens: the JetBrains text
//  button in its states (IDEButtonStyle), the flat icon control the title
//  bar and the panel headers use (IconButtonLabel), and the inline row
//  action that replaced bordered push buttons in the Problems panel.

import SwiftUI
import XCTest
@testable import FlashTeXMac

@MainActor
final class ControlSnapshotTests: XCTestCase {

    func testControlVocabulary() {
        assertSurfaceBothAppearances(ControlGallery(), named: "controls",
                                     size: CGSize(width: 560, height: 260))
    }
}

/// Rest states only — hover and pressed are per-pointer and cannot be
/// captured headlessly; the styles derive both from these tokens.
private struct ControlGallery: View {
    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.xl) {
            HStack(spacing: DS.Space.m) {
                Button("Apply") {}.ideDefault()
                Button("Cancel") {}.ideSecondary()
                Button("Reject") {}.ideSecondary(destructive: true)
                Button("Disabled") {}.ideSecondary().disabled(true)
            }
            HStack(spacing: DS.Space.xs) {
                IconButtonLabel(icon: "play.fill")
                IconButtonLabel(icon: "command")
                IconButtonLabel(icon: "moon", on: true)
                IconButtonLabel(icon: "plus.magnifyingglass", hovering: true)
                IconButtonLabel(icon: "tray")
            }
            HStack(spacing: DS.Space.m) {
                InlineActionLabel(title: "Fix…")
                InlineActionLabel(title: "Create chapters/results.tex", hovering: true)
                FilterChip(title: "Errors", selected: true) {}
                FilterChip(title: "Warnings", selected: false) {}
            }
        }
        .padding(DS.Space.xxl)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(DS.Colors.surfacePrimary)
    }
}
