import AppKit
import SwiftUI

/// Preview zoom (owner: mac-zoom). The preview panes fit the widest page to
/// the pane width; `ShellModel.previewZoom` multiplies that fit scale
/// (1.0 = fit width, clamped to 25 %…400 %) and is persisted in
/// `UserDefaults` under `FlashTeX.PreviewZoom.v1`. Pages wider than the pane
/// scroll horizontally; `PreviewAnchor` keeps the (page, fraction) under the
/// viewport's top edge across the zoom change.
enum PreviewZoom {
    static let range: ClosedRange<CGFloat> = 0.25...4
    /// Multiplicative step of Zoom In / Zoom Out and the header's −/+ buttons.
    static let step: CGFloat = 1.25
    static let storageKey = "FlashTeX.PreviewZoom.v1"

    static func clamped(_ zoom: CGFloat) -> CGFloat {
        guard zoom.isFinite, zoom > 0 else { return 1 }
        return min(max(zoom, range.lowerBound), range.upperBound)
    }

    /// Display scale: the pane's fit-to-width scale times the zoom multiplier.
    static func scale(fit: CGFloat, zoom: CGFloat) -> CGFloat { fit * clamped(zoom) }

    /// The zoom that makes a page render at 1 pt per screen point ("Actual Size"),
    /// clamped when a very narrow pane would require more than the 4x limit.
    static func actualSizeZoom(fit: CGFloat) -> CGFloat { fit > 0 ? clamped(1 / fit) : 1 }

    static func percent(fit: CGFloat, zoom: CGFloat) -> Int { Int((scale(fit: fit, zoom: zoom) * 100).rounded()) }

    static func load(_ defaults: UserDefaults) -> CGFloat {
        guard let raw = defaults.object(forKey: storageKey) as? Double else { return 1 }
        return clamped(CGFloat(raw))
    }

    static func store(_ zoom: CGFloat, in defaults: UserDefaults) {
        defaults.set(Double(clamped(zoom)), forKey: storageKey)
    }
}

/// Zoom percentage plus −/+ buttons for the preview header.
struct PreviewZoomControl: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        HStack(spacing: DS.Space.xxs) {
            Button { model.previewZoomOut() } label: { Image(systemName: "minus.magnifyingglass") }
                .help("Zoom Out (⌘-)").accessibilityLabel("Zoom out preview")
            Text("\(PreviewZoom.percent(fit: model.previewFitScale, zoom: model.previewZoom)) %")
                .font(DS.Fonts.monoSecondary).frame(minWidth: DS.Size.zoomReadoutMinWidth)
                .help("Preview zoom; ⌘0 actual size, ⌘9 fit width, or pinch on the preview")
                .accessibilityLabel("Preview zoom \(PreviewZoom.percent(fit: model.previewFitScale, zoom: model.previewZoom)) percent")
                .onTapGesture(count: 2) { model.previewFitWidth() }
            Button { model.previewZoomIn() } label: { Image(systemName: "plus.magnifyingglass") }
                .help("Zoom In (⌘=)").accessibilityLabel("Zoom in preview")
        }
        .buttonStyle(.borderless).controlSize(.small)
    }
}

/// Pinch on the preview multiplies the zoom; the base is captured when the gesture starts.
struct PreviewMagnify: ViewModifier {
    @Environment(ShellModel.self) var model
    @State private var base: CGFloat?

    func body(content: Content) -> some View {
        content.gesture(MagnificationGesture()
            .onChanged { value in
                let b = base ?? model.previewZoom
                if base == nil { base = b }
                model.previewZoom = PreviewZoom.clamped(b * value)
            }
            .onEnded { _ in base = nil })
    }
}

/// Pinch over the editor adjusts `EditorPreferences.fontSize` (the gutter and
/// highlighter read the font, so they follow). Installed as a local event
/// monitor because the editor's scroll view is created by `CompletingTextView`.
enum EditorFontMagnifier {
    @MainActor
    static func install(on scroll: NSScrollView) -> Any? {
        NSEvent.addLocalMonitorForEvents(matching: .magnify) { [weak scroll] event in
            guard let scroll, event.window === scroll.window else { return event }
            let p = scroll.convert(event.locationInWindow, from: nil)
            guard scroll.bounds.contains(p) else { return event }
            let prefs = EditorPreferences.shared
            prefs.fontSize = prefs.fontSize * (1 + Double(event.magnification))
            return nil
        }
    }
}

extension ShellModel {
    func previewZoomIn() { previewZoom = PreviewZoom.clamped(previewZoom * PreviewZoom.step) }
    func previewZoomOut() { previewZoom = PreviewZoom.clamped(previewZoom / PreviewZoom.step) }
    func previewActualSize() { previewZoom = PreviewZoom.actualSizeZoom(fit: previewFitScale) }
    func previewFitWidth() { previewZoom = 1 }
    func previewFitPage() { previewZoom = PreviewZoom.clamped(previewFitPageZoom) }

    static let editorFontStep: Double = 1
    func increaseEditorFontSize() { EditorPreferences.shared.fontSize += Self.editorFontStep }
    func decreaseEditorFontSize() { EditorPreferences.shared.fontSize -= Self.editorFontStep }
    func resetEditorFontSize() { EditorPreferences.shared.fontSize = EditorPreferences.defaultSnapshot.fontSize }
}
