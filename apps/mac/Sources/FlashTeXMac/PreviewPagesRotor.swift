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
/// rotor's `itemLoadingDelegate` (this object) to load it
/// (`accessibilityElement(withToken:)`), which scrolls to the page through
/// the same path as Page Up/Down and then hands back that page's view once
/// the lazy stack has built it (a stand-in element at the page's place until
/// then). The rotor is exposed by every page view and line element (the
/// places VoiceOver reads from), all forwarding to the one search-and-load
/// object the probe owns.
///
/// Search semantics follow `EditorRotorSearch`: no current item → the first
/// (next) or last (previous); otherwise strictly after / before the current
/// page; `filterString` is a case-insensitive substring of the label; no
/// wrap-around.
///
/// Load transition table (a page is BUILT when its view is mounted and
/// loaded — resident —, a PLACEHOLDER when mounted but elided, UNBUILT when
/// the lazy stack has no view for it; `pendingLoad` is the page a load is
/// still to hand VoiceOver). Pinned by the hosted rotor test.
///
/// | event                                | effect                                                                    |
/// |--------------------------------------|---------------------------------------------------------------------------|
/// | list, page BUILT                     | result targets the view                                                   |
/// | list, PLACEHOLDER or UNBUILT         | loading-token result (label only; nothing scrolls)                        |
/// | load, page not in the layout         | nil; nothing pending                                                      |
/// | load, BUILT                          | scroll (no animation, no spoken landing), return the view; nothing pending|
/// | load, PLACEHOLDER                    | scroll, return the placeholder; pending until it becomes resident         |
/// | load, UNBUILT                        | scroll, return a stand-in at the page's place; pending until built        |
/// | a newer load                         | replaces the pending page                                                 |
/// | pending page appears / turns resident| `layoutChanged` naming the real view; nothing pending                     |
/// | other page appears                   | nothing                                                                   |
/// | reader scrolls, steps, follows, links| pending dropped only when the view actually moved for it (a `.recompile` |
/// |                                      | follow that finds the caret visible, or a step with nowhere to go, is not |
/// |                                      | movement: the frame a rotor load's own window recompile brings must still |
/// |                                      | hand VoiceOver the page)                                                  |
/// | layout changes                       | stand-ins move with their pages; pages gone from the document lose theirs |
@MainActor
final class PreviewPagesRotor: NSObject, NSAccessibilityCustomRotorItemSearchDelegate, NSAccessibilityElementLoading {
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
    /// The page a load handed a stand-in for, until its view appears.
    private(set) var pendingLoad: Int?
    /// Pages whose view appeared after a stand-in load and were announced
    /// to VoiceOver (`layoutChanged` naming the view), in order.
    private(set) var appearanceNotices: [Int] = []

    init(probe: PreviewAnchorProbe) {
        self.probe = probe
        super.init()
        let rotor = NSAccessibilityCustomRotor(label: Self.rotorLabel, itemSearchDelegate: self)
        // Both delegates are weak; this object outlives the rotor (the probe holds it).
        rotor.itemLoadingDelegate = self
        rotors = [rotor]
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
    /// the page of the current item otherwise — its target view, a line
    /// element inside a page view, a stand-in (`standInPage` names its page),
    /// or the loading token it was made with.
    static func start(of parameters: NSAccessibilityCustomRotor.SearchParameters,
                      standInPage: (PreviewAXElement) -> Int? = { _ in nil }) -> Int? {
        guard let current = parameters.currentItem else { return nil }
        if let view = current.targetElement as? PreviewPageAXTarget { return view.previewPageNumber }
        if let element = current.targetElement as? PreviewAXElement {
            if let page = element.pageView as? PreviewPageAXTarget { return page.previewPageNumber } // a line
            return standInPage(element)
        }
        if let token = current.itemLoadingToken as? NSNumber { return token.intValue }
        return nil
    }

    func start(of parameters: NSAccessibilityCustomRotor.SearchParameters) -> Int? {
        Self.start(of: parameters) { element in self.standIns.first { $0.value === element }?.key }
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

    /// The result VoiceOver lists for `item`: the page's view when it is
    /// built and loaded, else a loading token (the page number) — a mounted
    /// placeholder of an elided page is not loaded, so choosing it must go
    /// through the loading delegate (scroll, pending handoff) too. Both
    /// under the item's label.
    func result(for item: Item) -> NSAccessibilityCustomRotor.ItemResult {
        let result: NSAccessibilityCustomRotor.ItemResult
        if let view = pageView(item.number), (view as? PreviewPageAXTarget)?.previewPageIsLoaded == true {
            result = NSAccessibilityCustomRotor.ItemResult(targetElement: view)
        } else {
            result = NSAccessibilityCustomRotor.ItemResult(itemLoadingToken: NSNumber(value: item.number), customLabel: item.label)
        }
        result.customLabel = item.label
        return result
    }

    /// The reader chose a not-yet-built page: scroll there without animation
    /// (the reader asked to go there; the Page Up/Down path, so the landing
    /// is announced), give the lazy stack a layout pass, and hand back the
    /// page's view. Should the stack build it only on a later turn, a
    /// stand-in at the page's place is returned; a mounted but not yet
    /// resident page (a windowed frame's placeholder) is handed back as it
    /// is. Either way the load stays pending until the page is loaded, when
    /// `pageViewDidAppear` posts a `layoutChanged` naming the real element.
    func load(_ token: NSAccessibilityLoadingToken) -> NSAccessibilityElementProtocol? {
        guard let number = (token as? NSNumber)?.intValue, let probe,
              probe.layout?.frame(of: number) != nil else { return nil } // a token from a longer document: nothing to load
        loads.append(number)
        pendingLoad = number
        // No spoken landing: VoiceOver reads the element it is handed.
        probe.scrollToTop(ofPage: number, animated: false, announce: false)
        probe.enclosingScrollView?.layoutSubtreeIfNeeded()
        if let view = pageView(number) {
            if (view as? PreviewPageAXTarget)?.previewPageIsLoaded == true { pendingLoad = nil }
            return view
        }
        return standIn(for: number).flatMap { PreviewAXElement.navigationOrder([$0]).first }
    }

    /// A page view joined the window, or became resident (PageV2AXView): if
    /// it is the page a load is pending for, tell VoiceOver where the real
    /// element is.
    func pageViewDidAppear(_ view: NSView) {
        guard let target = view as? PreviewPageAXTarget, let number = target.previewPageNumber,
              number == pendingLoad, target.previewPageIsLoaded else { return }
        pendingLoad = nil
        appearanceNotices.append(number)
        NSAccessibility.post(element: view, notification: .layoutChanged, userInfo: [.uiElements: [view]])
    }

    /// The reader moved on (scrolled, stepped, followed the caret, activated
    /// a link): a page still to appear must not pull VoiceOver's cursor later.
    func cancelPendingLoad() { pendingLoad = nil }

    /// The page column was re-laid out: stand-ins keep their page's place;
    /// a stand-in for a page the document no longer has is dropped.
    func layoutDidChange(_ layout: PreviewPageLayout) {
        for (number, element) in standIns {
            if let frame = layout.frame(of: number) { element.setViewFrame(frame) } else { standIns[number] = nil }
        }
        if let pending = pendingLoad, layout.frame(of: pending) == nil { pendingLoad = nil }
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

    /// `NSAccessibilityElementLoading`, as the rotor's `itemLoadingDelegate`:
    /// VoiceOver chose a loading-token item.
    nonisolated func accessibilityElement(withToken token: NSAccessibilityLoadingToken) -> NSAccessibilityElementProtocol? {
        MainActor.assumeIsolated { load(token) }
    }

    nonisolated func rotor(_ rotor: NSAccessibilityCustomRotor,
                           resultFor searchParameters: NSAccessibilityCustomRotor.SearchParameters) -> NSAccessibilityCustomRotor.ItemResult? {
        MainActor.assumeIsolated {
            let found = Self.resolve(items: items, start: start(of: searchParameters),
                                     forward: searchParameters.searchDirection == .next,
                                     filter: searchParameters.filterString)
            return found.map(result(for:))
        }
    }
}

extension PreviewAnchorProbe: PreviewPagesRotorSource {
    var previewPagesRotors: [NSAccessibilityCustomRotor] { pagesRotor.rotors }
    func previewPageElement(forToken token: NSAccessibilityLoadingToken) -> NSAccessibilityElementProtocol? { pagesRotor.load(token) }
    func previewPageDidAppear(_ view: NSView) { pagesRotor.pageViewDidAppear(view) }
}
