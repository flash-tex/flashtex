//  Proves the render loop itself works before anyone relies on it: the
//  dependency resolves, the target builds, both appearances render, and PNGs
//  land on disk where an agent can open them. If this fails, no other
//  snapshot result in this target means anything.

import SwiftUI
import XCTest

final class HarnessSmokeTests: XCTestCase {
    /// A deliberately trivial surface using semantic colours, so a failure
    /// here is the harness and never the app.
    private struct Probe: View {
        var body: some View {
            VStack(spacing: 8) {
                Text("FlashTeX").font(.title2)
                Text("render loop").foregroundStyle(.secondary)
            }
            .padding(24)
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .background(Color(nsColor: .windowBackgroundColor))
        }
    }

    func testHarnessRendersBothAppearances() {
        assertSurfaceBothAppearances(Probe(), named: "harness-probe",
                                     size: CGSize(width: 320, height: 160))
    }
}
