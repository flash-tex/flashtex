import SwiftUI
import FlashTeXProtocol
import FlashTeXAccessibility

/// The main window (mac-ui-redesign): a `NavigationSplitView` whose sidebar
/// is the project/outline/problems navigator (WorkspaceSidebar.swift) and
/// whose detail is the document tabs + editor and the preview side by side,
/// the Problems panel underneath (ProblemsPanel.swift) and a status bar at
/// the bottom. The window toolbar carries the everyday commands with their
/// menu shortcuts in tooltips, and View > Command Palette… (⌘⇧P) lists every
/// command of the accessibility table (CommandPalette.swift). Every action
/// here is an existing model operation; the redesign moves and labels them.
struct ContentView: View {
    @Environment(ShellModel.self) var model
    @Environment(\.openWindow) private var openWindow
    @State private var columns: NavigationSplitViewVisibility = .all
    /// Height of the Problems panel; remembered across launches.
    @AppStorage("FlashTeX.workspace.problemsHeight") private var problemsHeight: Double = ProblemsPanel.idealHeight

    var body: some View {
        @Bindable var model = model
        NavigationSplitView(columnVisibility: $columns) {
            WorkspaceSidebar()
                .navigationSplitViewColumnWidth(min: 200, ideal: 240, max: 360)
        } detail: {
            GeometryReader { geo in
                VStack(spacing: 0) {
                    HSplitView {
                        EditorPane().frame(minWidth: 340, maxWidth: .infinity)
                        PreviewPane().frame(minWidth: 380, maxWidth: .infinity)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    if model.problemsVisible {
                        // Drag the handle to give the diagnostics list more or less
                        // room; the list scrolls within whatever height it has.
                        // Never more than 40 % of the window: at 1000×640 the
                        // editor keeps ~15 lines instead of 10 (daniel-fable-ui-qa #3).
                        let panelCap = max(ProblemsPanel.minHeight, min(geo.size.height - 240, geo.size.height * 0.4))
                        PanelResizeHandle(height: $problemsHeight, range: ProblemsPanel.minHeight...panelCap)
                        ProblemsPanel().frame(height: min(problemsHeight, panelCap))
                    }
                    Divider()
                    StatusBar()
                }
            }
        }
        .navigationSplitViewStyle(.balanced)
        .inspector(isPresented: $model.captureInboxVisible) { // Captures (CaptureInbox.swift): View > Captures, ⌘⇧I
            CaptureInboxPanel(inbox: model.captureInbox).inspectorColumnWidth(min: 300, ideal: 360, max: 560)
        }
        .toolbar { WorkspaceToolbar(openWindow: openWindow) }
        .sheet(isPresented: $model.commandPaletteShown) { CommandPalette().environment(model) }
        .modifier(EditorNavigationSheets()) // Rename Symbol… / Wrap Selection in Environment… / Go to Symbol… (ShellModel+EditorNavigation.swift)
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
            .frame(height: 7)
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

/// Labelled toolbar items; each tooltip names the menu shortcut so the
/// keyboard workflow is discoverable from the toolbar itself.
private struct WorkspaceToolbar: ToolbarContent {
    @Environment(ShellModel.self) var model
    let openWindow: OpenWindowAction

    var body: some ToolbarContent {
        @Bindable var model = model
        ToolbarItemGroup(placement: .principal) {
            Button {
                if !model.outputBoundExplicitRetry() { model.compile() }
            } label: { Label("Compile", systemImage: "hammer.fill") }
                .labelStyle(.titleAndIcon)
                .disabled(!model.workerAttached)
                .help("Send the current buffers to the attached producer (File > Compile, ⌘B)")
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
                Label(model.workerAttached ? "Producer" : "Attach", systemImage: model.workerAttached ? "cpu.fill" : "cpu")
            }
            // `producerSummary`, not `workerStatus`: the toolbar must not re-evaluate per request (ShellModel toolbar mirrors).
            .help("Producer: " + (model.isFixture ? "fixture (not a real compile)" : model.producerSummary) + " — attach the built compiler (⌘⇧K), the Latin Modern render pipeline (⌘⇧R) or any executable (⌘K)")
        }
        ToolbarItemGroup(placement: .automatic) {
            Toggle(isOn: $model.previewV2) { Label("v2 pane", systemImage: "rectangle.on.rectangle") }
                .toggleStyle(.button)
                .help("Experimental display-list-v2 preview (File > Open Display List (v2)…)")
            Toggle(isOn: $model.darkPreview) { Label("Dark preview", systemImage: "moon") }
                .toggleStyle(.button)
                .help("Draw the preview pages dark (page and text colors only)")
            Button { openWindow(id: ProjectSearch.windowID) } label: { Label("Find in Project", systemImage: "magnifyingglass") }
                .help("Find in Project… (Edit, ⌘⇧F)")
            Button { openWindow(id: CitationRename.windowID) } label: { Label("Rename Citation", systemImage: "quote.bubble") }
                .help("Rename Citation… (Edit): reviewed plan across the project")
            Button { openWindow(id: EditHistoryPanel.windowID) } label: { Label("Durable History", systemImage: "clock.arrow.circlepath") }
                .help("Durable History… (Edit): undo/redo on the helper's edit ledger")
            Menu {
                Button("Export PDF…") { model.exportPDF() }.disabled(!model.toolbarHasResult)
                Button("Export PDF via Rust Writer…") { model.exportPDFViaRust() }.disabled(!model.toolbarHasResult)
                Button("Export PDF (exact, v2)…") { model.exportPDFExact() }.disabled(!model.toolbarHasV2Frame)
            } label: { Label("Export", systemImage: "square.and.arrow.up") }
                .help("Export PDF… (⌘⇧E), via Rust writer (⌘⌥E), or exact from the v2 display list (File menu)")
            Button { openWindow(id: "nearby") } label: { Label("Nearby", systemImage: "ipad.and.iphone") }
                .help("Nearby Companion… (Edit, ⌘⇧N): pair an iPad/iPhone to send captures")
            Toggle(isOn: $model.captureInboxVisible) {
                let n = model.captureInbox.items.count
                Label(n > 0 ? "Captures \(n)" : "Captures", systemImage: n > 0 ? "tray.full" : "tray")
            }
            .toggleStyle(.button)
            .help("Show or hide the Captures inspector (View, ⌘⇧I): captures from the iPad, their proposals, Insert at caret")
            .accessibilityIdentifier("toolbar.captures")
            Toggle(isOn: $model.problemsVisible) {
                let n = model.toolbarProblemCount
                Label(n > 0 ? "Problems \(n)" : "Problems", systemImage: n > 0 ? "exclamationmark.triangle.fill" : "exclamationmark.triangle")
            }
            .toggleStyle(.button)
            .help("Show or hide the Problems panel (View, ⌘⇧M)")
            Button { model.commandPaletteShown = true } label: { Label("Commands", systemImage: "command") }
                .help("Command Palette… (View, ⌘⇧P): every command with its shortcut")
                .accessibilityIdentifier("toolbar.command-palette")
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

private struct EditorPane: View {
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
        HStack(spacing: 8) {
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
        .padding(.horizontal, 8).padding(.vertical, 4)
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
        HStack(spacing: 8) {
            Text("bridge:").font(.caption.bold())
            Text(model.bridgeStatus).font(.caption)
                .foregroundStyle(model.bridgeAttached ? Color.secondary : Color.orange).lineLimit(1)
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
        .padding(.horizontal, 8).padding(.vertical, 3)
        .background(.bar)
    }
}

private struct PreviewPane: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        VStack(spacing: 0) {
            PreviewHeader()
            Divider()
            if model.previewV2 {
                PreviewV2Pane() // experimental v2 path (PreviewV2View.swift); v1 below stays the default
                    .modifier(PreviewMagnify()) // pinch to zoom (PreviewZoom.swift)
            } else if let result = model.result {
                PreviewView(result: result, dark: model.darkPreview, caretItems: model.caretItems,
                            zoom: model.previewZoom, onFitScale: { model.previewFitScale = $0 },
                            // "the pdf moves to where the changes are happening" (CaretFollow.swift)
                            follow: model.caretFollow.request,
                            onUserScroll: { model.caretFollow.userDidScrollPreview() }) { source, text in
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
                VStack(alignment: .leading, spacing: 8) {
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
                .padding(16).frame(minWidth: 480)
                .accessibilityElement(children: .contain).accessibilityLabel("Suggested fix preview")
            }
        }
    }
}

/// The preview column's header: source badge, compile status, freshness
/// (historical / stale / compiling) and the layout capabilities — the former
/// top banner, kept to one line with details in tooltips.
private struct PreviewHeader: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        // Reads the throttled chrome mirror (ShellChrome.swift), not `result`,
        // `editorRevision` or `inFlightRevision`: this header re-evaluated on
        // every keystroke and every reply (FT-071 main-thread sample).
        let chrome = model.chrome
        HStack(spacing: 8) {
            sourceBadge(chrome)
            if chrome.hasResult {
                Text(sourceName(chrome)).font(.caption).lineLimit(1)
                    .help(chrome.resultHelp)
                if model.previewDebugStatus, let status = chrome.resultStatus {
                    Text(status.rawValue).font(.caption.bold()).foregroundStyle(statusColor(status))
                }
                if chrome.resultStatus == .recovered && model.previewDebugStatus {
                    Text("provisional rendering").font(.caption).foregroundStyle(.orange).lineLimit(1).fixedSize()
                        .help("recovered: preview shown with provisional rendering")
                }
                // Fixed-size slot: toggling the indicator never changes the header's layout.
                Color.clear.frame(width: 12, height: 12)
                    .overlay { if chrome.compiling { ProgressView().controlSize(.mini) } }
                if let historical = chrome.historicalLabel {
                    Text(historical).font(.caption.bold()).foregroundStyle(.purple).lineLimit(1)
                        .help("A completed older snapshot is shown while the helper compiles the newer revision; navigation, caret sync, capture destinations and export return with the current preview.")
                } else if let stale = chrome.staleText { // the reply exceeded a bound, or "editor at rN — compiling…" (ShellChrome)
                    Text(stale)
                        .font(.caption).foregroundStyle(chrome.staleHighlighted ? .orange : .secondary).lineLimit(1) // routine "compiling…" is quiet; only bounds/no-producer are highlighted
                }
            } else if let err = chrome.loadError {
                Text(err).font(.caption).foregroundStyle(.red).lineLimit(1).help(err)
            } else {
                Text("Preview").font(.caption.bold()).foregroundStyle(.secondary)
            }
            Spacer()
            PreviewZoomControl() // PreviewZoom.swift: percentage and −/+
            ForEach(chrome.capabilityNotes, id: \.self) { note in
                Image(systemName: "exclamationmark.circle").foregroundStyle(.orange).help(note)
                    .accessibilityLabel(note)
            }
            Text(chrome.acceptedCapabilities.isEmpty ? "legacy layout" : chrome.acceptedCapabilities.joined(separator: ", "))
                .font(.caption2).foregroundStyle(.tertiary).lineLimit(1)
                .help(chrome.acceptedCapabilities.isEmpty
                      ? "No layout capability accepted for this result: U+2500 fraction bars are an approximation."
                      : "Capabilities the worker accepted for this result (typed rules / explicit font hints).")
        }
        .padding(.horizontal, 10).padding(.vertical, 5)
        .background(.bar)
    }

    private func sourceBadge(_ chrome: ShellChrome) -> some View {
        let (label, color): (String, Color) = switch chrome.previewSource {
        case .none: ("NONE", .gray)
        case .fixture: ("FIXTURE", .orange)
        case .worker: chrome.historicalLabel != nil ? ("HISTORICAL", .purple) : ("WORKER", .green)
        }
        return Text(label)
            .font(.caption2.bold())
            .padding(.horizontal, 6).padding(.vertical, 2)
            .background(color.opacity(0.25), in: Capsule())
            .help(chrome.previewSource == .fixture ? "Not a real compile." : model.producerSummary)
    }

    private func sourceName(_ chrome: ShellChrome) -> String {
        switch chrome.previewSource {
        case .none: "—"
        case .fixture: chrome.fixtureName ?? "fixture"
        case .worker(let name): name
        }
    }

    private func statusColor(_ s: RuntimeV1.Status) -> Color {
        switch s { case .ok: .green; case .recovered: .orange; case .failed: .red }
    }
}

/// Bottom status bar: editor revision, the helper's durable revision of the
/// active document, compile latency, the route (fixture / worker /
/// controller), then the last navigation, staleness or capture note, and an
/// exact-export progress control while one runs.
private struct StatusBar: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        // Reads the throttled chrome mirror (ShellChrome.swift): the revision,
        // latency, route tooltip, problem counts and notes change on every
        // keystroke / reply, and this bar re-evaluated with each of them.
        let chrome = model.chrome
        HStack(spacing: 12) {
            Label("r\(chrome.editorRevision)", systemImage: "pencil.line")
                .help("Editor revision (increments on every edit)")
            if let durable = chrome.durableRevision {
                Label("durable r\(durable)", systemImage: "internaldrive")
                    .help("Durable revision of \(model.activePath) in the helper's edit ledger")
            }
            if let ms = chrome.lastLatencyMs {
                Label(String(format: "%.0f ms", ms), systemImage: "timer")
                    .help(chrome.latencyHelp)
            }
            Label(route(chrome), systemImage: routeIcon(chrome))
                .help(chrome.routeHelp)
            WordCountStatusItem() // GH68: live word count + breakdown popover (WordCountStatusView.swift)
            VimModeStatusItem() // -- NORMAL -- / -- INSERT -- / -- VISUAL -- and the `:` line while Vim keybindings are on (VimMode.swift)
            let problems = chrome.problems
            if !problems.isEmpty {
                Button {
                    model.problemsVisible.toggle()
                } label: {
                    HStack(spacing: 6) {
                        if problems.errors > 0 { Label("\(problems.errors)", systemImage: "xmark.octagon.fill").foregroundStyle(.red) }
                        if problems.warnings > 0 { Label("\(problems.warnings)", systemImage: "exclamationmark.triangle.fill").foregroundStyle(.orange) }
                        if problems.gaps > 0 { Label("\(problems.gaps)", systemImage: "puzzlepiece.extension").foregroundStyle(.secondary) }
                    }
                }
                .buttonStyle(.plain)
                .help("Errors, warnings and not-implemented gaps of the last result — click to show or hide the Problems panel (⌘⇧M)")
            }
            Divider().frame(height: 12)
            Text(chrome.note ?? "Click text in the preview to select its source range.")
                .foregroundStyle(.secondary).lineLimit(1)
            Spacer()
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
        .font(.caption)
        .monospacedDigit()
        .padding(.horizontal, 12).padding(.vertical, 4)
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
        VStack(alignment: .leading, spacing: 10) {
            Text("Review capture \(proposal.captureId)").font(.headline)
            if model.isBridgeCapture(proposal.captureId) {
                Text("Bridge capture: approval asks the bridge for a prepared edit, verifies revision, SHA-256 and removed text, then inserts once. The LaTeX must stay as proposed.")
                    .font(.caption).foregroundStyle(.secondary)
                if let d = model.bridgeDestination {
                    Text("Bridge destination \(d.destinationId): \(d.path) bytes \(d.startByte)..<\(d.endByte)").font(.caption).foregroundStyle(.secondary)
                }
                if let cr = proposal.contextRevision {
                    Text("Context revision \(cr)\(cr == model.editorRevision ? "" : " (editor is at \(model.editorRevision))")").font(.caption).foregroundStyle(cr == model.editorRevision ? Color.secondary : Color.orange)
                }
            }
            if let a = model.anchor {
                Text("Inserts at \(a.path) byte \(a.byteOffset) (anchor \(a.id))").font(.caption).foregroundStyle(.secondary)
            } else {
                Label("No insertion point pinned — approve will fail until you pin one.", systemImage: "exclamationmark.triangle")
                    .font(.caption).foregroundStyle(.orange)
            }
            TextEditor(text: $latex)
                .font(.system(.body, design: .monospaced))
                .frame(minHeight: 140)
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
                    .font(.callout).foregroundStyle(.orange)
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
        .padding(16)
        .frame(width: 520)
        .onAppear { latex = proposal.latex; preview.update(from: model, latex: proposal.latex) }
        .onChange(of: latex) { _, new in failure = nil; preview.update(from: model, latex: new) }
        .onChange(of: model.editorRevision) { _, _ in preview.update(from: model, latex: latex) }
        .onChange(of: model.anchor) { _, _ in preview.update(from: model, latex: latex) }
        .onDisappear { preview.close() }
    }
}

/// Status-bar mode indicator for Vim keybindings (VimMode.swift): hidden
/// while the preference is off.
struct VimModeStatusItem: View {
    var body: some View {
        let status = VimMode.Status.shared
        if let indicator = status.indicator {
            Text(indicator)
                .fontWeight(.semibold)
                .help("Vim keybindings are on (Settings, or View > Toggle Vim Keybindings ⌃⌘V)")
                .accessibilityIdentifier("status.vimMode")
            if let line = status.commandLine, !line.isEmpty {
                Text(line).lineLimit(1).accessibilityIdentifier("status.vimCommandLine")
            }
        }
    }
}
