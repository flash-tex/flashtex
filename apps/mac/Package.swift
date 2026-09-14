// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "FlashTeXMac",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "FlashTeXMac", targets: ["FlashTeXMac"]),
        .library(name: "FlashTeXProtocol", targets: ["FlashTeXProtocol"]),
        .library(name: "FlashTeXAccessibility", targets: ["FlashTeXAccessibility"]),
    ],
    dependencies: [
        // Test-only: the reference companion client (apps/mac/tools/nearby-client)
        // drives the real NearbyListener in NearbyReferenceClientTests. The
        // package has no dependency back on this one, so there is no cycle.
        .package(path: "tools/nearby-client"),
    ],
    targets: [
        // Codable models for docs/contracts/runtime-v1.md plus offset conversion.
        .target(name: "FlashTeXProtocol"),
        .executableTarget(
            name: "FlashTeXMac",
            dependencies: ["FlashTeXProtocol", "FlashTeXAccessibility"],
            // The compiler's command inventory (crates/compiler/supported/
            // supported-latex.json), synced by scripts/sync-supported-latex.sh;
            // Completion.Vocabulary is decoded from it. make-app.sh copies it
            // into Contents/Resources.
            resources: [.copy("Resources/supported-latex.json")]
        ),
        .testTarget(
            name: "FlashTeXProtocolTests",
            dependencies: ["FlashTeXProtocol"]
        ),
        // Shared by both hosted test targets: builds the real `NSWindow`s the
        // tests measure, parked off every display so runs stay invisible to
        // whoever is using the Mac. Test-only; nothing in the app depends on it.
        .target(
            name: "HostedWindows",
            path: "Tests/HostedWindows"
        ),
        .testTarget(
            name: "FlashTeXMacTests",
            dependencies: ["FlashTeXMac", "HostedWindows", .product(name: "NearbyClient", package: "nearby-client")]
        ),
        // Pure accessibility models (reading sequence, editor navigation,
        // command table) plus the SwiftUI attachment views; depends only on
        // FlashTeXProtocol. Owner: mac-accessibility.
        .target(
            name: "FlashTeXAccessibility",
            dependencies: ["FlashTeXProtocol"]
        ),
        .testTarget(
            name: "FlashTeXAccessibilityTests",
            dependencies: ["FlashTeXAccessibility", "HostedWindows"]
        ),
    ]
)
