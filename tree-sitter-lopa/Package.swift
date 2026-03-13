// swift-tools-version:5.3

import Foundation
import PackageDescription

var sources = ["src/parser.c"]
if FileManager.default.fileExists(atPath: "src/scanner.c") {
    sources.append("src/scanner.c")
}

let package = Package(
    name: "TreeSitterLopa",
    products: [
        .library(name: "TreeSitterLopa", targets: ["TreeSitterLopa"]),
    ],
    dependencies: [
        .package(name: "SwiftTreeSitter", url: "https://github.com/tree-sitter/swift-tree-sitter", from: "0.9.0"),
    ],
    targets: [
        .target(
            name: "TreeSitterLopa",
            dependencies: [],
            path: ".",
            sources: sources,
            resources: [
                .copy("queries")
            ],
            publicHeadersPath: "bindings/swift",
            cSettings: [.headerSearchPath("src")]
        ),
        .testTarget(
            name: "TreeSitterLopaTests",
            dependencies: [
                "SwiftTreeSitter",
                "TreeSitterLopa",
            ],
            path: "bindings/swift/TreeSitterLopaTests"
        )
    ],
    cLanguageStandard: .c11
)
