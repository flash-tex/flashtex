import AppKit
import CoreGraphics
import CoreText
import FlashTeXProtocol

/// Renders a runtime-v1 `compile_result` to PDF with CoreGraphics.
///
/// This draws exactly the positioned items the compiler reported — one PDF page
/// per `pages` entry at `width_pt` × `height_pt`, each text item in a serif font
/// at `font_size_pt` with its baseline at `baseline_y_pt` from the top of the
/// page. It is not a TeX-engine PDF and carries no fonts, images, or links
/// beyond what the contract's text and rule items describe.
///
/// Typed `rule` items (negotiated `rules-v1`, bound to the result's accepted
/// set) are filled rectangles with `(x_pt, y_pt)` as the top-left corner. On the
/// legacy route only, U+2500 text runs are approximated as bars (`RuleConvention`).
/// Font hints (`font-hints-v1`) select the face; unresolvable families fall
/// back to Times and are reported by the shell as substitutions. Unknown item
/// kinds are not drawn here; the shell reports them on the negotiated route.
enum PDFExport {
    /// Font used for text items. PDF pages embed a subset of this font.
    static let fontName = "Times-Roman"

    static func render(_ result: RuntimeV1.CompileResult, dark: Bool = false) -> Data {
        let data = NSMutableData()
        guard let consumer = CGDataConsumer(data: data),
              let ctx = CGContext(consumer: consumer, mediaBox: nil, nil)
        else { return Data() }

        let background = dark ? CGColor(gray: 0.16, alpha: 1) : CGColor(gray: 1, alpha: 1)
        let foreground = dark ? CGColor(gray: 1, alpha: 1) : CGColor(gray: 0, alpha: 1)
        // The accepted set travels with the result (the shell verified it against
        // the request before applying), so the route is decided per result.
        let rulesNegotiated = result.layoutCapabilities?.contains(RuntimeV1.LayoutCapabilities.rulesV1) == true

        for page in result.pages {
            var mediaBox = CGRect(x: 0, y: 0, width: page.widthPt, height: page.heightPt)
            ctx.beginPDFPage([kCGPDFContextMediaBox as String: NSData(bytes: &mediaBox, length: MemoryLayout<CGRect>.size)] as CFDictionary)
            ctx.setFillColor(background)
            ctx.fill(mediaBox)
            draw(page, in: ctx, foreground: foreground, rulesNegotiated: rulesNegotiated)
            ctx.endPDFPage()
        }
        ctx.closePDF()
        return data as Data
    }

    /// Draws every text item with CoreText so the baseline lands exactly at
    /// `baseline_y_pt`. PDF space has a bottom-left origin, so y is flipped.
    private static func draw(_ page: RuntimeV1.Page, in ctx: CGContext, foreground: CGColor, rulesNegotiated: Bool) {
        ctx.textMatrix = .identity
        for item in page.items {
            if case .rule(let rule) = item {
                // Contract geometry: opaque, top-left anchored. Paint order = item order.
                ctx.setFillColor(foreground)
                ctx.fill(RuleGeometry.pdfRect(page: page, rule: rule))
                continue
            }
            guard case .text(let t) = item else { continue }
            if !rulesNegotiated, let r = RuleGeometry.legacyPDFRect(page: page, text: t) {
                ctx.setFillColor(foreground)
                ctx.fill(r)
                continue
            }
            let font = CTFontCreateWithName(PreviewFonts.resolve(hint: t.font, size: t.fontSizePt).postScriptName as CFString,
                                            t.fontSizePt, nil)
            let attributed = NSAttributedString(string: t.text, attributes: [
                .font: font,
                .foregroundColor: NSColor(cgColor: foreground) ?? .black,
            ])
            let line = CTLineCreateWithAttributedString(attributed)
            ctx.textPosition = CGPoint(x: t.xPt, y: page.heightPt - t.baselineYPt)
            CTLineDraw(line, ctx)
        }
    }
}

extension ShellModel {
    /// Why `File > Export PDF…` would refuse right now, or nil if it would open
    /// the save panel. Print… shares this predicate (`PrintController.exportWouldProceed`)
    /// so the two commands do not copy the historical / no-result / v1-elided guards.
    func exportPDFRefusal() -> String? {
        if let why = historicalRefusal(of: "export") { return why }
        guard let result else { return "Nothing to export: no compile result loaded." }
        if result.pages.isEmpty, v1PagesElided {
            return "The v1 layout pages were elided for the v2 pane (display-list-v2-only); use Export Exact PDF, or switch the v2 pane off and recompile."
        }
        return nil
    }

    /// `File > Export PDF…`: writes the current preview via `PDFExport.render`.
    /// Reports the saved path (or failure) in `captureNote`.
    func exportPDF() {
        if let why = exportPDFRefusal() { captureNote = why; return }
        guard let result else { return }
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.pdf]
        panel.nameFieldStringValue = "\(result.projectId)-r\(result.revision).pdf"
        panel.message = "Export the preview's reported layout as PDF (not a TeX-engine PDF)"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        do {
            // Dark preview is a viewing mode only; the exported document is always white.
            try PDFExport.render(result, dark: false).write(to: url, options: .atomic)
            captureNote = "Exported \(result.pages.count) page\(result.pages.count == 1 ? "" : "s") to \(url.path)"
        } catch {
            captureNote = "PDF export failed: \(error.localizedDescription)"
        }
    }
}
