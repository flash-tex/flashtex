import SwiftUI
import FlashTeXProtocol
import FlashTeXAccessibility

/// The main window: an IntelliJ-style tool-window shell built flat
/// (design-principles §4) — a left icon rail that toggles the tool column
/// (WorkspaceSidebar.swift: Project tree, and the Outline, which ships
/// collapsed), the document tabs + editor and the preview side by side, the
/// Problems panel underneath (ProblemsPanel.swift), and a full-width status
/// bar. Regions are separated by hairlines and a slight tonal shift, never
/// by distinct region backgrounds; the title bar carries at most three
/// interactive chips at rest (style-guide chrome budget). Every action here
/// is an existing model operation; View > Command Palette… (⌘⇧P) lists every
/// command of the accessibility table (CommandPalette.swift).
struct ContentView: View {
    @Environment(ShellModel.self) var model
    @Environment(\.openWindow) private var openWindow
    /// Tool-window visibility (the rail toggles them). The Outline ships
    /// collapsed; the Project tree shows by default.
    @AppStorage("FlashTeX.workspace.projectVisible") private var projectVisible = true
    @AppStorage("FlashTeX.workspace.outlineVisible") private var outlineVisible = false
    /// Width of the tool column; remembered across launches.
    @AppStorage("FlashTeX.workspace.toolColumnWidth") private var toolColumnWidth: Double = Double(DS.Layout.sidebarIdealWidth)
    /// Height of the Problems panel; remembered across launches.
    @AppStorage("FlashTeX.workspace.problemsHeight") private var problemsHeight: Double = ProblemsPanel.idealHeight

    var body: some View {
        @Bindable var model = model
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                ToolRail(projectVisible: $projectVisible, outlineVisible: $outlineVisible)
                Divider()
                if projectVisible || outlineVisible {
                    WorkspaceSidebar(projectVisible: projectVisible, outlineVisible: outlineVisible)
                        .frame(width: toolColumnWidth)
                    ColumnResizeHandle(width: $toolColumnWidth,
                                       range: DS.Layout.sidebarMinWidth...DS.Layout.sidebarMaxWidth)
                }
                GeometryReader { geo in
                    // Below the width where both columns fit, the preview
                    // collapses to a toggle (tab bar / preview header) rather
                    // than being crushed under its minimum.
                    let narrow = geo.size.width < DS.Layout.editorMinWidth + DS.Layout.previewMinWidth + DS.Layout.resizeHandleHeight
                    VStack(spacing: 0) {
                        HSplitView {
                            if !(narrow && model.narrowPreviewShown) {
                                EditorPane().frame(minWidth: DS.Layout.editorMinWidth, maxWidth: .infinity)
                            }
                            if !narrow || model.narrowPreviewShown {
                                PreviewPane().frame(minWidth: DS.Layout.previewMinWidth, maxWidth: .infinity)
                            }
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .onChange(of: narrow, initial: true) { _, now in
                            if model.narrowLayout != now { model.narrowLayout = now }
                        }
                        if model.problemsVisible {
                            // Drag the handle to give the diagnostics list more or less
                            // room; the list scrolls within whatever height it has.
                            // Never more than 40 % of the window: at 1000×640 the
                            // editor keeps ~15 lines instead of 10 (daniel-fable-ui-qa #3).
                            let panelCap = max(ProblemsPanel.minHeight, min(geo.size.height - DS.Layout.editorMinHeightAbovePanel, geo.size.height * DS.Layout.problemsMaxFraction))
                            PanelResizeHandle(height: $problemsHeight, range: ProblemsPanel.minHeight...panelCap)
                            ProblemsPanel().frame(height: min(problemsHeight, panelCap))
                        }
                    }
                }
            }
            Divider()
            StatusBar()
        }
        .inspector(isPresented: $model.captureInboxVisible) { // Captures (CaptureInbox.swift): View > Captures, ⌘⇧I
            CaptureInboxPanel(inbox: model.captureInbox).inspectorColumnWidth(min: DS.Layout.inspectorMinWidth, ideal: DS.Layout.inspectorIdealWidth, max: DS.Layout.inspectorMaxWidth)
        }
        .toolbar { WorkspaceToolbar(openWindow: openWindow) }
        .sheet(isPresented: $model.commandPaletteShown) { CommandPalette().environment(model) }
        .modifier(EditorNavigationSheets()) // Rename Symbol… / Wrap Selection in Environment… / Go to Symbol… (ShellModel+EditorNavigation.swift)
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
                .font(DS.Fonts.base)
                .foregroundStyle(isOn ? DS.Colors.accentSelection : DS.Colors.textSecondary)
                .frame(width: DS.Size.railButton, height: DS.Size.railButton)
                .background(
                    isOn ? DS.Colors.accentSelection.opacity(DS.State.badgeFillOpacity)
                         : hovering ? DS.Colors.textPrimary.opacity(DS.State.hoverOpacity) : .clear,
                    in: RoundedRectangle(cornerRadius: DS.Radius.tab))
        }
        .buttonStyle(.plain)
        .onHover { hovering = $0 }
        .help(help)
        .accessibilityLabel(label)
        .accessibilityAddTraits(isOn ? .isSelected : [])
    }
}

/// The vertical divider at the tool column's trailing edge, draggable left
/// and right (the cursor shows the resize arrows on hover).
private struct ColumnResizeHandle: View {
    @Binding var width: Double
    let range: ClosedRange<CGFloat>
    @State private var startWidth: Double?

    var body: some View {
        Rectangle().fill(.clear)
            .frame(width: DS.Layout.resizeHandleHeight)
            .overlay(Divider(), alignment: .center)
            .contentShape(Rectangle())
            .onHover { inside in if inside { NSCursor.resizeLeftRight.push() } else { NSCursor.pop() } }
            .gesture(DragGesture(minimumDistance: 1)
                .onChanged { value in
                    let start = startWidth ?? width
                    startWidth = start
                    width = min(max(start + value.translation.width, range.lowerBound), range.upperBound)
                }
                .onEnded { _ in startWidth = nil })
            .accessibilityHidden(true)
    }
}

/// The divider above the Problems panel, draggable up and down (the cursor
/// shows the resize arrows on hover). Keyboard users size it with the split
/// of the window itself; the panel is never taller than the window allows.
private struct PanelResizeHandle: View {
    @Binding var height: Double
    let range: ClosedRange<Double>
    @State private var startHeight: Double?

    var body: some View {
        Rectangle().fill(.clear)
            .frame(height: DS.Layout.resizeHandleHeight)
            .overlay(Divider(), alignment: .center)
            .contentShape(Rectangle())
            .onHover { inside in if inside { NSCursor.resizeUpDown.push() } else { NSCursor.pop() } }
            .gesture(DragGesture(minimumDistance: 1)
                .onChanged { value in
                    let start = startHeight ?? height
                    startHeight = start
                    height = min(max(start - value.translation.height, range.lowerBound), range.upperBound)
                }
                .onEnded { _ in startHeight = nil })
            .accessibilityHidden(true)
    }
}

// MARK: toolbar


/// The chrome budget (style-guide): at most three interactive chips at rest
/// — Compile (with the producer menu behind its chevron), Export, and the
/// command palette — plus the right-side panel toggles. Everything that used
/// to be a toolbar button here is reachable from its menu, its shortcut and
/// the palette; tooltips still name the shortcuts.
private struct WorkspaceToolbar: ToolbarContent {
    @Environment(ShellModel.self) var model
    let openWindow: OpenWindowAction

    var body: some ToolbarContent {
        @Bindable var model = model
        ToolbarItemGroup(placement: .principal) {
            // One chip: click compiles, the chevron holds the producer menu.
            Menu {
                Button("Attach Built Compiler") { _ = model.attachDiscoveredWorker() }
                    .help("⌘⇧K")
                Button("Attach Render Pipeline (Latin Modern)") { _ = model.attachDiscoveredRenderPipeline() }
                    .help("⌘⇧R")
                Button("Attach Worker Executable…") { model.attachWorkerPanel() }
                    .help("⌘K")
                Divider()
                Toggle("Auto-compile after edits", isOn: $model.autoCompile).disabled(!model.workerAttached)
                Button("Detach Worker") { model.detachWorker() }.disabled(!model.workerAttached)
                if model.isFixture { Divider(); Button("Reload Fixture") { model.reloadFixture() } }
            } label: {
                Label("Compile", systemImage: "hammer.fill")
            } primaryAction: {
                if !model.outputBoundExplicitRetry() { model.compile() }
            }
            // `producerSummary`, not `workerStatus`: the toolbar must not re-evaluate per request (ShellModel toolbar mirrors).
            .help("Compile (File > Compile, ⌘B) — producer: " + (model.isFixture ? "fixture (not a real compile)" : model.producerSummary) + ". The menu attaches the built compiler (⌘⇧K), the Latin Modern render pipeline (⌘⇧R) or any executable (⌘K).")
        }
        ToolbarItemGroup(placement: .automatic) {
            Menu {
                Button("Export PDF…") { model.exportPDF() }.disabled(!model.toolbarHasResult)
                Button("Export PDF via Rust Writer…") { model.exportPDFViaRust() }.disabled(!model.toolbarHasResult)
                Button("Export PDF (exact, v2)…") { model.exportPDFExact() }.disabled(!model.toolbarHasV2Frame)
            } label: { Label("Export", systemImage: "square.and.arrow.up") }
                .help("Export PDF… (⌘⇧E), via Rust writer (⌘⌥E), or exact from the v2 display list (File menu)")
            Button { model.commandPaletteShown = true } label: { Label("Commands", systemImage: "command") }
                .help("Command Palette… (View, ⌘⇧P): every command with its shortcut")
                .accessibilityIdentifier("toolbar.command-palette")
            Toggle(isOn: $model.captureInboxVisible) {
                let n = model.captureInbox.items.count
                Label(n > 0 ? "Captures \(n)" : "Captures", systemImage: n > 0 ? "tray.full" : "tray")
            }
            .toggleStyle(.button)
            .help("Show or hide the Captures inspector (View, ⌘⇧I): captures from the iPad, their proposals, Insert at caret")
            .accessibilityIdentifier("toolbar.captures")
        }
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
            DocumentTabBar()
            Divider()
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
            CaptureBar()
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
                .controlSize(.small)
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
                .controlSize(.small)
            }
        }
        .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xs)
        .background(.bar)
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
                Button("Convert") { model.convertLatestCapture() }.controlSize(.small)
                    .help("capture_convert for the latest received capture (Edit > Convert Capture)")
            }
        }
        .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xxs)
        .background(.bar)
    }
}

struct PreviewPane: View {
    @Environment(ShellModel.self) var model
    /// Page under the viewport's top edge (PreviewAnchorProbe reports it).
    @State private var currentPage = 1
    /// Pointer-over reveals the header's second control tier (§8).
    @State private var hovering = false

    var body: some View {
        VStack(spacing: 0) {
            PreviewHeader(currentPage: currentPage, hovering: hovering)
            Divider()
            if model.previewV2 {
                PreviewV2Pane() // experimental v2 path (PreviewV2View.swift); v1 below stays the default
                    .modifier(PreviewMagnify()) // pinch to zoom (PreviewZoom.swift)
            } else if let result = model.result {
                PreviewView(result: result, dark: model.darkPreview, caretItems: model.caretItems,
                            zoom: model.previewZoom, onFitScale: { model.previewFitScale = $0 },
                            // "the pdf moves to where the changes are happening" (CaretFollow.swift)
                            follow: model.caretFollow.request,
                            onUserScroll: { model.caretFollow.userDidScrollPreview() },
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
                    Text("Attach a producer from the toolbar (⌘⇧K builds, ⌘⇧R Latin Modern) and compile (⌘B), or File > Open Compile Result Fixture… (⌘⇧O).")
                }
            }
        }
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
                        Button("Cancel") { model.quickFix = nil }.keyboardShortcut(.cancelAction)
                        Button("Apply") { model.applyQuickFix() }.keyboardShortcut(.defaultAction)
                    }
                }
                .padding(DS.Space.xl).frame(minWidth: DS.Layout.sheetMinWidth)
                .accessibilityElement(children: .contain).accessibilityLabel("Suggested fix preview")
            }
        }
        .onHover { hovering = $0 }
    }
}

/// The preview column's header, in the two control tiers of
/// design-principles §8. Always visible and dimmed: the page indicator and
/// zoom percentage, plus quiet *state* — a mini spinner while compiling, a
/// FIXTURE/HISTORICAL badge when the pages are not the worker's current
/// result, staleness text, and capability warnings. On pointer-over the
/// second tier fades in without moving anything: zoom −/+, fit width, fit
/// page, and the preview-scoped v2/dark switches. Producer and capability
/// detail live in tooltips, not the header line.
struct PreviewHeader: View {
    @Environment(ShellModel.self) var model
    var currentPage = 1
    var hovering = false

    var body: some View {
        @Bindable var model = model
        // Reads the throttled chrome mirror (ShellChrome.swift), not `result`,
        // `editorRevision` or `inFlightRevision`: this header re-evaluated on
        // every keystroke and every reply (FT-071 main-thread sample).
        let chrome = model.chrome
        HStack(spacing: DS.Space.m) {
            if model.narrowLayout && model.narrowPreviewShown {
                Button { model.narrowPreviewShown = false } label: { Image(systemName: "chevron.left") }
                    .buttonStyle(.accessoryBar).controlSize(.small)
                    .help("Back to the editor (the window is too narrow for editor and preview side by side)")
                    .accessibilityLabel("Back to editor")
            }
            // Fixed-size slot: the compile indicator never reflows the header.
            Color.clear.frame(width: DS.Size.inlineStatusSlot, height: DS.Size.inlineStatusSlot)
                .overlay { if chrome.compiling { ProgressView().controlSize(.mini).accessibilityLabel("Compiling") } }
            stateBadge(chrome)
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
            Spacer()
            // The hover tier: fades in place, never reflows (§14).
            HStack(spacing: DS.Space.xs) {
                Toggle(isOn: $model.previewV2) { Image(systemName: "rectangle.on.rectangle") }
                    .toggleStyle(.button).buttonStyle(.accessoryBar).controlSize(.small)
                    .help("Experimental display-list-v2 preview (File > Open Display List (v2)…)")
                    .accessibilityLabel("v2 preview pane")
                Toggle(isOn: $model.darkPreview) { Image(systemName: "moon") }
                    .toggleStyle(.button).buttonStyle(.accessoryBar).controlSize(.small)
                    .help("Draw the preview pages dark (page and text colors only)")
                    .accessibilityLabel("Dark preview")
                Button { model.previewFitWidth() } label: { Image(systemName: "arrow.left.and.right.square") }
                    .buttonStyle(.accessoryBar).controlSize(.small)
                    .help("Fit Width (⌘9)").accessibilityLabel("Fit width")
                Button { model.previewFitPage() } label: { Image(systemName: "arrow.up.and.down.square") }
                    .buttonStyle(.accessoryBar).controlSize(.small)
                    .help("Fit Page (⌘⇧9)").accessibilityLabel("Fit page")
                Button { model.previewZoomOut() } label: { Image(systemName: "minus.magnifyingglass") }
                    .buttonStyle(.accessoryBar).controlSize(.small)
                    .help("Zoom Out (⌘-)").accessibilityLabel("Zoom out preview")
                Button { model.previewZoomIn() } label: { Image(systemName: "plus.magnifyingglass") }
                    .buttonStyle(.accessoryBar).controlSize(.small)
                    .help("Zoom In (⌘=)").accessibilityLabel("Zoom in preview")
            }
            .opacity(hovering ? 1 : 0)
            .allowsHitTesting(hovering)
            .animation(DS.Motion.quick, value: hovering)
            .accessibilityHidden(!hovering)
            // The rest tier: page and zoom, always visible, dimmed (§8).
            Text("\(currentPage) / \(max(model.toolbarPageCount, 1))")
                .font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textSecondary)
                .help("Page under the top of the view")
                .accessibilityLabel("Page \(currentPage) of \(max(model.toolbarPageCount, 1))")
            Text("\(PreviewZoom.percent(fit: model.previewFitScale, zoom: model.previewZoom)) %")
                .font(DS.Fonts.monoSecondary).foregroundStyle(DS.Colors.textSecondary)
                .frame(minWidth: DS.Size.zoomReadoutMinWidth)
                .help("Preview zoom; double-click for Fit Width (⌘9), ⌘0 actual size, or pinch on the preview")
                .accessibilityLabel("Preview zoom \(PreviewZoom.percent(fit: model.previewFitScale, zoom: model.previewZoom)) percent")
                .onTapGesture(count: 2) { model.previewFitWidth() }
        }
        .padding(.horizontal, DS.Space.m).padding(.vertical, DS.Space.xs)
        .background(.bar)
        .help(chrome.previewSource == .fixture
              ? "Fixture\(chrome.fixtureName.map { ": " + $0 } ?? "") — not a real compile. Layout: \(chrome.acceptedCapabilities.isEmpty ? "legacy (U+2500 fraction bars are an approximation)" : chrome.acceptedCapabilities.joined(separator: ", "))"
              : model.producerSummary + " — layout: \(chrome.acceptedCapabilities.isEmpty ? "legacy (U+2500 fraction bars are an approximation)" : chrome.acceptedCapabilities.joined(separator: ", "))")
    }

    /// Quiet state badge: shown only when the pages are *not* the worker's
    /// current result — FIXTURE (not a real compile) or HISTORICAL (an older
    /// snapshot while the newer revision compiles). A healthy live preview
    /// shows nothing here.
    @ViewBuilder
    private func stateBadge(_ chrome: ShellChrome) -> some View {
        let label: (text: String, color: Color)? = switch chrome.previewSource {
        case .none: nil
        case .fixture: ("FIXTURE", DS.Colors.severityWarning)
        case .worker: chrome.historicalLabel != nil ? ("HISTORICAL", DS.Colors.statusHistorical) : nil
        }
        if let label {
            Text(label.text)
                .font(DS.Fonts.header)
                .padding(.horizontal, DS.Space.s).padding(.vertical, DS.Space.xxs)
                .background(label.color.opacity(DS.State.badgeFillOpacity), in: Capsule())
        }
    }

    private func statusColor(_ s: RuntimeV1.Status) -> Color {
        switch s { case .ok: DS.Colors.severitySuccess; case .recovered: DS.Colors.severityWarning; case .failed: DS.Colors.severityError }
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
            Label("r\(chrome.editorRevision)", systemImage: "pencil.line")
                .help("Editor revision (increments on every edit)")
            if let durable = chrome.durableRevision {
                Label("durable r\(durable)", systemImage: "internaldrive")
                    .help("Durable revision of \(model.activePath) in the helper's edit ledger")
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
                    .controlSize(.small)
                    .help("Terminate flashtex-pdf-exact; nothing is written to the destination")
                    .accessibilityIdentifier("export.cancel")
            } else if let note = chrome.captureNote {
                Text(note).foregroundStyle(.secondary).lineLimit(1).help(note)
            }
        }
        .font(DS.Fonts.secondary)
        .monospacedDigit()
        .padding(.horizontal, DS.Space.l).padding(.vertical, DS.Space.xs)
        .frame(height: DS.Row.statusBar)
        .background(.bar)
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
                // Non-destructive exit: the sheet blocks the whole window, so
                // without this the Captures inspector's Insert is unreachable
                // for exactly the proposals it could act on.
                Button("Close") { model.dismissReviewWithoutDeciding() }
                    .keyboardShortcut(.cancelAction)
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
            .background(.bar)
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Vim status line")
        }
    }
}
