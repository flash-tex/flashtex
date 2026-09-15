import SwiftUI
import FlashTeXProtocol
import FlashTeXAccessibility

/// The main window: an IntelliJ-style tool-window shell built flat
/// (design-principles §4) — a left icon rail that toggles the tool column
/// (WorkspaceSidebar.swift: Project tree, and the Outline, which ships
/// collapsed), the document tabs + editor and the preview side by side, the
/// Problems panel underneath (ProblemsPanel.swift), and a full-width status
/// bar. Regions are separated by hairlines and a slight tonal shift, never
/// by distinct region backgrounds. The single title bar carries every
/// resting control as pinned icon-only items — Compile and the palette
/// leading, the preview controls and Export trailing (IDEToolbar.swift;
/// owner direction on #653 supersedes the old three-chip budget). Every
/// action here is an existing model operation; View > Command Palette…
/// (⌘⇧P) lists every command of the accessibility table
/// (CommandPalette.swift).
struct ContentView: View {
    @Environment(ShellModel.self) var model
    @EnvironmentObject private var nearby: NearbyState
    /// Tool-window visibility (the rail toggles them). The Outline ships
    /// collapsed; the Project tree shows by default.
    @AppStorage("FlashTeX.workspace.projectVisible") private var projectVisible = true
    @AppStorage("FlashTeX.workspace.outlineVisible") private var outlineVisible = false

    var body: some View {
        @Bindable var model = model
        VStack(spacing: 0) {
            // The single title-bar control row, in the traffic lights' own
            // region (TitleBar.swift; the safe-area ignore below lets it
            // occupy the transparent title bar).
            TitleBarRow()
            HStack(spacing: 0) {
                ToolRail(projectVisible: $projectVisible, outlineVisible: $outlineVisible)
                // The structural splits are AppKit (WorkspaceSplit.swift):
                // sidebar | editor | preview over the Problems panel, with
                // real minimums, drag, persistence and the narrow-layout
                // collapse. Split state follows the workspace flags.
                WorkspaceSplitPane(nearby: nearby,
                                   projectVisible: projectVisible,
                                   outlineVisible: outlineVisible,
                                   problemsVisible: model.problemsVisible,
                                   narrowPreviewShown: model.narrowPreviewShown)
            }
            // The status bar spans the full width, rail included (VS Code),
            // with no separator above it (Islands: status bar border
            // transparent — the chrome tone alone separates it).
            StatusBar()
        }
        .ignoresSafeArea(.container, edges: .top) // the title-bar row occupies the transparent title bar region
        .background(WindowChromeConfigurator()) // transparent title bar, hidden title, height-only toolbar (WindowChrome.swift)
        .background(DS.Colors.surfaceSecondary.ignoresSafeArea()) // one chrome surface up into the title bar
        .modifier(HideToolbarBackground())
        .inspector(isPresented: $model.captureInboxVisible) { // Captures (CaptureInbox.swift): View > Captures, ⌘⇧I
            CaptureInboxPanel(inbox: model.captureInbox).inspectorColumnWidth(min: DS.Layout.inspectorMinWidth, ideal: DS.Layout.inspectorIdealWidth, max: DS.Layout.inspectorMaxWidth)
        }
        .sheet(isPresented: $model.commandPaletteShown) { CommandPalette().environment(model) }
        .modifier(EditorNavigationSheets()) // Rename / Wrap / Change Environment… / Go to Symbol… / Go to Line (ShellModel+EditorNavigation.swift)
    }
}

/// macOS 15's declarative half of the transparent title bar; the pre-15
/// fallback is `NSWindow.titlebarAppearsTransparent` (WindowChrome.swift),
/// which macOS 14 windows get from the same configurator.
private struct HideToolbarBackground: ViewModifier {
    func body(content: Content) -> some View {
        if #available(macOS 15, *) {
            content.toolbarBackgroundVisibility(.hidden, for: .windowToolbar)
        } else {
            content
        }
    }
}

/// The left icon rail (IntelliJ tool-window stripe, one notch calmer): one
/// icon per tool window, toggling it. Selected = accent icon on a selection
/// pill; the bottom group holds the bottom panel's toggle. Every button is a
/// real button with a spoken name and a shortcut in its tooltip.
private struct ToolRail: View {
    @Environment(ShellModel.self) var model
    @Binding var projectVisible: Bool
    @Binding var outlineVisible: Bool

    var body: some View {
        @Bindable var model = model
        VStack(spacing: DS.Space.s) {
            RailButton(icon: "folder", label: "Project", isOn: $projectVisible,
                       help: "Show or hide the project tree")
            RailButton(icon: "list.bullet.indent", label: "Outline", isOn: $outlineVisible,
                       help: "Show or hide the outline of the active document")
            Spacer()
            RailButton(icon: "exclamationmark.triangle", label: "Problems", isOn: $model.problemsVisible,
                       help: "Show or hide the Problems panel (⌘⇧M)")
        }
        .padding(.vertical, DS.Space.m)
        .frame(width: DS.Layout.railWidth)
        .frame(maxHeight: .infinity)
        .background(DS.Colors.surfaceSecondary)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Tool windows")
    }
}

private struct RailButton: View {
    let icon: String
    let label: String
    @Binding var isOn: Bool
    let help: String
    @State private var hovering = false

    var body: some View {
        Button { isOn.toggle() } label: {
            Image(systemName: icon)
                .font(DS.Fonts.railIcon)
                .foregroundStyle(isOn ? DS.Colors.accentSelection : DS.Colors.textSecondary)
                .frame(width: DS.Size.railButton, height: DS.Size.railButton)
                .background(
                    isOn ? DS.Colors.accentSelection.opacity(DS.State.badgeFillOpacity)
                         : hovering ? DS.Colors.textPrimary.opacity(DS.State.hoverOpacity) : .clear,
                    in: RoundedRectangle(cornerRadius: DS.Radius.tab))
        }
        .buttonStyle(PressableStyle())
        .onHover { hovering = $0 }
        .help(help)
        .accessibilityLabel(label)
        .accessibilityAddTraits(isOn ? .isSelected : [])
    }
}

// MARK: pieces
//
// Each pane is its own view reading the model from the environment, so
// `@Observable` tracking scopes invalidation: a keystroke (documents,
// editorRevision) re-evaluates EditorPane and the status bar; a compile
// result the status bar, PreviewPane and ProblemsPanel; bridge traffic the
// bridge bar only.

struct EditorPane: View {
    @Environment(ShellModel.self) var model
    /// Vim `:set nu` / `:set nonu` (VimMode.swift); the gutter is on by default.
    @State private var lineNumbers = true

    var body: some View {
        @Bindable var model = model
        VStack(spacing: 0) {
            // Switching goes through ProjectDocuments so each document's
            // caret/selection is kept and a pending insertion is never
            // applied to the wrong buffer (ProjectDocuments.swift).
            // No line under the strip: tab strip and editor share one
            // surface (Islands); the active tab's underline marks the edge.
            DocumentTabBar()
            SourceEditorView(
                text: Binding(get: { model.activeText }, set: { model.updateActiveText($0) }),
                selection: model.selection,
                pendingEdit: model.pendingEdit,
                marks: model.editorMarks,
                result: model.result,
                editorRevision: model.editorRevision,
                projectIndexMetadata: model.completionMetadata,
                projectFiles: model.documents.map(\.path), // `\input{` completion (Completion.swift)
                onCaretChange: { model.caretUTF16 = $0 },
                onSelectionChange: { if model.caretLengthUTF16 != $0.length { model.caretLengthUTF16 = $0.length } }, // every keystroke reports length 0; an equal write still invalidates its readers
                onEditApplied: { model.editApplied($0, newText: $1) },
                onEditRefused: { model.editRefused($0, reason: $1) },
                caretFix: model.caretFix, // the hint the editor draws, and the only thing Tab accepts
                onAcceptCaretFix: { model.acceptCaretFix() },
                onDismissCaretFix: { model.dismissCaretFix() },
                autoClosePairs: EditorPreferences.shared.autoCloseBraces ? model.autoClosePairs : [] // EditorPreferences.swift gates the braces lane set
                ,
                syntaxHighlighting: true, // SyntaxHighlighter.swift / EditorIntelligence.swift (mac-syntax-highlight)
                showLineNumbers: lineNumbers,
                onDefinitionRequest: { target in
                    switch target {
                    case .label, .citation: model.goToMatching() // caret already on the token
                    case .environment, .command: model.goToDefinition() // ShellModel+EditorNavigation.swift: \newcommand/\newenvironment first, else the matching \begin/\end
                    case .file(let path, _): Task { await model.project.openDocument(path, role: .opened) }
                    }
                },
                userDefinition: { model.definitionSummary(forCommand: $0) }, // hover peek of \newcommand bodies (EditorNavigation.swift)
                hoverContext: { model.editorHoverContext() }, // what \ref/\cite/\includegraphics resolve to (EditorHoverResolution.swift)
                mathPreviewContext: { // inline math hover preview (MathHoverPreview.swift)
                    model.displayListV2?.frame.map {
                        MathHoverPreview.Context(path: model.activePath, frame: $0, previewIsStale: model.previewIsStale, dark: model.darkPreview)
                    }
                },
                onExCommand: { command in // Vim `:` commands (VimMode.swift) mapped to the shell's own actions
                    switch command {
                    case .write: model.saveTexInteractive(); return nil
                    case .writeQuit: model.saveTexInteractive(); return closeActiveDocument(discardingEdits: false)
                    case .quit(let force): return closeActiveDocument(discardingEdits: force)
                    case .edit(let path): Task { await model.project.openDocument(path, role: .opened) }; return nil
                    case .setNumber(let on): lineNumbers = on; return nil
                    }
                }
            )
            // Vim's status line belongs to the window being edited, so it sits
            // directly under the source text — not in the window's status bar,
            // which the Problems panel pushes two panes away from the caret
            // (owner report). Takes no space at all while Vim is off.
            VimStatusLine()
            // Like the bridge line below: shown once the capture flow is in
            // play (an anchor pinned, proposals queued, or the inspector
            // open), never as a permanent strip under the editor at rest.
            if model.anchor != nil || !model.proposals.isEmpty || model.captureInboxVisible { CaptureBar() }
            // The bridge line is lifecycle telemetry: shown once a bridge is
            // attached or a capture exists, not as a permanent orange
            // "no bridge attached" strip under the editor (daniel-fable-ui-qa #5).
            if model.bridgeStatus != "no bridge attached" || !model.bridgeCaptures.isEmpty || model.bridgeDestination != nil { BridgeBar() }
        }
    }

    /// `:q` / `:wq`: a non-entry document leaves the project (its tab closes);
    /// the entry document cannot be closed, which the command line reports.
    private func closeActiveDocument(discardingEdits: Bool) -> String? {
        let path = model.activePath
        guard path != model.project.entryPath else { return "E37: cannot close the entry document \(path)" }
        Task { await model.project.detachDocument(path, discardingEdits: discardingEdits) }
        return nil
    }
}

private struct CaptureBar: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        HStack(spacing: DS.Space.m) {
            Button("Pin insertion point") { model.pinAnchorAtCaret() }
                .ideSecondary()
                .help("Use the caret as the destination for capture proposals (⌘⌥P)")
            if let a = model.anchor {
                Text("anchor \(a.id) · \(a.path) byte \(a.byteOffset) @ rev \(a.revision)")
                    .font(.caption).foregroundStyle(.secondary)
            } else {
                Text("no insertion point pinned").font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            if !model.proposals.isEmpty {
                Button("Review \(model.proposals.count) proposal\(model.proposals.count == 1 ? "" : "s")") {
                    model.reviewing = model.proposals.first
                }
                .ideDefault()
            }
        }
        .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xs)
        .background(DS.Colors.surfaceSecondary)
        .accessibleCaptureBar(anchor: model.anchor.map { "\($0.id) at \($0.path) byte \($0.byteOffset), revision \($0.revision)" }, proposals: model.proposals.count) // FlashTeXAccessibility
        .sheet(item: Binding(get: { model.reviewing.map { ReviewItem(proposal: $0) } },
                             set: { model.reviewing = $0?.proposal })) { item in
            ProposalReviewSheet(proposal: item.proposal)
        }
    }
}

/// Bridge lifecycle line: attached/error status, the pinned bridge
/// destination, and the latest capture's state (plain text, never a prompt).
private struct BridgeBar: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        HStack(spacing: DS.Space.m) {
            Text("bridge:").font(DS.Fonts.header)
            Text(model.bridgeStatus).font(.caption)
                .foregroundStyle(model.bridgeAttached ? DS.Colors.textSecondary : DS.Colors.severityWarning).lineLimit(1)
            if let d = model.bridgeDestination {
                Text("· destination \(d.destinationId) bytes \(d.startByte)..<\(d.endByte) @ rev \(d.pinnedRevision)\(d.valid ? "" : " (invalid)")")
                    .font(.caption).foregroundStyle(.secondary).lineLimit(1)
            }
            Spacer()
            if let c = model.bridgeCaptures.last {
                Text("\(c.captureId): \(c.state.rawValue) — \(c.note)").font(.caption).foregroundStyle(.secondary).lineLimit(1)
                    .help(c.note)
            }
            if model.latestConvertibleCapture?.state == .received {
                Button("Convert") { model.convertLatestCapture() }.ideSecondary()
                    .help("capture_convert for the latest received capture (Edit > Convert Capture)")
            }
        }
        .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xxs)
        .background(DS.Colors.surfaceSecondary)
    }
}

struct PreviewPane: View {
    @Environment(ShellModel.self) var model
    /// Page under the viewport's top edge (PreviewAnchorProbe reports it).
    @State private var currentPage = 1
    /// Pointer-over reveals the header's second control tier (§8).
    @State private var hovering = false

    /// Bumped by scroll, page and zoom changes; each bump shows the HUD
    /// briefly (design change from #653 review: page/zoom moved off the
    /// removed header row into a transient overlay).
    @State private var hudActivity = 0

    var body: some View {
        ZStack(alignment: .topTrailing) {
            if model.previewV2 {
                PreviewV2Pane() // experimental v2 path (PreviewV2View.swift); v1 below stays the default
                    .modifier(PreviewMagnify()) // pinch to zoom (PreviewZoom.swift)
            } else if let result = model.result {
                PreviewView(result: result, dark: model.darkPreview, caretItems: model.caretItems,
                            zoom: model.previewZoom, onFitScale: { model.previewFitScale = $0 },
                            // "the pdf moves to where the changes are happening" (CaretFollow.swift)
                            follow: model.caretFollow.request,
                            onUserScroll: { model.caretFollow.userDidScrollPreview(); hudActivity &+= 1 },
                            onVisiblePage: { currentPage = $0 },
                            onFitPageZoom: { model.previewFitPageZoom = $0 }) { source, text in
                    guard let source else { model.navigationNote = "This item has no source mapping."; return }
                    model.navigate(to: source, expectedText: text)
                }
                .modifier(PreviewMagnify()) // pinch to zoom (PreviewZoom.swift)
            } else {
                ContentUnavailableView {
                    Label("No preview yet", systemImage: "doc.richtext")
                } description: {
                    Text("Attach a producer (File > Attach Built Compiler ⌘⇧K, or ⌘⇧R for Latin Modern) and compile (⌘B), or File > Open Compile Result Fixture… (⌘⇧O).")
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(DS.Colors.surfaceGround)
            }
            // The transient page/zoom HUD and the quiet state cluster
            // (compile spinner, FIXTURE/HISTORICAL, staleness) float over
            // the pages; nothing reserves a header row any more (#653).
            if model.previewV2 || model.chrome.hasResult {
                PreviewHUD(currentPage: currentPage, hovering: hovering, activity: hudActivity)
            }
            // The narrow layout's way back to the editor floats top-leading
            // (it lived on the removed header row).
            if model.narrowLayout && model.narrowPreviewShown {
                NarrowBackButton()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(DS.Space.l)
            }
        }
        .onChange(of: model.previewZoom) { _, _ in hudActivity &+= 1 }
        .onChange(of: currentPage) { _, _ in hudActivity &+= 1 }
        .sheet(isPresented: Binding(get: { model.quickFix != nil }, set: { if !$0 { model.quickFix = nil } })) {
            if let p = model.quickFix {
                VStack(alignment: .leading, spacing: DS.Space.m) {
                    Text("Suggested fix").font(.headline)
                    Text(p.summary).font(.caption).foregroundStyle(.secondary)
                    Text("Before").font(.caption.bold())
                    Text(p.before).font(.system(.body, design: .monospaced)).textSelection(.enabled)
                    Text("After").font(.caption.bold())
                    Text(p.after).font(.system(.body, design: .monospaced)).textSelection(.enabled)
                    Text("Heuristic suggestion from the explanation catalogue; applied as one undoable edit only when you choose Apply.")
                        .font(.caption2).foregroundStyle(.tertiary)
                    HStack {
                        Spacer()
                        Button("Cancel") { model.quickFix = nil }.keyboardShortcut(.cancelAction).ideSecondary()
                        Button("Apply") { model.applyQuickFix() }.keyboardShortcut(.defaultAction).ideDefault()
                    }
                }
                .padding(DS.Space.xl).frame(minWidth: DS.Layout.sheetMinWidth)
                .accessibilityElement(children: .contain).accessibilityLabel("Suggested fix preview")
            }
        }
        .onHover { hovering = $0 }
    }
}

/// The floating preview HUD (owner feedback on #653: no header row over
/// the pages). Two duties, one chip: the always-quiet state — a mini
/// spinner while compiling, FIXTURE/HISTORICAL when the pages are not the
/// worker's current result, staleness and capability warnings — pins the
/// chip visible; the page / zoom readout shows transiently on pointer-over
/// and for a beat after scroll, page or zoom changes, then fades. Fading
/// never reflows anything (§14); a healthy live preview at rest shows no
/// chrome at all over the pages (§8, Canvas treatment).
private struct PreviewHUD: View {
    @Environment(ShellModel.self) var model
    var currentPage = 1
    var hovering = false
    /// Bumped by the pane on scroll/page/zoom; each bump re-arms the fade.
    var activity = 0
    @State private var activityVisible = false

    var body: some View {
        // Reads the throttled chrome mirror (ShellChrome.swift), not
        // `result` / `inFlightRevision`: the old header re-evaluated on
        // every keystroke and reply (FT-071 main-thread sample).
        let chrome = model.chrome
        let badge = Self.stateBadge(chrome)
        let pinned = chrome.compiling || badge != nil || chrome.staleHighlighted
            || chrome.loadError != nil || !chrome.capabilityNotes.isEmpty
            || (model.previewDebugStatus && chrome.resultStatus != nil)
        let visible = hovering || pinned || activityVisible
        HStack(spacing: DS.Space.m) {
            // Fixed-size slot: the compile indicator never reflows the chip.
            Color.clear.frame(width: DS.Size.inlineStatusSlot, height: DS.Size.inlineStatusSlot)
                .overlay { if chrome.compiling { ProgressView().controlSize(.mini).accessibilityLabel("Compiling") } }
            if let badge {
                Text(badge.text)
                    .font(DS.Fonts.header)
                    .padding(.horizontal, DS.Space.s).padding(.vertical, DS.Space.xxs)
                    .background(badge.color.opacity(DS.State.badgeFillOpacity), in: Capsule())
            }
            if model.previewDebugStatus, let status = chrome.resultStatus {
                Text(status.rawValue).font(DS.Fonts.header).foregroundStyle(statusColor(status))
            }
            if let historical = chrome.historicalLabel {
                Text(historical).font(DS.Fonts.header).foregroundStyle(DS.Colors.statusHistorical).lineLimit(1)
                    .help("A completed older snapshot is shown while the helper compiles the newer revision; navigation, caret sync, capture destinations and export return with the current preview.")
            } else if let stale = chrome.staleText { // the reply exceeded a bound, or "editor at rN — compiling…" (ShellChrome)
                Text(stale)
                    .font(DS.Fonts.secondary).foregroundStyle(chrome.staleHighlighted ? DS.Colors.severityWarning : DS.Colors.textSecondary).lineLimit(1) // routine "compiling…" is quiet; only bounds/no-producer are highlighted
            }
            if let err = chrome.loadError {
                Text(err).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.severityError).lineLimit(1).help(err)
            }
            ForEach(chrome.capabilityNotes, id: \.self) { note in
                Image(systemName: "exclamationmark.circle").foregroundStyle(DS.Colors.severityWarning).help(note)
                    .accessibilityLabel(note)
            }
            if !model.previewV2, chrome.hasResult {
                Text("\(currentPage) / \(max(model.toolbarPageCount, 1))")
                    .font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textSecondary)
                    .help("Page under the top of the view")
                    .accessibilityLabel("Page \(currentPage) of \(max(model.toolbarPageCount, 1))")
            }
            Text("\(PreviewZoom.percent(fit: model.previewFitScale, zoom: model.previewZoom)) %")
                .font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textSecondary)
                .help("Preview zoom; double-click for Fit Width (⌘9), ⌘0 actual size, or pinch on the preview")
                .accessibilityLabel("Preview zoom \(PreviewZoom.percent(fit: model.previewFitScale, zoom: model.previewZoom)) percent")
                .onTapGesture(count: 2) { model.previewFitWidth() }
        }
        .padding(.horizontal, DS.Space.l).padding(.vertical, DS.Space.s)
        .background(DS.Colors.surfaceRaised, in: RoundedRectangle(cornerRadius: DS.Radius.panel))
        .overlay(RoundedRectangle(cornerRadius: DS.Radius.panel).strokeBorder(DS.Colors.componentBorder, lineWidth: DS.Size.hairline))
        .shadow(color: .black.opacity(DS.Preview.hudShadowOpacity), radius: DS.Preview.pageShadowRadius)
        .padding(DS.Space.l)
        .opacity(visible ? 1 : 0)
        .allowsHitTesting(visible)
        .animation(DS.Motion.quick, value: visible)
        .accessibilityHidden(!visible)
        .help(chrome.previewSource == .fixture
              ? "Fixture\(chrome.fixtureName.map { ": " + $0 } ?? "") — not a real compile. Layout: \(chrome.acceptedCapabilities.isEmpty ? "legacy (U+2500 fraction bars are an approximation)" : chrome.acceptedCapabilities.joined(separator: ", "))"
              : model.producerSummary + " — layout: \(chrome.acceptedCapabilities.isEmpty ? "legacy (U+2500 fraction bars are an approximation)" : chrome.acceptedCapabilities.joined(separator: ", "))")
        // The linger timer is a real pending Task for as long as it runs;
        // under XCTest that outlives the surface being captured and makes
        // both this shot and the next one depend on when it fires, so
        // snapshots see the hover tier only (same reasoning as
        // ThinSplitViewController.autosaveEnabled).
        .task(id: activity) {
            guard activity > 0, !PreviewHUD.lingerSuppressed else { return }
            activityVisible = true
            try? await Task.sleep(for: .seconds(DS.Motion.hudLinger))
            guard !Task.isCancelled else { return }
            activityVisible = false
        }
    }

    /// True under XCTest: the linger timer never runs, so a captured
    /// surface does not depend on when it happens to fire.
    static let lingerSuppressed = ProcessInfo.processInfo.environment["XCTestConfigurationFilePath"] != nil
        || ProcessInfo.processInfo.environment["XCTestSessionIdentifier"] != nil

    /// Quiet state badge: shown only when the pages are *not* the worker's
    /// current result — FIXTURE (not a real compile) or HISTORICAL (an older
    /// snapshot while the newer revision compiles). A healthy live preview
    /// shows nothing here.
    static func stateBadge(_ chrome: ShellChrome) -> (text: String, color: Color)? {
        switch chrome.previewSource {
        case .none: nil
        case .fixture: ("FIXTURE", DS.Colors.severityWarning)
        case .worker: chrome.historicalLabel != nil ? ("HISTORICAL", DS.Colors.statusHistorical) : nil
        }
    }

    private func statusColor(_ s: RuntimeV1.Status) -> Color {
        switch s { case .ok: DS.Colors.severitySuccess; case .recovered: DS.Colors.severityWarning; case .failed: DS.Colors.severityError }
    }
}

/// Back to the editor while the window is too narrow for both columns
/// (design-principles §4); floats where the removed header row carried it.
private struct NarrowBackButton: View {
    @Environment(ShellModel.self) var model
    @State private var hovering = false

    var body: some View {
        Button { model.narrowPreviewShown = false } label: {
            Image(systemName: "chevron.left")
                .font(DS.Fonts.toolbarIcon)
                .foregroundStyle(DS.Colors.textSecondary)
                .frame(width: DS.Size.toolbarButton, height: DS.Size.toolbarButton)
                .background(DS.Colors.surfaceRaised, in: RoundedRectangle(cornerRadius: DS.Radius.tab))
                .overlay(RoundedRectangle(cornerRadius: DS.Radius.tab).strokeBorder(DS.Colors.componentBorder, lineWidth: DS.Size.hairline))
                .background(hovering ? DS.Colors.hover : .clear)
                .contentShape(Rectangle())
        }
        .buttonStyle(PressableStyle())
        .onHover { hovering = $0 }
        .help("Back to the editor (the window is too narrow for editor and preview side by side)")
        .accessibilityLabel("Back to editor")
    }
}

/// Bottom status bar: editor revision, the helper's durable revision of the
/// active document, compile latency, the route (fixture / worker /
/// controller), then the last navigation, staleness or capture note, and an
/// exact-export progress control while one runs.
struct StatusBar: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        // Reads the throttled chrome mirror (ShellChrome.swift): the revision,
        // latency, route tooltip, problem counts and notes change on every
        // keystroke / reply, and this bar re-evaluated with each of them.
        let chrome = model.chrome
        HStack(spacing: DS.Space.l) {
            // The LaTeX-semantic breadcrumb leads (design-principles §5):
            // where the caret is, in the document's own vocabulary. The
            // palette and outline are the real navigation; segments still
            // click through to their section.
            StatusBreadcrumb()
            Divider().frame(height: DS.Size.inlineDividerHeight)
            Text(chrome.note ?? "Click text in the preview to select its source range.")
                .foregroundStyle(.secondary).lineLimit(1)
            Spacer()
            WordCountStatusItem() // GH68: live word count + breakdown popover (WordCountStatusView.swift)
            if let ms = chrome.lastLatencyMs {
                Label(String(format: "%.0f ms", ms), systemImage: "timer")
                    .help(chrome.latencyHelp)
            }
            Label(route(chrome), systemImage: routeIcon(chrome))
                .help(chrome.routeHelp)
            // Revision counters are diagnostics, not writing state (owner,
            // #653): shown only with View > Show Preview Debug Status.
            if model.previewDebugStatus {
                Label("r\(chrome.editorRevision)", systemImage: "pencil.line")
                    .help("Editor revision (increments on every edit)")
                if let durable = chrome.durableRevision {
                    Label("durable r\(durable)", systemImage: "internaldrive")
                        .help("Durable revision of \(model.activePath) in the helper's edit ledger")
                }
            }
            // The Vim mode indicator is NOT here: it is `VimStatusLine`, at the
            // bottom of the editor pane where vim puts a window's status line.
            let problems = chrome.problems
            if !problems.isEmpty {
                Button {
                    model.problemsVisible.toggle()
                } label: {
                    HStack(spacing: DS.Space.s) {
                        if problems.errors > 0 { Label("\(problems.errors)", systemImage: "xmark.octagon.fill").foregroundStyle(DS.Colors.severityError) }
                        if problems.warnings > 0 { Label("\(problems.warnings)", systemImage: "exclamationmark.triangle.fill").foregroundStyle(DS.Colors.severityWarning) }
                        if problems.gaps > 0 { Label("\(problems.gaps)", systemImage: "puzzlepiece.extension").foregroundStyle(DS.Colors.textSecondary) }
                    }
                }
                .buttonStyle(.plain)
                .help("Errors, warnings and not-implemented gaps of the last result — click to show or hide the Problems panel (⌘⇧M)")
            }
            if case .running(let pid, _) = model.exportSession.state { // ShellModel+ExportSession.swift
                ProgressView().controlSize(.small)
                Text("Exporting exact PDF (flashtex-pdf-exact pid \(pid))…")
                    .foregroundStyle(.secondary).lineLimit(1)
                Button("Cancel") { model.cancelExactExport() }
                    .ideSecondary()
                    .help("Terminate flashtex-pdf-exact; nothing is written to the destination")
                    .accessibilityIdentifier("export.cancel")
            } else if let note = chrome.captureNote {
                Text(note).foregroundStyle(.secondary).lineLimit(1).help(note)
            }
        }
        .font(DS.Fonts.secondary)
        .monospacedDigit()
        .padding(.horizontal, DS.Space.l)
        .frame(height: DS.Row.statusBar)
        .background(DS.Colors.surfaceSecondary)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Status bar")
    }

    private func route(_ chrome: ShellChrome) -> String {
        switch chrome.route {
        case .fixture: "fixture"
        case .controller: "controller"
        case .worker: "worker"
        case .none: "no producer"
        }
    }

    private func routeIcon(_ chrome: ShellChrome) -> String {
        switch chrome.route {
        case .fixture: "doc.badge.gearshape"
        case .controller, .worker: "bolt.horizontal.circle.fill"
        case .none: "bolt.horizontal.circle"
        }
    }
}

/// The status bar's leading breadcrumb: `main.tex › Chapter 2 › 2.3 Setup`.
/// The outline is rescanned only when the buffer settles (like the sidebar);
/// caret moves just re-pick the chain from the cached items, debounced so a
/// keystroke never pays for it on its own frame.
private struct StatusBreadcrumb: View {
    @Environment(ShellModel.self) var model
    @State private var items: [DocumentOutline.Item] = []
    @State private var chain: [DocumentOutline.Item] = []

    var body: some View {
        HStack(spacing: DS.Space.xs) {
            // The same colour-coded identity as the tree and the tabs (§6)
            // leads the breadcrumb — one of the deliberate small accents
            // (owner: the editor feels bland).
            let style = FileTypeStyle.of(path: model.activePath, entry: model.activePath == model.chrome.entryPath)
            Image(systemName: style.systemImage)
                .font(DS.Fonts.secondary).foregroundStyle(style.color)
                .accessibilityHidden(true)
            Text(model.activePath).lineLimit(1)
                .foregroundStyle(DS.Colors.textSecondary)
            ForEach(chain) { item in
                Text("›").foregroundStyle(DS.Colors.textTertiary).accessibilityHidden(true)
                Button { model.reveal(outlineItem: item) } label: {
                    Text(item.displayTitle.isEmpty ? "(untitled)" : item.displayTitle)
                        .lineLimit(1).truncationMode(.tail)
                        .foregroundStyle(DS.Colors.textSecondary)
                }
                .buttonStyle(.plain)
                .help("\\\(item.command){\(item.title)} — line \(item.line); click to select it")
                .accessibilityLabel("\(item.command) \(item.title), line \(item.line)")
            }
        }
        .task(id: "\(model.activePath)@\(model.chrome.editorRevision)") {
            if !items.isEmpty { try? await Task.sleep(for: .milliseconds(150)) }
            guard !Task.isCancelled else { return }
            items = model.outline
            chain = DocumentOutline.breadcrumb(at: model.caretUTF16, in: items)
        }
        .task(id: model.caretUTF16) {
            try? await Task.sleep(for: .milliseconds(80))
            guard !Task.isCancelled else { return }
            let new = DocumentOutline.breadcrumb(at: model.caretUTF16, in: items)
            if new != chain { chain = new }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Breadcrumb")
    }
}

private struct ReviewItem: Identifiable {
    let proposal: RuntimeV1.CaptureProposal
    var id: String { proposal.captureId }
}

/// Review sheet: the reviewer sees ambiguities and dependencies, may edit the
/// LaTeX, and explicitly approves or rejects. Nothing is inserted otherwise.
private struct ProposalReviewSheet: View {
    @Environment(ShellModel.self) var model
    @Environment(\.dismiss) private var dismiss
    let proposal: RuntimeV1.CaptureProposal
    @State private var latex: String = ""
    /// Why the last Approve did not insert. The model writes every refusal to
    /// `captureNote`, but that is rendered in the status bar and the Captures
    /// inspector — both behind this window-modal sheet. Without showing it
    /// here, a refused approval looks exactly like a dead button.
    @State private var failure: String?
    @StateObject private var preview = ProposalPreview(executable: ShellModel.locateCompiler()) // shadow compile (ProposalPreview.swift)

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.m) {
            Text("Review capture \(proposal.captureId)").font(.headline)
            if model.isBridgeCapture(proposal.captureId) {
                Text("Bridge capture: approval asks the bridge for a prepared edit, verifies revision, SHA-256 and removed text, then inserts once. The LaTeX must stay as proposed.")
                    .font(.caption).foregroundStyle(.secondary)
                if let d = model.bridgeDestination {
                    Text("Bridge destination \(d.destinationId): \(d.path) bytes \(d.startByte)..<\(d.endByte)").font(.caption).foregroundStyle(.secondary)
                }
                if let cr = proposal.contextRevision {
                    Text("Context revision \(cr)\(cr == model.editorRevision ? "" : " (editor is at \(model.editorRevision))")").font(.caption).foregroundStyle(cr == model.editorRevision ? DS.Colors.textSecondary : DS.Colors.severityWarning)
                }
            }
            if let a = model.anchor {
                Text("Inserts at \(a.path) byte \(a.byteOffset) (anchor \(a.id))").font(.caption).foregroundStyle(.secondary)
            } else {
                Label("No insertion point pinned — approve will fail until you pin one.", systemImage: "exclamationmark.triangle")
                    .font(.caption).foregroundStyle(DS.Colors.severityWarning)
            }
            TextEditor(text: $latex)
                .font(.system(.body, design: .monospaced))
                .frame(minHeight: DS.Layout.sheetTextEditorMinHeight)
                .border(.separator)
            ProposalPreviewView(preview: preview)
            if !proposal.ambiguities.isEmpty {
                Text("Ambiguities").font(.subheadline.bold())
                ForEach(proposal.ambiguities, id: \.self) { Text("• \($0)").font(.caption) }
            }
            if !proposal.requiredDependencies.isEmpty {
                Text("Required packages: " + proposal.requiredDependencies.joined(separator: ", ")).font(.caption)
            }
            if let failure {
                Label(failure, systemImage: "exclamationmark.triangle.fill")
                    .font(.callout).foregroundStyle(DS.Colors.severityWarning)
                    .textSelection(.enabled)
                    .accessibilityIdentifier("review.failure")
            }
            HStack {
                Button("Reject", role: .destructive) { model.rejectProposal(proposal); dismiss() }
                    .ideSecondary(destructive: true)
                // Non-destructive exit: the sheet blocks the whole window, so
                // without this the Captures inspector's Insert is unreachable
                // for exactly the proposals it could act on.
                Button("Close") { model.dismissReviewWithoutDeciding() }
                    .keyboardShortcut(.cancelAction)
                    .ideSecondary()
                    .accessibilityIdentifier("review.close")
                Spacer()
                ProposalApproveWarning(preview: preview)
                Button("Approve and insert") {
                    failure = nil
                    if model.isBridgeCapture(proposal.captureId) {
                        Task {
                            if case .inserted = await model.approveBridgeProposal(proposal, latex: latex) { dismiss() }
                            else { failure = model.captureNote ?? "The insertion was refused." }
                        }
                    } else if case .inserted = model.approveProposal(proposal, latex: latex) { dismiss() }
                    else { failure = model.captureNote ?? "The insertion was refused." }
                }
                .keyboardShortcut(.defaultAction)
                .ideDefault()
                .disabled((model.anchor == nil && !model.isBridgeCapture(proposal.captureId)) || latex.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.sheetWidth)
        .onAppear { latex = proposal.latex; preview.update(from: model, latex: proposal.latex) }
        .onChange(of: latex) { _, new in failure = nil; preview.update(from: model, latex: new) }
        .onChange(of: model.editorRevision) { _, _ in preview.update(from: model, latex: latex) }
        .onChange(of: model.anchor) { _, _ in preview.update(from: model, latex: latex) }
        .onDisappear { preview.close() }
    }
}

/// Vim's status line (VimMode.swift): `-- NORMAL --` / `-- INSERT --` /
/// `-- VISUAL --` and the `:`/`/` command line, at the bottom of the editor
/// pane. Vim puts the status line at the bottom of the window being edited,
/// so it stays with the text even when the Problems panel is open below or
/// the preview is beside it; the global status bar would put it two panes
/// away from the caret it describes.
///
/// While the preference is off `VimMode.Status.shared.indicator` is nil and
/// the body produces no view at all — the editor gains no empty strip.
struct VimStatusLine: View {
    var body: some View {
        let status = VimMode.Status.shared
        if let indicator = status.indicator {
            HStack(spacing: DS.Space.m) {
                Text(indicator)
                    .fontWeight(.semibold)
                    .help("Vim keybindings are on (Settings, or View > Toggle Vim Keybindings ⌃⌘V)")
                    .accessibilityIdentifier("status.vimMode")
                if let line = status.commandLine, !line.isEmpty {
                    Text(line).lineLimit(1).accessibilityIdentifier("status.vimCommandLine")
                }
                Spacer(minLength: 0)
            }
            .font(DS.Fonts.secondary)
            .monospacedDigit()
            .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xxs)
            .background(DS.Colors.surfaceSecondary)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Vim status line")
        }
    }
}
