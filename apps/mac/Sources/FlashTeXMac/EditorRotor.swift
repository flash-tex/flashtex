import Accessibility
import AppKit
import FlashTeXAccessibility
import FlashTeXProtocol

/// VoiceOver rotor over the source editor (lane mac-editor-a11y-3): the
/// "Headings" rotor (`\chapter` … `\paragraph`), an "Environments" rotor and
/// a "Diagnostics" rotor (ux-editor-diagnostics-voiceover), all from
/// `AccessibleEditorModel.rotorItems`. Read-only: a chosen item becomes the
/// text view's selection (AppKit moves the selection to the result's
/// `targetRange`); nothing edits the buffer.
///
/// Search semantics follow `NSAccessibilityCustomRotor.SearchParameters`:
/// no current item → the first (next) or last (previous) item; a current
/// item whose range location is `NSNotFound` → from the first/last character;
/// otherwise strictly after / strictly before the current item's location, so
/// the caret position VoiceOver passes as the current item finds the next
/// heading even when it is off-screen. `filterString` is a case-insensitive
/// substring match on the item label (type-ahead). No wrap-around: nil at
/// either end, which is what VoiceOver expects ("no more headings").
///
/// Headings and environments come from the text and are cached until the
/// next edit. Diagnostics come from the owner's marks (`marks`), which
/// change with every compile reply rather than with edits, so their items
/// are cached against the marks themselves: an underline that appears
/// without a keystroke is in the rotor at the next search.
@MainActor
final class EditorRotorSearch: NSObject, NSAccessibilityCustomRotorItemSearchDelegate {
    typealias Category = AccessibleEditorModel.RotorCategory
    typealias Item = AccessibleEditorModel.RotorItem

    private(set) weak var textView: NSTextView?
    /// Categories exposed, in rotor order. Captures live in `ShellModel`
    /// (the anchor), not in the text view, so they are not here.
    static let categories: [Category] = [.headings, .environments, .diagnostics]

    /// The diagnostic marks the owner draws (`SourceEditorView`'s painter),
    /// asked when the Diagnostics rotor is searched; a bare text view has none.
    var marks: () -> [EditorDiagnostics.Mark]

    /// Text-derived items per category, valid for one `(edit generation, length)`
    /// of the storage, with the model they came from (diagnostics reuse its lines).
    private var cache: (generation: Int, length: Int, model: AccessibleEditorModel, items: [Category: [Item]])?
    /// Diagnostics items for one `(edit generation, marks)` pair.
    private var diagnosticsCache: (generation: Int, marks: [EditorDiagnostics.Mark], items: [Item])?
    private var generation = 0
    private var storageObserver: NSObjectProtocol?
    /// Evidence for tests: how many times the text model was rebuilt.
    private(set) var rebuilds = 0

    init(textView: NSTextView, marks: @escaping () -> [EditorDiagnostics.Mark] = { [] }) {
        self.textView = textView
        self.marks = marks
        super.init()
        rotors = Self.categories.map { category in
            let rotor = category == .headings
                ? NSAccessibilityCustomRotor(rotorType: .heading, itemSearchDelegate: self)
                : NSAccessibilityCustomRotor(label: category.title, itemSearchDelegate: self)
            rotorCategories[ObjectIdentifier(rotor)] = category
            return rotor
        }
        if let storage = textView.textStorage {
            storageObserver = NotificationCenter.default.addObserver(
                forName: NSTextStorage.didProcessEditingNotification, object: storage, queue: nil
            ) { [weak self] note in
                guard let storage = note.object as? NSTextStorage, storage.editedMask.contains(.editedCharacters) else { return }
                MainActor.assumeIsolated { self?.invalidate() }
            }
        }
    }

    deinit {
        if let storageObserver { NotificationCenter.default.removeObserver(storageObserver) }
    }

    /// Drops the cached items (an edit happened, or the owner replaced the text).
    func invalidate() { generation += 1 }

    /// The rotors AppKit returns from `accessibilityCustomRotors`. The
    /// headings rotor uses the built-in `.heading` type so VoiceOver lists it
    /// under its own Headings rotor; the others are custom-label rotors.
    private(set) var rotors: [NSAccessibilityCustomRotor] = []
    private var rotorCategories: [ObjectIdentifier: Category] = [:]

    func category(of rotor: NSAccessibilityCustomRotor) -> Category? { rotorCategories[ObjectIdentifier(rotor)] }

    /// Items of `category` for the current text (cached until the next edit;
    /// diagnostics also until the marks change).
    func items(_ category: Category) -> [Item] {
        guard let textView else { return [] }
        let length = textView.textStorage?.length ?? (textView.string as NSString).length
        if cache == nil || cache?.generation != generation || cache?.length != length {
            // One model per generation: every category comes from the same text.
            let model = AccessibleEditorModel(text: SourceEditorView.nativeText(of: textView))
            var items: [Category: [Item]] = [:]
            for c in Self.categories where c != .diagnostics { items[c] = model.rotorItems(c) }
            rebuilds += 1
            cache = (generation, length, model, items)
        }
        guard let cache else { return [] }
        if category == .diagnostics { return diagnosticItems(model: cache.model) }
        return cache.items[category] ?? []
    }

    /// "Error at line 3: message — recovery: …" per mark, in document order;
    /// marks outside the current text are left out (the model drops them).
    private func diagnosticItems(model: AccessibleEditorModel) -> [Item] {
        let current = marks()
        if let d = diagnosticsCache, d.generation == generation, d.marks == current { return d.items }
        var withMarks = model
        withMarks.marks = current.map {
            AccessibleEditorModel.Mark(nsRange: $0.nsRange, severity: $0.severity, message: $0.message, recovery: $0.recovery,
                                       spokenSeverity: EditorCaretDiagnostics.spokenSeverity($0))
        }
        let items = withMarks.rotorItems(.diagnostics)
        diagnosticsCache = (generation, current, items)
        return items
    }

    /// Where a search starts, as AppKit describes it: nil for "from the
    /// first/last item, inclusive"; `.character(offset)` for a current item
    /// with a real range (strictly after/before it).
    enum Start: Equatable { case fromEnds, character(Int) }

    static func start(of parameters: NSAccessibilityCustomRotor.SearchParameters) -> Start {
        guard let current = parameters.currentItem else { return .fromEnds }
        let range = current.targetRange
        if range.location == NSNotFound { return .fromEnds }
        return .character(range.location)
    }

    /// Pure search over sorted `items` (document order). `filter` empty matches all.
    static func resolve(items: [Item], start: Start, forward: Bool, filter: String) -> Item? {
        let matching = filter.isEmpty ? items : items.filter { $0.label.localizedCaseInsensitiveContains(filter) }
        switch (start, forward) {
        case (.fromEnds, true): return matching.first
        case (.fromEnds, false): return matching.last
        case (.character(let at), true): return matching.first { $0.utf16.location > at }
        case (.character(let at), false): return matching.last { $0.utf16.location < at }
        }
    }

    func result(for item: Item) -> NSAccessibilityCustomRotor.ItemResult? {
        guard let textView else { return nil }
        let result = NSAccessibilityCustomRotor.ItemResult(targetElement: textView)
        result.targetRange = item.utf16
        result.customLabel = item.label
        return result
    }

    nonisolated func rotor(_ rotor: NSAccessibilityCustomRotor,
                           resultFor searchParameters: NSAccessibilityCustomRotor.SearchParameters) -> NSAccessibilityCustomRotor.ItemResult? {
        // AppKit calls this on the main thread. `ItemResult` is not Sendable,
        // so it leaves the isolated closure through a local rather than as
        // the closure's value (which Swift 6 rejects).
        nonisolated(unsafe) var found: NSAccessibilityCustomRotor.ItemResult?
        MainActor.assumeIsolated {
            guard let category = category(of: rotor) else { return }
            let item = Self.resolve(items: items(category), start: Self.start(of: searchParameters),
                                    forward: searchParameters.searchDirection == .next,
                                    filter: searchParameters.filterString)
            found = item.flatMap(result(for:))
        }
        return found
    }
}

// MARK: - Diagnostics under the insertion point

/// What VoiceOver hears about the diagnostic marks the caret is on: the
/// "Line L, column C" announcement gains "; Error: message" when the caret
/// lands on an underline (an author who cannot see the squiggle otherwise
/// learns of it only from the Problems list), and the text view's custom
/// content (`AXCustomContentProvider`, read with VO-⌘-/ or automatically for
/// high-importance entries) carries the message, its recovery line and the
/// catalogue explanation while the caret stays there. Pure over the marks
/// the editor already draws; nothing here touches the buffer.
enum EditorCaretDiagnostics {
    /// Marks whose range contains `caret` — either end inclusive, so a caret
    /// just after an underlined `\foo` still counts — in document order.
    static func marks(at caret: Int, in marks: [EditorDiagnostics.Mark]) -> [EditorDiagnostics.Mark] {
        marks.filter { $0.nsRange.location <= caret && caret <= NSMaxRange($0.nsRange) }
            .sorted { $0.nsRange.location < $1.nsRange.location }
    }

    /// "Error", "Warning", or "Not implemented" for a FlashTeX gap (the same
    /// rule the painter uses to pick the grey underline).
    static func spokenSeverity(_ mark: EditorDiagnostics.Mark) -> String {
        if EditorDiagnostics.isGap(mark.message) { return DiagnosticRowAccessibility.gapWord }
        return mark.severity == .error ? "Error" : "Warning"
    }

    /// "; Error: message; Warning: message" for the marks under the caret; empty when none.
    static func announcementSuffix(_ here: [EditorDiagnostics.Mark]) -> String {
        here.map { "; " + spokenSeverity($0) + ": " + $0.message }.joined()
    }

    /// One high-importance entry per mark ("Error: message"), then its
    /// recovery line and explanation as ordinary entries.
    static func customContent(_ here: [EditorDiagnostics.Mark]) -> [AXCustomContent] {
        here.flatMap { mark -> [AXCustomContent] in
            let main = AXCustomContent(label: spokenSeverity(mark), value: mark.message)
            main.importance = .high
            var out = [main]
            if let line = mark.recoveryLine { out.append(AXCustomContent(label: "Recovery", value: line)) }
            if let explanation = mark.explanation, !explanation.isEmpty {
                out.append(AXCustomContent(label: "Explanation", value: explanation))
            }
            return out
        }
    }
}

extension CompletingTextView: AXCustomContentProvider {
    /// The diagnostics at the insertion point (`diagnosticMarks`), computed
    /// when an assistive client asks; nothing to store.
    var accessibilityCustomContent: [AXCustomContent]! {
        get { EditorCaretDiagnostics.customContent(EditorCaretDiagnostics.marks(at: selectedRange().location, in: diagnosticMarks())) }
        set {}
    }
}
