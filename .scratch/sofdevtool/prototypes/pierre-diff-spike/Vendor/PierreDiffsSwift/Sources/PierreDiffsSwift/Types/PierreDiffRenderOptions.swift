//
//  PierreDiffRenderOptions.swift
//  PierreDiffsSwift
//
//  Created by James Rochabrun on 6/4/26.
//

import Foundation

/// Built-in @pierre/diffs theme pair names.
public struct PierreDiffTheme: Codable, Sendable, Equatable {
  /// Theme name used when SwiftUI is in dark mode.
  public let dark: String

  /// Theme name used when SwiftUI is in light mode.
  public let light: String

  public init(dark: String, light: String) {
    self.dark = dark
    self.light = light
  }

  /// The default Pierre theme pair.
  public static let pierre = PierreDiffTheme(
    dark: "pierre-dark",
    light: "pierre-light"
  )

  /// Softer Pierre theme pair introduced by @pierre/diffs 1.2.
  public static let pierreSoft = PierreDiffTheme(
    dark: "pierre-dark-soft",
    light: "pierre-light-soft"
  )
}

/// How changed lines are marked in the diff gutter/body.
public enum DiffIndicatorStyle: String, CaseIterable, Identifiable, Codable, Sendable {
  case classic
  case bars
  case none

  public var id: String { rawValue }

  public var displayName: String {
    switch self {
    case .classic:
      return "Classic"
    case .bars:
      return "Bars"
    case .none:
      return "None"
    }
  }
}

/// How inline changes are highlighted inside changed lines.
public enum LineDiffType: String, CaseIterable, Identifiable, Codable, Sendable {
  case wordAlt = "word-alt"
  case word
  case char
  case none

  public var id: String { rawValue }

  public var displayName: String {
    switch self {
    case .wordAlt:
      return "Word Alt"
    case .word:
      return "Word"
    case .char:
      return "Character"
    case .none:
      return "None"
    }
  }
}

/// Built-in hunk separator styles supported by @pierre/diffs.
public enum HunkSeparatorStyle: String, CaseIterable, Identifiable, Codable, Sendable {
  case simple
  case metadata
  case lineInfo = "line-info"
  case lineInfoBasic = "line-info-basic"

  public var id: String { rawValue }

  public var displayName: String {
    switch self {
    case .simple:
      return "Simple"
    case .metadata:
      return "Metadata"
    case .lineInfo:
      return "Line Info"
    case .lineInfoBasic:
      return "Line Info Basic"
    }
  }
}

/// Additional @pierre/diffs render options exposed by PierreDiffView.
///
/// Defaults preserve the wrapper's historical rendering behavior.
public struct PierreDiffRenderOptions: Codable, Sendable, Equatable {
  public var theme: PierreDiffTheme
  public var diffIndicators: DiffIndicatorStyle
  public var hunkSeparators: HunkSeparatorStyle
  public var lineDiffType: LineDiffType
  public var disableLineNumbers: Bool
  public var disableFileHeader: Bool
  public var disableBackground: Bool
  public var expandUnchanged: Bool
  public var collapsedContextThreshold: Int?
  public var maxLineDiffLength: Int?
  public var expansionLineCount: Int?
  public var tokenizeMaxLength: Int?
  public var tokenizeMaxLineLength: Int?
  public var stickyHeader: Bool

  public init(
    theme: PierreDiffTheme = .pierre,
    diffIndicators: DiffIndicatorStyle = .bars,
    hunkSeparators: HunkSeparatorStyle = .lineInfo,
    lineDiffType: LineDiffType = .wordAlt,
    disableLineNumbers: Bool = false,
    disableFileHeader: Bool = false,
    disableBackground: Bool = false,
    expandUnchanged: Bool = false,
    collapsedContextThreshold: Int? = nil,
    maxLineDiffLength: Int? = nil,
    expansionLineCount: Int? = nil,
    tokenizeMaxLength: Int? = nil,
    tokenizeMaxLineLength: Int? = nil,
    stickyHeader: Bool = false
  ) {
    self.theme = theme
    self.diffIndicators = diffIndicators
    self.hunkSeparators = hunkSeparators
    self.lineDiffType = lineDiffType
    self.disableLineNumbers = disableLineNumbers
    self.disableFileHeader = disableFileHeader
    self.disableBackground = disableBackground
    self.expandUnchanged = expandUnchanged
    self.collapsedContextThreshold = collapsedContextThreshold
    self.maxLineDiffLength = maxLineDiffLength
    self.expansionLineCount = expansionLineCount
    self.tokenizeMaxLength = tokenizeMaxLength
    self.tokenizeMaxLineLength = tokenizeMaxLineLength
    self.stickyHeader = stickyHeader
  }
}
