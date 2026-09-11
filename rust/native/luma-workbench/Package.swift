// swift-tools-version: 5.9

import PackageDescription

// Thin native PTY host for the Rust workbench (ADR-0007). `LumaWorkbenchCore` holds the logic that
// can be tested without a GUI; `LumaWorkbench` is the AppKit shell around SwiftTerm.
let package = Package(
    name: "luma-workbench",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "LumaWorkbench", targets: ["LumaWorkbench"]),
        .library(name: "LumaWorkbenchCore", targets: ["LumaWorkbenchCore"])
    ],
    dependencies: [
        .package(url: "https://github.com/migueldeicaza/SwiftTerm.git", exact: "1.15.0")
    ],
    targets: [
        .target(name: "LumaWorkbenchCore"),
        .executableTarget(
            name: "LumaWorkbench",
            dependencies: [
                "LumaWorkbenchCore",
                .product(name: "SwiftTerm", package: "SwiftTerm")
            ]
        ),
        .testTarget(
            name: "LumaWorkbenchCoreTests",
            dependencies: ["LumaWorkbenchCore"]
        )
    ]
)
