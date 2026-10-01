import FlashTeXPadKit
import NearbyClient
import PencilKit
import PhotosUI
import SwiftUI

/// Primary screen, one tap to the Mac (lane mac-capture-fluid): draw with
/// Apple Pencil (or a finger), take a photo with the camera (the Photos
/// picker where there is no camera, e.g. the simulator), pick an instruction
/// chip or type one, tap **Send**. Prepare and Send are one step: the PNG is
/// rendered, validated (`CaptureQueue.validate`), drafted to disk and sent as
/// a transfer-v1 `capture_submit`; the Captures panel follows the Mac's
/// states (received → converting → proposal ready → inserted) and shows the
/// returned LaTeX/TikZ read-only. The Mac converts and the Mac user approves
/// insertion — from here (Insert) or on the Mac.
///
/// Layout: the canvas fills the whole scene, edge to edge (`PencilCanvas`,
/// ignoring the safe area); every control floats over it inside the safe
/// area — sidebar and connection at the top-leading corner, canvas actions at
/// the top-trailing corner, the instruction + Send composer under them, the
/// Captures panel at the trailing edge (a sheet in compact width). The
/// controls fade while a stroke is being drawn, and the chevron collapses
/// them to one button. PencilKit's tool picker keeps the bottom edge.
///
/// Gestures (`CanvasGestures.swift`): two-finger double-tap undoes,
/// three-finger double-tap redoes, and the Apple Pencil double-tap undoes
/// (or follows the system setting), each switchable in the settings popover.
struct CaptureView: View {
    @EnvironmentObject var model: PadModel
    /// Reveals the navigation sidebar, which the full-screen canvas hides.
    var showSidebar: () -> Void = {}

    @StateObject private var canvas = CanvasController()
    @Environment(\.horizontalSizeClass) private var sizeClass
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    @AppStorage(CanvasSettings.twoFingerUndoKey) private var twoFingerUndo = true
    @AppStorage(CanvasSettings.threeFingerRedoKey) private var threeFingerRedo = true
    @AppStorage(CanvasSettings.pencilDoubleTapKey) private var pencilDoubleTap = PencilDoubleTapMode.undo
    @AppStorage(CanvasSettings.fingerDrawingKey) private var fingerDrawing = true
    @AppStorage(CanvasSettings.autoHideControlsKey) private var autoHideControls = true
    static let capturesPanelOpenKey = "flashtexpad.capture.capturesPanelOpen"
    @AppStorage(CaptureView.capturesPanelOpenKey) private var capturesOpen = false

    @State private var photo: PhotosPickerItem?
    @State private var picked: UIImage?
    @State private var pickedSource: CaptureRecord.Source = .photo
    @State private var instructions = PadModel.defaultInstructions[0]
    @State private var problem: String?
    @State private var sending = false
    @State private var cameraShown = false
    @State private var settingsShown = false
    @State private var collapsed = false
    @State private var fadedForDrawing = false
    @State private var fadeWork: DispatchWorkItem?

    static var cameraAvailable: Bool { UIImagePickerController.isSourceTypeAvailable(.camera) }

    private var settings: CanvasSettings {
        CanvasSettings(twoFingerUndo: twoFingerUndo, threeFingerRedo: threeFingerRedo, pencilDoubleTap: pencilDoubleTap,
                       fingerDrawing: fingerDrawing, autoHideControls: autoHideControls)
    }

    private var sidePanel: Bool { sizeClass != .compact }

    var body: some View {
        // The canvas sizes the screen; everything else is an overlay on it,
        // so no control can ever push the canvas off the edges.
        PencilCanvas(controller: canvas)
            .ignoresSafeArea()
            .accessibilityIdentifier("capture.canvas")
            .overlay { overlays }
            .animation(reduceMotion ? nil : .easeInOut(duration: 0.2), value: canvas.toast)
            .animation(.easeInOut(duration: 0.2), value: fadedForDrawing)
        // Regular width: no bar, the canvas is the whole window. Compact
        // width: the split view is a stack, so keep the bar and its back
        // button as a second way out besides the floating sidebar button.
        .toolbar(sidePanel ? .hidden : .visible, for: .navigationBar)
        .navigationTitle("Capture")
        .onAppear { canvas.settings = settings }
        .onChange(of: settings) { _, s in canvas.settings = s }
        .onChange(of: canvas.isDrawing) { _, drawing in fade(drawing) }
        .onChange(of: photo) { _, item in
            guard let item else { return }
            Task {
                if let data = try? await item.loadTransferable(type: Data.self), let img = UIImage(data: data) {
                    picked = img; pickedSource = .photo
                }
            }
        }
        .fullScreenCover(isPresented: $cameraShown) {
            CameraPicker { img in picked = img; pickedSource = .photo; cameraShown = false } onCancel: { cameraShown = false }
                .ignoresSafeArea()
        }
        .sheet(isPresented: Binding(get: { capturesOpen && !sidePanel }, set: { if !$0 { capturesOpen = false } })) {
            NavigationStack {
                CapturesList()
                    .navigationTitle("Captures")
                    .navigationBarTitleDisplayMode(.inline)
                    .toolbar { Button("Done") { capturesOpen = false } }
            }
            .presentationDetents([.medium, .large])
        }
    }

    private var overlays: some View {
        ZStack {
            if let img = picked { pickedImage(img) }

            controls
                .opacity(fadedForDrawing ? 0 : 1)
                .allowsHitTesting(!fadedForDrawing)

            if let t = canvas.toast {
                Text(t.text)
                    .font(.headline)
                    .padding(.horizontal, 18).padding(.vertical, 10)
                    .floatingChrome(Capsule())
                    .transition(.opacity.combined(with: .scale(scale: 0.9)))
                    .id(t.id)
                    .accessibilityIdentifier("capture.toast")
                    .allowsHitTesting(false)
            }
        }
    }

    /// Controls fade out while the Pencil is down and come back shortly after
    /// it lifts, so they never sit on top of a stroke being drawn.
    private func fade(_ drawing: Bool) {
        fadeWork?.cancel()
        guard autoHideControls else { fadedForDrawing = false; return }
        if drawing { fadedForDrawing = true; return }
        let work = DispatchWorkItem { fadedForDrawing = false }
        fadeWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.6, execute: work)
    }

    // MARK: floating controls

    private var controls: some View {
        VStack(alignment: .leading, spacing: 10) {
            // Side by side with the image sources inline, then with them in
            // a menu, then stacked (a narrow Split View window).
            ViewThatFits(in: .horizontal) {
                HStack(alignment: .top, spacing: 10) {
                    leadingCluster
                    Spacer(minLength: 8)
                    trailingCluster(imageSourcesInline: true)
                }
                HStack(alignment: .top, spacing: 10) {
                    leadingCluster
                    Spacer(minLength: 8)
                    trailingCluster(imageSourcesInline: false)
                }
                VStack(alignment: .leading, spacing: 8) {
                    leadingCluster
                    trailingCluster(imageSourcesInline: false)
                }
            }
            HStack(alignment: .top, spacing: 10) {
                if !collapsed {
                    composer.frame(maxWidth: 560, alignment: .leading)
                }
                Spacer(minLength: 0)
                if capturesOpen, sidePanel, !collapsed {
                    capturesCard
                }
            }
            .frame(maxHeight: .infinity, alignment: .top)
        }
        .padding(.horizontal, 12)
        .padding(.top, 8)
        .padding(.bottom, 12)
    }

    private var leadingCluster: some View {
        HStack(spacing: 8) {
            chromeButton("Show sidebar", "sidebar.left", id: "capture.sidebar") { showSidebar() }
            connectionStatus
        }
        .padding(.leading, 4).padding(.trailing, 12).padding(.vertical, 4)
        .floatingChrome(Capsule())
    }

    private var connectionStatus: some View {
        HStack(spacing: 6) {
            Circle().fill(model.link.isConnected ? Color.green : Color.secondary.opacity(0.6))
                .frame(width: 8, height: 8)
                .accessibilityHidden(true)
            VStack(alignment: .leading, spacing: 0) {
                Text(model.link.isConnected ? "Connected to \(model.pairedMac?.macName ?? "Mac")"
                     : "Not connected — \(model.linkStatus)\(model.linkError.map { ": \($0)" } ?? "") — pair in Mac link")
                    .font(.footnote.weight(.medium))
                    .foregroundStyle(model.link.isConnected ? .primary : .secondary)
                    .lineLimit(1).truncationMode(.tail)
                    .accessibilityIdentifier("capture.connection")
                Group {
                    if let d = model.destination {
                        Text("→ \(d.path) rev \(d.baseRevision)").font(.caption2.monospaced())
                            .help("destination \(d.destinationId)")
                    } else {
                        Text("→ the Mac's caret").font(.caption2)
                    }
                }
                .foregroundStyle(.secondary).lineLimit(1)
            }
            .frame(maxWidth: 300, alignment: .leading)
            if !model.link.isConnected, model.pairedMac != nil, !model.reconnecting {
                Button("Reconnect") { Task { await model.autoReconnect() } }.buttonStyle(.bordered).controlSize(.small)
                    .accessibilityIdentifier("capture.reconnect")
            }
            if model.reconnecting { ProgressView().controlSize(.small) }
        }
    }

    private func trailingCluster(imageSourcesInline: Bool) -> some View {
        HStack(spacing: 4) {
            if !collapsed {
                actionButtons(imageSourcesInline: imageSourcesInline)
                Divider().frame(height: 22)
            }
            chromeButton(collapsed ? "Show controls" : "Hide controls",
                         collapsed ? "chevron.down" : "chevron.up", id: "capture.collapse") {
                withAnimation(.easeInOut(duration: 0.2)) { collapsed.toggle() }
            }
        }
        .padding(.horizontal, 6).padding(.vertical, 4)
        .floatingChrome(Capsule())
    }

    private func actionButtons(imageSourcesInline: Bool) -> some View {
        HStack(spacing: 4) {
            chromeButton("Undo", "arrow.uturn.backward", id: "capture.undo") { canvas.undo() }
                .disabled(!canvas.canUndo)
            chromeButton("Redo", "arrow.uturn.forward", id: "capture.redo") { canvas.redo() }
                .disabled(!canvas.canRedo)
            chromeButton(canvas.toolsVisible ? "Hide tools" : "Show tools", "pencil.tip.crop.circle", id: "capture.tools",
                         selected: canvas.toolsVisible) { canvas.toolsVisible.toggle() }
            if imageSourcesInline {
                imageSourceButtons(inline: true)
            } else {
                Menu {
                    imageSourceButtons(inline: false)
                } label: {
                    chromeLabel("Add image", "photo.badge.plus")
                }
                .accessibilityIdentifier("capture.imageMenu")
            }
            chromeButton("Clear", "trash", id: "capture.clear") {
                canvas.clear(); picked = nil; problem = nil
                canvas.toolsVisible = true
            }
            chromeButton(capturesOpen ? "Hide captures" : "Show captures", "tray.full", id: "capture.capturesToggle",
                         selected: capturesOpen) { capturesOpen.toggle() }
                .overlay(alignment: .topTrailing) {
                    if !model.captures.isEmpty {
                        Text("\(model.captures.count)").font(.caption2.bold()).foregroundStyle(.white)
                            .padding(.horizontal, 4).background(Capsule().fill(Color.accentColor))
                            .offset(x: 2, y: -2).accessibilityHidden(true)
                    }
                }
            chromeButton("Canvas settings", "gearshape", id: "capture.settings") { settingsShown = true }
                .popover(isPresented: $settingsShown) { settingsForm }
        }
    }

    /// Camera (where there is one), Photos and the bundled sample: icon
    /// buttons in the toolbar, or rows of the "Add image" menu when narrow.
    @ViewBuilder private func imageSourceButtons(inline: Bool) -> some View {
        if Self.cameraAvailable {
            Button { cameraShown = true } label: { sourceLabel("Camera", "camera", inline) }
                .accessibilityIdentifier("capture.camera")
        }
        PhotosPicker(selection: $photo, matching: .images) {
            sourceLabel(Self.cameraAvailable ? "Photo…" : "Photo… (no camera here)", "photo", inline)
        }
        .accessibilityIdentifier("capture.photo")
        Button { loadSample() } label: { sourceLabel("Sample image", "photo.on.rectangle", inline) }
            .accessibilityIdentifier("capture.sample")
    }

    @ViewBuilder private func sourceLabel(_ title: String, _ symbol: String, _ inline: Bool) -> some View {
        if inline { chromeLabel(title, symbol) } else { Label(title, systemImage: symbol) }
    }

    /// A 36 pt icon with a full-circle hit area (an icon-only label alone
    /// is only hittable on its glyph, and a miss would draw on the canvas).
    private func chromeLabel(_ title: String, _ symbol: String, selected: Bool = false) -> some View {
        Label(title, systemImage: symbol)
            .labelStyle(.iconOnly)
            .frame(width: 36, height: 36)
            .background(Circle().fill(selected ? Color.accentColor.opacity(0.18) : .clear))
            .contentShape(Circle())
    }

    private func chromeButton(_ title: String, _ symbol: String, id: String, selected: Bool = false,
                              action: @escaping () -> Void) -> some View {
        Button(action: action) { chromeLabel(title, symbol, selected: selected) }
        .accessibilityIdentifier(id)
        .hoverEffect(.highlight)
    }

    private var composer: some View {
        VStack(alignment: .leading, spacing: 8) {
            // One instruction field with recent-instruction chips.
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 6) {
                    ForEach(model.recentInstructions, id: \.self) { chip in
                        Button(chip) { instructions = chip }
                            .buttonStyle(.bordered).controlSize(.small)
                            .tint(chip.caseInsensitiveCompare(instructions) == .orderedSame ? .accentColor : .secondary)
                            .accessibilityIdentifier("capture.chip.\(chip)")
                    }
                }
            }
            .accessibilityIdentifier("capture.chips")
            HStack(spacing: 8) {
                TextField("Instruction for the Mac, e.g. “convert this to TikZ”", text: $instructions, axis: .vertical)
                    .lineLimit(1...3)
                    .textFieldStyle(.roundedBorder)
                    .accessibilityIdentifier("capture.instructions")
                Text("\(canvas.strokeCount) stroke\(canvas.strokeCount == 1 ? "" : "s")")
                    .font(.footnote.monospacedDigit()).foregroundStyle(.secondary)
                    .fixedSize()
                    .accessibilityIdentifier("capture.strokes")
                Button { Task { await send() } } label: {
                    Label(sending ? "Sending…" : "Send", systemImage: "paperplane.fill")
                }
                .buttonStyle(.borderedProminent)
                .disabled(sending)
                .accessibilityIdentifier("capture.send")
            }
            if let p = problem {
                Text(p).foregroundStyle(.red).font(.footnote).accessibilityIdentifier("capture.problem")
            } else if !model.link.isConnected {
                Text("not connected").font(.footnote).foregroundStyle(.secondary)
            }
        }
        .padding(10)
        .floatingChrome(RoundedRectangle(cornerRadius: 18, style: .continuous))
    }

    private var capturesCard: some View {
        CapturesList()
            .scrollContentBackground(.hidden)
            .frame(width: 380)
            .frame(maxHeight: .infinity)
            .clipShape(RoundedRectangle(cornerRadius: 18, style: .continuous))
            .floatingChrome(RoundedRectangle(cornerRadius: 18, style: .continuous))
            .transition(.move(edge: .trailing).combined(with: .opacity))
    }

    private var settingsForm: some View {
        NavigationStack {
            Form {
                Section {
                    Toggle("Two-finger double-tap to undo", isOn: $twoFingerUndo)
                        .accessibilityIdentifier("canvas.settings.twoFingerUndo")
                    Toggle("Three-finger double-tap to redo", isOn: $threeFingerRedo)
                        .accessibilityIdentifier("canvas.settings.threeFingerRedo")
                    Picker("Apple Pencil double-tap", selection: $pencilDoubleTap) {
                        ForEach(PencilDoubleTapMode.allCases) { Text($0.label).tag($0) }
                    }
                    .accessibilityIdentifier("canvas.settings.pencilDoubleTap")
                } header: {
                    Text("Gestures")
                } footer: {
                    Text("“System setting” follows Settings › Apple Pencil › Double-tap (normally switch to the eraser). If double-tap is turned off there, it does nothing here either.")
                }
                Section {
                    Toggle("Draw with a finger", isOn: $fingerDrawing)
                        .accessibilityIdentifier("canvas.settings.fingerDrawing")
                    Toggle("Hide controls while drawing", isOn: $autoHideControls)
                        .accessibilityIdentifier("canvas.settings.autoHideControls")
                } header: {
                    Text("Canvas")
                } footer: {
                    Text("With finger drawing off, one finger scrolls and only Apple Pencil draws.")
                }
            }
            .navigationTitle("Canvas")
            .navigationBarTitleDisplayMode(.inline)
        }
        .frame(minWidth: 380, minHeight: 560)
    }

    private func pickedImage(_ img: UIImage) -> some View {
        VStack(spacing: 12) {
            Image(uiImage: img).resizable().scaledToFit()
                .background(Color.white)
                .clipShape(RoundedRectangle(cornerRadius: 12))
                .shadow(radius: 8)
                .accessibilityIdentifier("capture.pickedImage")
                .accessibilityLabel("Image to send")
            Button("Back to canvas") { picked = nil }.buttonStyle(.bordered)
        }
        .padding(.horizontal, 40).padding(.top, 160).padding(.bottom, 40)
        .frame(maxWidth: 900, maxHeight: .infinity)
    }

    func loadSample() {
        guard let url = Bundle.main.url(forResource: "sample-capture", withExtension: "png"), let data = try? Data(contentsOf: url),
              let img = UIImage(data: data) else { problem = "sample-capture.png missing from bundle"; return }
        picked = img; pickedSource = .sample
    }

    /// Renders the canvas (or the picked image) to PNG; the model validates,
    /// drafts and sends in one step (`PadModel.sendNow`).
    func send() async {
        problem = nil
        guard let (png, source, size) = renderCapture() else { return }
        sending = true
        defer { sending = false }
        switch await model.sendNow(png: png, source: source, instructions: instructions, pixelSize: (width: size.0, height: size.1)) {
        case .success:
            canvas.toolsVisible = false // hide the PencilKit tool picker so the status list is readable
            canvas.reset(); picked = nil
            withAnimation(.easeInOut(duration: 0.2)) { capturesOpen = true; collapsed = false }
        case .failure(let why):
            problem = "\(why)"
        }
    }

    func renderCapture() -> (Data, CaptureRecord.Source, (Int, Int))? {
        if let img = picked {
            guard let d = img.pngData() else { problem = "could not encode the image as PNG"; return nil }
            return (d, pickedSource, (Int(img.size.width * img.scale), Int(img.size.height * img.scale)))
        }
        switch canvas.renderPNG() {
        case .success(let r): return (r.png, .pencil, r.pixels)
        case .failure(.empty): problem = "draw something first (or take a photo)"; return nil
        case .failure(.encoding): problem = "could not encode the drawing as PNG"; return nil
        }
    }
}

/// Floating control surface: Liquid Glass on iOS 26, a material elsewhere.
private struct FloatingChrome<S: Shape>: ViewModifier {
    let shape: S
    func body(content: Content) -> some View {
        if #available(iOS 26.0, *) {
            content.glassEffect(.regular.interactive(), in: shape)
        } else {
            content
                .background(.regularMaterial, in: shape)
                .overlay(shape.stroke(Color.primary.opacity(0.08)))
                .shadow(color: .black.opacity(0.12), radius: 8, y: 2)
        }
    }
}

extension View {
    func floatingChrome<S: Shape>(_ shape: S) -> some View { modifier(FloatingChrome(shape: shape)) }
}

/// In-app camera (AVFoundation through `UIImagePickerController`), shown only
/// where `isSourceTypeAvailable(.camera)`; the simulator gets the Photos picker.
///
/// `UIImagePickerController` owns its capture session, so — as in
/// `PairingScannerView` — continuous autofocus is asserted on the shared
/// `AVCaptureDevice` once the picker's session has started, and re-asserted
/// on subject-area changes. Unlike the QR scanner this does NOT restrict the
/// range to `.near`: a capture may be a whiteboard across the room as easily
/// as a page on the desk.
struct CameraPicker: UIViewControllerRepresentable {
    let onImage: (UIImage) -> Void
    let onCancel: () -> Void

    func makeUIViewController(context: Context) -> UIImagePickerController {
        let c = UIImagePickerController()
        c.sourceType = .camera
        c.cameraCaptureMode = .photo
        c.delegate = context.coordinator
        context.coordinator.startFocusing(after: PairingScannerView.focusSettleDelay)
        return c
    }
    func updateUIViewController(_ c: UIImagePickerController, context: Context) {}
    func makeCoordinator() -> Coordinator { Coordinator(self) }

    static func dismantleUIViewController(_ c: UIImagePickerController, coordinator: Coordinator) {
        coordinator.stopFocusing()
    }

    final class Coordinator: NSObject, UIImagePickerControllerDelegate, UINavigationControllerDelegate {
        let parent: CameraPicker
        private var focus: CameraFocus.Reapplier?
        private var focusWork: DispatchWorkItem?
        init(_ p: CameraPicker) { parent = p }

        func startFocusing(after delay: TimeInterval) {
            let work = DispatchWorkItem { [weak self] in
                self?.focus = CameraFocus.Reapplier(nearRange: false)
            }
            focusWork = work
            DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: work)
        }

        func stopFocusing() {
            focusWork?.cancel()
            focusWork = nil
            focus = nil
        }
        func imagePickerController(_ picker: UIImagePickerController, didFinishPickingMediaWithInfo info: [UIImagePickerController.InfoKey: Any]) {
            if let img = (info[.editedImage] ?? info[.originalImage]) as? UIImage { parent.onImage(img) } else { parent.onCancel() }
        }
        func imagePickerControllerDidCancel(_ picker: UIImagePickerController) { parent.onCancel() }
    }
}

struct CapturesList: View {
    @EnvironmentObject var model: PadModel

    var body: some View {
        List {
            Section("Captures (\(model.captures.count))") {
                if model.captures.isEmpty { Text("none yet").foregroundStyle(.secondary) }
                ForEach(model.captures) { c in
                    HStack(alignment: .top) {
                        if let img = UIImage(data: c.png) {
                            Image(uiImage: img).resizable().scaledToFit().frame(width: 72, height: 54).background(Color.white).border(.gray.opacity(0.3))
                        }
                        VStack(alignment: .leading, spacing: 2) {
                            Text(c.id).font(.caption.monospaced())
                            Text("\(c.source.rawValue) · \(c.png.count) bytes · “\(c.instructions)”").font(.caption).foregroundStyle(.secondary)
                            Text(c.status.label).font(.footnote).accessibilityIdentifier("capture.status.\(c.id)")
                            if case .received(let r) = c.status {
                                Text(verbatim: "capture_received capture_id=\(r.captureId) durable=\(r.durable) has_proposal=\(r.hasProposal) applied=\(r.applied)")
                                    .font(.caption2.monospaced()).foregroundStyle(.secondary)
                                if let d = c.destinationId, let rev = c.baseRevision {
                                    Text("sent for destination \(d) base_revision \(rev)").font(.caption2.monospaced()).foregroundStyle(.secondary)
                                }
                                if c.outcome?.state == "inserted" {
                                    HStack(spacing: 4) {
                                        Image(systemName: "checkmark.circle.fill")
                                        Text("Inserted on Mac ✓").accessibilityIdentifier("capture.inserted.\(c.id)")
                                    }
                                    .font(.footnote.bold()).foregroundStyle(.green)
                                    .transition(.scale.combined(with: .opacity))
                                }
                                if let label = c.outcomeLabel {
                                    Text("Mac: \(label)").font(.footnote).foregroundStyle(c.outcomeIsFinal ? .primary : .secondary)
                                        .accessibilityIdentifier("capture.outcome.\(c.id)")
                                    if let o = c.outcome {
                                        Text(verbatim: "capture_status_ack state=\(o.state) durable=\(o.durable)" + (o.note.map { " note=“\($0)”" } ?? ""))
                                            .font(.caption2.monospaced()).foregroundStyle(.secondary)
                                    }
                                } else {
                                    Text("Mac: waiting for the first capture_status reply…").font(.footnote).foregroundStyle(.secondary)
                                        .accessibilityIdentifier("capture.outcome.\(c.id)")
                                }
                                if let latex = c.outcome?.latex {
                                    Text(c.reviewableLatex == nil
                                         ? "Returned LaTeX/TikZ (read-only):"
                                         : "Returned LaTeX/TikZ — read it, then Insert:")
                                        .font(.caption2).foregroundStyle(.secondary)
                                    Text(latex).font(.caption.monospaced()).padding(6)
                                        .frame(maxWidth: .infinity, alignment: .leading)
                                        .background(Color.gray.opacity(0.12)).cornerRadius(4)
                                        .accessibilityIdentifier("capture.latex.\(c.id)")
                                        .accessibilityLabel("returned latex \(latex)")
                                }
                                // The Insert button: approving the proposal
                                // shown right above, from here, instead of
                                // walking to the Mac to click Insert again.
                                // It appears only once there *is* a proposal
                                // to read — nothing is ever inserted without
                                // this tap (transfer-v1).
                                if c.reviewableLatex != nil {
                                    if let wrap = model.destination?.caretContext?.label {
                                        Text("Will land in: \(wrap)").font(.caption2).foregroundStyle(.secondary)
                                            .accessibilityIdentifier("capture.wrap.\(c.id)")
                                    }
                                    Button {
                                        Task { await model.insertCapture(c.id) }
                                    } label: {
                                        Label(c.inserting ? "Inserting…" : "Insert on Mac", systemImage: "text.insert")
                                    }
                                    .buttonStyle(.borderedProminent).font(.caption)
                                    .disabled(c.inserting || !model.link.isConnected)
                                    .accessibilityIdentifier("capture.insert.\(c.id)")
                                }
                                if let problem = c.insertProblem {
                                    Text(problem).font(.caption2).foregroundStyle(.red)
                                        .accessibilityIdentifier("capture.insertproblem.\(c.id)")
                                }
                                if !c.outcomeIsFinal {
                                    Button("Refresh status") { Task { await model.refreshOutcome(c.id) } }.buttonStyle(.bordered).font(.caption)
                                        .disabled(!model.link.isConnected).accessibilityIdentifier("capture.refresh.\(c.id)")
                                }
                            }
                            if case .refused(_, let msg) = c.status { Text(msg).font(.caption2).foregroundStyle(.red) }
                            if case .disconnected = c.status {
                                Button("Retry (same capture_id)") { Task { await model.send(c.id) } }.buttonStyle(.bordered).font(.caption)
                                    .accessibilityIdentifier("capture.retry.\(c.id)")
                            }
                            if case .drafted = c.status {
                                HStack {
                                    Button("Send") { Task { await model.send(c.id) } }.buttonStyle(.borderedProminent).font(.caption).disabled(!model.link.isConnected)
                                    Button("Discard") { model.discard(c.id) }.buttonStyle(.bordered).font(.caption)
                                }
                            }
                        }
                    }
                    .accessibilityIdentifier("capture.row.\(c.id)")
                    .animation(.default, value: c.outcome?.state)
                }
            }
            Section {
                Text("Status is what nearby-v1 returns to a companion: the capture_received receipt (durable / has_proposal / applied at receipt time) or an error code, then the Mac-side outcome from capture_status (journaled → converting → proposal ready → inserted / rejected / failed) polled every 2 s until final. The LaTeX/TikZ is shown so it can be read before it is approved: Insert sends capture_insert, which names the exact text shown here by its SHA-256, so a Mac whose proposal has moved on refuses rather than inserting something unread. The Mac still applies the one undoable edit, wrapped for wherever its caret is. Approving on the Mac instead still works. Retry after a disconnect re-sends the same capture_id; the Mac de-duplicates. Drafts, receipts and outcomes are kept on this iPad across relaunches.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .accessibilityIdentifier("captures.list")
    }
}
