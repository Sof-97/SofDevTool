import PierreDiffsSwift

public enum TextDiffRenderPolicy {
    public static func lineDiffType(old: String, new: String) -> LineDiffType {
        requiresWholeLineHighlighting(old: old, new: new) ? .none : .wordAlt
    }

    public static func requiresWholeLineHighlighting(old: String, new: String) -> Bool {
        containsComplexEmoji(old) || containsComplexEmoji(new)
    }

    private static func containsComplexEmoji(_ value: String) -> Bool {
        value.contains { character in
            character.unicodeScalars.count > 1
                && character.unicodeScalars.contains { $0.properties.isEmoji }
        }
    }
}
