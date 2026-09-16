import AppKit
import SwiftUI

/// The main window's structural core: real `NSSplitViewController`s
/// (context/DECISION-swift-vs-alternatives.md — SwiftUI for pixels, AppKit
/// for structure). `HSplitView` offers no minimums, no collapse, no divider
/// styling and no persistence; these do, for free. The panes themselves stay
/// SwiftUI (`WorkspaceSidebar`, `EditorPane`, `PreviewPane`, `ProblemsPanel`)
/// hosted per split item, so all behaviour — every action, shortcut and
/// model seam — is untouched.
///
/// Layout (design-principles §4):
///
///     sidebar │ editor │ preview        ← EditorPreviewSplitViewController
///             │ ───────────────         ← CenterSplitViewController (stacked)
///             │ problems
///     ↑ WorkspaceSplitViewController
///
/// The icon rail and the status bar frame this in `ContentView`.

// MARK: - divider styling

/// A hairline divider in the Islands border tone: panels separate by tone,
/// not by system chrome.
final class ThinSplitView: NSSplitView {
    override var dividerThickness: CGFloat { DS.Size.hairline }
    override var dividerColor: NSColor { DS.NSColors.gutterHairline }
}

/// Base controller over a `ThinSplitView` with the CodeEdit-documented
/// quirk fixed: AppKit hides dividers when only one arranged item remains
/// visible, which reads as the panel bleeding into its neighbour.
class ThinSplitViewController: NSSplitViewController {
    /// Split positions persist through `NSSplitView` autosave — except under
    /// XCTest, where accumulated positions would make snapshot geometry
    /// nondeterministic between runs.
    static let autosaveEnabled = ProcessInfo.processInfo.environment["XCTestConfigurationFilePath"] == nil
        && ProcessInfo.processInfo.environment["XCTestSessionIdentifier"] == nil

    init(vertical: Bool, autosaveName: String?) {
        super.init(nibName: nil, bundle: nil)
        let sv = ThinSplitView()
        sv.isVertical = vertical
        sv.dividerStyle = .thin
        if let autosaveName, Self.autosaveEnabled { sv.autosaveName = autosaveName }
        splitView = sv
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError() }

    override func splitView(_ splitView: NSSplitView, shouldHideDividerAt dividerIndex: Int) -> Bool {
        guard splitViewItems.count > 1 else { return false }
        return super.splitView(splitView, shouldHideDividerAt: dividerIndex)
    }
}

// MARK: - editor | preview

/// The centre columns. Owns the narrow-layout rule (design-principles §4):
/// below the width where both columns fit, the preview collapses to a toggle
/// rather than being crushed under its minimum.
final class EditorPreviewSplitViewController: ThinSplitViewController {
    let editorItem: NSSplitViewItem
    let previewItem: NSSplitViewItem
    /// Reports narrowness whenever `setNarrow` changes it.
    var onNarrowChange: ((Bool) -> Void)?
    private var narrow = false
    /// Which column shows while narrow (`ShellModel.narrowPreviewShown`),
    /// pushed by the representable and re-applied on every layout pass.
    var previewShownWhileNarrow = false { didSet { applyNarrow() } }

    init(editor: NSViewController, preview: NSViewController) {
        editorItem = NSSplitViewItem(viewController: editor)
        editorItem.minimumThickness = DS.Layout.editorMinWidth
        // canCollapse stays true: programmatic `isCollapsed` is what drives
        // the narrow layout, and a divider drag past the minimum collapsing
        // a column is VS Code behaviour anyway.
        editorItem.canCollapse = true
        editorItem.holdingPriority = NSLayoutConstraint.Priority(249)
        previewItem = NSSplitViewItem(viewController: preview)
        previewItem.minimumThickness = DS.Layout.previewMinWidth
        previewItem.canCollapse = true
        previewItem.holdingPriority = NSLayoutConstraint.Priority(251)
        super.init(vertical: true, autosaveName: "FlashTeX.workspace.split.editorPreview")
        addSplitViewItem(editorItem)
        addSplitViewItem(previewItem)
    }

    /// Set by the owner (WorkspaceSplitViewController), which measures the
    /// width the two columns *would* have — a metric that does not move when
    /// a column collapses, so the state cannot flip-flop.
    func setNarrow(_ value: Bool) {
        let changed = value != narrow
        narrow = value
        // Enforce on every layout pass, not only on changes: a transient
        // zero-width pass while the window reaches its size can make AppKit
        // collapse a `canCollapse` column on its own; this puts the columns
        // back in the state the narrow rule wants.
        applyNarrow()
        if changed { onNarrowChange?(value) }
    }

    /// One of the two columns leaves entirely in the narrow layout; both are
    /// shown otherwise (design-principles §4).
    func applyNarrow() {
        let collapseEditor = narrow && previewShownWhileNarrow
        let collapsePreview = narrow && !previewShownWhileNarrow
        if editorItem.isCollapsed != collapseEditor { editorItem.isCollapsed = collapseEditor }
        if previewItem.isCollapsed != collapsePreview { previewItem.isCollapsed = collapsePreview }
    }
}

// MARK: - centre | problems

final class CenterSplitViewController: ThinSplitViewController {
    let problemsItem: NSSplitViewItem

    init(editorPreview: NSViewController, problems: NSViewController) {
        problemsItem = NSSplitViewItem(viewController: problems)
        super.init(vertical: false, autosaveName: "FlashTeX.workspace.split.problems")
        let top = NSSplitViewItem(viewController: editorPreview)
        top.minimumThickness = DS.Layout.editorMinHeightAbovePanel
        top.canCollapse = false
        top.holdingPriority = NSLayoutConstraint.Priority(249)
        problemsItem.minimumThickness = DS.Layout.problemsMinHeight
        problemsItem.canCollapse = false
        problemsItem.holdingPriority = NSLayoutConstraint.Priority(251)
        addSplitViewItem(top)
        addSplitViewItem(problemsItem)
    }

    /// The Problems panel never takes more than its share of the window
    /// (DS.Layout.problemsMaxFraction) — at 1000×640 the editor keeps ~15
    /// lines instead of 10.
    override func viewDidLoad() {
        super.viewDidLoad()
        let cap = problemsItem.viewController.view.heightAnchor.constraint(
            lessThanOrEqualTo: splitView.heightAnchor, multiplier: DS.Layout.problemsMaxFraction)
        cap.priority = NSLayoutConstraint.Priority(999)
        cap.isActive = true
    }

    /// First appearance with no autosaved position: give the panel its ideal
    /// height (the divider position must be set after layout has a height —
    /// and on a vertical stack it needs setting twice, see CodeEdit).
    private var appliedInitialPosition = false
    override func viewDidLayout() {
        super.viewDidLayout()
        guard !appliedInitialPosition, view.bounds.height > 0, !problemsItem.isCollapsed else { return }
        appliedInitialPosition = true
        if splitView.autosaveName == nil
            || UserDefaults.standard.object(forKey: "NSSplitView Subview Frames FlashTeX.workspace.split.problems") == nil {
            let position = max(0, view.bounds.height - DS.Layout.problemsIdealHeight)
            splitView.setPosition(position, ofDividerAt: 0)
            splitView.setPosition(position, ofDividerAt: 0)
        }
    }
}

// MARK: - sidebar | centre

final class WorkspaceSplitViewController: ThinSplitViewController {
    let sidebarItem: NSSplitViewItem
    let centerVC: CenterSplitViewController
    let editorPreviewVC: EditorPreviewSplitViewController

    init(sidebar: NSViewController, editor: NSViewController, preview: NSViewController, problems: NSViewController) {
        editorPreviewVC = EditorPreviewSplitViewController(editor: editor, preview: preview)
        centerVC = CenterSplitViewController(editorPreview: editorPreviewVC, problems: problems)
        // Deliberately NOT `sidebarWithViewController:` — that applies the
        // translucent sidebar material and forces collapse/spring-loading;
        // the IDE look wants a flat opaque tool window (brief §2).
        sidebarItem = NSSplitViewItem(viewController: sidebar)
        sidebarItem.minimumThickness = DS.Layout.sidebarMinWidth
        sidebarItem.maximumThickness = DS.Layout.sidebarMaxWidth
        sidebarItem.canCollapse = false
        sidebarItem.holdingPriority = NSLayoutConstraint.Priority(260)
        super.init(vertical: true, autosaveName: "FlashTeX.workspace.split.sidebar")
        addSplitViewItem(sidebarItem)
        addSplitViewItem(NSSplitViewItem(viewController: centerVC))
    }

    /// VS Code's default: `min(300, windowWidth / 4)`, floored at the
    /// sidebar minimum — applied only when there is no autosaved position.
    private var appliedInitialPosition = false
    override func viewDidLayout() {
        super.viewDidLayout()
        if !appliedInitialPosition, view.bounds.width > 0 {
            appliedInitialPosition = true
            if splitView.autosaveName == nil
                || UserDefaults.standard.object(forKey: "NSSplitView Subview Frames FlashTeX.workspace.split.sidebar") == nil {
                let width = max(DS.Layout.sidebarMinWidth, min(DS.Layout.sidebarDefaultWidth, view.bounds.width / 4))
                splitView.setPosition(width, ofDividerAt: 0)
            }
        }
        reportNarrow()
    }

    /// The narrow decision (design-principles §4): what the editor+preview
    /// columns would get — window width minus the sidebar as it stands —
    /// measured here because it does not move when a column collapses.
    private func reportNarrow() {
        let sidebarWidth = sidebarItem.isCollapsed ? 0 : sidebarItem.viewController.view.frame.width
        let available = view.bounds.width - sidebarWidth - splitView.dividerThickness
        let narrow = available < DS.Layout.editorMinWidth + DS.Layout.previewMinWidth + DS.Layout.resizeHandleHeight
        editorPreviewVC.setNarrow(narrow)
    }
}

// MARK: - SwiftUI bridge

/// Hosts the split tree and keeps its collapse state in step with the
/// workspace flags. All panes read `ShellModel` from the environment exactly
/// as before; this view only builds structure.
struct WorkspaceSplitPane: NSViewControllerRepresentable {
    @Environment(ShellModel.self) var model
    var nearby: NearbyState
    var projectVisible: Bool
    var outlineVisible: Bool
    /// Read in ContentView's body (so `@Observable` tracking re-runs the
    /// update when they change) and pushed into the split tree here.
    var problemsVisible: Bool
    var narrowPreviewShown: Bool

    func makeNSViewController(context: Context) -> WorkspaceSplitViewController {
        let model = self.model
        let sidebar = NSHostingController(
            rootView: AnyView(WorkspaceSidebar(projectVisible: projectVisible, outlineVisible: outlineVisible)
                .environment(model).environmentObject(nearby)))
        sidebar.sizingOptions = []
        let editor = NSHostingController(rootView: EditorPane().environment(model).environmentObject(nearby))
        editor.sizingOptions = []
        let preview = NSHostingController(rootView: PreviewPane().environment(model).environmentObject(nearby))
        preview.sizingOptions = []
        let problems = NSHostingController(rootView: ProblemsPanel().environment(model).environmentObject(nearby))
        problems.sizingOptions = []
        let vc = WorkspaceSplitViewController(sidebar: sidebar, editor: editor, preview: preview, problems: problems)
        context.coordinator.sidebarHost = sidebar
        vc.editorPreviewVC.onNarrowChange = { narrow in
            DispatchQueue.main.async {
                if model.narrowLayout != narrow { model.narrowLayout = narrow }
            }
        }
        return vc
    }

    func updateNSViewController(_ vc: WorkspaceSplitViewController, context: Context) {
        context.coordinator.sidebarHost?.rootView =
            AnyView(WorkspaceSidebar(projectVisible: projectVisible, outlineVisible: outlineVisible)
                .environment(model).environmentObject(nearby))
        let sidebarCollapsed = !(projectVisible || outlineVisible)
        if vc.sidebarItem.isCollapsed != sidebarCollapsed { vc.sidebarItem.isCollapsed = sidebarCollapsed }
        let problemsCollapsed = !problemsVisible
        if vc.centerVC.problemsItem.isCollapsed != problemsCollapsed { vc.centerVC.problemsItem.isCollapsed = problemsCollapsed }
        if vc.editorPreviewVC.previewShownWhileNarrow != narrowPreviewShown {
            vc.editorPreviewVC.previewShownWhileNarrow = narrowPreviewShown
        }
    }

    func makeCoordinator() -> Coordinator { Coordinator() }

    @MainActor final class Coordinator {
        var sidebarHost: NSHostingController<AnyView>?
    }
}
