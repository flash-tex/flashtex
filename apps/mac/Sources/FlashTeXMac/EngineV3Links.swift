import Foundation
import FlashTeXDisplayListV3

/// Hyperref links in the engine-v3 preview (display-list-v3 §4.5): the
/// page's `LINKS` and `DESTS`, which are pdfTeX's own link annotations and
/// destinations with their final rectangles. The v2 pane's rules
/// (`DisplayListLinks`) carry over: the last link under the point wins,
/// only http, https and mailto URIs open, and an internal destination
/// scrolls the preview. Pure apart from `DisplayListLinks.openURL`.
enum EngineV3Links {
    /// Scaled points per big point (page space is sp, y down from the top).
    static let spPerBP = 65781.76

    /// A link's rectangle in page points, y down (it includes the link margin).
    static func rect(_ link: DL3Link) -> CGRect {
        guard link.rect.count == 4 else { return .null }
        let l = Double(link.rect[0]) / spPerBP, t = Double(link.rect[1]) / spPerBP
        let r = Double(link.rect[2]) / spPerBP, b = Double(link.rect[3]) / spPerBP
        return CGRect(x: min(l, r), y: min(t, b), width: abs(r - l), height: abs(b - t))
    }

    /// The link under `point` (page points, y down); the last one wins, as in v2.
    static func hit(_ page: DL3Page, at point: CGPoint) -> DL3Link? {
        page.links.last { rect($0).contains(point) }
    }

    enum Action: Equatable {
        case openURI(URL)
        /// Scroll the preview to this place (page index, page points, y down).
        case reveal(CaretFollow.Target)
        case rejectedScheme(String)
        case unknownDestination(String)
        /// A link the preview does not follow (another file, a raw action, a thread).
        case unsupported(String)
    }

    /// What a click on `link` does; `pages` are the pages the session holds.
    static func action(for link: DL3Link, pages: [Int: DL3Page]) -> Action {
        let data = link.dataString
        if !link.file.isEmpty {
            return .unsupported("link to another file (\(String(decoding: link.file, as: UTF8.self)))")
        }
        switch link.kind {
        case 4:
            if let url = DisplayListLinks.allowedURL(from: data) { return .openURI(url) }
            return .rejectedScheme(URL(string: data)?.scheme ?? "")
        case 1, 2:
            let named = link.kind == 1
            for i in pages.keys.sorted() {
                if let d = pages[i]?.dests.first(where: { $0.named == named && $0.name == link.data }) {
                    return .reveal(target(of: d, page: i))
                }
            }
            return .unknownDestination(data)
        case 3:
            // "N spec": pdfTeX's `goto page N` counts pages from 1.
            guard let n = data.split(separator: " ").first.flatMap({ Int($0) }), n >= 1 else {
                return .unknownDestination(data)
            }
            return .reveal(CaretFollow.Target(page: n - 1, rect: CGRect(x: 0, y: 0, width: 1, height: 12)))
        case 6: return .unsupported("article thread")
        default: return .unsupported("raw link action")
        }
    }

    /// Where a destination puts the view: its left and top for the kinds
    /// that set them (xyz, fith, fitv, fitbh, fitbv, fitr), else the page top.
    /// A dest carries only the words pdfTeX writes for its kind, the others
    /// 0 (display-list-v3 §4.5): `rect[0]`/`rect[1]` are its left and top,
    /// except a fitr's, whose rectangle's corners may come either way round.
    static func target(of d: DL3Dest, page: Int) -> CaretFollow.Target {
        let usesLeft: Set<UInt8> = [0, 3, 6, 7], usesTop: Set<UInt8> = [0, 2, 5, 7]
        let r = d.rect.count == 4 ? d.rect : [0, 0, 0, 0]
        let (left, top) = d.kind == 7 ? (min(r[0], r[2]), min(r[1], r[3])) : (r[0], r[1])
        let x = usesLeft.contains(d.kind) ? Double(left) / spPerBP : 0
        let y = usesTop.contains(d.kind) ? Double(top) / spPerBP : 0
        // As v2's `previewTarget`: a 1 × 12 pt mark at the destination.
        return CaretFollow.Target(page: page, rect: CGRect(x: max(0, x), y: max(0, y), width: 1, height: 12))
    }

    /// The hover tooltip: the URI, the destination's name, or the page.
    static func tooltip(for link: DL3Link) -> String {
        switch link.kind {
        case 3: return "Page " + (link.dataString.split(separator: " ").first.map(String.init) ?? "?")
        default: return link.dataString
        }
    }
}

extension ShellModel {
    /// A click on an engine-v3 link: the v2 pane's `activatePreviewLink`
    /// policy and notes. Allowlisted URIs open through `NSWorkspace`; the
    /// pane scrolls to a `.reveal` itself.
    @discardableResult
    func activateEngineV3Link(_ link: DL3Link, pages: [Int: DL3Page]) -> EngineV3Links.Action {
        let action = EngineV3Links.action(for: link, pages: pages)
        switch action {
        case .openURI(let url):
            navigationNote = DisplayListLinks.openURL(url) ? "Opened \(url.absoluteString)" : "Could not open \(url.absoluteString)"
        case .reveal(let target):
            navigationNote = "Scrolled to page \(target.page + 1)"
        case .rejectedScheme(let scheme):
            navigationNote = "Blocked link scheme ‘\(scheme.isEmpty ? "none" : scheme)’ (allowed: http, https, mailto)"
        case .unknownDestination(let name):
            navigationNote = "Unknown destination \(name)"
        case .unsupported(let what):
            navigationNote = "The preview does not follow this link (\(what))"
        }
        return action
    }
}
