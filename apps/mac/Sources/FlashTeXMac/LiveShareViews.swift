import AppKit
import CoreImage
import CoreImage.CIFilterBuiltins
import FlashTeXCollabSession
import SwiftUI

/// The eight presence colours (proposal §4): system colours, so each has a
/// light and a dark value, assigned by the hub in join order. The colour is
/// never the only cue: names go on the caret flag and in every list.
enum LiveShareColours {
    static let palette: [NSColor] = [.systemBlue, .systemOrange, .systemGreen, .systemPurple, .systemPink, .systemTeal, .systemBrown, .systemRed]
    static func colour(_ index: Int) -> NSColor { palette[((index % palette.count) + palette.count) % palette.count] }
}

/// The session panel: the invitation (QR and link), joiners waiting for
/// approval, who is here, and End/Leave. File ▸ Start Live Share Session…
/// and the status-bar item open it.
struct LiveShareSheet: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        let ls = model.liveShare
        VStack(alignment: .leading, spacing: DS.Space.l) {
            HStack {
                Text("Live Share").font(.title2.bold())
                Text("Preview").font(.caption.bold()).foregroundStyle(.secondary)
                    .padding(.horizontal, DS.Space.s).overlay(Capsule().stroke(.secondary.opacity(0.5)))
            }
            if ls.isActive {
                Text(ls.isHost ? "You are sharing \(ls.projectName). Everyone edits the same sources; each Mac compiles its own preview."
                               : "You are in \(ls.projectName)\(ls.canEdit ? "" : " as a viewer"). Each Mac compiles its own preview.")
                    .font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            }
            ForEach(ls.approvals) { a in ApprovalRow(approval: a) }
            if ls.isHost { InviteBlock() }
            if ls.isActive { ParticipantsBlock() }
            if let note = ls.note {
                Text(note).font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                    .accessibilityIdentifier("liveshare.note")
            }
            HStack {
                if ls.isActive {
                    Button(ls.isHost ? "End Session" : "Leave Session", role: .destructive) { ls.leave() }
                        .accessibilityIdentifier("liveshare.leave")
                }
                Spacer()
                Button("Done") { ls.sheetShown = false }.keyboardShortcut(.defaultAction)
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.sheetWidth)
    }
}

private struct ApprovalRow: View {
    @Environment(ShellModel.self) var model
    let approval: LiveShareController.Approval

    var body: some View {
        VStack(alignment: .leading, spacing: DS.Space.s) {
            Text("\(approval.name) (\(approval.deviceKind == "ipad" ? "iPad" : "Mac")) wants to join.")
                .font(.headline)
            Text("Names are self-asserted on the local network: check that this is who you invited.")
                .font(.caption).foregroundStyle(.secondary)
            HStack {
                Button("Allow Editing") { model.liveShare.answer(approval, .allowEdit) }
                    .keyboardShortcut(.defaultAction)
                    .accessibilityIdentifier("liveshare.allowEdit")
                Button("Allow Viewing") { model.liveShare.answer(approval, .allowView) }
                Button("Deny", role: .destructive) { model.liveShare.answer(approval, .deny) }
            }
        }
        .padding(DS.Space.m)
        .background(RoundedRectangle(cornerRadius: DS.Radius.tab).fill(Color.accentColor.opacity(0.08)))
        .accessibilityElement(children: .contain)
    }
}

private struct InviteBlock: View {
    @Environment(ShellModel.self) var model
    @State private var copied = false

    var body: some View {
        let ls = model.liveShare
        VStack(alignment: .leading, spacing: DS.Space.m) {
            Text("Invitation").font(.headline)
            if let invite = ls.invite {
                HStack(alignment: .top, spacing: DS.Space.l) {
                    if let qr = LiveShareQR.image(invite.link) {
                        Image(nsImage: qr).interpolation(.none).resizable().frame(width: 132, height: 132)
                            .accessibilityLabel("QR code of the invitation link")
                    }
                    VStack(alignment: .leading, spacing: DS.Space.s) {
                        Text(invite.link).font(.system(.caption, design: .monospaced)).textSelection(.enabled)
                            .lineLimit(5).fixedSize(horizontal: false, vertical: true)
                            .accessibilityIdentifier("liveshare.link")
                        HStack {
                            Button(copied ? "Copied" : "Copy Link") {
                                NSPasteboard.general.clearContents()
                                NSPasteboard.general.setString(invite.link, forType: .string)
                                copied = true
                            }
                            .accessibilityIdentifier("liveshare.copy")
                            Button("New Invitation") { ls.newInvite(); copied = false }
                                .help("Each invitation admits one person, once, within 10 minutes")
                        }
                        Text("Single use, valid for 10 minutes. Paste it into File ▸ Join Live Share Session… on the other Mac (or a second FlashTeX on this one).")
                            .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
                    }
                }
            } else {
                HStack { ProgressView().controlSize(.small); Text("Starting…").foregroundStyle(.secondary) }
            }
        }
    }
}

private struct ParticipantsBlock: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        let ls = model.liveShare
        VStack(alignment: .leading, spacing: DS.Space.s) {
            Text("In this session").font(.headline)
            ForEach(ls.participants, id: \.id) { p in
                HStack(spacing: DS.Space.s) {
                    Circle().fill(Color(nsColor: LiveShareColours.colour(p.colourIndex))).frame(width: 10, height: 10)
                        .accessibilityHidden(true)
                    Text(p.isLocal ? "\(p.name) (you)" : p.name)
                    if !p.isLocal, let m = ls.hubMembers.first(where: { $0.id == p.id }), !m.canEdit {
                        Text("viewer").font(.caption).foregroundStyle(.secondary)
                    }
                    Spacer()
                    if ls.isHost, !p.isLocal {
                        Button("Remove") { ls.remove(p.id) }.controlSize(.small)
                    }
                }
            }
        }
    }
}

/// File ▸ Join Live Share Session…: paste the invitation, choose a name.
struct LiveShareJoinSheet: View {
    @Environment(ShellModel.self) var model
    @State private var link = ""
    @State private var name = LiveShareController.defaultDisplayName

    var body: some View {
        let ls = model.liveShare
        VStack(alignment: .leading, spacing: DS.Space.l) {
            Text("Join Live Share Session").font(.title2.bold())
            Text("Paste the invitation the host copied for you. The shared project opens in this window; each Mac compiles its own preview.")
                .font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            TextField("flashtex-collab://join?…", text: $link, axis: .vertical)
                .lineLimit(2...4).textFieldStyle(.roundedBorder).font(.system(.body, design: .monospaced))
                .accessibilityLabel("Invitation link")
                .accessibilityIdentifier("liveshare.joinLink")
            TextField("Your name", text: $name).textFieldStyle(.roundedBorder)
                .accessibilityLabel("Your name, as others see it")
            switch ls.phase {
            case .connecting: status("Connecting to the host…")
            case .awaitingApproval: status("Waiting for the host to let you in…")
            default: EmptyView()
            }
            if let note = ls.note {
                Text(note).font(.callout).foregroundStyle(DS.Colors.severityWarning).fixedSize(horizontal: false, vertical: true)
                    .accessibilityIdentifier("liveshare.joinNote")
            }
            HStack {
                Spacer()
                Button("Cancel") {
                    if ls.phase == .connecting || ls.phase == .awaitingApproval { ls.leave() }
                    ls.joinSheetShown = false
                }
                .keyboardShortcut(.cancelAction)
                Button("Join") { ls.join(link: link, name: name) }
                    .keyboardShortcut(.defaultAction)
                    .disabled(link.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || ls.phase == .connecting || ls.phase == .awaitingApproval)
                    .accessibilityIdentifier("liveshare.join")
            }
        }
        .padding(DS.Space.xl)
        .frame(width: DS.Layout.sheetWidth)
        .onAppear {
            if link.isEmpty, let s = NSPasteboard.general.string(forType: .string), s.hasPrefix(CollabInvite.scheme + "://") { link = s }
        }
    }

    private func status(_ text: String) -> some View {
        HStack(spacing: DS.Space.s) { ProgressView().controlSize(.small); Text(text).foregroundStyle(.secondary) }
    }
}

/// Status bar: who is in the session, coloured as their carets; click for
/// the session panel. Reads only the participant list, which changes on
/// join and leave, never per keystroke or caret move.
struct LiveShareStatusItem: View {
    @Environment(ShellModel.self) var model

    var body: some View {
        let ls = model.liveShare
        if ls.isActive || ls.phase == .connecting || ls.phase == .awaitingApproval {
            Button { ls.sheetShown = true } label: {
                HStack(spacing: DS.Space.xs) {
                    Image(systemName: "person.2.fill")
                    ForEach(ls.participants.prefix(5), id: \.id) { p in
                        Circle().fill(Color(nsColor: LiveShareColours.colour(p.colourIndex))).frame(width: 8, height: 8)
                    }
                    Text(label(ls))
                }
            }
            .buttonStyle(.plain)
            .help(ls.participants.map { $0.isLocal ? "\($0.name) (you)" : $0.name }.joined(separator: ", "))
            .accessibilityLabel("Live Share: \(label(ls))")
            .accessibilityValue(ls.participants.map(\.name).joined(separator: ", "))
            .accessibilityIdentifier("liveshare.status")
        }
    }

    private func label(_ ls: LiveShareController) -> String {
        switch ls.phase {
        case .connecting: return "Connecting…"
        case .awaitingApproval: return "Waiting for approval…"
        case .reconnecting: return "Offline, reconnecting…"
        default:
            let n = ls.participants.count
            return n <= 1 ? "Live Share" : "\(n) people"
        }
    }
}

/// Settings ▸ Live Share: the preview's master switch (off by default).
struct LiveShareSettingsSection: View {
    @AppStorage(LiveShareController.enabledKey) private var enabled = false

    var body: some View {
        Section {
            Toggle("Live Share (preview)", isOn: $enabled)
                .accessibilityIdentifier("settings.liveShare")
            Text("Edit a project together with other Macs on the same network, or with a second FlashTeX on this Mac. Adds File ▸ Start Live Share Session… and Join Live Share Session…. Sources are shared; figures are not yet. Each Mac compiles its own preview.")
                .font(.caption).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
        }
    }
}

/// File menu items, present only while the preview is on.
struct LiveShareMenuItems: View {
    var model: ShellModel
    @AppStorage(LiveShareController.enabledKey) private var enabled = false

    var body: some View {
        Group {
            if enabled || LiveShareController.enabled {
                Divider()
                Button(model.liveShare.isHost ? "Live Share Session…" : "Start Live Share Session…") { model.liveShare.startHosting() }
                    .disabled(model.liveShare.isActive && !model.liveShare.isHost)
                Button("Join Live Share Session…") { model.liveShare.presentJoin() }
                    .disabled(model.liveShare.isActive)
                if model.liveShare.isActive {
                    Button(model.liveShare.isHost ? "End Live Share Session" : "Leave Live Share Session") { model.liveShare.leave() }
                }
            }
        }
    }
}

enum LiveShareQR {
    static func image(_ text: String) -> NSImage? {
        let f = CIFilter.qrCodeGenerator()
        f.message = Data(text.utf8)
        f.correctionLevel = "M"
        guard let out = f.outputImage?.transformed(by: CGAffineTransform(scaleX: 6, y: 6)) else { return nil }
        let rep = NSCIImageRep(ciImage: out)
        let img = NSImage(size: rep.size)
        img.addRepresentation(rep)
        return img
    }
}
