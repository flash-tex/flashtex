import AppKit
import SwiftUI

/// The single title-bar control row (owner feedback on #653): every resting
/// control lives in the bar that holds the traffic lights — Compile and the
/// command palette leading, the preview controls that used to be the preview
/// header's own row (dark pages, fit width/page, zoom) trailing over the
/// preview column, then Open display list / Export PDF and the Captures
/// inspector toggle. Icons only; tooltips carry the names and shortcuts.
///
/// Drawn as content, IntelliJ-fashion, not as `NSToolbarItem`s: macOS 26
/// floats every toolbar item on a Liquid Glass platter (`NSToolbarPlatterView`
/// renders the capsule as its own layer contents, past any public API), and
/// the owner's direction is flat JetBrains/Android Studio buttons on one
/// chrome surface. The window keeps an empty `NSToolbar` purely so AppKit
/// gives the title bar its unified-compact height and centres the traffic
/// lights in it (WindowChrome.swift); this row occupies exactly that region
/// and `TitleBarDragSurface` keeps it dragging like a real title bar.
///
/// Every action here is an existing model operation with its menu, shortcut
/// and palette entry intact; nothing in this row is a new behaviour.
struct TitleBarRow: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        HStack(spacing: DS.Space.xs) {
            CompileTitleBarButton()
            TitleBarButton(icon: "command", label: "Command palette",
                           help: "Command Palette… (View, ⌘⇧P): every command with its shortcut",
                           identifier: "toolbar.command-palette") { $0.commandPaletteShown = true }
            Spacer(minLength: DS.Space.m)
            DarkPreviewTitleBarToggle()
            TitleBarButton(icon: "arrow.left.and.right.square", label: "Fit width",
                           help: "Fit Width (⌘9)") { $0.previewFitWidth() }
            TitleBarButton(icon: "arrow.up.and.down.square", label: "Fit page",
                           help: "Fit Page (⌘⇧9)") { $0.previewFitPage() }
            TitleBarButton(icon: "minus.magnifyingglass", label: "Zoom out preview",
                           help: "Zoom Out (⌘-)") { $0.previewZoomOut() }
            TitleBarButton(icon: "plus.magnifyingglass", label: "Zoom in preview",
                           help: "Zoom In (⌘=)") { $0.previewZoomIn() }
            Spacer(minLength: DS.Space.m).frame(maxWidth: DS.Space.xxl)
            TitleBarButton(icon: "folder", label: "Open display list",
                           help: "Open Display List (v2)… (File menu): a flashtex-render --v2 JSON file for the v2 preview pane") { $0.openDisplayListV2Panel() }
            ExportTitleBarMenu()
            CapturesTitleBarToggle()
        }
        .padding(.leading, DS.Layout.trafficLightClearance)
        .padding(.trailing, DS.Space.m)
        .frame(height: DS.Layout.titleBarHeight)
        .frame(maxWidth: .infinity)
        .background { TitleBarDragSurface() } // empty areas drag the window; double-click follows the system titlebar action
        .background(DS.Colors.surfaceSecondary)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Toolbar")
    }
}

/// Fills the custom title-bar row behind its controls: pressing an empty
/// area moves the window exactly like a stock title bar, and double-click
/// performs the system titlebar action (zoom, or what the user configured).
private struct TitleBarDragSurface: NSViewRepresentable {
    final class DragView: NSView {
        override var mouseDownCanMoveWindow: Bool { true }

        override func mouseUp(with event: NSEvent) {
            if event.clickCount == 2, let window {
                switch UserDefaults.standard.string(forKey: "AppleActionOnDoubleClick") {
                case "Minimize": window.performMiniaturize(nil)
                case "None": break
                default: window.performZoom(nil)
                }
            }
            super.mouseUp(with: event)
        }
    }

    func makeNSView(context: Context) -> DragView { DragView() }
    func updateNSView(_ view: DragView, context: Context) {}
}

// MARK: - the JetBrains title-bar button

/// A plain single-action title-bar button.
private struct TitleBarButton: View {
    let icon: String
    let label: String
    let help: String
    var identifier: String?
    let action: @MainActor (ShellModel) -> Void
    @Environment(ShellModel.self) var model
    @State private var hovering = false

    var body: some View {
        Button { action(model) } label: {
            IconButtonLabel(icon: icon, hovering: hovering)
        }
        .buttonStyle(PressableStyle())
        .onHover { hovering = $0 }
        .help(help)
        .accessibilityLabel(label)
        .accessibilityIdentifier(identifier ?? "toolbar.\(icon)")
    }
}

/// The Compile button: a plain single-click action (⌘B) drawn as the
/// JetBrains run-green triangle — the one deliberate colour accent in the
/// title bar. No menu here (owner + review on #653): the press-and-hold
/// `primaryAction:` menu was undiscoverable, and its contents — attaching
/// compiler/render-pipeline executables, reloading fixtures — are a
/// developer harness that stays in the File menu and the command palette.
/// Auto-compile moved to Settings > Compile (EditorPreferences.swift).
private struct CompileTitleBarButton: View {
    @Environment(ShellModel.self) var model
    @State private var hovering = false

    var body: some View {
        Button {
            if !model.outputBoundExplicitRetry() { model.compile() }
        } label: {
            Image(systemName: "play.fill")
                .font(DS.Fonts.toolbarIcon)
                .foregroundStyle(DS.Colors.runAccent)
                .frame(width: DS.Size.toolbarButton, height: DS.Size.toolbarButton)
                .background(hovering ? DS.Colors.hover : .clear,
                            in: RoundedRectangle(cornerRadius: DS.Radius.tab))
                .contentShape(Rectangle())
        }
        .buttonStyle(PressableStyle())
        .onHover { hovering = $0 }
        // `producerSummary`, not `workerStatus`: the title bar must not re-evaluate per request (ShellModel toolbar mirrors).
        .help("Compile (File > Compile, ⌘B) — producer: " + (model.isFixture ? "fixture (not a real compile)" : model.producerSummary))
        .accessibilityLabel("Compile")
        .accessibilityIdentifier("toolbar.compile")
    }
}

private struct DarkPreviewTitleBarToggle: View {
    @Environment(ShellModel.self) var model
    @State private var hovering = false

    var body: some View {
        @Bindable var model = model
        Toggle(isOn: $model.darkPreview) {
            IconButtonLabel(icon: "moon", on: model.darkPreview, hovering: hovering)
        }
        .toggleStyle(.button).buttonStyle(PressableStyle())
        .onHover { hovering = $0 }
        .help("Draw the preview pages dark (page and text colors only)")
        .accessibilityLabel("Dark preview")
    }
}

/// Every export route in one menu — a plain menu that opens on a single
/// normal click (never a press-and-hold `primaryAction:` pattern).
private struct ExportTitleBarMenu: View {
    @Environment(ShellModel.self) var model
    @State private var hovering = false

    var body: some View {
        // The glyph is drawn by `IconButtonLabel` and the menu opens from an
        // invisible label above it: `.borderlessButton` menus paint their
        // label in the primary label colour, which broke the quiet secondary
        // tint every sibling icon carries.
        IconButtonLabel(icon: "square.and.arrow.up", hovering: hovering)
            .overlay {
                Menu {
                    Button("Export PDF…") { model.exportPDF() }.disabled(!model.toolbarHasResult)
                    Button("Export PDF via Rust Writer…") { model.exportPDFViaRust() }.disabled(!model.toolbarHasResult)
                    Button("Export PDF (exact, v2)…") { model.exportPDFExact() }.disabled(!model.toolbarHasV2Frame)
                    Button("Export PDF (v2)…") { model.exportPDFV2() }.disabled(!model.toolbarHasV2Frame)
                } label: {
                    Color.clear.frame(width: DS.Size.toolbarButton, height: DS.Size.toolbarButton)
                        .contentShape(Rectangle())
                }
                .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
            }
            .onHover { hovering = $0 }
        .help("Export PDF… (⌘⇧E), via Rust writer (⌘⌥E), exact from the v2 display list, or the v2 pane's own writer")
        .accessibilityLabel("Export PDF")
    }
}

private struct CapturesTitleBarToggle: View {
    @Environment(ShellModel.self) var model
    @State private var hovering = false

    var body: some View {
        @Bindable var model = model
        let n = model.captureInbox.items.count
        Toggle(isOn: $model.captureInboxVisible) {
            IconButtonLabel(icon: "tray", on: model.captureInboxVisible, hovering: hovering)
                .overlay(alignment: .topTrailing) {
                    if n > 0 {
                        Text("\(n)")
                            .font(DS.Fonts.header)
                            .foregroundStyle(.white)
                            .padding(.horizontal, DS.Space.xs)
                            .background(DS.Colors.accentSelection, in: Capsule())
                            .accessibilityHidden(true)
                    }
                }
        }
        .toggleStyle(.button).buttonStyle(PressableStyle())
        .onHover { hovering = $0 }
        .help("Show or hide the Captures inspector (View, ⌘⇧I): captures from the iPad, their proposals, Insert at caret")
        .accessibilityLabel(n > 0 ? "Captures, \(n) item\(n == 1 ? "" : "s")" : "Captures")
        .accessibilityIdentifier("toolbar.captures")
    }
}
