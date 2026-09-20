import Foundation
import FlashTeXProtocol

/// Keeps the shell's copy of the bridge's pinned destination honest across
/// ordinary edits, so a companion is never told a destination the bridge will
/// refuse (lane mac-nearby-errors, follow-up to the capture-acceptance audit).
///
/// The bridge (`crates/bridge/src/lib.rs`, `edit`) keeps anchors itself. For
/// a *fixed* anchor (the explicit ⌘⌥P pin) an edit that overlaps the pinned
/// range, sits exactly on it, or inserts at a zero-width pin marks the anchor
/// invalid ("invalidate instead of guessing affinity"), an edit entirely
/// before it shifts its bytes, an edit after it leaves it alone. A *caret*
/// anchor (the automatic destination the Mac pins for a companion) follows
/// edits the way the caret does — text typed at or before it shifts it, an
/// edit spanning it collapses it after the replacement — and is never
/// invalidated. `current_revision` follows every edit. transfer-v1 has no op
/// to query anchor state, and a capture bound to an invalid anchor is refused
/// with `destination_reselection_required`. The shell mirrors those exact
/// rules locally. A dropped explicit pin is shown as "(invalid)" in the bridge
/// row; companions are then answered with the caret, pinned on demand
/// (`nearbyDestinationPinningCaretIfNeeded`), never with the dead id.
enum DestinationTracking {
    /// The bridge's anchor after `document_edit` replacing `startByte..<endByte`
    /// with `replacementBytes` bytes at `revision`. Same document only; the
    /// caller checks project and path.
    static func follow(_ anchor: TransferV1.Anchor, startByte: Int, endByte: Int, replacementBytes: Int, revision: Int) -> TransferV1.Anchor {
        var a = anchor
        guard a.valid else { return a }  // the bridge only touches valid anchors
        let shift = replacementBytes - (endByte - startByte)
        if a.isCaret {
            if endByte <= a.startByte {
                a.startByte += shift
                a.endByte += shift
            } else if startByte <= a.endByte {
                a.startByte = startByte + replacementBytes
                a.endByte = a.startByte
            }
        } else if (startByte == endByte && startByte >= a.startByte && startByte <= a.endByte)
            || (startByte < a.endByte && endByte > a.startByte)
            || (a.startByte == a.endByte && startByte <= a.startByte && endByte > a.startByte) {
            a.valid = false
        } else if endByte <= a.startByte {
            a.startByte += shift
            a.endByte += shift
        }
        a.currentRevision = revision
        return a
    }

    /// `follow` for the region an editor change replaced.
    static func follow(_ anchor: TransferV1.Anchor, region: SourceMapping.ChangedRegion, revision: Int) -> TransferV1.Anchor {
        follow(anchor, startByte: region.startByte, endByte: region.oldEndByte,
               replacementBytes: region.replacement.utf8.count, revision: revision)
    }
}

extension ShellModel {
    /// What the companion is told (`hello_ack.destination` / `destination`).
    /// With a bridge attached, the bridge's anchor is authoritative: its pin
    /// while valid, `nil` once an edit dropped it or nothing is pinned there —
    /// never the local anchor, whose id the bridge would refuse. Without a
    /// bridge, the local anchor (the in-memory inbox accepts against it).
    func announcedNearbyDestination(bridgeAttached: Bool, bridgeAnchor: TransferV1.Anchor?, localAnchor: InsertionAnchor?) -> NearbyV1.Destination? {
        if bridgeAttached {
            guard let a = bridgeAnchor, a.valid else { return nil }
            // A caret anchor is wherever the caret is now, so the revision that
            // goes with its position is the current one (informational to the
            // bridge in that mode); a fixed pin's is the revision it was pinned at.
            return .init(destinationId: a.destinationId, projectId: a.projectId, path: a.path,
                         baseRevision: a.isCaret ? a.currentRevision : a.pinnedRevision,
                         caretContext: caretContext(path: a.path, byte: a.startByte))
        }
        guard let anchor = localAnchor else { return nil }
        return .init(destinationId: anchor.id, projectId: projectId, path: anchor.path,
                     baseRevision: anchor.revision,
                     caretContext: caretContext(path: anchor.path, byte: anchor.byteOffset))
    }

    /// The caret context for an announced destination, so the companion can say
    /// what will happen to a capture before it is sent ("display math",
    /// "already inside $…$", "verbatim — inserted literally"). Derived from
    /// the open document; nil when that document is not open here.
    func caretContext(path: String, byte: Int) -> CaretContext? {
        guard let document = documents.first(where: { $0.path == path }) else { return nil }
        return CaretContext.derive(document.text, caretByte: byte)
    }
}
