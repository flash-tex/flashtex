import SwiftUI
import XCTest
@testable import FlashTeXMac

/// `DS.Fonts.base`/`secondary`/`header` switched from a fixed `Font.system(size:)`
/// to the style-based `Font.system(.body)`/`.subheadline`, so they follow
/// System Settings > Accessibility > Display > Larger Text instead of a pinned
/// point size — but only at whatever text size macOS is actually running with;
/// a hermetic `swift test` cannot drive that system setting per test, and a
/// per-view `dynamicTypeSize`/`sizeCategory` override does not move SwiftUI
/// text rendering on macOS (measured directly: fitting size and `@ScaledMetric`
/// both stayed flat across `.large` and `.accessibility5`/`.accessibilityXXXL`
/// overrides). So this pins the one thing that IS verifiable headlessly: the
/// resting size at today's system setting is pixel-identical to the fixed size
/// it replaced, i.e. the conversion changed nothing visible right now.
final class DesignSystemFontsTests: XCTestCase {
    private func fittingSize(_ font: Font) -> CGSize {
        NSHostingView(rootView: Text("Ag").font(font).fixedSize()).fittingSize
    }

    func testBaseRestingSizeMatchesTheFixedSizeItReplaced() {
        XCTAssertEqual(fittingSize(DS.Fonts.base), fittingSize(Font.system(size: 13)))
    }

    func testSecondaryRestingSizeMatchesTheFixedSizeItReplaced() {
        XCTAssertEqual(fittingSize(DS.Fonts.secondary), fittingSize(Font.system(size: 11)))
    }

    func testHeaderRestingSizeMatchesTheFixedSizeItReplaced() {
        XCTAssertEqual(fittingSize(DS.Fonts.header), fittingSize(Font.system(size: 11, weight: .semibold)))
    }
}
