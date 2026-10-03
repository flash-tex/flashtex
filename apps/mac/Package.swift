// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "FlashTeXMac",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "FlashTeXMac", targets: ["FlashTeXMac"]),
        .library(name: "FlashTeXProtocol", targets: ["FlashTeXProtocol"]),
        .library(name: "FlashTeXEditorCore", targets: ["FlashTeXEditorCore"]),
        .library(name: "FlashTeXAccessibility", targets: ["FlashTeXAccessibility"]),
        .library(name: "FlashTeXDisplayListV3", targets: ["FlashTeXDisplayListV3"]),
    ],
    dependencies: [
        // Test-only: the reference companion client (apps/mac/tools/nearby-client)
        // drives the real NearbyListener in NearbyReferenceClientTests. The
        // package has no dependency back on this one, so there is no cycle.
        .package(path: "tools/nearby-client"),
        // Test-only: renders SwiftUI/AppKit views to PNGs inside a normal
        // `swift test` run, so a design pass can look at what it changed
        // instead of writing blind. Headless and deterministic -- no window
        // server, no screen-recording permission, which is what makes it
        // usable over SSH. Nothing in the app depends on it.
        .package(url: "https://github.com/pointfreeco/swift-snapshot-testing", from: "1.17.0"),
        // TeXpand's config reader (FlashTeXEditorCore): pure Swift, MIT, full
        // TOML 1.1. Chosen over TOMLKit/swift-toml (toml++ aborts on some
        // malformed headers) in docs/texpand/HOST.md. Pinned: bump deliberately.
        .package(url: "https://github.com/dduan/TOMLDecoder", exact: "0.4.5"),
    ],
    targets: [
        // Codable models for docs/contracts/runtime-v1.md plus offset conversion.
        .target(name: "FlashTeXProtocol"),
        // display-list-v3 (docs/protocol/display-list-v3.md): the engine
        // host's socket protocol and page decoder. MIT, Foundation only (no
        // AppKit, no engine code: the licence boundary is the host process,
        // DESIGN.md §3); decodes every stream exactly as the Rust reference
        // decoder (crates/display-list-v3) does.
        .target(name: "FlashTeXDisplayListV3"),
        // The display-list-v3 preview renderer (Core Graphics/Core Text):
        // decoded pages drawn exactly as Core Graphics draws the engine's PDF
        // (zero-tolerance parity, FlashTeXPreviewV3Tests).
        .target(name: "FlashTeXPreviewV3", dependencies: ["FlashTeXDisplayListV3"]),
        .testTarget(
            name: "FlashTeXPreviewV3Tests",
            dependencies: ["FlashTeXPreviewV3", "FlashTeXDisplayListV3"]
        ),
        .testTarget(
            name: "FlashTeXDisplayListV3Tests",
            dependencies: ["FlashTeXDisplayListV3"],
            resources: [.copy("Fixtures")]
        ),
        // Platform-free editor logic shared with the iPad app (apps/ios links
        // it through a symlink in FlashTeXPadKit): the syntax token model,
        // environment editing rules, the Return key, the delimiter matcher,
        // auto-close policy and the supported-latex vocabulary decoder.
        // Foundation only — no AppKit/UIKit may be imported here.
        .target(
            name: "FlashTeXEditorCore",
            dependencies: ["FlashTeXProtocol", .product(name: "TOMLDecoder", package: "TOMLDecoder")]
        ),
        .executableTarget(
            name: "FlashTeXMac",
            dependencies: ["FlashTeXProtocol", "FlashTeXAccessibility", "FlashTeXEditorCore", "FlashTeXDisplayListV3", "FlashTeXPreviewV3"],
            // The compiler's command inventory (crates/compiler/supported/
            // supported-latex.json), synced by scripts/sync-supported-latex.sh;
            // Completion.Vocabulary is decoded from it. make-app.sh copies it
            // into Contents/Resources.
            resources: [
                .copy("Resources/supported-latex.json"),
                // JetBrains Mono (SIL OFL 1.1, licence bundled): the editor's
                // default face, registered per process at first font
                // resolution (EditorFontRegistration.swift).
                .copy("Resources/Fonts"),
            ]
        ),
        .testTarget(
            name: "FlashTeXProtocolTests",
            dependencies: ["FlashTeXProtocol"]
        ),
        // TeXpand (docs/texpand/): the abbreviation engine in
        // FlashTeXEditorCore, tested headlessly -- no app, no window, no
        // AppKit.
        .testTarget(
            name: "TeXpandTests",
            dependencies: ["FlashTeXEditorCore"],
            exclude: ["Golden"] // golden tables, read by path (GoldenTests.swift)
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
            dependencies: ["FlashTeXMac", "FlashTeXEditorCore", "HostedWindows", .product(name: "NearbyClient", package: "nearby-client")]
        ),
        // One test per UI surface, rendering it to `Tests/DesignSnapshots/
        // __Snapshots__/`. Kept apart from FlashTeXMacTests so a design pass
        // can run just this target in seconds, and so a snapshot diff never
        // reads as a behaviour regression.
        .testTarget(
            name: "DesignSnapshots",
            dependencies: [
                "FlashTeXMac",
                "HostedWindows",
                .product(name: "SnapshotTesting", package: "swift-snapshot-testing"),
            ],
            exclude: ["__Snapshots__"] // the recorded reference PNGs
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
