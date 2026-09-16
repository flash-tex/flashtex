import AppKit
import Foundation
import FlashTeXProtocol

/// Mac consumer of `display-list-v2-links` (protocol/proposals/display-list-v2-links.md):
/// capability gating, page-tick hit-testing, and the URI scheme allowlist.
/// Pure aside from `openURL` (NSWorkspace); the NSView only asks these helpers.
enum DisplayListLinks {
    static let capability = RenderingV2.linksCapability
    /// Schemes `NSWorkspace` may open. `file:` and `javascript:` are rejected
    /// (the writer does not filter schemes; the preview does).
    static let allowedURISchemes: Set<String> = ["http", "https", "mailto"]

    /// Live frames honour `navigation` only when the producer echoed the
    /// capability (proposal §1). A list opened from disk has no echo: the
    /// object is used if present so fixtures stay clickable.
    static func effective(_ navigation: RenderingV2.Navigation?, accepted: [String], live: Bool) -> RenderingV2.Navigation? {
        guard let navigation else { return nil }
        if live, !accepted.contains(capability) { return nil }
        return navigation
    }

    /// `layout_capabilities` may list this only together with `display-list-v2`.
    /// Adding it is `setLiveV2` (mode set / configure_layout); this only
    /// strips a stray request that named links without v2.
    static func sent(with capabilities: [String]) -> [String] {
        guard capabilities.contains(V2Live.capability) else {
            return capabilities.filter { $0 != capability }
        }
        return capabilities
    }

    /// View point → page ticks (envelope `bp_2pow20`, y down). `scale` is
    /// points-on-screen per page point; `origin` is the page's top-left in
    /// the same view coordinates as `viewX`/`viewY`.
    static func ticks(viewX: Double, viewY: Double, scale: Double, originX: Double = 0, originY: Double = 0) -> (x: Int64, y: Int64) {
        let pageX = scale == 0 ? 0 : (viewX - originX) / scale
        let pageY = scale == 0 ? 0 : (viewY - originY) / scale
        return (Int64((pageX * Double(RenderingV2.ticksPerPoint)).rounded(.down)),
                Int64((pageY * Double(RenderingV2.ticksPerPoint)).rounded(.down)))
    }

    /// Last matching link on `page` wins (later line pieces / later entries).
    static func hit(_ navigation: RenderingV2.Navigation, page: Int, tickX: Int64, tickY: Int64) -> RenderingV2.Navigation.Link? {
        var found: RenderingV2.Navigation.Link?
        for link in navigation.links where link.page == page {
            if link.rects.contains(where: { $0.contains(x: tickX, y: tickY) }) { found = link }
        }
        return found
    }

    static func hit(_ navigation: RenderingV2.Navigation, page: Int, viewX: Double, viewY: Double, scale: Double, originX: Double = 0, originY: Double = 0) -> RenderingV2.Navigation.Link? {
        let t = ticks(viewX: viewX, viewY: viewY, scale: scale, originX: originX, originY: originY)
        return hit(navigation, page: page, tickX: t.x, tickY: t.y)
    }

    static func tooltip(for link: RenderingV2.Navigation.Link) -> String {
        switch link.target {
        case .uri(let u): return u
        case .destination(let d): return d
        }
    }

    /// Allowlisted `http`/`https`/`mailto` only. Scheme comparison is
    /// case-insensitive; a missing scheme is rejected.
    static func allowedURL(from uri: String) -> URL? {
        guard let url = URL(string: uri), let scheme = url.scheme?.lowercased(), allowedURISchemes.contains(scheme) else { return nil }
        return url
    }

    enum Action: Equatable {
        case openURI(URL)
        case reveal(RenderingV2.Navigation.Destination)
        case rejectedScheme(String)
        case unknownDestination(String)
    }

    static func action(for link: RenderingV2.Navigation.Link, destinations: [String: RenderingV2.Navigation.Destination]) -> Action {
        switch link.target {
        case .uri(let uri):
            if let url = allowedURL(from: uri) { return .openURI(url) }
            let scheme = URL(string: uri)?.scheme ?? ""
            return .rejectedScheme(scheme)
        case .destination(let name):
            if let dest = destinations[name] { return .reveal(dest) }
            return .unknownDestination(name)
        }
    }

    /// Replaceable so tests never call `NSWorkspace`.
    static var openURL: (URL) -> Bool = { NSWorkspace.shared.open($0) }

    static func previewTarget(for destination: RenderingV2.Navigation.Destination) -> CaretFollow.Target {
        let x = RenderingV2.points(destination.x)
        let y = RenderingV2.points(destination.y)
        // Proposal §4: destination y is approximate until the producer pins it.
        return CaretFollow.Target(page: destination.page, rect: CGRect(x: x, y: y, width: 1, height: 12))
    }
}
