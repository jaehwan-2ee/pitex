// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "TexApp",
    platforms: [
        .macOS(.v15)
    ],
    products: [
        .library(name: "AppPorts", targets: ["AppPorts"]),
        .library(name: "ProjectFeature", targets: ["ProjectFeature"]),
        .library(name: "EditorFeature", targets: ["EditorFeature"]),
        .library(name: "EditorMacAdapter", targets: ["EditorMacAdapter"]),
        .library(name: "BuildFeature", targets: ["BuildFeature"]),
        .library(name: "PDFFeature", targets: ["PDFFeature"]),
        .library(name: "SettingsFeature", targets: ["SettingsFeature"]),
        .library(name: "MacPlatform", targets: ["MacPlatform"]),
        .library(name: "AppShell", targets: ["AppShell"])
    ],
    dependencies: [
        .package(path: "../TexCore"),
        // 1.20.x ships Metal shaders that fail to compile under the current
        // toolchain/SDK; stay on the last CoreGraphics-renderer release.
        .package(url: "https://github.com/migueldeicaza/SwiftTerm.git", .upToNextMinor(from: "1.19.0"))
    ],
    targets: [
        .target(
            name: "AppPorts",
            dependencies: [
                .product(name: "TexDomain", package: "TexCore"),
                .product(name: "ProjectCore", package: "TexCore"),
                .product(name: "DocumentSessionCore", package: "TexCore"),
                .product(name: "BuildCore", package: "TexCore"),
                .product(name: "SyncTeXCore", package: "TexCore"),
                .product(name: "AICore", package: "TexCore")
            ]
        ),
        .target(
            name: "ProjectFeature",
            dependencies: [
                .product(name: "ProjectCore", package: "TexCore"),
                .product(name: "DocumentSessionCore", package: "TexCore"),
                "AppPorts"
            ]
        ),
        .target(
            name: "EditorFeature",
            dependencies: [
                .product(name: "DocumentSessionCore", package: "TexCore"),
                .product(name: "LanguageCore", package: "TexCore"),
                .product(name: "AICore", package: "TexCore"),
                "AppPorts"
            ]
        ),
        .target(
            name: "EditorMacAdapter",
            dependencies: [
                "EditorFeature",
                .product(name: "DocumentSessionCore", package: "TexCore"),
                "AppPorts"
            ]
        ),
        .target(
            name: "BuildFeature",
            dependencies: [
                .product(name: "BuildCore", package: "TexCore"),
                .product(name: "DocumentSessionCore", package: "TexCore"),
                "AppPorts"
            ]
        ),
        .target(
            name: "PDFFeature",
            dependencies: [
                .product(name: "SyncTeXCore", package: "TexCore"),
                .product(name: "DocumentSessionCore", package: "TexCore"),
                "AppPorts"
            ]
        ),
        .target(name: "SettingsFeature", dependencies: ["AppPorts"]),
        .target(
            name: "MacPlatform",
            dependencies: [
                "AppPorts",
                .product(name: "SwiftTerm", package: "SwiftTerm")
            ]
        ),
        .target(
            name: "AppShell",
            dependencies: [
                "AppPorts",
                "ProjectFeature",
                "EditorFeature",
                "EditorMacAdapter",
                "BuildFeature",
                "PDFFeature",
                "SettingsFeature",
                "MacPlatform"
            ]
        ),
        .testTarget(name: "TexAppTests", dependencies: ["AppShell"])
    ],
    swiftLanguageModes: [.v6]
)
