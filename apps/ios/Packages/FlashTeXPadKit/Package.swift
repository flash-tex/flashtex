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
// Nothing under the two symlinks is owned or edited by apps/ios.
let package = Package(
    name: "FlashTeXPadKit",
    platforms: [.iOS(.v17), .macOS(.v14)],
    products: [
        .library(name: "FlashTeXPadKit", targets: ["FlashTeXPadKit"]),
        .library(name: "NearbyClient", targets: ["NearbyClient"]),
        .library(name: "FlashTeXProtocol", targets: ["FlashTeXProtocol"]),
    ],
    targets: [
        .target(name: "FlashTeXProtocol"),
        .target(name: "NearbyClient"),
        .target(
            name: "FlashTeXPadKit",
            dependencies: ["FlashTeXProtocol", "NearbyClient"],
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
