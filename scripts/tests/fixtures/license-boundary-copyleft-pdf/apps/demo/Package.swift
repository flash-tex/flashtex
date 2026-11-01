// swift-tools-version:5.9
// A comment naming poppler is not a violation.
import PackageDescription

let package = Package(
    name: "Demo",
    targets: [
        .target(name: "Demo", linkerSettings: [.linkedLibrary("mupdf")]) // FIXTURE-VIOLATION
    ]
)
