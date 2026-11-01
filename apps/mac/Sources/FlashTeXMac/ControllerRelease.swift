import Foundation

/// When the helper route sends the NEXT edit while one is in flight
/// (ShellModel+Controller.swift, `controllerSubmitEdit` / `applyDurableDocument`).
///
/// - `holdUntilPreview` (default): the in-flight edit is released only when
///   the preview compiled from its durable revision arrived. Every submitted
///   edit is painted; intermediate keystrokes coalesce into the next edit.
/// - `hybrid` (`FLASHTEX_CONTROLLER_RELEASE=hybrid`): same hold, but once the
///   in-flight edit is durable and has been in flight longer than a bound the
///   newest buffer is sent anyway, so a slow compile does not starve the
///   ledger of fresh durable text. The bound adapts to the last observed
///   edit → preview latency: `2 × last`, clamped to 40…250 ms (`ReleaseBound`).
///   A released edit's compile may then be superseded by the helper (its
///   preview never arrives, or arrives as a labelled historical frame when
///   `completed-snapshots-v1` is negotiated).
///
/// Historical mode alone (`FLASHTEX_COMPLETED_SNAPSHOTS=1` without `hybrid`)
/// keeps releasing on the durable receipt, as measured in
/// docs/evidence/historical-preview-2026-09-12T1010Z.md.
enum ControllerReleasePolicy: String, Equatable {
    case holdUntilPreview = "hold"
    case hybrid

    /// `FLASHTEX_CONTROLLER_RELEASE`: `hybrid` opts in; unset/anything else holds.
    static func fromEnvironment(_ environment: [String: String] = ProcessInfo.processInfo.environment) -> ControllerReleasePolicy {
        environment["FLASHTEX_CONTROLLER_RELEASE"] == "hybrid" ? .hybrid : .holdUntilPreview
    }
}

/// The hybrid policy's bound and decision, kept pure for tests.
enum ReleaseBound {
    static let minimumMs: Double = 40
    static let maximumMs: Double = 250
    /// Default factor over the last observed latency; `FLASHTEX_CONTROLLER_RELEASE_FACTOR`
    /// (a positive number) overrides it for measurement only.
    static let defaultMultiplier: Double = 2
    static let multiplier: Double = {
        if let s = ProcessInfo.processInfo.environment["FLASHTEX_CONTROLLER_RELEASE_FACTOR"], let f = Double(s), f > 0 { return f }
        return defaultMultiplier
    }()

    /// `multiplier × lastEditToPreviewMs` clamped to 40…250 ms; the minimum when
    /// nothing was observed yet.
    static func boundMs(lastEditToPreviewMs: Double?, multiplier: Double = ReleaseBound.multiplier) -> Double {
        guard let last = lastEditToPreviewMs, last.isFinite, last > 0 else { return minimumMs }
        return min(maximumMs, max(minimumMs, last * multiplier))
    }

    /// Whether the in-flight edit may be released now under `hybrid`: it must be
    /// durable (the helper holds its text), there must be newer text to send,
    /// and it must have been in flight at least the bound.
    static func shouldRelease(policy: ControllerReleasePolicy, durable: Bool, queued: Bool,
                              inFlightMs: Double, lastEditToPreviewMs: Double?) -> Bool {
        guard policy == .hybrid, durable, queued else { return false }
        return inFlightMs >= boundMs(lastEditToPreviewMs: lastEditToPreviewMs)
    }

    /// Delay until the bound elapses for an edit that has been in flight
    /// `inFlightMs` (0 when already past it).
    static func remainingMs(inFlightMs: Double, lastEditToPreviewMs: Double?) -> Double {
        max(0, boundMs(lastEditToPreviewMs: lastEditToPreviewMs) - inFlightMs)
    }
}
