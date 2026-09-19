// swift-tools-version: 5.9
import PackageDescription

// FlashTeXPadKit: the iPad app's non-UI logic. It links the EXISTING contracts
// without a second protocol:
//   - `FlashTeXProtocol`  <- symlink to apps/mac/Sources/FlashTeXProtocol
//                            (runtime-v1 / transfer-v1 Codable models, UTF-8
//                            byte-offset discipline)
//   - `NearbyClient`      <- symlink to apps/mac/tools/nearby-client/Sources/NearbyClient
//                            (the reference companion client: TLS-PSK pairing,
//                            hello, destination_query, capture_submit)
//   - `FlashTeXEditorCore` <- symlink to apps/mac/Sources/FlashTeXEditorCore
//                            (the platform-free editor logic both editors run:
//                            syntax token model, environment editing rules,
//                            the Return key, delimiter matching, auto-close,
//                            the supported-latex vocabulary decoder)
// Nothing under the three symlinks is owned by apps/ios alone: a change there
// is a change to the Mac editor too and runs both test suites.
let package = Package(
    name: "FlashTeXPadKit",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "FlashTeXPadKit", targets: ["FlashTeXPadKit"]),
        .library(name: "NearbyClient", targets: ["NearbyClient"]),
        .library(name: "FlashTeXProtocol", targets: ["FlashTeXProtocol"]),
        .library(name: "FlashTeXEditorCore", targets: ["FlashTeXEditorCore"]),
    ],
    targets: [
        .target(name: "FlashTeXProtocol"),
        .target(name: "NearbyClient"),
        .target(name: "FlashTeXEditorCore", dependencies: ["FlashTeXProtocol"]),
        .target(
            name: "FlashTeXPadKit",
            dependencies: ["FlashTeXProtocol", "NearbyClient", "FlashTeXEditorCore"],
            // Complete checking on this target only (not the Mac-owned
            // NearbyClient / FlashTeXProtocol symlink targets). Experimental
            // rather than .unsafeFlags: Xcode refuses unsafe flags in
            // dependency packages. Tools 5.9; enableUpcomingFeature also
            // exists (5.8+) but the Swift 6.3 frontend still honours this
            // experimental spelling as -strict-concurrency=complete.
            swiftSettings: [
                .enableExperimentalFeature("StrictConcurrency")
            ]
        ),
        // XCTest is hosted by the generated iOS project (FlashTeXPadTests).
        // Keeping a second SwiftPM test target here makes Xcode discover the
        // symlinked protocol/client sources as overlapping test sources.
    ]
)
