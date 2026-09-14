//  The main window and its chrome surfaces, rendered from the checked-in
//  demo fixture (DesignFixtures.swift). One test per surface so a design
//  change re-records only what it touched; the full-shell shots are the
//  whole-window review the design-review agent starts from.

import SwiftUI
import XCTest
@testable import FlashTeXMac

@MainActor
final class ShellSnapshotTests: XCTestCase {

    func testMainWindow() {
        let model = DesignFixtures.project()
        assertWindowSurfaceBothAppearances(ContentView().environment(model).environmentObject(DesignFixtures.nearby()), named: "shell",
                                           size: CGSize(width: 1440, height: 900))
    }

    func testMainWindowWithProblems() {
        let model = DesignFixtures.projectWithProblems()
        assertWindowSurfaceBothAppearances(ContentView().environment(model).environmentObject(DesignFixtures.nearby()), named: "shell-problems",
                                           size: CGSize(width: 1440, height: 900))
    }

    /// The minimum supported width: three columns must still be usable.
    func testMainWindowNarrow() {
        let model = DesignFixtures.project()
        assertWindowSurfaceBothAppearances(ContentView().environment(model).environmentObject(DesignFixtures.nearby()), named: "shell-narrow",
                                           size: CGSize(width: 900, height: 600))
    }

    func testTabBar() {
        let model = DesignFixtures.project()
        assertSurfaceBothAppearances(DocumentTabBar().environment(model), named: "tabbar",
                                     size: CGSize(width: 900, height: 30))
    }

    func testSidebar() {
        let model = DesignFixtures.project()
        assertSurfaceBothAppearances(WorkspaceSidebar().environment(model), named: "sidebar",
                                     size: CGSize(width: 260, height: 700))
    }

    func testProblemsPanel() {
        let model = DesignFixtures.projectWithProblems()
        assertSurfaceBothAppearances(ProblemsPanel().environment(model), named: "problems",
                                     size: CGSize(width: 1000, height: 260))
    }

    func testStatusBar() {
        let model = DesignFixtures.projectWithProblems()
        assertSurfaceBothAppearances(StatusBar().environment(model), named: "statusbar",
                                     size: CGSize(width: 1440, height: 24))
    }

    func testCommandPalette() {
        let model = DesignFixtures.project()
        assertSurfaceBothAppearances(CommandPalette().environment(model), named: "palette",
                                     size: CGSize(width: 620, height: 440))
    }
}
