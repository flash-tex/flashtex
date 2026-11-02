import SwiftUI

@main
struct FlashTeXPadApp: App {
    @StateObject private var model = PadModel()
    @Environment(\.scenePhase) private var scenePhase

    init() {
        // UI tests: start from the default canvas gestures and a closed
        // Captures panel whatever a previous run left in the defaults.
        if ProcessInfo.processInfo.arguments.contains("-flashtexpad-canvas-defaults") {
            for key in CanvasSettings.allKeys + [CaptureView.capturesPanelOpenKey] {
                UserDefaults.standard.removeObject(forKey: key)
            }
        }
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(model)
                .onAppear {
                    // Deterministic launch states for tests and screenshots
                    // (`-flashtexpad-open sample|fixture`); no network is touched.
                    let args = ProcessInfo.processInfo.arguments
                    model.pairFromLaunchArgument()
                    // Auto-reconnect to the last paired Mac (Keychain pairing; remembered
                    // address, then Bonjour by fingerprint). Skipped when a test pairs
                    // explicitly or asks for a quiet launch.
                    if Self.reconnectsByItself, model.pairedMac != nil {
                        Task { await model.autoReconnect() }
                    }
                    if let i = args.firstIndex(of: "-flashtexpad-open"), i + 1 < args.count {
                        switch args[i + 1] {
                        case "sample": model.openBundledSample()
                        case "fixture": model.openReviewFixture()
                        default: break
                        }
                    }
                }
                // Back from another app: reconnect a link iOS dropped while the
                // app was suspended; going away: finish the capture writes.
                .onChange(of: scenePhase) { _, phase in Self.sceneChanged(to: phase, model: model) }
        }
    }

    /// Background: the grant is taken here, synchronously; active: reconnect.
    @MainActor static func sceneChanged(to phase: ScenePhase, model: PadModel) {
        switch phase {
        case .background:
            model.enteredBackground()
        case .active where reconnectsByItself:
            Task { await model.scenePhaseChanged(to: .active) }
        default:
            break
        }
    }

    /// Not when a test pairs explicitly, asks for a quiet launch, or hosts XCTest.
    static var reconnectsByItself: Bool {
        let args = ProcessInfo.processInfo.arguments
        return !args.contains("-flashtexpad-test-mac") && !args.contains("-flashtexpad-no-autoreconnect")
            && ProcessInfo.processInfo.environment["XCTestConfigurationFilePath"] == nil
    }
}
