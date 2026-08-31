import Foundation
import ICU

struct ICURegexBackend: RegexBackend {
    func execute(
        _ request: RegexRequest,
        policy: RegexExecutionPolicy,
        cancellation: RegexCancellation
    ) throws -> RegexResult {
        let context = ICURegexOperationContext(
            cancellation: cancellation,
            deadline: DispatchTime.now().uptimeNanoseconds &+ policy.elapsedDeadlineNanoseconds
        )
        let pattern = Array(request.pattern.utf16)
        let input = Array(request.text.utf16)
        let replacement = Array(request.replacement.utf16)
        let names = captureNames(in: request.pattern)

        return try pattern.withUnsafeBufferPointer { patternBuffer in
            var parseError = UParseError()
            var status = U_ZERO_ERROR
            guard
                let regex = uregex_open(
                    patternBuffer.baseAddress,
                    Int32(patternBuffer.count),
                    icuFlags(request.flags),
                    &parseError,
                    &status
                )
            else {
                if isFailure(status) {
                    throw RegexFailure.invalidPattern(
                        line: Int(parseError.line) + 1,
                        offset: Int(parseError.offset) + 1
                    )
                }
                throw RegexFailure.backendFailure(Int32(status.rawValue))
            }
            defer { uregex_close(regex) }

            return try input.withUnsafeBufferPointer { inputBuffer in
                try configure(
                    regex,
                    input: inputBuffer,
                    policy: policy,
                    context: context,
                    status: &status
                )
                let matches = try collectMatches(
                    regex,
                    input: request.text,
                    names: names,
                    policy: policy,
                    context: context,
                    status: &status
                )
                let preview = try replacement.withUnsafeBufferPointer { replacementBuffer in
                    try replacementPreview(
                        regex,
                        replacement: replacementBuffer,
                        policy: policy,
                        context: context,
                        status: &status
                    )
                }
                return RegexResult(matches: matches, replacementPreview: preview)
            }
        }
    }

    private func configure(
        _ regex: OpaquePointer,
        input: UnsafeBufferPointer<UInt16>,
        policy: RegexExecutionPolicy,
        context: ICURegexOperationContext,
        status: inout UErrorCode
    ) throws {
        status = U_ZERO_ERROR
        uregex_setText(regex, input.baseAddress, Int32(input.count), &status)
        uregex_setTimeLimit(regex, policy.engineStepLimit, &status)
        uregex_setStackLimit(regex, policy.heapBacktrackingLimitBytes, &status)
        let pointer = Unmanaged.passUnretained(context).toOpaque()
        uregex_setMatchCallback(regex, icuRegexMatchCallback, pointer, &status)
        uregex_setFindProgressCallback(regex, icuRegexFindProgressCallback, pointer, &status)
        try throwIfFailed(status, context: context)
    }

    private func collectMatches(
        _ regex: OpaquePointer,
        input: String,
        names: [Int: String],
        policy: RegexExecutionPolicy,
        context: ICURegexOperationContext,
        status: inout UErrorCode
    ) throws -> [RegexMatch] {
        status = U_ZERO_ERROR
        let groupCount = Int(uregex_groupCount(regex, &status))
        try throwIfFailed(status, context: context)
        var results: [RegexMatch] = []
        var totalCaptures = 0
        let nsInput = input as NSString

        while uregex_findNext(regex, &status) != 0 {
            try throwIfFailed(status, context: context)
            guard results.count < policy.maximumMatches else { throw RegexFailure.matchLimit }
            let start = Int(uregex_start(regex, 0, &status))
            let end = Int(uregex_end(regex, 0, &status))
            try throwIfFailed(status, context: context)
            if start == end, !isComposedCharacterBoundary(start, in: nsInput) {
                continue
            }
            let wholeRange = NSRange(location: start, length: end - start)
            guard wholeRange.location >= 0, NSMaxRange(wholeRange) <= nsInput.length else {
                throw RegexFailure.backendFailure(-1)
            }
            totalCaptures += groupCount
            guard totalCaptures <= policy.maximumCaptures else { throw RegexFailure.captureLimit }
            var captures: [RegexCapture] = []
            captures.reserveCapacity(groupCount)
            for group in 1..<(groupCount + 1) {
                let groupStart = Int(uregex_start(regex, Int32(group), &status))
                let groupEnd = Int(uregex_end(regex, Int32(group), &status))
                try throwIfFailed(status, context: context)
                if groupStart < 0 {
                    captures.append(RegexCapture(index: group, name: names[group], value: nil, range: nil))
                } else {
                    let range = NSRange(location: groupStart, length: groupEnd - groupStart)
                    captures.append(
                        RegexCapture(
                            index: group,
                            name: names[group],
                            value: nsInput.substring(with: range),
                            range: RegexTextRange(location: range.location, length: range.length)
                        ))
                }
            }
            results.append(
                RegexMatch(
                    ordinal: results.count,
                    value: nsInput.substring(with: wholeRange),
                    range: RegexTextRange(location: wholeRange.location, length: wholeRange.length),
                    captures: captures
                ))
        }
        try throwIfFailed(status, context: context)
        return results
    }

    private func isComposedCharacterBoundary(_ location: Int, in input: NSString) -> Bool {
        guard location > 0, location < input.length else { return true }
        let sequence = input.rangeOfComposedCharacterSequence(at: location)
        return sequence.location == location
    }

    private func replacementPreview(
        _ regex: OpaquePointer,
        replacement: UnsafeBufferPointer<UInt16>,
        policy: RegexExecutionPolicy,
        context: ICURegexOperationContext,
        status: inout UErrorCode
    ) throws -> String {
        status = U_ZERO_ERROR
        uregex_reset(regex, 0, &status)
        try throwIfFailed(status, context: context)

        var probeStatus = U_ZERO_ERROR
        let required = Int(
            uregex_replaceAll(
                regex,
                replacement.baseAddress,
                Int32(replacement.count),
                nil,
                0,
                &probeStatus
            ))
        if probeStatus != U_BUFFER_OVERFLOW_ERROR && isFailure(probeStatus) {
            try throwIfFailed(probeStatus, context: context, replacement: true)
        }
        guard required <= policy.maximumReplacementOutputUTF16Length else {
            throw RegexFailure.replacementOutputLimit
        }
        status = U_ZERO_ERROR
        uregex_reset(regex, 0, &status)
        try throwIfFailed(status, context: context)
        var output = [UInt16](repeating: 0, count: max(required + 1, 1))
        let written = output.withUnsafeMutableBufferPointer { buffer in
            uregex_replaceAll(
                regex,
                replacement.baseAddress,
                Int32(replacement.count),
                buffer.baseAddress,
                Int32(buffer.count),
                &status
            )
        }
        try throwIfFailed(status, context: context, replacement: true)
        return String(decoding: output.prefix(Int(written)), as: UTF16.self)
    }

    private func throwIfFailed(
        _ status: UErrorCode,
        context: ICURegexOperationContext,
        replacement: Bool = false
    ) throws {
        guard isFailure(status) else { return }
        switch status {
        case U_REGEX_TIME_OUT: throw RegexFailure.engineStepLimit
        case U_REGEX_STACK_OVERFLOW: throw RegexFailure.backtrackingLimit
        case U_REGEX_STOPPED_BY_CALLER:
            if context.cancellation.isCancelled { throw RegexFailure.cancelled }
            throw RegexFailure.deadlineExceeded
        default:
            if replacement { throw RegexFailure.invalidReplacement }
            throw RegexFailure.backendFailure(Int32(status.rawValue))
        }
    }

    private func isFailure(_ status: UErrorCode) -> Bool { status.rawValue > U_ZERO_ERROR.rawValue }

    private func icuFlags(_ flags: RegexFlags) -> UInt32 {
        var result: UInt32 = 0
        if flags.caseInsensitive { result |= UInt32(UREGEX_CASE_INSENSITIVE.rawValue) }
        if flags.multiline { result |= UInt32(UREGEX_MULTILINE.rawValue) }
        if flags.dotMatchesNewlines { result |= UInt32(UREGEX_DOTALL.rawValue) }
        if flags.commentsAndWhitespace { result |= UInt32(UREGEX_COMMENTS.rawValue) }
        return result
    }

    private func captureNames(in pattern: String) -> [Int: String] {
        var names: [Int: String] = [:]
        var group = 0
        var escaped = false
        var inCharacterClass = false
        let characters = Array(pattern)
        var index = 0
        while index < characters.count {
            let character = characters[index]
            if escaped {
                escaped = false
                index += 1
                continue
            }
            if character == "\\" {
                escaped = true
                index += 1
                continue
            }
            if character == "[" {
                inCharacterClass = true
                index += 1
                continue
            }
            if character == "]" {
                inCharacterClass = false
                index += 1
                continue
            }
            guard character == "(", !inCharacterClass else {
                index += 1
                continue
            }
            if index + 1 < characters.count, characters[index + 1] == "?" {
                if index + 3 < characters.count, characters[index + 2] == "<",
                    characters[index + 3] != "=", characters[index + 3] != "!"
                {
                    let start = index + 3
                    var end = start
                    while end < characters.count, characters[end] != ">" { end += 1 }
                    if end < characters.count {
                        group += 1
                        names[group] = String(characters[start..<end])
                    }
                }
            } else {
                group += 1
            }
            index += 1
        }
        return names
    }
}

private final class ICURegexOperationContext {
    let cancellation: RegexCancellation
    let deadline: UInt64

    init(cancellation: RegexCancellation, deadline: UInt64) {
        self.cancellation = cancellation
        self.deadline = deadline
    }

    var shouldContinue: Bool {
        !cancellation.isCancelled && DispatchTime.now().uptimeNanoseconds < deadline
    }
}

private func icuRegexMatchCallback(_ pointer: UnsafeRawPointer?, _ steps: Int32) -> UBool {
    guard let pointer else { return 0 }
    let context = Unmanaged<ICURegexOperationContext>.fromOpaque(pointer).takeUnretainedValue()
    return context.shouldContinue ? 1 : 0
}

private func icuRegexFindProgressCallback(
    _ pointer: UnsafeRawPointer?,
    _ matchIndex: Int64
) -> UBool {
    guard let pointer else { return 0 }
    let context = Unmanaged<ICURegexOperationContext>.fromOpaque(pointer).takeUnretainedValue()
    return context.shouldContinue ? 1 : 0
}
