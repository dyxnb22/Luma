// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "Luma",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "Luma", targets: ["LumaApp"]),
        .library(name: "LumaCore", targets: ["LumaCore"]),
        .library(name: "LumaModules", targets: ["LumaModules"]),
        .library(name: "LumaServices", targets: ["LumaServices"]),
        .library(name: "LumaInfrastructure", targets: ["LumaInfrastructure"])
    ],
    dependencies: [],
    targets: [
        .executableTarget(
            name: "LumaApp",
            dependencies: ["LumaCore", "LumaModules", "LumaServices", "LumaInfrastructure"]
        ),
        .target(
            name: "LumaCore",
            dependencies: []
        ),
        .target(
            name: "LumaModules",
            dependencies: ["LumaCore", "LumaServices"]
        ),
        .target(
            name: "LumaServices",
            dependencies: ["LumaCore", "LumaInfrastructure"],
            linkerSettings: [
                .linkedFramework("Translation", .when(platforms: [.macOS])),
                .linkedFramework("EventKit", .when(platforms: [.macOS])),
                .linkedFramework("AVFoundation", .when(platforms: [.macOS])),
                .linkedFramework("UserNotifications", .when(platforms: [.macOS]))
            ]
        ),
        .target(
            name: "LumaInfrastructure",
            dependencies: ["LumaCore"]
        ),
        .testTarget(
            name: "LumaCoreTests",
            dependencies: ["LumaCore", "LumaModules"]
        ),
        .testTarget(
            name: "LumaModulesTests",
            dependencies: ["LumaModules", "LumaCore", "LumaServices"]
        ),
        .testTarget(
            name: "LumaInfrastructureTests",
            dependencies: ["LumaInfrastructure", "LumaCore"]
        ),
        .testTarget(
            name: "LumaServicesTests",
            dependencies: ["LumaServices", "LumaCore"]
        )
    ]
)
