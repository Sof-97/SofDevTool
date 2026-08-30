import PierreDiffsSwift
@testable import PierreDiffSpikeSupport
import Testing

@Test func ordinaryTextKeepsWordLevelHighlighting() {
    #expect(TextDiffRenderPolicy.lineDiffType(old: "café", new: "CAFÉ") == .wordAlt)
}

@Test func zwjEmojiUsesWholeLineHighlighting() {
    #expect(TextDiffRenderPolicy.lineDiffType(old: "👩🏽‍💻", new: "👩🏽‍🚀") == .none)
}

@Test func flagsAndKeycapsUseWholeLineHighlighting() {
    #expect(TextDiffRenderPolicy.lineDiffType(old: "🇮🇹 1️⃣", new: "🇫🇷 2️⃣") == .none)
}

@Test func singleScalarEmojiKeepsWordLevelHighlighting() {
    #expect(TextDiffRenderPolicy.lineDiffType(old: "😀", new: "😃") == .wordAlt)
}
