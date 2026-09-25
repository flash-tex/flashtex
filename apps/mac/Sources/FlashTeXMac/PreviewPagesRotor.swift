import AppKit
import FlashTeXAccessibility

/// VoiceOver's "Pages" rotor over the preview pane. The v2 pane lays its
/// pages out lazily (a 1000-page document must not build 1000 views), so a
/// page that is off screen has no accessibility view and is missing from the
/// Landmarks rotor; this rotor lists every page of the document instead —
/// "Page 3 of 12", "Page 7 of 12, not loaded" for an elided page of a
/// windowed frame — from the layout the anchor probe already tracks.
///
/// Items for pages whose accessibility view exists carry it as the target;
/// the others are `itemLoadingToken` results (the page number), which is the
/// AppKit shape for "not built yet": VoiceOver lists them by `customLabel`
/// without touching anything, and only when the reader CHOOSES one asks the
/// rotor's owner to load it (`accessibilityElement(withToken:)`), which
/// scrolls to the page through the same path as Page Up/Down and then hands
/// back that page's view once the lazy stack has built it (a stand-in
/// element at the page's place until then). The rotor is exposed by every
/// page view and line element (the places VoiceOver reads from), all
/// forwarding to the one search object the probe owns.
///
/// Search semantics follow `EditorRotorSearch`: no current item → the first
/// (next) or last (previous); otherwise strictly after / before the current
/// page; `filterString` is a case-insensitive substring of the label; no
/// wrap-around.
@MainActor
final class PreviewPagesRotor: NSObject, NSAccessibilityCustomRotorItemSearchDelegate {
    struct Item: Equatable {
        var number: Int
        var label: String
    }

    static let rotorLabel = "Pages"

    private(set) weak var probe: PreviewAnchorProbe?
    private(set) var rotors: [NSAccessibilityCustomRotor] = []
    /// Stand-ins handed to VoiceOver for pages the lazy stack had not built
    /// when they were chosen, keyed by page (one per page, reused).
    private var standIns: [Int: PreviewAXElement] = [:]
    /// Evidence for tests: pages loaded through the rotor, in order.
    private(set) var loads: [Int] = []

    init(probe: PreviewAnchorProbe) {
        self.probe = probe
        super.init()
        rotors = [NSAccessibilityCustomRotor(label: Self.rotorLabel, itemSearchDelegate: self)]
    }

    // MARK: pure

    static func label(number: Int, totalPages: Int, elided: Bool) -> String {
        "Page \(number)\(totalPages > 0 ? " of \(totalPages)" : "")" + (elided ? ", not loaded" : "")
    }

    /// One item per page of `layout`, in order.
    static func items(layout: PreviewPageLayout, elided: Set<Int>) -> [Item] {
        let total = layout.pages.count
        return layout.pages.map { Item(number: $0.number, label: label(number: $0.number, totalPages: total, elided: elided.contains($0.number))) }
    }

    /// Where a search starts: nil for "from the first/last item, inclusive";
    /// the page of the current item otherwise (its target view, or the
    /// loading token it was made with).
    static func start(of parameters: NSAccessibilityCustomRotor.SearchParameters) -> Int? {
        guard let current = parameters.currentItem else { return nil }
        if let view = current.targetElement as? PreviewPageAXTarget { return view.previewPageNumber }
        if let token = current.itemLoadingToken as? NSNumber { return token.intValue }
        return nil
    }

    static func resolve(items: [Item], start: Int?, forward: Bool, filter: String) -> Item? {
        let matching = filter.isEmpty ? items : items.filter { $0.label.localizedCaseInsensitiveContains(filter) }
        switch (start, forward) {
        case (nil, true): return matching.first
        case (nil, false): return matching.last
        case (let at?, true): return matching.first { $0.number > at }
        case (let at?, false): return matching.last { $0.number < at }
        }
    }

    // MARK: the live pane

    var items: [Item] {
        guard let probe, let layout = probe.layout else { return [] }
        return Self.items(layout: layout, elided: probe.elidedPages)
    }

    /// The accessibility view of page `number` if the lazy stack has built it.
    func pageView(_ number: Int) -> NSView? {
        guard let document = probe?.enclosingScrollView?.documentView else { return nil }
        return Self.pageView(number, under: document)
    }

    static func pageView(_ number: Int, under view: NSView) -> NSView? {
        if let target = view as? PreviewPageAXTarget, target.previewPageNumber == number { return view }
        for sub in view.subviews { if let found = pageView(number, under: sub) { return found } }
        return nil
    }

    /// The result VoiceOver lists for `item`: the page's view when it exists,
    /// else a loading token (the page number) — both under the item's label.
    func result(for item: Item) -> NSAccessibilityCustomRotor.ItemResult {
        let result: NSAccessibilityCustomRotor.ItemResult
        if let view = pageView(item.number) {
            result = NSAccessibilityCustomRotor.ItemResult(targetElement: view)
        } else {
            result = NSAccessibilityCustomRotor.ItemResult(itemLoadingToken: NSNumber(value: item.number), customLabel: item.label)
        }
        result.customLabel = item.label
        return result
    }

    /// The reader chose a not-yet-built page: scroll there (the Page Up/Down
    /// path, so the landing is announced), give the lazy stack a layout pass,
    /// and hand back the page's view — or a stand-in at the page's place when
    /// the stack builds it on a later turn (VoiceOver's next move finds the
    /// real one).
    func load(_ token: NSAccessibilityLoadingToken) -> NSAccessibilityElementProtocol? {
        guard let number = (token as? NSNumber)?.intValue, let probe else { return nil }
        loads.append(number)
        probe.scrollToTop(ofPage: number)
        probe.enclosingScrollView?.layoutSubtreeIfNeeded()
        if let view = pageView(number) { return view }
        return standIn(for: number).flatMap { PreviewAXElement.navigationOrder([$0]).first }
    }

    /// A stand-in element for page `number` at the page's frame in the probe's
    /// coordinates (the probe backs the whole padded page column, so layout
    /// frames are its own), labelled like the rotor item.
    func standIn(for number: Int) -> PreviewAXElement? {
        guard let probe, let layout = probe.layout, let frame = layout.frame(of: number) else { return nil }
        let label = Self.label(number: number, totalPages: layout.pages.count, elided: probe.elidedPages.contains(number))
        if let existing = standIns[number] {
            existing.setViewFrame(frame)
            existing.setAccessibilityLabel(label)
            return existing
        }
        let element = PreviewAXElement.make(role: .group, label: label, viewFrame: frame, pageView: probe, parent: probe)
        element.setAccessibilitySubrole(PreviewAccessibility.landmarkSubrole)
        element.setAccessibilityRoleDescription(PreviewAccessibility.pageRoleDescription)
        standIns[number] = element
        return element
    }

    nonisolated func rotor(_ rotor: NSAccessibilityCustomRotor,
                           resultFor searchParameters: NSAccessibilityCustomRotor.SearchParameters) -> NSAccessibilityCustomRotor.ItemResult? {
        MainActor.assumeIsolated {
            let found = Self.resolve(items: items, start: Self.start(of: searchParameters),
                                     forward: searchParameters.searchDirection == .next,
                                     filter: searchParameters.filterString)
            return found.map(result(for:))
        }
    }
}

extension PreviewAnchorProbe: PreviewPagesRotorSource {
    var previewPagesRotors: [NSAccessibilityCustomRotor] { pagesRotor.rotors }
    func previewPageElement(forToken token: NSAccessibilityLoadingToken) -> NSAccessibilityElementProtocol? { pagesRotor.load(token) }
}
