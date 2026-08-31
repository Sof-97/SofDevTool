import Foundation
import Testing

@testable import SofDevTool

@Suite("Color Conversion Utility")
struct ColorConversionTests {
    @Test func publishedAndHandReviewedVectorsUseOneSRGBModel() throws {
        let red = try ColorConversionEngine.parse("hsl(0, 100%, 50%)")
        #expect(red == SRGBColor(red: 1, green: 0, blue: 0, alpha: 1))
        #expect(ColorConversionEngine.outputs(for: red).hex == "#ff0000")
        #expect(ColorConversionEngine.outputs(for: red).rgb == "rgb(255 0 0)")

        let teal = try ColorConversionEngine.parse("rgb(0%, 50%, 50% / 25%)")
        #expect(ColorConversionEngine.outputs(for: teal).hex == "#00808040")
    }

    @Test func shorthandAlphaAndAchromaticHueArePreserved() throws {
        let shorthand = try ColorConversionEngine.parse("#0f08")
        #expect(ColorConversionEngine.outputs(for: shorthand).hex == "#00ff0088")
        #expect(ColorConversionEngine.outputs(for: shorthand).rgb == "rgb(0 255 0 / 0.5333)")
        #expect(
            ColorConversionEngine.outputs(for: try ColorConversionEngine.parse("hsla(240deg, 0%, 50%, 0.5)"))
                .hsl == "hsl(0deg 0% 50.1961% / 0.502)")
    }

    @Test func hueWrapsWhileOtherRangesAndSyntaxAreStrict() throws {
        let wrapped = try ColorConversionEngine.parse("hsl(360deg 100% 50%)")
        let zero = try ColorConversionEngine.parse("hsl(0deg 100% 50%)")
        let negative = try ColorConversionEngine.parse("hsl(-120deg 100% 50%)")
        let positive = try ColorConversionEngine.parse("hsl(240deg 100% 50%)")
        #expect(wrapped == zero)
        #expect(negative == positive)
        for input in ["#12", "rgb(256 0 0)", "rgb(nan 0 0)", "hsl(0 101% 50%)", "display-p3(1 0 0)"] {
            #expect(throws: UtilityError.self) { try ColorConversionEngine.parse(input) }
        }
    }

    @Test func formattingIsRoundTripStableAtEightBitPrecision() throws {
        for input in ["#12345678", "rgb(12.5% 37.5% 90% / 33%)", "hsl(281deg 37% 42% / 0.73)"] {
            let color = try ColorConversionEngine.parse(input)
            let outputs = ColorConversionEngine.outputs(for: color)
            let fromHex = try ColorConversionEngine.parse(outputs.hex)
            let fromRGB = try ColorConversionEngine.parse(outputs.rgb)
            let fromHSL = try ColorConversionEngine.parse(outputs.hsl)
            #expect(fromHex == color)
            #expect(fromRGB == color)
            #expect(fromHSL == color)
        }
    }

    @Test func snapshotRetainsSourceAndNormalizedOutputs() throws {
        let color = try ColorConversionEngine.parse("rgba(1, 2, 3, 0.5)")
        let snapshot = ColorConversionSnapshot(
            source: "rgba(1, 2, 3, 0.5)", color: color, outputs: ColorConversionEngine.outputs(for: color))
        #expect(
            try JSONDecoder().decode(ColorConversionSnapshot.self, from: JSONEncoder().encode(snapshot))
                == snapshot)
    }
}
