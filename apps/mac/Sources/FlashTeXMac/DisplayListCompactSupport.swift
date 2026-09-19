import Foundation
import FlashTeXProtocol

/// `display-list-v2-compact` in the Mac shell
/// (protocol/proposals/display-list-v2-compact.md): the shell requests it
/// per request next to `display-list-v2` (never as part of the mode set —
/// `DisplayListDelta.perRequestCapabilities`), the fast reader decodes it
/// into the same `RenderingV2` values as the full encoding, and the delta
/// consumer keys its exact accounting on the installed base's encoding.
/// `FLASHTEX_DISPLAY_LIST_COMPACT=0` stops requesting it (the producer then
/// answers in the full encoding, byte-identical to today).
extension DisplayListCompact {
    /// On unless `FLASHTEX_DISPLAY_LIST_COMPACT=0`.
    static var enabled: Bool { ProcessInfo.processInfo.environment["FLASHTEX_DISPLAY_LIST_COMPACT"] != "0" }
}
