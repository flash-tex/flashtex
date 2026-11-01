import Foundation

/// The `supported_features` list the Mac sends with every `capture_convert`
/// (transfer-v1). `flashtex-bridge` does not forward it (issues #51/#23): it
/// gives the conversion provider its own list for the document's engine,
/// which `capture_convert.engine` names (`crates/bridge/src/features.rs`;
/// retirement plan #1236, S3r). The Mac's list is still sent for wire
/// compatibility, and it is the bridge's new-engine list, byte for byte
/// (`NEW_ENGINE_FEATURES`, MIT; `CaptureFeaturesTests` pins the two
/// together). The engine typesets what pdfLaTeX does with the document's own
/// packages, so the list is a policy, not an enumeration. The bridge accepts
/// at most 64 entries of at most 128 bytes each (`crates/bridge/src/context.rs`).
enum CaptureFeatures {
    /// `crates/bridge/src/features.rs` `NEW_ENGINE_FEATURES`.
    static let policy: [String] = [
        "POLICY: pdfLaTeX-compatible destination: any construct pdfLaTeX typesets with the packages this document already loads",
        "POLICY: math in $...$, \\[ ... \\] or equation; amsmath environments (align, gather, cases …) only if the document loads amsmath",
        "POLICY: \\mathbb, \\mathcal and the like only if the document loads their package (amssymb, amsfonts)",
        "POLICY: never add \\usepackage lines or new macros; a construct needing a package the document lacks is UNSUPPORTED",
    ]

    /// The list sent as `supported_features`.
    static func supportedFeatures() -> [String] { policy }

    /// Default capture instructions from this shell (file picker, sample panel).
    static let defaultInstructions = "Transcribe the selected handwriting to LaTeX; preserve notation. Wrap every formula in $...$ or \\[...\\]. Use only the supported features listed; report anything else in ambiguities instead of substituting."
}
