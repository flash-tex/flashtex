import AppKit
import FlashTeXDisplayListV3
import FlashTeXPreviewV3
import Observation
import SwiftUI

/// Performance modes (DESIGN.md §1.2 "Performance modes", lane PERF-MODES):
/// Low Memory, Balanced (the default) and High Performance. Each is a named
/// profile of the engine host's knobs (sent as `HELLO.profile` and, when the
/// setting changes, `PROFILE`: docs/protocol/display-list-v3.md §6.9) and of
/// the app's own caches. A mode never changes what is typeset or drawn, only
/// what is kept in memory and worked out ahead.
enum PerformanceMode: String, CaseIterable, Identifiable, Sendable {
    case lowMemory = "low-memory"
    case balanced
    case highPerformance = "high-performance"

    var id: String { rawValue }

    /// The host's profile name (spec §6.9).
    var hostProfile: String { rawValue }

    var title: String {
        switch self {
        case .lowMemory: "Low Memory"
        case .balanced: "Balanced"
        case .highPerformance: "High Performance"
        }
    }

    /// The one-line explanation under the picker.
    var explanation: String {
        switch self {
        case .lowMemory: "Uses the least memory. Edits far from where you last typed, and scrolling back to pages you left, can take a little longer."
        case .balanced: "Fast previews with moderate memory. Recommended."
        case .highPerformance: "The fastest previews. Uses more memory: up to a quarter of this Mac's memory for very long documents."
        }
    }

    // MARK: the app's knobs

    /// Page rasters kept over all pages (`EngineV3RasterHolder.budget`).
    var keptRasters: Int {
        switch self {
        case .lowMemory: 1
        case .balanced: 2
        case .highPerformance: 8
        }
    }

    /// Screens above and below the visible rect whose pages are laid out
    /// and drawn (each holds a page bitmap): scrolling into them shows a
    /// drawn page at once.
    var overscanScreens: CGFloat {
        switch self {
        case .lowMemory: 0.5
        case .balanced: 1
        case .highPerformance: 3
        }
    }

    /// Decoded images the renderer keeps (`DL3ResourceCache` limits).
    var imageLimits: (count: Int, bytes: Int) {
        switch self {
        case .lowMemory: (64, 96 << 20)
        case .balanced: (256, 512 << 20)
        case .highPerformance: (1024, 2 << 30)
        }
    }

    // MARK: the setting

    static let defaultsKey = "performanceMode"

    /// The mode the user chose (Preferences > Performance), Balanced by default.
    static var stored: PerformanceMode {
        get { UserDefaults.standard.string(forKey: defaultsKey).flatMap(PerformanceMode.init(rawValue:)) ?? .balanced }
        set {
            UserDefaults.standard.set(newValue.rawValue, forKey: defaultsKey)
            apply(newValue)
        }
    }

    private static let lock = NSLock()
    nonisolated(unsafe) private static var _current: PerformanceMode?

    /// The mode in effect, readable from any thread (the tile queue's
    /// raster budget). The stored setting until `apply` sets another.
    static var current: PerformanceMode {
        lock.lock(); defer { lock.unlock() }
        if let c = _current { return c }
        let s = stored
        _current = s
        return s
    }

    /// Posted on the main thread when the mode changes; open documents'
    /// hosts switch (`EngineV3Session`).
    static let changed = Notification.Name("FlashTeXPerformanceModeChanged")

    /// Make `mode` the one in effect: the app's caches take its limits now
    /// and every open document's host is told (a notification).
    static func apply(_ mode: PerformanceMode) {
        lock.lock()
        let was = _current
        _current = mode
        lock.unlock()
        let (count, bytes) = mode.imageLimits
        DL3ResourceCache.shared.setImageLimits(count: count, bytes: bytes)
        if was != mode {
            let post = { NotificationCenter.default.post(name: changed, object: nil, userInfo: ["mode": mode.rawValue]) }
            if Thread.isMainThread { post() } else { DispatchQueue.main.async(execute: post) }
        }
    }
}

/// Watches the system's memory pressure and, in Balanced or High
/// Performance, suggests Low Memory (a banner over the preview). It never
/// switches by itself.
@MainActor @Observable
final class PerformanceAdvisor {
    static let shared = PerformanceAdvisor()

    /// A suggestion is showing.
    private(set) var suggestingLowMemory = false
    /// "Not Now" was chosen: no further suggestion until the app restarts.
    private var declined = false
    @ObservationIgnored private var source: DispatchSourceMemoryPressure?

    /// Start watching (once; at the first engine-v3 preview).
    func start() {
        guard source == nil else { return }
        PerformanceMode.apply(PerformanceMode.current) // the app's caches take the stored mode's limits
        let s = DispatchSource.makeMemoryPressureSource(eventMask: [.warning, .critical], queue: .main)
        s.setEventHandler { [weak self] in
            MainActor.assumeIsolated { self?.pressure() }
        }
        s.resume()
        source = s
        NotificationCenter.default.addObserver(forName: PerformanceMode.changed, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { self?.modeChanged() }
        }
    }

    /// The system reported memory pressure (warning or critical).
    func pressure() {
        guard !declined, PerformanceMode.current != .lowMemory else { return }
        suggestingLowMemory = true
    }

    func switchToLowMemory() {
        suggestingLowMemory = false
        PerformanceMode.stored = .lowMemory
    }

    func notNow() {
        suggestingLowMemory = false
        declined = true
    }

    /// The mode changed some other way (Preferences): drop the suggestion.
    func modeChanged() {
        if PerformanceMode.current == .lowMemory { suggestingLowMemory = false }
    }
}

/// The suggestion to switch to Low Memory, over the preview.
struct MemoryPressureBanner: View {

    static let headline = "This Mac is low on memory."
    static let detail = "Low Memory mode makes the preview use less memory, at a little speed. Change it any time in Settings > Performance."

    var body: some View {
        let advisor = PerformanceAdvisor.shared
        if advisor.suggestingLowMemory {
            HStack(alignment: .firstTextBaseline, spacing: DS.Space.m) {
                Image(systemName: "memorychip")
                    .foregroundStyle(DS.Colors.severityWarning)
                    .accessibilityHidden(true)
                VStack(alignment: .leading, spacing: DS.Space.xs) {
                    Text(Self.headline).font(DS.Fonts.secondary.weight(.semibold))
                    Text(Self.detail).font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 0)
                Button("Use Low Memory") { advisor.switchToLowMemory() }
                    .ideSecondary()
                    .accessibilityIdentifier("performance.suggest.low-memory")
                Button("Not Now") { advisor.notNow() }
                    .ideSecondary()
            }
            .padding(DS.Space.m)
            .background(DS.Colors.surfaceSecondary)
            .accessibilityElement(children: .combine)
            .accessibilityLabel(Self.headline + " " + Self.detail)
            .accessibilityIdentifier("performance.suggest")
        }
    }
}

/// Settings > Performance.
struct PerformanceSettingsSection: View {
    @State private var mode = PerformanceMode.stored

    var body: some View {
        Section("Performance") {
            Picker("Mode", selection: Binding(get: { mode }, set: { mode = $0; PerformanceMode.stored = $0 })) {
                ForEach(PerformanceMode.allCases) { Text($0.title).tag($0) }
            }
            .pickerStyle(.radioGroup)
            .accessibilityIdentifier("performance.mode")
            Text(mode.explanation)
                .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                .fixedSize(horizontal: false, vertical: true)
            Text("Applies at once to open documents. Every mode typesets and draws the same pages; only memory use and speed differ.")
                .font(DS.Fonts.secondary).foregroundStyle(DS.Colors.textSecondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}
