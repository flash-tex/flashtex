import AppKit
import SwiftUI
import FlashTeXProtocol

/// Invisible, hit-test-free accessibility elements laid over one preview
/// page: a landmark per page, a group per line, one static-text element per
/// item, in the `AccessibleDocumentModel` reading order. Items with a source
/// expose a "Go to source" action that calls the same closure as a mouse
/// click. The visible rendering and mouse behavior are untouched.
///
/// The elements are `NSAccessibilityElement`s built lazily inside one
/// `NSView` the first time an assistive client asks for the page's children
/// (and rebuilt after the page changes). A SwiftUI `ForEach` of ~1000
/// per-item views cost ~200 ms of diffing on every compile result; the lazy
/// tree costs nothing per keystroke unless VoiceOver is reading the page.
public struct AccessibilityOverlay: View {
    let page: RuntimeV1.Page
    let totalPages: Int
    let scale: CGFloat
    let fontName: (Double) -> String
    let onSelect: (RuntimeV1.SourceRange?, String?) -> Void

    struct Slot: Identifiable {
        let element: AccessibleDocumentModel.Element
        let frame: CGRect
        let priority: Double
        var id: Int { element.itemIndex }
    }

    /// - Parameters:
    ///   - page: the page being drawn.
    ///   - totalPages: page count for "Page 1 of 2" (0 = unknown).
    ///   - scale: display scale (1 = 1 pt per screen point).
    ///   - fontName: PostScript name of the face the preview draws a given
    ///     point size with (default Times-Roman); used only to size AX frames.
    ///   - onSelect: the preview's click handler (source range, item text).
    public init(page: RuntimeV1.Page, totalPages: Int = 0, scale: CGFloat,
                fontName: @escaping (Double) -> String = { _ in "Times-Roman" },
                onSelect: @escaping (RuntimeV1.SourceRange?, String?) -> Void) {
        self.page = page
        self.totalPages = totalPages
        self.scale = scale
        self.fontName = fontName
        self.onSelect = onSelect
    }

    /// Reading-order lines of the page (computed on demand, not per render).
    var lines: [AccessibleDocumentModel.Line] {
        AccessibleDocumentModel.lines(of: page, documents: [:], compiledDocuments: nil)
    }

    /// Elements in reading order with their frames; higher sort priority reads first.
    var slots: [Slot] { Self.slots(page: page, scale: scale, fontName: fontName, lines: lines) }

    static func slots(page: RuntimeV1.Page, scale: CGFloat, fontName: (Double) -> String,
                      lines: [AccessibleDocumentModel.Line]) -> [Slot] {
        let ordered = lines.flatMap(\.elements)
        return ordered.enumerated().map { i, e in
            Slot(element: e, frame: frame(for: e, in: page, scale: scale, fontName: fontName),
                 priority: Double(ordered.count - i))
        }
    }

    public var body: some View {
        PageAccessibilityView(page: page, totalPages: totalPages, scale: scale, fontName: fontName, onSelect: onSelect)
            .frame(width: page.widthPt * scale, height: page.heightPt * scale, alignment: .topLeading)
            .allowsHitTesting(false)
    }

    var pageLabel: String {
        AccessibleDocumentModel.PageSummary(number: page.number, lines: lines, totalPages: totalPages).label
    }

    /// Bounding box of an element in view points: measured with the preview's
    /// face (Times-Roman unless told otherwise) or the rule rectangle.
    static func frame(for element: AccessibleDocumentModel.Element, in page: RuntimeV1.Page, scale: CGFloat,
                      fontName: (Double) -> String = { _ in "Times-Roman" }) -> CGRect {
        guard element.itemIndex < page.items.count, case .text(let item) = page.items[element.itemIndex] else { return .zero }
        if let r = RuleConvention.rect(for: item) {
            return CGRect(x: r.x * scale, y: r.y * scale, width: r.width * scale, height: max(1, r.height * scale))
        }
        let font = NSFont(name: fontName(item.fontSizePt), size: item.fontSizePt)
            ?? NSFont(name: "Times-Roman", size: item.fontSizePt) ?? NSFont.systemFont(ofSize: item.fontSizePt)
        let width = NSAttributedString(string: item.text, attributes: [.font: font]).size().width
        let top = item.baselineYPt - Double(font.ascender)
        let height = Double(font.ascender - font.descender)
        return CGRect(x: item.xPt * scale, y: top * scale, width: max(1, width * scale), height: max(1, height * scale))
    }
}

/// Hosts `PageAXView`; updates hand it the new page and drop the cached tree.
private struct PageAccessibilityView: NSViewRepresentable {
    let page: RuntimeV1.Page
    let totalPages: Int
    let scale: CGFloat
    let fontName: (Double) -> String
    let onSelect: (RuntimeV1.SourceRange?, String?) -> Void

    func makeNSView(context: Context) -> PageAXView { PageAXView() }

    func updateNSView(_ view: PageAXView, context: Context) {
        view.update(page: page, totalPages: totalPages, scale: scale, fontName: fontName, onSelect: onSelect)
    }
}

/// What the preview's accessibility tree says about itself, shared by the
/// view, the help text and the tests.
public enum PreviewAccessibility {
    /// Subrole that puts a labelled group in VoiceOver's Landmarks rotor
    /// (the WebKit/AppKit spelling; AppKit exposes no Swift constant for it).
    public static let landmarkSubrole = NSAccessibility.Subrole(rawValue: "AXLandmarkRegion")
    public static let pageRoleDescription = "page"
    public static let lineRoleDescription = "line"
    public static let goToSourceAction = "Go to source"
    /// Help text of an item: where it is on the page.
    public static func itemHelp(_ e: AccessibleDocumentModel.Element) -> String { "Page \(e.page), line \(e.line)" }
}

/// One node of the lazy preview tree. Frames are stored in the page view's
/// (flipped) coordinates and converted to screen space through the view, so
/// VoiceOver's cursor outline lands on the drawn text, not its mirror image.
final class PreviewAXElement: NSAccessibilityElement {
    private(set) weak var pageView: NSView?
    /// Frame in `pageView` coordinates (top-left origin).
    private(set) var viewFrame: CGRect = .zero

    static func make(role: NSAccessibility.Role, label: String, viewFrame: CGRect, pageView: NSView,
                     parent: Any) -> PreviewAXElement {
        let e = PreviewAXElement()
        e.pageView = pageView
        e.viewFrame = viewFrame
        e.setAccessibilityRole(role)
        e.setAccessibilityLabel(label)
        e.setAccessibilityParent(parent)
        // Parent-space frame for clients that read the tree without a window.
        let parentOrigin = (parent as? PreviewAXElement)?.viewFrame.origin ?? .zero
        e.setAccessibilityFrameInParentSpace(viewFrame.offsetBy(dx: -parentOrigin.x, dy: -parentOrigin.y))
        return e
    }

    /// `NSAccessibilityElement` implements every `NSAccessibilityElement`
    /// protocol member through `NSAccessibility` but does not declare the
    /// protocol, so `accessibilityChildrenInNavigationOrder` (typed
    /// `[NSAccessibilityElementProtocol]`) cannot hold one without this
    /// runtime adoption. Declared once, before the first tree is built.
    static let adoptsElementProtocol: Bool = {
        guard let proto = objc_getProtocol("NSAccessibilityElement") else { return false }
        return class_addProtocol(PreviewAXElement.self, proto) || class_conformsToProtocol(PreviewAXElement.self, proto)
    }()

    static func navigationOrder(_ elements: [PreviewAXElement]) -> [NSAccessibilityElementProtocol] {
        guard adoptsElementProtocol else { return [] }
        return elements.compactMap { $0 as AnyObject as? NSAccessibilityElementProtocol }
    }

    func setNavigationChildren(_ children: [PreviewAXElement]) {
        setAccessibilityChildren(children)
        setAccessibilityChildrenInNavigationOrder(Self.navigationOrder(children))
    }

    override func accessibilityFrame() -> NSRect {
        guard let pageView, pageView.window != nil else { return super.accessibilityFrame() }
        return NSAccessibility.screenRect(fromView: pageView, rect: viewFrame)
    }
}

/// The page container: a landmark whose children are one group per line,
/// each holding one static-text element per item, created only when asked
/// for. Never hit-tested, never drawn.
final class PageAXView: NSView {
    private var page: RuntimeV1.Page?
    private var totalPages = 0
    private var scale: CGFloat = 1
    private var fontName: (Double) -> String = { _ in "Times-Roman" }
    private var onSelect: (RuntimeV1.SourceRange?, String?) -> Void = { _, _ in }
    private var cached: [PreviewAXElement]?
    private var cachedLabel: String?

    override var isFlipped: Bool { true }
    override func hitTest(_ point: NSPoint) -> NSView? { nil }
    override var isOpaque: Bool { false }

    func update(page: RuntimeV1.Page, totalPages: Int, scale: CGFloat,
                fontName: @escaping (Double) -> String, onSelect: @escaping (RuntimeV1.SourceRange?, String?) -> Void) {
        let changed = self.page != page || self.totalPages != totalPages || self.scale != scale
        self.page = page; self.totalPages = totalPages; self.scale = scale
        self.fontName = fontName; self.onSelect = onSelect
        guard changed else { return }
        let hadTree = cached != nil
        cached = nil; cachedLabel = nil
        // Only a client that already read this page needs to hear about the change.
        if hadTree { NSAccessibility.post(element: self, notification: .layoutChanged) }
    }

    /// Whether an assistive client has asked for this page's tree since the last change.
    var hasBuiltTree: Bool { cached != nil }

    override func isAccessibilityElement() -> Bool { true }
    override func accessibilityRole() -> NSAccessibility.Role? { .group }
    override func accessibilitySubrole() -> NSAccessibility.Subrole? { PreviewAccessibility.landmarkSubrole }
    override func accessibilityRoleDescription() -> String? { PreviewAccessibility.pageRoleDescription }
    override func accessibilityLabel() -> String? {
        if let cachedLabel { return cachedLabel }
        guard let page else { return nil }
        let lines = AccessibleDocumentModel.lines(of: page, documents: [:], compiledDocuments: nil)
        let label = AccessibleDocumentModel.PageSummary(number: page.number, lines: lines, totalPages: totalPages).label
        cachedLabel = label
        return label
    }

    override func accessibilityChildren() -> [Any]? { elements() }
    override func accessibilityChildrenInNavigationOrder() -> [NSAccessibilityElementProtocol]? {
        PreviewAXElement.navigationOrder(elements())
    }

    /// Builds the tree on first request; `AccessibilityOverlay.slots` defines
    /// the item order and frames so the two stay identical. Line frames are
    /// the union of their items' frames.
    private func elements() -> [PreviewAXElement] {
        if let cached { return cached }
        guard let page else { return [] }
        let lines = AccessibleDocumentModel.lines(of: page, documents: [:], compiledDocuments: nil)
        let slots = AccessibilityOverlay.slots(page: page, scale: scale, fontName: fontName, lines: lines)
        var slotByItem: [Int: AccessibilityOverlay.Slot] = [:]
        for s in slots { slotByItem[s.element.itemIndex] = s }
        let onSelect = self.onSelect
        let out: [PreviewAXElement] = lines.map { line in
            let frames = line.elements.compactMap { slotByItem[$0.itemIndex]?.frame }
            let union = frames.dropFirst().reduce(frames.first ?? .zero) { $0.union($1) }
            let group = PreviewAXElement.make(role: .group, label: line.label, viewFrame: union, pageView: self, parent: self)
            group.setAccessibilityRoleDescription(PreviewAccessibility.lineRoleDescription)
            let items: [PreviewAXElement] = line.elements.map { e in
                let ax = PreviewAXElement.make(role: .staticText, label: e.label,
                                               viewFrame: slotByItem[e.itemIndex]?.frame ?? .zero,
                                               pageView: self, parent: group)
                ax.setAccessibilityValue(e.value)
                ax.setAccessibilityHelp(PreviewAccessibility.itemHelp(e))
                if let source = e.source {
                    ax.setAccessibilityCustomActions([
                        NSAccessibilityCustomAction(name: PreviewAccessibility.goToSourceAction) { onSelect(source, e.text); return true },
                    ])
                }
                return ax
            }
            group.setNavigationChildren(items)
            return group
        }
        cached = out
        return out
    }
}

/// The diagnostics-list row as VoiceOver hears it: pure, so the test target
/// can check it against the keyboard navigator's announcements
/// (`EditorDiagnosticNavigation.Step`) and the editor mark's spoken form.
public struct DiagnosticRowAccessibility: Equatable {
    public var label: String
    public var value: String
    /// Set when the row has no source (nothing to act on).
    public var hint: String?
    public var actions: [String]

    public static let noSourceHint = "No source mapping; listed only."
    public static let goToSourceAction = PreviewAccessibility.goToSourceAction

    /// A grouped row (identical diagnostics folded into one row by the
    /// shell's `EditorDiagnostics.groups`): how many places share the
    /// message, which occurrence the row currently stands on (zero-based,
    /// document order) and where that occurrence is ("main.tex line 41",
    /// "main.tex bytes 10 to 20" or "no source"). Spoken as
    /// "12 places, 3 of 12, main.tex line 41" after the message.
    public struct GroupInfo: Equatable {
        public var count: Int
        public var occurrence: Int
        public var location: String

        public init(count: Int, occurrence: Int, location: String) {
            self.count = count; self.occurrence = occurrence; self.location = location
        }

        /// "12 places, 3 of 12, main.tex line 41"; nil for a single place.
        public var spoken: String? {
            guard count > 1 else { return nil }
            return "\(count) places, \(occurrence + 1) of \(count), \(location)"
        }
    }

    /// - Parameters:
    ///   - status: the result's status; a `recovered` result reads
    ///     "no provisional rendering" for a diagnostic without a recovery
    ///     note, exactly as the visible row and the keyboard navigator do.
    ///   - explanation: the offline explanation line the visible row shows
    ///     under the recovery line (`EditorDiagnosticNavigation.Item.explanation`), if any.
    ///   - group: for a grouped row, the group count, the occurrence the
    ///     row stands on and its location; the label then ends with
    ///     ", 12 places, 3 of 12, main.tex line 41" (nil or count 1: unchanged).
    public init(_ diagnostic: RuntimeV1.Diagnostic, index: Int, total: Int, status: RuntimeV1.Status,
                explanation: String? = nil, group: GroupInfo? = nil) {
        let element = AccessibleDocumentModel.DiagnosticElement(
            index: index, severity: diagnostic.severity, message: diagnostic.message,
            recovery: diagnostic.recovery, source: diagnostic.source, utf16Range: nil, textKnown: false, lines: [])
        label = "Diagnostic \(index + 1) of \(total): \(element.label)" + (group?.spoken.map { ", " + $0 } ?? "")
        var parts: [String] = []
        if let line = Self.recoveryLine(recovery: diagnostic.recovery, status: status) { parts.append(line) }
        if let explanation, !explanation.isEmpty { parts.append(explanation) }
        if let s = diagnostic.source { parts.append("\(s.path) bytes \(s.startByte) to \(s.endByte)") }
        value = parts.joined(separator: "; ")
        hint = diagnostic.source == nil ? Self.noSourceHint : nil
        actions = diagnostic.source == nil ? [] : [Self.goToSourceAction]
    }

    /// Same rule as the shell's `EditorDiagnostics.recoveryLine`: the
    /// worker's note, else "no provisional rendering" only for a `recovered`
    /// result (an `ok`/`failed` result without a note says nothing).
    public static func recoveryLine(recovery: String?, status: RuntimeV1.Status) -> String? {
        if let recovery { return "recovery: " + recovery }
        return status == .recovered ? "no provisional rendering" : nil
    }
}

public extension View {
    /// Makes a diagnostics-list row one element: "Diagnostic 1 of 2: Error:
    /// message" with the recovery line, the explanation line and source
    /// bytes as the value and a "Go to source" action when the diagnostic
    /// has a source. The visible row is unchanged.
    ///
    /// `group` (a grouped row) appends "12 places, 3 of 12, main.tex line 41"
    /// to the label; `goToSource` should then jump to that occurrence.
    func accessibleDiagnostic(_ diagnostic: RuntimeV1.Diagnostic, index: Int, total: Int,
                              status: RuntimeV1.Status, explanation: String? = nil,
                              group: DiagnosticRowAccessibility.GroupInfo? = nil,
                              goToSource: @escaping () -> Void) -> some View {
        let row = DiagnosticRowAccessibility(diagnostic, index: index, total: total, status: status, explanation: explanation,
                                             group: group)
        let base = accessibilityElement(children: .ignore)
            .accessibilityLabel(row.label)
            .accessibilityValue(row.value)
        return Group {
            if let hint = row.hint {
                base.accessibilityHint(hint)
            } else {
                base.accessibilityAction(named: DiagnosticRowAccessibility.goToSourceAction, goToSource)
            }
        }
    }

    /// Older call shape (no status): reads as a `recovered` result, so a
    /// diagnostic without a note says "no provisional rendering".
    func accessibleDiagnostic(_ diagnostic: RuntimeV1.Diagnostic, index: Int, total: Int,
                              goToSource: @escaping () -> Void) -> some View {
        accessibleDiagnostic(diagnostic, index: index, total: total, status: .recovered, goToSource: goToSource)
    }

    /// Labels the capture bar as a group whose value reads the pinned
    /// insertion point and queued proposals; its buttons stay reachable.
    func accessibleCaptureBar(anchor: String?, proposals: Int) -> some View {
        accessibilityElement(children: .contain)
            .accessibilityLabel("Capture bar")
            .accessibilityValue((anchor.map { "Insertion point pinned: \($0)" } ?? "No insertion point pinned")
                                + "; \(proposals) proposal\(proposals == 1 ? "" : "s") to review")
    }
}

/// The "Accessibility Help" window: focus order, what VoiceOver reads in
/// each pane, and the command table grouped by menu. `FlashTeXMacApp`
/// presents it from Help > FlashTeX Accessibility Help as
/// `Window("Accessibility Help", id: AccessibilityHelpView.windowID)`.
public struct AccessibilityHelpView: View {
    public static let windowID = "a11y-help"
    public static let windowTitle = "Accessibility Help"
    public static let menuItem = "FlashTeX Accessibility Help"

    /// Per-pane VoiceOver notes beyond the focus-order table.
    public static let voiceOverNotes: [String] = [
        "Tool rail: the leftmost stripe; one toggle per tool window (Project, Outline, and Problems at the bottom), each reading its name and whether it is selected. Toggling shows or hides that tool window; ⌘⇧M is Problems' shortcut.",
        "Sidebar: stacked tool windows toggled from the rail; Project rows read “path, entry, bibliography, edited, active” and open includes read “path, not open, included from …; activate to open”; Outline rows (when the Outline tool window is shown) read “section Title, line n”, “environment name, line n” or “label key, line n” inside Sections / Environments / Labels disclosure groups with their counts. Problems counts are a status-bar button; the list is the bottom panel.",
        "Tabs: the “Open documents” group; each tab is a button reading “path, entry, edited” with the selected trait on the active one, followed by “Detach path” on non-entry members; then the Project menu, the kind indicator and the byte counts.",
        "Editor: the text view is “LaTeX source”; every caret move that is not a typing step says “Line L, column C” (or the selection extent). ⌘⇧] and ⌘⇧[ move to the next or previous diagnostic and say “Error n of m, line L: message — recovery note”.",
        "Completion popup (Esc or ⌃Space): a list named “Completions”; each row reads the candidate, its kind (command, environment, label, citation, word) and where it comes from; ↑/↓ or Tab/⇧Tab choose and each choice is announced as “n of m: candidate, kind, origin”, Return inserts, Esc closes; the list never takes the keyboard from the editor.",
        "Preview: use the Landmarks rotor to jump between pages (“Page n of m, k lines”); inside a page each line is a group (“Page n, line k: text”) and each item is static text whose value gives its size and whether it has a source; the “Go to source” action selects the source in the editor.",
        "Problems: the panel header reads the counts, the “Problems severity filter” segments and “Hide Problems”; each list row is “Diagnostic n of m: Error or Warning: message” (grouped rows add “k places, j of k, path line n”); its value is the recovery line and source bytes; rows with a source have the “Go to source” action, rows without say “No source mapping; listed only.”",
        "Command palette (⌘⇧P): a sheet whose “Command palette search” field has the keyboard; ↑/↓ move through the filtered rows, each read as its help line (title, shortcut, menu, description; keys that cannot be run from the palette say so), Return runs the row, Esc closes.",
        "Capture bar: one group whose value reads the pinned insertion point and how many proposals are waiting; the review sheet approves with Return.",
        "Settings (⌘,): a form named “Editor preferences”; each control reads its label and value (“Editor font size, 13 points”, “Tab width, 4 columns”); hints explain the wrap, appearance, brace and completion switches.",
        "Durable History (Edit > Durable History…): the header reads the durable revision; Undo and Redo read how many steps are available; the retention gauge reads its percentage; each stack row is “undo/redo n of m: label”; empty stacks say so.",
        "Find in Project (⌘⇧F): the literal field reads its hint (Return searches or goes to the selected match, ↑/↓ move the selection); results read “match n of m, file, line, snippet”; replacement previews read “replacement n of m … becomes …”; status lines are labelled “Search status” and “Replacement status”.",
        "Nearby Companion (⌘⇧N): the status row reads “Pairing state: <state>” with a countdown-free value; the step indicator reads “Pairing step n of 4: <step>, <in progress | interrupted | done>”; the code reads its six digits one by one and the expiry reads “n seconds left”; each companion row reads its name, pair id, connected/not connected, paired and last-seen times; every transition (code shown, companion connected, paired, cancelled, interrupted, reconnected, disconnected, capture received or refused) is announced.",
    ]

    public init() {}

    /// Menus in the order the menu bar shows them.
    static var menus: [(menu: String, entries: [AccessibilityCommand.Entry])] {
        var order: [String] = []
        var byMenu: [String: [AccessibilityCommand.Entry]] = [:]
        for e in AccessibilityCommand.entries {
            if byMenu[e.menu] == nil { order.append(e.menu) }
            byMenu[e.menu, default: []].append(e)
        }
        return order.map { ($0, byMenu[$0] ?? []) }
    }

    public var body: some View {
        List {
            Section("Focus order: \(FocusOrder.description)") {
                ForEach(FocusOrder.helpLines, id: \.self) { Text($0).font(.callout) }
                ForEach(FocusOrder.statusLines, id: \.self) { Text($0).font(.callout).foregroundStyle(.secondary) }
            }
            Section("What VoiceOver reads") {
                ForEach(Self.voiceOverNotes, id: \.self) { Text($0).font(.callout) }
            }
            Section("Panels: keyboard focus order") {
                ForEach(PanelFocusOrder.helpLines, id: \.self) { Text($0).font(.callout) }
            }
            ForEach(Self.menus, id: \.menu) { group in
                Section("\(group.menu) commands") {
                    ForEach(group.entries, id: \.command) { e in
                        VStack(alignment: .leading, spacing: 2) {
                            Text("\(e.title) — \(e.shortcuts.joined(separator: " or "))").bold()
                            Text(e.description + (e.requires.map { " Requires \($0)." } ?? "")).font(.callout)
                        }
                        .accessibilityElement(children: .combine)
                        .accessibilityLabel(e.helpLine)
                    }
                }
            }
        }
        .accessibilityLabel(Self.windowTitle)
        .frame(minWidth: 520, minHeight: 400)
    }
}
