import XCTest
import SwiftTreeSitter
import TreeSitterLopa

final class TreeSitterLopaTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_lopa())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading lopa grammar")
    }
}
