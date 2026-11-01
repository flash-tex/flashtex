//  The main window and its chrome surfaces, rendered from the checked-in
//  demo fixture (DesignFixtures.swift). One test per surface so a design
//  change re-records only what it touched; the full-shell shots are the
//  whole-window review the design-review agent starts from.

import SwiftUI
import XCTest
@testable import FlashTeXMac

@MainActor
final class ShellSnapshotTests: XCTestCase {

    /// Pixel comparison only means something on the machine the references
    /// were recorded on; elsewhere this skips loudly (SnapshotEnvironment.swift).
    override func setUp() async throws {
        try SnapshotEnvironment.requireComparableToReferences()
    }

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

    /// A healthy live preview at rest: the pages carry no chrome at all —
    /// the page/zoom HUD is transient (hover, scroll, zoom) and the state
    /// badge appears only when the pages are not the worker's current
    /// result. The fixture shot above is the pinned-HUD counterpart.
    func testMainWindowLivePreview() {
        let model = DesignFixtures.liveProject()
        assertWindowSurfaceBothAppearances(ContentView().environment(model).environmentObject(DesignFixtures.nearby()), named: "shell-live",
                                           size: CGSize(width: 1440, height: 900))
    }

    /// The minimum supported width: three columns must still be usable.
    func testMainWindowNarrow() {
        let model = DesignFixtures.project()
        assertWindowSurfaceBothAppearances(ContentView().environment(model).environmentObject(DesignFixtures.nearby()), named: "shell-narrow",
                                           size: CGSize(width: 900, height: 600))
    }

    /// Empty states: no compile result yet — the preview column explains the
    /// path forward and nothing shouts.
    func testMainWindowEmpty() {
        let model = DesignFixtures.emptyProject()
        assertWindowSurfaceBothAppearances(ContentView().environment(model).environmentObject(DesignFixtures.nearby()), named: "shell-empty",
                                           size: CGSize(width: 1440, height: 900))
    }

    func testTabBar() {
        let model = DesignFixtures.project()
        assertSurfaceBothAppearances(DocumentTabBar().environment(model), named: "tabbar",
                                     size: CGSize(width: 900, height: DS.Row.tab))
    }

    func testSidebar() {
        let model = DesignFixtures.project()
        assertSurfaceBothAppearances(WorkspaceSidebar(projectVisible: true, outlineVisible: true).environment(model), named: "sidebar",
                                     size: CGSize(width: 260, height: 700))
    }

    func testProblemsPanel() {
        let model = DesignFixtures.projectWithProblems()
        // Longer settle: the AppKit-backed List needs a beat to lay rows out
        // deterministically.
        assertSurfaceBothAppearances(ProblemsPanel().environment(model), named: "problems",
                                     size: CGSize(width: 1000, height: 260), settle: 0.6)
    }

    func testStatusBar() {
        let model = DesignFixtures.projectWithProblems()
        // Longer settle: the word count and breadcrumb are debounced.
        assertSurfaceBothAppearances(StatusBar().environment(model), named: "statusbar",
                                     size: CGSize(width: 1440, height: DS.Row.statusBar), settle: 0.8)
    }

    func testSettings() {
        let model = DesignFixtures.project()
        assertSurfaceBothAppearances(SettingsRootView().environment(model), named: "settings",
                                     size: CGSize(width: 500, height: 720), settle: 0.5)
    }

    func testCommandPalette() {
        let model = DesignFixtures.project()
        assertSurfaceBothAppearances(CommandPalette().environment(model), named: "palette",
                                     size: CGSize(width: 620, height: 440))
    }
}
