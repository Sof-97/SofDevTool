import Foundation
import Testing

@testable import SofDevTool

@Suite("Whitespace Conversion Utility")
struct WhitespaceConversionTests {
    @Test func explicitOperationsHaveIndependentSemantics() throws {
        #expect(try WhitespaceConversionEngine.transform("  café \n", operation: .edgeTrim) == "café")
        #expect(
            try WhitespaceConversionEngine.transform("  a  \r\n\tb\t\r", operation: .perLineTrim)
                == "a\r\nb\r")
        #expect(
            try WhitespaceConversionEngine.transform("a\t  b\n c", operation: .collapseHorizontalWhitespace)
                == "a b\n c")
        #expect(
            try WhitespaceConversionEngine.transform(" a\n\t b ", operation: .collapseAllWhitespace)
                == " a b ")
    }

    @Test func lineEndingNormalizationAndBlankRemovalPreserveDeclaredBoundary() throws {
        #expect(
            try WhitespaceConversionEngine.transform(
                "a\r\nb\rc\n", operation: .normalizeLineEndings, lineEnding: .lf) == "a\nb\nc\n")
        #expect(
            try WhitespaceConversionEngine.transform(
                "a\r\nb\rc\n", operation: .normalizeLineEndings, lineEnding: .crlf) == "a\r\nb\r\nc\r\n")
        #expect(
            try WhitespaceConversionEngine.transform("a\n \n\tb\n", operation: .removeBlankLines)
                == "a\n\tb\n")
    }

    @Test func tabsUseStopsAndSpacesToTabsOnlyTouchesIndentation() throws {
        #expect(
            try WhitespaceConversionEngine.transform(
                "a\tb\n\t👩🏽\u{200D}💻", operation: .tabsToSpaces, tabWidth: 4) == "a   b\n    👩🏽\u{200D}💻")
        #expect(
            try WhitespaceConversionEngine.transform(
                "        code  aligned", operation: .spacesToTabs, tabWidth: 4) == "\t\tcode  aligned")
        #expect(throws: UtilityError.self) {
            try WhitespaceConversionEngine.transform("x", operation: .tabsToSpaces, tabWidth: 0)
        }
    }

    @Test func dedentUsesVisualColumnsForMixedIndentation() throws {
        let input = "\talpha\n  beta\n\t  gamma\n   \n"
        #expect(
            try WhitespaceConversionEngine.transform(input, operation: .dedent, tabWidth: 4)
                == "  alpha\nbeta\n    gamma\n   \n")
        #expect(
            try WhitespaceConversionEngine.transform(" \t\n\t ", operation: .dedent, tabWidth: 4)
                == " \t\n\t ")
    }

    @Test func snapshotRoundTripsDeliberateNoOp() throws {
        let snapshot = WhitespaceConversionSnapshot(
            input: "cafe", output: "cafe", operation: .edgeTrim, lineEnding: .lf, tabWidth: 4)
        #expect(
            try JSONDecoder().decode(WhitespaceConversionSnapshot.self, from: JSONEncoder().encode(snapshot))
                == snapshot)
    }
}
