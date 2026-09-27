import XCTest
@testable import FlashTeXMac

/// File › New Project's template picker as VoiceOver hears it
/// (lane `ux-project-scaffold-template-voiceover`): each Template radio
/// option speaks its name AND description. SwiftUI materialises its
/// accessibility tree only for an assistive client (see
/// `PanelAccessibilityTests`), so the wording is pinned through the pure
/// rule the picker calls and the wiring is pinned at the source level, the
/// way `PreviewPaneAccessibilityTests` pins the pane's container label.
@MainActor
final class ProjectScaffoldVoiceOverTests: XCTestCase {
    static let sources = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent()
        .deletingLastPathComponent().appendingPathComponent("Sources")

    /// Every template option exposes a non-empty label containing its display
    /// name and its description (the `title`/`summary` strings, reused — no
    /// new text invented for the screen reader).
    func testEveryTemplateOptionLabelContainsItsNameAndDescription() {
        for template in ProjectTemplate.allCases {
            let label = template.accessibilityLabel
            XCTAssertFalse(label.isEmpty, template.rawValue)
            XCTAssertTrue(label.contains(template.title), "\(template.rawValue): label \(label)")
            XCTAssertTrue(label.contains(template.summary), "\(template.rawValue): label \(label)")
        }
    }

    /// The Template picker wires each option to that rule (source-level pin:
    /// the live AX tree is unavailable without an assistive client).
    func testTemplatePickerWiresEachOptionToItsAccessibilityLabel() throws {
        let source = try String(contentsOf: Self.sources.appendingPathComponent("FlashTeXMac/ProjectScaffoldViews.swift"), encoding: .utf8)
        let picker = try XCTUnwrap(source.range(of: "Picker(\"Template\""))
        let row = source[picker.lowerBound...]
        let end = row.range(of: ".pickerStyle(.radioGroup)")?.upperBound ?? row.endIndex
        let pickerSource = row[..<end]
        XCTAssertTrue(pickerSource.contains("ForEach(ProjectTemplate.allCases)"), "every template is an option")
        XCTAssertTrue(pickerSource.contains(".accessibilityLabel($0.accessibilityLabel)"), "each option speaks name + description")
    }
}
