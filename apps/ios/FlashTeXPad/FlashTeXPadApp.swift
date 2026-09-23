import SwiftUI

@main
struct FlashTeXPadApp: App {
    @StateObject private var model = PadModel()

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
                    if !args.contains("-flashtexpad-test-mac"), !args.contains("-flashtexpad-no-autoreconnect"),
                       ProcessInfo.processInfo.environment["XCTestConfigurationFilePath"] == nil,
                       model.pairedMac != nil {
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
        }
    }
}
