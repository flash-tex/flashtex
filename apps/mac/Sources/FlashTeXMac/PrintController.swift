import AppKit
import PDFKit
import FlashTeXProtocol

/// Builds print operations for File > Print… (compiled PDF) and File > Print Source…
/// (editor text). PDF bytes come from `PDFExport.render` — the same path as
/// Export PDF… — so what prints is exactly what would be exported. Operations
/// are constructed without running them; tests assert page count, paper size
/// and job title. The live editor is never the printed view.
@MainActor
enum PrintController {
    /// A built `NSPrintOperation` plus the values tests can assert without
    /// showing a panel or sending a job.
    struct Prepared {
        var operation: NSPrintOperation
        var pageCount: Int
        var paperSize: NSSize
        var jobTitle: String
        /// Editor buffer used for Print Source; nil for document PDF print.
        var text: String?
        /// Bytes `PDFExport.render` produced; nil for Print Source.
        var pdfData: Data?
        /// Keeps the PDF document or off-screen text view alive with the operation.
        fileprivate var keepAlive: AnyObject
    }

    enum Outcome {
        case ready(Prepared)
        case refused(String)
    }

    /// Display name used as the print job title (the open document's file name).
    static func documentName(from model: ShellModel) -> String {
        if let url = model.documentURL { return url.lastPathComponent }
        return URL(fileURLWithPath: model.activePath).lastPathComponent
    }

    /// Menu enablement for File > Print… (the function the File menu calls).
    /// Reads only change-only mirrors — do not read `result` from the App scene.
    /// A `.failed` result or an empty `pages` array is not printable:
    /// `PDFExport.render` would emit a blank letter page. Export PDF… still
    /// uses `toolbarHasResult`; Print is stricter. `retainedMarks` is the
    /// diagnostics last-good snapshot and is not a print source (Export does
    /// not fall back to it either).
    static func documentEnabled(_ model: ShellModel) -> Bool {
        documentEnabled(hasResult: model.toolbarHasResult, status: model.resultStatus, pageCount: model.toolbarPageCount)
    }

    /// Shared by the File-menu function and `printableResult` (same file).
    fileprivate static func documentEnabled(hasResult: Bool, status: RuntimeV1.Status?, pageCount: Int) -> Bool {
        hasResult && status != .failed && pageCount > 0
    }

    /// Tooltip for File > Print…; names why the item is disabled.
    static func documentHelp(_ model: ShellModel) -> String {
        documentHelp(hasResult: model.toolbarHasResult, status: model.resultStatus, pageCount: model.toolbarPageCount)
    }

    fileprivate static func documentHelp(hasResult: Bool, status: RuntimeV1.Status?, pageCount: Int) -> String {
        if !hasResult {
            return "Nothing to print: no compiled PDF (compile the document first)."
        }
        if status == .failed || pageCount == 0 {
            return "Compile failed — nothing to print"
        }
        return "Print the compiled document PDF (⌘P); same bytes as Export PDF…"
    }

    /// Menu enablement for File > Print Source… (the function the File menu calls).
    static func sourceEnabled(_ model: ShellModel) -> Bool { model.toolbarHasDocument }

    /// Tooltip for File > Print Source….
    static func sourceHelp(_ model: ShellModel) -> String {
        sourceEnabled(model)
            ? "Print the editor text with line numbers (monospaced); the live editor is not used"
            : "Nothing to print: no document is open."
    }

    /// The same refusal Export PDF… would store in `captureNote`, or the result
    /// it would render. Print uses this so a stale editor still prints the last
    /// applied compile (never an in-flight partial) and a historical preview
    /// refuses instead of silently printing the older snapshot as current.
    /// Failed / empty-page results are refused here too (not only in the menu).
    static func printableResult(from model: ShellModel) -> Outcome {
        if let why = model.historicalRefusal(of: "print") { return .refused(why) }
        guard let result = model.result else {
            return .refused("Nothing to print: no compile result loaded.")
        }
        if result.pages.isEmpty, model.v1PagesElided {
            return .refused("The v1 layout pages were elided for the v2 pane (display-list-v2-only); use Export Exact PDF, or switch the v2 pane off and recompile.")
        }
        if !documentEnabled(hasResult: true, status: result.status, pageCount: result.pages.count) {
            return .refused("Compile failed — nothing to print")
        }
        guard let prepared = prepareDocument(result: result, jobTitle: documentName(from: model)) else {
            return .refused("PDF print failed: the compiled result did not produce a printable document.")
        }
        return .ready(prepared)
    }

    /// True when Export PDF… would open the save panel (shared `exportPDFRefusal`).
    static func exportWouldProceed(_ model: ShellModel) -> Bool {
        model.exportPDFRefusal() == nil
    }

    static func makeDocumentPrint(from model: ShellModel, showsPrintPanel: Bool = false) -> Outcome {
        switch printableResult(from: model) {
        case .refused(let why): return .refused(why)
        case .ready(let prepared):
            prepared.operation.showsPrintPanel = showsPrintPanel
            prepared.operation.showsProgressPanel = showsPrintPanel
            return .ready(prepared)
        }
    }

    static func makeSourcePrint(from model: ShellModel, showsPrintPanel: Bool = false) -> Outcome {
        guard !model.documents.isEmpty else {
            return .refused("Nothing to print: no document is open.")
        }
        return .ready(prepareSource(text: model.activeText, jobTitle: documentName(from: model),
                                    showsPrintPanel: showsPrintPanel))
    }

    /// Builds a PDFKit print operation from the compiled result. Does not run it.
    static func prepareDocument(result: RuntimeV1.CompileResult, jobTitle: String,
                                showsPrintPanel: Bool = false) -> Prepared? {
        let data = PDFExport.render(result, dark: false)
        guard let pdf = PDFDocument(data: data), pdf.pageCount > 0 else { return nil }
        let paper: NSSize
        if let page = pdf.page(at: 0) {
            paper = page.bounds(for: .mediaBox).size
        } else {
            paper = NSSize(width: 612, height: 792)
        }
        let info = NSPrintInfo()
        info.paperSize = paper
        info.orientation = paper.width > paper.height ? NSPrintInfo.PaperOrientation.landscape : .portrait
        info.leftMargin = 0
        info.rightMargin = 0
        info.topMargin = 0
        info.bottomMargin = 0
        info.horizontalPagination = .fit
        info.verticalPagination = .fit
        // PDFKit: `printOperationForPrintInfo:scalingMode:autoRotate:` — page
        // size follows the PDF (`pageScaleNone`); the system panel is optional.
        guard let operation = pdf.printOperation(for: info, scalingMode: .pageScaleNone, autoRotate: true) else { return nil }
        operation.jobTitle = jobTitle
        operation.showsPrintPanel = showsPrintPanel
        operation.showsProgressPanel = showsPrintPanel
        operation.printInfo.paperSize = paper
        let owner = DocumentPrintOwner(pdf)
        return Prepared(operation: operation, pageCount: pdf.pageCount, paperSize: paper,
                        jobTitle: jobTitle, text: nil, pdfData: data, keepAlive: owner)
    }

    /// Builds an NSTextView print operation on a copy of `text` (line numbers in
    /// the left inset). Does not run it; the live editor is not referenced.
    static func prepareSource(text: String, jobTitle: String, showsPrintPanel: Bool = false) -> Prepared {
        let info = NSPrintInfo.shared.copy() as! NSPrintInfo
        info.horizontalPagination = .automatic
        info.verticalPagination = .automatic
        info.isHorizontallyCentered = false
        info.isVerticallyCentered = false
        let printableWidth = max(200, info.paperSize.width - info.leftMargin - info.rightMargin)
        let view = SourcePrintTextView(text: text, width: printableWidth)
        view.layoutForPrinting()
        let operation = NSPrintOperation(view: view, printInfo: info)
        operation.jobTitle = jobTitle
        operation.showsPrintPanel = showsPrintPanel
        operation.showsProgressPanel = showsPrintPanel
        let pageCount = max(1, Int(ceil(view.frame.height / max(info.paperSize.height - info.topMargin - info.bottomMargin, 1))))
        return Prepared(operation: operation, pageCount: pageCount, paperSize: info.paperSize,
                        jobTitle: jobTitle, text: view.string, pdfData: nil, keepAlive: view)
    }
}

extension ShellModel {
    /// `File > Print…` (⌘P): the compiled document PDF, same bytes as Export PDF….
    func printDocument() {
        switch PrintController.makeDocumentPrint(from: self, showsPrintPanel: true) {
        case .refused(let why): captureNote = why
        case .ready(let prepared):
            _ = prepared.operation.run()
        }
    }

    /// `File > Print Source…`: editor text with line numbers, from a copy.
    func printSource() {
        switch PrintController.makeSourcePrint(from: self, showsPrintPanel: true) {
        case .refused(let why): captureNote = why
        case .ready(let prepared):
            _ = prepared.operation.run()
        }
    }
}

private final class DocumentPrintOwner {
    let pdf: PDFDocument
    init(_ pdf: PDFDocument) { self.pdf = pdf }
}

/// Off-screen monospaced text view used only for Print Source. Line numbers are
/// drawn in the left inset so `string` stays equal to the editor buffer.
private final class SourcePrintTextView: NSTextView {
    static let gutter: CGFloat = 36
    static let fontSize: CGFloat = 11

    init(text: String, width: CGFloat) {
        let storage = NSTextStorage(string: text)
        let manager = NSLayoutManager()
        storage.addLayoutManager(manager)
        let container = NSTextContainer(size: NSSize(width: max(1, width - SourcePrintTextView.gutter), height: CGFloat.greatestFiniteMagnitude))
        container.widthTracksTextView = true
        container.lineFragmentPadding = 4
        manager.addTextContainer(container)
        super.init(frame: NSRect(x: 0, y: 0, width: width, height: 32), textContainer: container)
        string = text
        font = NSFont.monospacedSystemFont(ofSize: Self.fontSize, weight: .regular)
        textColor = .textColor
        backgroundColor = .white
        drawsBackground = true
        isEditable = false
        isSelectable = false
        isRichText = false
        isHorizontallyResizable = false
        isVerticallyResizable = true
        textContainerInset = NSSize(width: Self.gutter / 2, height: 8)
        minSize = NSSize(width: width, height: 0)
        maxSize = NSSize(width: width, height: CGFloat.greatestFiniteMagnitude)
        textContainer?.containerSize = NSSize(width: max(1, width - Self.gutter), height: CGFloat.greatestFiniteMagnitude)
        textContainer?.widthTracksTextView = true
    }

    @available(*, unavailable) required init?(coder: NSCoder) { fatalError() }

    func layoutForPrinting() {
        guard let manager = layoutManager, let container = textContainer else { return }
        manager.ensureLayout(for: container)
        let used = manager.usedRect(for: container)
        let height = max(32, ceil(used.maxY + textContainerInset.height * 2 + 8))
        frame = NSRect(x: 0, y: 0, width: frame.width, height: height)
    }

    override var isFlipped: Bool { true }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        drawLineNumbers(in: dirtyRect)
    }

    private func drawLineNumbers(in dirtyRect: NSRect) {
        guard let manager = layoutManager, textContainer != nil else { return }
        let ns = string as NSString
        let font = NSFont.monospacedSystemFont(ofSize: Self.fontSize - 1, weight: .regular)
        let attrs: [NSAttributedString.Key: Any] = [
            .font: font,
            .foregroundColor: NSColor.secondaryLabelColor,
        ]
        let inset = textContainerInset
        var glyphIndex = 0
        var lineNumber = 1
        let glyphCount = manager.numberOfGlyphs
        if glyphCount == 0 {
            let label = "1" as NSString
            label.draw(at: NSPoint(x: 6, y: inset.height), withAttributes: attrs)
            return
        }
        while glyphIndex < glyphCount {
            var lineRange = NSRange()
            let rect = manager.lineFragmentRect(forGlyphAt: glyphIndex, effectiveRange: &lineRange)
            let charIndex = manager.characterIndexForGlyph(at: glyphIndex)
            let isLineStart = charIndex == 0 || ns.character(at: charIndex - 1) == 0x0a
            if isLineStart {
                let label = "\(lineNumber)" as NSString
                let size = label.size(withAttributes: attrs)
                let y = rect.minY + inset.height + max(0, (rect.height - size.height) / 2)
                if NSIntersectsRect(dirtyRect, NSRect(x: 0, y: y, width: Self.gutter, height: size.height)) {
                    label.draw(at: NSPoint(x: Self.gutter - 8 - size.width + inset.width, y: y), withAttributes: attrs)
                }
                lineNumber += 1
            }
            glyphIndex = NSMaxRange(lineRange)
        }
        if ns.length > 0, ns.character(at: ns.length - 1) == 0x0a {
            let extra = manager.extraLineFragmentRect
            if extra.height > 0 {
                let label = "\(lineNumber)" as NSString
                let size = label.size(withAttributes: attrs)
                label.draw(at: NSPoint(x: Self.gutter - 8 - size.width + inset.width,
                                       y: extra.minY + inset.height), withAttributes: attrs)
            }
        }
    }
}
