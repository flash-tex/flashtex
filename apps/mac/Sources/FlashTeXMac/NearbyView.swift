import AppKit
import Combine
import SwiftUI

// MARK: - flow controller (NearbyState → PairingFlow.Machine → effects)

/// Drives `PairingFlow.Machine` from `NearbyState`'s published values, runs
/// the machine's effects against the transport and the `PairingJournal`, and
/// posts accessibility announcements. One controller per `NearbyState`, kept
/// alive across window open/close so an attempt is tracked (and journaled)
/// even while the window is closed.
///
/// Listener events (`NearbyState.onEvent`: sessions opening and closing, hello,
/// byte progress, captures, refusals) drive the finer states; the published
/// values cover advertising, the transport's own code, and the pair list.
/// Attempt generations come from the journal and are handed to the transport
/// (`beginPairing(resuming:expiresAt:generation:)`), so the generation a stored
/// pairing carries (`PairRecord.generation`, pairs.json v2) is the one the
/// window showed; a hello for another generation is stale.
@MainActor
final class PairingFlowController: ObservableObject {
    @Published private(set) var machine: PairingFlow.Machine
    /// Everything announced so far (VoiceOver text; also test evidence).
    @Published private(set) var announcements: [String] = []
    /// Inputs dropped because they belonged to an older attempt or another pairing.
    @Published private(set) var staleInputs: [PairingFlow.Input] = []
    let journal: PairingJournal
    private(set) weak var nearby: NearbyState?
    /// Posts `text` to assistive technology; tests replace it.
    var announcer: (String) -> Void = { text in
        if #available(macOS 14.0, *) { AccessibilityNotification.Announcement(text).post() }
    }
    private var cancellables: Set<AnyCancellable> = []
    private var expiry: DispatchWorkItem?
    /// `NearbyState.pairingCode` as last observed (nil when the transport's
    /// own code is not being shown, including for resumed attempts).
    private var transportCode: String?
    /// Set right before we ask the transport to withdraw its code, so the
    /// resulting `pairingCode = nil` is not mistaken for expiry.
    private var expectingWithdrawal = false
    private var knownPairIds: Set<String>
    private var lastStatus: String?

    private static let shared = NSMapTable<NearbyState, PairingFlowController>.weakToStrongObjects()

    /// The controller for `nearby`, created on first use with the default journal.
    static func controller(for nearby: NearbyState, journal: PairingJournal? = nil) -> PairingFlowController {
        if let c = shared.object(forKey: nearby) { return c }
        let c = PairingFlowController(nearby: nearby, journal: journal ?? PairingJournal(url: PairingJournal.defaultURL()))
        shared.setObject(c, forKey: nearby)
        return c
    }

    init(nearby: NearbyState, journal: PairingJournal, now: Date = Date(), announcer: ((String) -> Void)? = nil) {
        self.nearby = nearby
        self.journal = journal
        if let announcer { self.announcer = announcer }
        machine = PairingFlow.Machine(phase: nearby.isAdvertising ? .advertising : .off,
                                      generation: journal.generation, isAdvertising: nearby.isAdvertising)
        knownPairIds = Set(nearby.pairs.map(\.pairId))
        lastStatus = nearby.status
        if let e = journal.loadError { announce(e) }
        subscribe(nearby)
        if let pending = journal.pending {
            apply(.restored(pending), now: now)
        }
    }

    var phase: PairingFlow.Phase { machine.phase }

    // MARK: user actions

    func showCode() {
        guard phase.canShowCode(), let nearby else { return }
        // Issue the code with the journal's generation so the transport, the
        // stored record and the window agree on which attempt confirmed.
        let generation = journal.nextGeneration()
        nearby.beginPairing(resuming: Pairing.generateCode(), expiresAt: Date().addingTimeInterval(Pairing.codeLifetime),
                            generation: generation) // → $pairingCode → .codeIssued
    }

    func cancel() { apply(.cancel) }
    func resume() { apply(.resume) }
    func dismiss() { apply(.dismiss) }

    /// The bootstrap payload the QR image carries while a code is shown or
    /// being verified (same code, this Mac's salt and name); nil otherwise.
    var bootstrapPayload: PairingBootstrapPayload? {
        guard let nearby else { return nil }
        switch phase {
        case .codeShown(let a), .verifying(let a):
            return PairingBootstrapPayload(code: a.code, salt: nearby.store.salt, macName: nearby.macName)
        default:
            return nil
        }
    }

    /// "Copy code" / ⌘C on the code: puts the digits (no display spacing) on
    /// `pasteboard` and announces it. False when no code is shown.
    @discardableResult
    func copyCode(to pasteboard: NSPasteboard = .general) -> Bool {
        guard let a = machine.attempt, !a.isExpired(at: Date()) else { return false }
        let text = Pairing.clipboardCode(a.code)
        pasteboard.clearContents()
        guard pasteboard.setString(text, forType: .string) else { return false }
        announce("Pairing code \(Pairing.spokenCode(a.code)) copied.")
        return true
    }

    func setAdvertising(_ on: Bool) {
        guard let nearby else { return }
        if on {
            nearby.startAdvertising()
        } else {
            if transportCode != nil { expectingWithdrawal = true }
            nearby.stopAdvertising()
        }
    }

    func forget(pairId: String) {
        nearby?.forget(pairId: pairId) // → $pairs → .forgotten
    }

    /// Listener events, delivered by `NearbyState.onEvent` on the main actor
    /// before `NearbyState` itself acts on them.
    func observe(_ event: NearbyListener.Event) {
        let g = machine.generation
        switch event {
        case .connectionOpened:
            apply(.bootstrapSessionOpened(generation: g))
        case .hello(let id, let name, let bootstrap):
            guard bootstrap else {
                apply(.otherCompanionConnected(generation: g))
                apply(.companionConnected(pairId: id, companionName: name))
                return
            }
            // The generation that confirmed is persisted with the record
            // (pairs.json v2); a stored value from another attempt is stale.
            let confirmed = nearby?.store.pair(id: id)?.generation
                ?? nearby?.coordinator.confirmedGeneration(pairId: id) ?? g
            apply(.confirmed(pairId: id, companionName: name, generation: confirmed))
        case .connectionClosed(let id, let reason):
            apply(.peerGone(pairId: id, reason: reason, generation: g))
            // A known companion dropping is announced; unauthenticated peers
            // (handshake failures, cancelled bootstrap sessions) have no name.
            if let id, let r = nearby?.store.pair(id: id) {
                apply(.companionDisconnected(pairId: id, companionName: r.companionName, reason: reason))
            }
        case .receiving(let id, let bytes, let expected):
            // Unauthenticated peers (before hello) have no identity and no row.
            guard let id else { return }
            apply(.receiving(pairId: id, companionName: nearby?.store.pair(id: id)?.companionName,
                             captureId: nil, bytes: bytes, total: expected))
        case .capture(let id):
            apply(.captureReceived(pairId: nil, captureId: id))
        case .captureDuplicate(let pairId, let captureId):
            apply(.captureReceived(pairId: pairId, captureId: captureId))
        case .captureRefused(let pairId, let captureId, let code, _):
            apply(.captureRefused(pairId: pairId, captureId: captureId, code: code))
        case .failed(let why):
            apply(.listenerFailed("listener error: \(why)"))
        case .ready, .stopped:
            break
        }
    }

    // MARK: transport observation

    private func subscribe(_ nearby: NearbyState) {
        // Raw listener events first (chained after any earlier consumer).
        let previous = nearby.onEvent
        nearby.onEvent = { [weak self] event in
            previous?(event)
            self?.observe(event)
        }
        // `@Published` publishers fire on willSet with the new value; the
        // store and coordinator are already current at that point.
        nearby.$isAdvertising.dropFirst().removeDuplicates().sink { [weak self] on in
            self?.apply(.advertising(on))
        }.store(in: &cancellables)

        nearby.$pairingCode.dropFirst().sink { [weak self, weak nearby] code in
            guard let self, let nearby else { return }
            self.transportCodeChanged(to: code, nearby: nearby)
        }.store(in: &cancellables)

        nearby.$pairs.dropFirst().sink { [weak self] pairs in
            guard let self else { return }
            let ids = Set(pairs.map(\.pairId))
            if let a = self.machine.attempt, let r = pairs.first(where: { $0.pairId == a.pairId }), !self.knownPairIds.contains(a.pairId) {
                self.apply(.confirmed(pairId: r.pairId, companionName: r.companionName, generation: a.generation))
            }
            for gone in self.knownPairIds.subtracting(ids) { self.apply(.forgotten(pairId: gone)) }
            self.knownPairIds = ids
        }.store(in: &cancellables)

        // `listener failed:` (start() threw) has no event; `.failed` events do.
        nearby.$status.dropFirst().sink { [weak self] status in
            guard let self, status != self.lastStatus else { return }
            self.lastStatus = status
            if status.hasPrefix("listener failed") { self.apply(.listenerFailed(status)) }
        }.store(in: &cancellables)
    }

    private func transportCodeChanged(to code: String?, nearby: NearbyState) {
        guard code != transportCode else { return }
        let previous = transportCode
        transportCode = code
        if let code {
            // A resumed attempt is already the machine's; only a new code is issued.
            if let a = machine.attempt, a.code == code { return }
            let now = Date()
            let pending = nearby.coordinator.current
            // `showCode()` hands the journal's generation to the transport; a
            // code minted elsewhere (`beginPairing()`) carries the coordinator's
            // own counter and gets the next journal generation instead.
            let generation = (pending?.generation).flatMap { $0 > machine.generation ? $0 : nil } ?? journal.nextGeneration()
            let attempt = PairingFlow.Attempt(
                generation: generation, code: code,
                pairId: Pairing.derive(code: code, salt: nearby.store.salt).pairId,
                startedAt: now,
                expiresAt: pending?.expiresAt ?? now.addingTimeInterval(Pairing.codeLifetime))
            apply(.codeIssued(attempt), now: now)
            return
        }
        guard previous != nil, let a = machine.attempt else { expectingWithdrawal = false; return }
        if let r = nearby.store.pair(id: a.pairId) {
            apply(.confirmed(pairId: r.pairId, companionName: r.companionName, generation: a.generation))
        } else if expectingWithdrawal {
            expectingWithdrawal = false // our own cancel or advertising-off; the machine already moved
        } else if let until = nearby.codeExpiresAt, until.timeIntervalSinceNow > 1 {
            apply(.withdrawn(generation: a.generation, reason: "the transport withdrew the code"))
        } else {
            apply(.codeExpired(generation: a.generation))
        }
    }

    // MARK: machine + effects

    @discardableResult
    func apply(_ input: PairingFlow.Input, now: Date = Date()) -> PairingFlow.Outcome {
        var m = machine
        let outcome = m.apply(input, now: now)
        machine = m
        if outcome.stale { staleInputs.append(input) }
        for effect in outcome.effects { perform(effect, now: now) }
        return outcome
    }

    private func perform(_ effect: PairingFlow.Effect, now: Date) {
        switch effect {
        case .persist(let a):
            journal.setPending(a)
            scheduleExpiry(a)
        case .clearJournal:
            journal.setPending(nil)
            expiry?.cancel()
            expiry = nil
        case .cancelTransport(let a):
            expiry?.cancel()
            expiry = nil
            guard let nearby else { return }
            if nearby.pairingCode == a.code {
                expectingWithdrawal = true
                nearby.cancelPairing() // restarts the listener without the bootstrap key
            } else if nearby.coordinator.current?.code == a.code {
                nearby.coordinator.cancel()
                if nearby.isAdvertising { nearby.startAdvertising() }
            }
        case .resumeTransport(let a):
            guard let nearby else { return }
            // Serves the same code with the journal's generation and expiry; the
            // transport publishes it like a fresh code (`transportCodeChanged`
            // recognises the attempt) and runs its own expiry timer.
            nearby.beginPairing(resuming: a.code, expiresAt: a.expiresAt, generation: a.generation)
            journal.setPending(a)
            scheduleExpiry(a)
        case .closeSession(let pairId):
            nearby?.closeConnection(pairId: pairId)
        case .announce(let text):
            announce(text)
        }
    }

    private func announce(_ text: String) {
        announcements.append(text)
        if announcements.count > 50 { announcements.removeFirst(announcements.count - 50) }
        announcer(text)
    }

    private func scheduleExpiry(_ a: PairingFlow.Attempt) {
        expiry?.cancel()
        let item = DispatchWorkItem { [weak self] in self?.expired(a) }
        expiry = item
        DispatchQueue.main.asyncAfter(deadline: .now() + a.remaining(at: Date()) + 0.25, execute: item)
    }

    /// The transport expires its own codes (and restarts without the key);
    /// this timer is the belt to that suspender, and drops an attempt that
    /// only the coordinator still holds.
    private func expired(_ a: PairingFlow.Attempt) {
        apply(.codeExpired(generation: a.generation))
        guard let nearby, nearby.pairingCode == nil, nearby.coordinator.current?.code == a.code else { return }
        nearby.coordinator.cancel()
        if nearby.isAdvertising { nearby.startAdvertising() }
    }
}

// MARK: - window

/// `Edit > Nearby Companion…` (⌘⇧N): advertising, the pairing flow, paired
/// devices and received captures. Everything shown is the real listener state
/// as seen through `PairingFlowController`; captures land in an in-memory
/// inbox until the bridge client takes over.
struct NearbyView: View {
    @EnvironmentObject private var nearby: NearbyState
    @Environment(ShellModel.self) private var model
    @State private var controller: PairingFlowController?
    private static var autostarted = false

    var body: some View {
        Group {
            if let controller {
                NearbyFlowView(controller: controller, nearby: nearby, model: model)
            } else {
                ProgressView().padding()
            }
        }
        .onAppear {
            guard controller == nil else { return }
            let c = PairingFlowController.controller(for: nearby)
            controller = c
            // Automation (evidence runs): `FLASHTEX_NEARBY_AUTOSTART=code` shows a
            // pairing code as soon as the window opens, once per launch.
            if ProcessInfo.processInfo.environment["FLASHTEX_NEARBY_AUTOSTART"] == "code", !Self.autostarted {
                Self.autostarted = true
                c.showCode()
            }
            consumeCodeRequest(c)
        }
        .onChange(of: nearby.codeRequested) { _, _ in if let controller { consumeCodeRequest(controller) } }
    }

    /// The Captures panel asked for a code (`NearbyState.codeRequested`).
    private func consumeCodeRequest(_ c: PairingFlowController) {
        guard nearby.codeRequested else { return }
        nearby.codeRequested = false
        if c.phase.canShowCode() { c.showCode() }
    }
}

struct NearbyFlowView: View {
    @ObservedObject var controller: PairingFlowController
    @ObservedObject var nearby: NearbyState
    var model: ShellModel

    enum Focus: Hashable { case showCode, code, resume, dismiss, cancel }
    @FocusState private var focus: Focus?

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.l) {
            advertisingSection
            Divider()
            pairingSection
            Divider()
            pairedSection
            Divider()
            CapturesSection(inbox: model.nearbyInbox, bridgeCaptures: model.bridgeCaptures,
                            lastCaptureId: nearby.lastReceivedCaptureId)
            if let e = nearby.lastReceiveError {
                Divider()
                refusedSection(e)
            }
            Divider()
            logSection
        }
        .padding(DS.Space.xl)
        .frame(minWidth: DS.Layout.nearbyWindowMinWidth, idealWidth: DS.Layout.nearbyWindowIdealWidth, minHeight: DS.Layout.nearbyWindowMinHeight)
        .onChange(of: controller.phase) { _, phase in moveFocus(for: phase) }
    }

    private func moveFocus(for phase: PairingFlow.Phase) {
        switch phase {
        case .codeShown, .verifying: focus = .code
        case .interrupted(let a, _, _): focus = a.isExpired(at: Date()) ? .dismiss : .resume
        case .failed, .paired: focus = .dismiss
        case .receiving: focus = .cancel
        case .off, .advertising: focus = .showCode
        }
    }

    // MARK: advertising

    private var advertisingSection: some View {
        VStack(alignment: .leading, spacing: DS.Space.s) {
            HStack {
                Text("Nearby companion").font(.headline)
                Spacer()
                Toggle("Advertise", isOn: Binding(
                    get: { nearby.isAdvertising },
                    set: { controller.setAdvertising($0) }))
                .toggleStyle(.switch)
                .accessibilityLabel("Advertise on the local network")
                .accessibilityValue(nearby.isAdvertising ? "on, port \(nearby.port.map(String.init) ?? "unknown")" : "off")
                .accessibilityIdentifier("nearby.advertise")
            }
            Text(nearby.status).font(.caption).foregroundStyle(.secondary)
                .accessibilityLabel("Listener status")
                .accessibilityValue(nearby.status)
                .accessibilityIdentifier("nearby.listener.status")
            Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 2) {
                GridRow { Text("Bonjour").foregroundStyle(.secondary); Text("\(nearby.serviceType)  “\(nearby.macName)”") }
                GridRow { Text("Port").foregroundStyle(.secondary); Text(nearby.port.map(String.init) ?? "—") }
                GridRow { Text("Mac id (fp)").foregroundStyle(.secondary); Text(nearby.fingerprint).font(.system(.body, design: .monospaced)) }
                GridRow { Text("TLS").foregroundStyle(.secondary); Text("1.2, \(nearby.cipherSuite), no resumption") }
                GridRow {
                    Text("Transport").foregroundStyle(.secondary)
                    Text(nearby.metrics.line)
                        .accessibilityLabel("Transport metrics")
                        .accessibilityValue(nearby.metrics.line)
                        .accessibilityIdentifier("nearby.transport.metrics")
                }
            }
            .font(.callout)
            .accessibilityElement(children: .combine)
            .accessibilityLabel("Service details")
            .accessibilityValue("Bonjour \(nearby.serviceType) named \(nearby.macName); port \(nearby.port.map(String.init) ?? "none"); Mac id \(nearby.fingerprint); TLS 1.2 \(nearby.cipherSuite); transport \(nearby.metrics.line)")
            Text("Proposal nearby-v1 — not yet a published contract. Captures are kept in memory only (durable: false).")
                .font(.caption2).foregroundStyle(.secondary)
        }
    }

    // MARK: pairing flow

    private var pairingSection: some View {
        VStack(alignment: .leading, spacing: DS.Space.m) {
            Text("Pair a companion").font(.headline)
            statusRow
            if let step = controller.phase.step {
                PairingStepIndicator(step: step)
            }
            switch controller.phase {
            case .codeShown(let a), .verifying(let a):
                codeRow(a, verifying: { if case .verifying = controller.phase { return true }; return false }())
                Text("Enter this code on the companion while “\(nearby.macName)” is selected. The code is valid for one pairing.")
                    .font(.caption).foregroundStyle(.secondary)
            case .interrupted(let a, _, _):
                interruptedRow(a)
            case .failed:
                HStack {
                    Button("Dismiss") { controller.dismiss() }
                        .keyboardShortcut(.cancelAction)
                        .focused($focus, equals: .dismiss)
                        .accessibilityIdentifier("nearby.pairing.dismiss")
                    showCodeButton(title: "Show New Code")
                }
            case .paired(let p):
                HStack {
                    Label("Paired with \(p.companionName)", systemImage: "checkmark.circle.fill").foregroundStyle(DS.Colors.severitySuccess)
                    Spacer()
                    Button("Dismiss") { controller.dismiss() }
                        .keyboardShortcut(.cancelAction)
                        .focused($focus, equals: .dismiss)
                        .accessibilityIdentifier("nearby.pairing.dismiss")
                }
            case .receiving(let r):
                HStack(spacing: DS.Space.m) {
                    if let total = r.total, total > 0 {
                        ProgressView(value: Double(r.bytes), total: Double(total))
                    } else {
                        ProgressView()
                    }
                    Text(byteProgress(r)).font(.callout).monospacedDigit()
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel("Receiving capture")
                .accessibilityValue(controller.phase.accessibilityValue())
                .accessibilityIdentifier("nearby.pairing.receiving")
                HStack {
                    Text("Cancel closes the companion's session; it can resend with the same capture_id.")
                        .font(.caption2).foregroundStyle(.secondary)
                    Spacer()
                    Button("Cancel") { controller.cancel() }
                        .keyboardShortcut(.cancelAction)
                        .focused($focus, equals: .cancel)
                        .accessibilityLabel("Cancel receiving")
                        .accessibilityIdentifier("nearby.pairing.cancel")
                }
            case .off, .advertising:
                HStack {
                    showCodeButton(title: "Show Pairing Code")
                    Text("A 6-digit code, valid for \(Int(Pairing.codeLifetime)) s; starts advertising if needed.")
                        .font(.caption).foregroundStyle(.secondary)
                }
            }
        }
    }

    /// One line that always names the state precisely (advertising, code
    /// shown, verifying, paired, receiving n/m bytes, interrupted, error).
    private var statusRow: some View {
        Group {
            if controller.machine.attempt != nil {
                // Only the phases with a countdown need a clock.
                TimelineView(.periodic(from: .now, by: 1)) { ctx in statusLine(now: ctx.date) }
            } else {
                statusLine(now: Date())
            }
        }
    }

    private func statusLine(now: Date) -> some View {
        let phase = controller.phase
        return HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
            Text(phase.title).font(.subheadline.weight(.semibold))
                .foregroundStyle(statusColor(phase))
            Text(phase.detail(now: now)).font(.caption).foregroundStyle(.secondary)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Pairing state: \(phase.title)")
        .accessibilityValue(phase.accessibilityValue(now: now))
        .accessibilityIdentifier("nearby.pairing.state")
    }

    private func byteProgress(_ r: PairingFlow.Receiving) -> String {
        let f = ByteCountFormatter()
        f.countStyle = .file
        let got = f.string(fromByteCount: Int64(r.bytes))
        return r.total.map { "\(got) of \(f.string(fromByteCount: Int64($0)))" } ?? "\(got) so far"
    }

    private func statusColor(_ phase: PairingFlow.Phase) -> Color {
        switch phase {
        case .failed: return .red
        case .interrupted: return .orange
        case .paired: return .green
        default: return .primary
        }
    }

    private func showCodeButton(title: String) -> some View {
        Button(title) { controller.showCode() }
            .keyboardShortcut(.defaultAction)
            .focused($focus, equals: .showCode)
            .disabled(!controller.phase.canShowCode())
            .accessibilityHint("Shows a six-digit code to type on the companion; starts advertising if needed.")
            .accessibilityIdentifier("nearby.pairing.show")
    }

    private func codeRow(_ a: PairingFlow.Attempt, verifying: Bool) -> some View {
        HStack(alignment: .top, spacing: DS.Space.xl) {
            // QR image: the same bootstrap payload the reference client's
            // `pair --qr` accepts (PairingBootstrapPayload). Text fallback beside it.
            if let payload = controller.bootstrapPayload, let qr = PairingQR.nsImage(for: payload, side: 120) {
                Image(nsImage: qr)
                    .interpolation(.none)
                    .resizable()
                    .frame(width: DS.Size.imageTile, height: DS.Size.imageTile)
                    .background(DS.Colors.qrGround)
                    .accessibilityLabel("Pairing QR code")
                    .accessibilityValue("Scan with the companion; carries code \(Pairing.spokenCode(a.code)) and this Mac's identity.")
                    .accessibilityIdentifier("nearby.pairing.qr")
            }
            VStack(alignment: .leading, spacing: DS.Space.s) {
                HStack(alignment: .firstTextBaseline, spacing: DS.Space.l) {
                    Text(Pairing.displayCode(a.code))
                        .font(DS.Fonts.pairingCode)
                        .textSelection(.enabled)
                        .focusable()
                        .focused($focus, equals: .code)
                        .onCopyCommand { // ⌘C while the code has focus
                            controller.copyCode()
                            return [NSItemProvider(object: Pairing.clipboardCode(a.code) as NSString)]
                        }
                        .accessibilityLabel("Pairing code")
                        .accessibilityValue(Pairing.spokenCode(a.code))
                        .accessibilityHint("Type these six digits on the companion, or press Command C to copy them.")
                        .accessibilityIdentifier("nearby.pairing.code")
                    Button("Copy code") { controller.copyCode() }
                        .accessibilityLabel("Copy pairing code")
                        .accessibilityHint("Puts the six digits on the clipboard.")
                        .accessibilityIdentifier("nearby.pairing.copy")
                }
                HStack(spacing: DS.Space.l) {
                    TimelineView(.periodic(from: .now, by: 1)) { ctx in
                        let left = Int(a.remaining(at: ctx.date).rounded(.up))
                        Text("expires in \(left) s").font(.callout).foregroundStyle(left <= 15 ? .red : .secondary)
                            .accessibilityLabel("Code expiry")
                            .accessibilityValue("\(left) seconds left")
                            .accessibilityAddTraits(.updatesFrequently)
                            .accessibilityIdentifier("nearby.pairing.expiry")
                    }
                    if verifying {
                        ProgressView().controlSize(.small)
                            .accessibilityLabel("Verifying companion")
                    }
                }
                Text("Scan the QR code, or type the digits on the companion.")
                    .font(.caption).foregroundStyle(.secondary)
                    .accessibilityHidden(true)
            }
            Spacer()
            Button("Cancel") { controller.cancel() }
                .keyboardShortcut(.cancelAction)
                .focused($focus, equals: .cancel)
                .accessibilityLabel("Cancel pairing")
                .accessibilityIdentifier("nearby.pairing.cancel")
        }
    }

    private func interruptedRow(_ a: PairingFlow.Attempt) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: DS.Space.xl) {
            if !a.isExpired(at: Date()) {
                Text(Pairing.displayCode(a.code))
                    .font(DS.Fonts.pairingCode)
                    .foregroundStyle(.secondary)
                    .accessibilityLabel("Interrupted pairing code")
                    .accessibilityValue(Pairing.spokenCode(a.code))
                    .accessibilityIdentifier("nearby.pairing.code")
                Spacer()
                Button("Resume") { controller.resume() }
                    .keyboardShortcut(.defaultAction)
                    .focused($focus, equals: .resume)
                    .accessibilityHint("Serves this code again for its remaining time.")
                    .accessibilityIdentifier("nearby.pairing.resume")
                Button("Cancel") { controller.cancel() }
                    .keyboardShortcut(.cancelAction)
                    .focused($focus, equals: .cancel)
                    .accessibilityLabel("Cancel interrupted pairing")
                    .accessibilityIdentifier("nearby.pairing.cancel")
            } else {
                Button("Dismiss") { controller.dismiss() }
                    .keyboardShortcut(.cancelAction)
                    .focused($focus, equals: .dismiss)
                    .accessibilityIdentifier("nearby.pairing.dismiss")
                showCodeButton(title: "Show New Code")
            }
        }
    }

    // MARK: paired devices

    private var pairedSection: some View {
        VStack(alignment: .leading, spacing: DS.Space.s) {
            Text("Paired companions").font(.headline)
            if nearby.pairs.isEmpty {
                Text("None yet.").font(.callout).foregroundStyle(.secondary)
                    .accessibilityLabel("Paired companions: none yet")
            } else {
                ForEach(nearby.pairs) { pair in
                    let connected = nearby.connectedPairIds.contains(pair.pairId)
                    let a11y = PairingAccessibility.deviceRow(pair, connected: connected)
                    HStack {
                        VStack(alignment: .leading, spacing: DS.Size.hairline) {
                            HStack(spacing: DS.Space.s) {
                                Text(pair.companionName).font(.callout)
                                if connected {
                                    Text("connected").font(DS.Fonts.secondary).padding(.horizontal, DS.Space.xs).padding(.vertical, DS.Size.hairline)
                                        .background(DS.Colors.severitySuccess.opacity(DS.State.badgeFillOpacity), in: Capsule())
                                }
                            }
                            Text("\(pair.pairId) · paired \(pair.createdAt.formatted(date: .abbreviated, time: .shortened))"
                                 + (pair.lastSeenAt.map { " · seen \($0.formatted(date: .abbreviated, time: .shortened))" } ?? ""))
                                .font(.caption).foregroundStyle(.secondary)
                        }
                        .accessibilityElement(children: .ignore)
                        .accessibilityLabel(a11y.label)
                        .accessibilityValue(a11y.value)
                        .accessibilityIdentifier("nearby.device.\(pair.pairId)")
                        Spacer()
                        Picker("Permission", selection: Binding(
                            get: { pair.effectivePermission },
                            set: { nearby.setPermission(pairId: pair.pairId, $0) })) {
                            ForEach(CompanionPermission.allCases) { Text($0.title).tag($0) }
                        }
                        .pickerStyle(.menu)
                        .labelsHidden()
                        .fixedSize()
                        .accessibilityLabel("Permission for \(pair.companionName)")
                        .accessibilityHint("Captures allowed, or view only: a view-only companion's captures are refused with capture_not_permitted.")
                        .accessibilityIdentifier("nearby.device.\(pair.pairId).permission")
                        Button("Forget") { controller.forget(pairId: pair.pairId) }
                            .accessibilityLabel("Forget \(pair.companionName)")
                            .accessibilityHint("Removes the pairing and closes its connection.")
                            .accessibilityIdentifier("nearby.device.\(pair.pairId).forget")
                    }
                }
            }
            Text("Keys live in \(nearby.store.url.path) (mode 0600), not the Keychain.")
                .font(.caption2).foregroundStyle(.secondary)
        }
    }

    // MARK: refused captures

    /// Declared after the paired rows: the controls' source order is the
    /// window's Tab order (`PanelFocusOrder`, checked by the accessibility tests).
    private func refusedSection(_ e: NearbyState.ReceiveError) -> some View {
        VStack(alignment: .leading, spacing: DS.Space.xs) {
            Text("Refused capture").font(.headline).foregroundStyle(DS.Colors.severityError)
            Text(e.summary).font(.system(.callout, design: .monospaced)).textSelection(.enabled)
                .accessibilityLabel("Last refused capture")
                .accessibilityValue(e.summary)
                .accessibilityIdentifier("nearby.refused.last")
            HStack {
                Text("\(nearby.receiveErrors.count) refusal(s), \(nearby.duplicateCaptureCount) duplicate(s) acknowledged")
                    .font(.caption).foregroundStyle(.secondary)
                Spacer()
                Button("Clear") { nearby.clearReceiveErrors() }
                    .accessibilityLabel("Clear refused captures")
                    .accessibilityIdentifier("nearby.refused.clear")
            }
        }
    }

    // MARK: activity

    private var logSection: some View {
        VStack(alignment: .leading, spacing: DS.Space.xs) {
            Text("Activity").font(.headline)
            ScrollView {
                VStack(alignment: .leading, spacing: DS.Size.hairline) {
                    ForEach(Array(nearby.log.suffix(30).enumerated()), id: \.offset) { _, line in
                        Text(line).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(minHeight: DS.Layout.searchPreviewMinHeight, maxHeight: DS.Layout.nearbyEventsMaxHeight)
            .accessibilityLabel("Activity log")
            .accessibilityValue(nearby.log.suffix(5).joined(separator: ". "))
            .accessibilityIdentifier("nearby.log")
        }
    }
}

/// Numbered steps of the pairing sequence with the current one highlighted;
/// one accessibility element ("Pairing step 2 of 4: …") so VoiceOver reads
/// the position, not four capsules.
struct PairingStepIndicator: View {
    var step: PairingFlow.Step

    var body: some View {
        HStack(spacing: DS.Space.s) {
            ForEach(1...PairingFlow.Step.count, id: \.self) { k in
                let status = step.status(of: k)
                HStack(spacing: DS.Space.xs) {
                    Image(systemName: symbol(status))
                        .foregroundStyle(color(status))
                    Text(PairingFlow.Step.shortNames[k - 1])
                        .font(.caption.weight(status == .active || status == .interrupted ? .semibold : .regular))
                        .lineLimit(1).fixedSize()
                        .foregroundStyle(status == .pending ? .secondary : .primary)
                }
                .padding(.horizontal, DS.Space.s).padding(.vertical, DS.Space.xxs)
                .background(status == .pending ? Color.clear : color(status).opacity(DS.State.badgeFillOpacity), in: Capsule())
                if k < PairingFlow.Step.count {
                    Image(systemName: "chevron.right").font(.caption2).foregroundStyle(.tertiary)
                }
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(step.accessibilityLabel)
        .accessibilityIdentifier("nearby.pairing.steps")
    }

    private func symbol(_ s: PairingFlow.Step.Status) -> String {
        switch s {
        case .pending: return "circle"
        case .active: return "circle.fill"
        case .done: return "checkmark.circle.fill"
        case .interrupted: return "exclamationmark.circle.fill"
        }
    }

    private func color(_ s: PairingFlow.Step.Status) -> Color {
        switch s {
        case .pending: return .secondary
        case .active: return .accentColor
        case .done: return .green
        case .interrupted: return .orange
        }
    }
}

/// Received captures, one row per capture with its state: in-memory inbox
/// entries (never durable) and captures the attached bridge holds.
struct CapturesSection: View {
    @ObservedObject var inbox: NearbyInbox
    var bridgeCaptures: [BridgeSession.Capture]
    var lastCaptureId: String?

    var rows: [PairingAccessibility.CaptureRow] {
        var out = inbox.received.map { c in
            PairingAccessibility.CaptureRow(captureId: c.captureId, source: .inbox, state: "received",
                                            durable: false, note: "\(c.image.mimeType), \(c.image.dataBase64.utf8.count) base64 bytes")
        }
        out += bridgeCaptures.map {
            PairingAccessibility.CaptureRow(captureId: $0.captureId, source: .bridge, state: $0.state.rawValue,
                                            durable: nil, note: $0.note)
        }
        return out
    }

    private var lastId: String { lastCaptureId ?? inbox.lastCaptureId ?? "—" }

    var body: some View {
        let rows = rows
        VStack(alignment: .leading, spacing: DS.Space.xs) {
            Text("Received captures").font(.headline)
            Text("Last capture id: \(lastId)")
                .font(.system(.callout, design: .monospaced)).textSelection(.enabled)
                .accessibilityLabel("Last capture id")
                .accessibilityValue(lastId)
                .accessibilityIdentifier("nearby.captures.last")
            if rows.isEmpty {
                Text(inbox.lastNote ?? "No captures received yet.")
                    .font(.caption).foregroundStyle(.secondary)
            } else {
                ForEach(rows.suffix(8)) { row in CaptureRowView(row: row) }
                if let note = inbox.lastNote {
                    Text(note).font(.caption).foregroundStyle(.secondary)
                }
            }
        }
    }
}

struct CaptureRowView: View {
    var row: PairingAccessibility.CaptureRow

    var body: some View {
        let inMemory = row.durable == false
        HStack(spacing: DS.Space.m) {
            Image(systemName: inMemory ? "tray" : "checkmark.seal")
                .foregroundStyle(inMemory ? Color.secondary : DS.Colors.severitySuccess)
            Text(row.captureId).font(.system(.callout, design: .monospaced))
            Text("\(row.state) · \(row.durabilityText)").font(.caption).foregroundStyle(.secondary)
            if !row.note.isEmpty {
                Text(row.note).font(.caption2).foregroundStyle(.tertiary).lineLimit(1)
            }
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(row.accessibilityLabel)
        .accessibilityValue(row.accessibilityValue)
        .accessibilityIdentifier("nearby.capture.\(row.id)")
    }
}
