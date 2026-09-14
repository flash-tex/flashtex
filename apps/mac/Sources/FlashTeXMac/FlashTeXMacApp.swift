import AppKit
import SwiftUI
import FlashTeXAccessibility

/// A bare SwiftPM executable has no bundle, so AppKit defaults to an
/// accessory-style process with no Dock icon and, when launched from a
/// non-GUI context, no visible window. Force a regular, activated app.
final class AppDelegate: NSObject, NSApplicationDelegate {
    /// Set by the App so quitting can check for unsaved edits.
    weak var model: ShellModel?

    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard let model, model.project.anyDirty, model.documentURL != nil || !model.activeText.isEmpty else { return .terminateNow }
        let dirty = model.project.listing.filter(\.isDirty).map(\.path)
        let alert = NSAlert()
        alert.messageText = "Save changes to \(model.documentURL == nil ? "the unsaved buffer" : dirty.joined(separator: ", "))?"
        alert.informativeText = "Your edits since the last save will be lost if you don't save."
        alert.addButton(withTitle: "Save")
        alert.addButton(withTitle: "Don't Save")
        alert.addButton(withTitle: "Cancel")
        switch alert.runModal() {
        case .alertFirstButtonReturn:
            // Non-entry members first (their own rooted files, synchronous
            // compare-and-replace like saveTex), then the entry.
            var allSaved = true
            for path in dirty where path != model.project.entryPath {
                switch model.project.saveDocumentNow(path) {
                case .saved: continue
                case .conflict(let c): allSaved = false; model.captureNote = c.summary
                case .failed(let why): allSaved = false; model.captureNote = why
                }
            }
            if model.project.isDirty(model.project.entryPath) {
                if model.activePath != model.project.entryPath { model.project.switchDocument(to: model.project.entryPath) }
                if !model.saveTex() {
                    allSaved = false
                    // The rooted save helper reported an on-disk conflict: resolve it first.
                    if model.files.conflict != nil { model.resolveConflictPanel() }
                }
            }
            return allSaved && !model.project.anyDirty ? .terminateNow : .terminateCancel
        case .alertSecondButtonReturn:
            // Every dirty member stays recoverable next launch (DirtySnapshots.swift); nothing is written to the files.
            model.preserveDirtyBuffers(reason: "quit without saving")
            return .terminateNow
        default: return .terminateCancel
        }
    }

    /// Held for the app's lifetime: without it App Nap and timer coalescing
    /// quantize the main run loop of a backgrounded window to ~30 ms turns, so
    /// a compile result that arrived in 1 ms waited a whole turn to be applied
    /// (measured with tools/typing-bench: fixture keystroke->paint p50 80 ms).
    private var liveActivity: NSObjectProtocol?

    /// Coming back to the app rechecks the document on disk (external edits
    /// become an explicit conflict state, never a silent overwrite).
    func applicationDidBecomeActive(_ notification: Notification) {
        if let model { Task { @MainActor in await model.refreshDiskStatus() } } // helper route when attached
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        liveActivity = ProcessInfo.processInfo.beginActivity(
            options: [.userInitiatedAllowingIdleSystemSleep, .latencyCritical],
            reason: "Live LaTeX preview follows every keystroke")
        // Automation launches (validation suites, launch-check) must never steal
        // keyboard focus from a person typing at the machine.
        if ProcessInfo.processInfo.environment["FLASHTEX_NO_ACTIVATE"] != "1" {
            NSApp.activate(ignoringOtherApps: true)
            NSApp.windows.first?.makeKeyAndOrderFront(nil)
        } else {
            NSApp.windows.first?.orderBack(nil)
        }
        // Automation: place the main window at an explicit screen frame
        // ("x,y,w,h" in screen points) so evidence captures by window id are
        // not cropped by a restored off-screen frame.
        if let spec = ProcessInfo.processInfo.environment["FLASHTEX_WINDOW_FRAME"] {
            let p = spec.split(separator: ",").compactMap { Double($0.trimmingCharacters(in: .whitespaces)) }
            if p.count == 4 {
                // After SwiftUI restored the saved frame, so the hook wins.
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                    (NSApp.windows.first { $0.title == "FlashTeX" } ?? NSApp.windows.first)?
                        .setFrame(NSRect(x: p[0], y: p[1], width: p[2], height: p[3]), display: true)
                }
            }
        }
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
}

@main
struct FlashTeXMacApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @State private var model = ShellModel()
    @StateObject private var nearby = NearbyState()
    @Environment(\.openWindow) private var openWindow

    var body: some Scene {
        WindowGroup("FlashTeX") {
            ContentView()
                .environment(model)
                .environmentObject(nearby) // Captures inspector: status pill, pairing code (CaptureInbox.swift)
                .frame(minWidth: 1200, minHeight: 640) // sidebar + editor + preview + Problems panel
                .onAppear {
                    appDelegate.model = model; nearby.attach(sink: model, destinations: model); TypingBench.shared.install(model: model)
                    // A paired iPad reconnects at launch without opening any window (mac-capture-fluid).
                    if CaptureInboxFeature.autoAdvertise(pairs: nearby.pairs.count) { nearby.startAdvertising() }
                    // Automation: open a secondary window at launch for evidence captures.
                    if ProcessInfo.processInfo.environment["FLASHTEX_SHOW_PALETTE"] == "1" { model.commandPaletteShown = true } // evidence captures of the command palette
                    // Evidence captures of the completion list: place the caret after the
                    // first occurrence of the given text and open the list (⌃Space) once
                    // the window is up, exactly as a keystroke would.
                    if let needle = ProcessInfo.processInfo.environment["FLASHTEX_SHOW_COMPLETION"], !needle.isEmpty {
                        DispatchQueue.main.asyncAfter(deadline: .now() + 2.5) {
                            guard let tv = TypingBenchDriver.findTextView(in: NSApp.windows.compactMap(\.contentView)) else { return }
                            let r = (tv.string as NSString).range(of: needle)
                            guard r.location != NSNotFound else { return }
                            tv.window?.makeFirstResponder(tv)
                            tv.setSelectedRange(NSRange(location: r.location + r.length, length: 0))
                            tv.scrollRangeToVisible(tv.selectedRange())
                            if let e = NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: .control, timestamp: ProcessInfo.processInfo.systemUptime,
                                                        windowNumber: tv.window?.windowNumber ?? 0, context: nil, characters: " ",
                                                        charactersIgnoringModifiers: " ", isARepeat: false, keyCode: 49) {
                                tv.keyDown(with: e)
                            }
                        }
                    }
                    if let id = ProcessInfo.processInfo.environment["FLASHTEX_OPEN_WINDOW"], ["nearby", AccessibilityHelpView.windowID, EditHistoryPanel.windowID, ProjectSearch.windowID, CitationRename.windowID].contains(id) { openWindow(id: id) }
                }
        }
        .defaultSize(width: 1500, height: 950) // first launch; the saved frame wins afterwards
        // The menu bar is part of the App scene graph: any model property a
        // command reads re-evaluates the whole scene when it changes, and SwiftUI
        // then re-reads every window's root preferences (toolbar, title…) —
        // `AppKitWindowController.updateRootView` was ~20 % of the main thread
        // while typing (FT-071 sample) because `result`/`displayListV2` were
        // read here per reply. Commands read the change-only mirrors instead.
        .commands {
            NavigationCommands(model: model) // Navigation.swift
            DiagnosticsCommands(model: model) // DiagnosticsPanel.swift: Edit > Copy Diagnostics as Text (⌘⌥C)
            FindCommands() // EditorFind.swift: Edit > Find submenu (⌘F, ⌥⌘F, ⌘G, ⇧⌘G, ⌘E, ⌘J)
            ProjectSearchCommands(openWindow: openWindow) // ProjectSearchPanel.swift: ⌘⇧F Find in Project…
            CitationRenameCommands(openWindow: openWindow) // CitationRename.swift: Edit > Rename Citation… (no shortcut)
            CommandGroup(after: .toolbar) {
                Toggle("Show Preview Debug Status", isOn: Binding(get: { model.previewDebugStatus }, set: { model.previewDebugStatus = $0 }))
                    .help("Show the compile status word, provisional-rendering note, display-list identity line and display-list diagnostics in the preview pane (off by default).")
                // Helper display-candidate route (ShellModel+DisplayCandidates.swift): default OFF; untrusted v2 siblings painted in the v2 pane.
                Toggle("Helper Display Candidates", isOn: Binding(get: { model.displayCandidates.requested }, set: { model.setDisplayCandidates($0) }))
                    .disabled(!model.controllerAttached)
                    .help("Ask the attached preview controller to forward the producer's display-list-v2 sibling (untrusted; validated natively before paint). Status: \(model.displayCandidates.status)")
            }
            CommandGroup(after: .sidebar) {
                // View menu (mac-ui-redesign): the command palette lists every
                // AccessibilityCommand with its shortcut (CommandPalette.swift);
                // the Problems panel is the bottom diagnostics panel (ProblemsPanel.swift).
                Button("Command Palette…") { model.commandPaletteShown.toggle() }
                    .keyboardShortcut("p", modifiers: [.command, .shift])
                Button("Toggle Problems") { model.problemsVisible.toggle() }
                    .keyboardShortcut("m", modifiers: [.command, .shift])
                Button("Toggle Captures") { model.captureInboxVisible.toggle() } // CaptureInbox.swift
                    .keyboardShortcut("i", modifiers: [.command, .shift])
                Button("Toggle Vim Keybindings") { EditorPreferences.shared.vimKeybindings.toggle() } // VimMode.swift: modal editing in the source editor
                    .keyboardShortcut("v", modifiers: [.control, .command])
                Divider()
                // Preview zoom (PreviewZoom.swift): multiplier over fit-to-width.
                Button("Zoom In") { model.previewZoomIn() }
                    .keyboardShortcut("=")
                Button("Zoom Out") { model.previewZoomOut() }
                    .keyboardShortcut("-")
                Button("Actual Size") { model.previewActualSize() }
                    .keyboardShortcut("0")
                Button("Fit Width") { model.previewFitWidth() }
                    .keyboardShortcut("9")
                Divider()
                // Editor text size: EditorPreferences.fontSize (8…36 pt).
                Button("Increase Editor Font Size") { model.increaseEditorFontSize() }
                    .keyboardShortcut("=", modifiers: [.command, .option])
                Button("Decrease Editor Font Size") { model.decreaseEditorFontSize() }
                    .keyboardShortcut("-", modifiers: [.command, .option])
                Button("Reset Editor Font Size") { model.resetEditorFontSize() }
                    .keyboardShortcut("0", modifiers: [.command, .option])
            }
            CommandGroup(after: .help) {
                Button("FlashTeX Accessibility Help") { openWindow(id: AccessibilityHelpView.windowID) }
            }
            CommandGroup(after: .pasteboard) {
                Divider()
                Button("Pin Insertion Point") { model.pinAnchorAtCaret() }
                    .keyboardShortcut("p", modifiers: [.command, .option]) // ⌘⇧P is the command palette (View)
                Button("Open Capture Proposal…") { model.openProposalPanel() } // ⌘⇧I moved to View > Toggle Captures (mac-capture-fluid)
                Button("Restore Discarded Buffer") { model.restoreDiscardedBuffer() }
                    .disabled(model.recoverableBuffer == nil)
                Divider()
                Button("Attach Capture Bridge") { model.attachDiscoveredBridge() }
                Button("Detach Capture Bridge") { model.detachBridge() }
                    .disabled(!model.bridgeAttached)
                Button("Submit Sample Capture…") { model.submitSampleCapturePanel() }
                    .keyboardShortcut("u", modifiers: [.command, .shift])
                    .disabled(!model.bridgeAttached)
                Button("Convert Capture") { model.convertLatestCapture() }
                    .keyboardShortcut("g", modifiers: [.command, .shift])
                    .disabled(model.latestConvertibleCapture == nil)
                Button("Retry Bridge Receipt") { model.retryBridgeReceipt() }
                    .disabled(model.bridge?.pendingTransaction == nil)
                Button("Retry Bridge Reconciliation") { Task { await model.retryBridgeReconciliation() } }
                    .disabled(!model.bridgeAttached)
                Divider()
                Button("Nearby Companion…") { openWindow(id: "nearby") }
                    .keyboardShortcut("n", modifiers: [.command, .shift])
                Button("Durable History…") { openWindow(id: EditHistoryPanel.windowID) } // EditHistoryPanel.swift
            }
            CommandGroup(replacing: .newItem) {
                Button("New Project…") { model.scaffold.presentNewProject() } // ProjectScaffoldViews.swift
                    .keyboardShortcut("n", modifiers: [.command, .option]) // ⌘⇧N is Nearby Companion
                Button("New File…") { model.scaffold.presentNewFile() }
                    .keyboardShortcut("n")
                    .disabled(model.project.projectRoot == nil)
                Button("Open LaTeX File…") { model.openTexPanel() }
                    .keyboardShortcut("o")
                Button("Save") { model.saveTexInteractive() }
                    .keyboardShortcut("s")
                Button("Resolve On-Disk Conflict…") { model.resolveConflictPanel() }
                    .disabled(model.files.conflict == nil)
                Button("Reload From Disk…") { model.reloadFromDiskInteractive() }
                    .disabled(model.documentURL == nil)
                Button("Restore Unsaved Snapshot…") { model.restoreDirtySnapshotsInteractive() } // DirtySnapshots.swift
                    .disabled(model.files.offeredSnapshots.isEmpty)
                Button("Save As…") { model.saveTexAs() }
                    .keyboardShortcut("s", modifiers: [.command, .shift])
                Divider()
                Button("Open Compile Result Fixture…") { model.openFixturePanel() }
                    .keyboardShortcut("o", modifiers: [.command, .shift])
                // Developer-only: no shortcut, and confirms/detaches the real
                // document before replacing it with fixture content (#72).
                Button("Reload Fixture") { model.reloadFixture() }
                Button("Open Display List (v2)…") { model.openDisplayListV2Panel() } // experimental, PreviewV2View.swift
                Button("Export PDF…") { model.exportPDF() }
                    .keyboardShortcut("e", modifiers: [.command, .shift])
                    .disabled(!model.toolbarHasResult)
                Button("Export PDF via Rust Writer…") { model.exportPDFViaRust() }
                    .keyboardShortcut("e", modifiers: [.command, .option])
                    .disabled(!model.toolbarHasResult) // change-only mirror (see .commands)
                Button("Export PDF (exact, v2)…") { model.exportPDFExact() } // ExactPDFExport.swift
                    .disabled(!model.toolbarHasV2Frame) // change-only mirror (see .commands)
                Divider()
                Button("Attach Built Compiler") { model.attachDiscoveredWorker() }
                    .keyboardShortcut("k", modifiers: [.command, .shift])
                Button("Attach Render Pipeline (Latin Modern)") { model.attachDiscoveredRenderPipeline() }
                    .keyboardShortcut("r", modifiers: [.command, .shift])
                    .help("Attach flashtex-render (crates/render-pipeline) — the producer whose metrics are Latin Modern, so the preview shows Computer Modern-style text")
                Button("Attach Worker Executable…") { model.attachWorkerPanel() }
                    .keyboardShortcut("k")
                Button("Compile") { if !model.outputBoundExplicitRetry() { model.compile() } }
                    .keyboardShortcut("b")
                    .disabled(!model.workerAttached)
                Button("Detach Worker") { model.detachWorker() }
                    .disabled(!model.workerAttached)
            }
        }
        .commands {
            // Separate `.commands` so this is not an 11th child of the builder
            // above (SwiftUI's CommandsBuilder limit). Replaces the system Print
            // that would otherwise print the editor view.
            CommandGroup(replacing: .printItem) {
                Button("Print…") { model.printDocument() }
                    .keyboardShortcut("p")
                    .disabled(!model.toolbarHasResult) // change-only mirror (see .commands)
                    .help(PrintController.documentHelp(hasResult: model.toolbarHasResult))
                Button("Print Source…") { model.printSource() }
                    .help(PrintController.sourceHelp(hasDocument: true))
            }
        }
        Window("Nearby Companion", id: "nearby") {
            NearbyView().environmentObject(nearby).environment(model)
        }
        .windowResizability(.contentSize)
        Window("Durable History", id: EditHistoryPanel.windowID) {
            EditHistoryPanel().environment(model) // undo/redo on the helper's ledger
        }
        Window("Accessibility Help", id: AccessibilityHelpView.windowID) {
            AccessibilityHelpView() // FlashTeXAccessibility: focus order, VoiceOver notes, command table
        }
        Settings { EditorPreferencesView() } // EditorPreferences.swift (⌘,)
        ProjectSearchWindow(model: model) // ProjectSearchPanel.swift: Find in Project (⌘⇧F)
        CitationRenameWindow(model: model) // CitationRename.swift: Rename Citation (reviewed plan_citation_rename → apply_group)
    }
}
