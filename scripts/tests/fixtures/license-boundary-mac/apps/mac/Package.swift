// swift-tools-version:5.9
// A comment naming gpl-host and .linkedLibrary is not a violation.
import PackageDescription

let package = Package(
    name: "App",
    targets: [
        .executableTarget(name: "App", linkerSettings: [.linkedFramework("AppKit")]),
        .target(name: "Linked", linkerSettings: [.linkedLibrary("gpl_host")]), // FIXTURE-VIOLATION
    ]
)
