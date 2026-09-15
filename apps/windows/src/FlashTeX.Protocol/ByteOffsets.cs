// name: ByteOffsets.cs
// purpose: UTF-8 byte offset <-> .NET UTF-16 char index conversion helpers, ported
//   from apps/mac/Sources/FlashTeXProtocol/ByteOffsets.swift. Safety-critical: keeps
//   diagnostics/selections in sync with the Rust engine's byte-range addressing.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Text;

namespace FlashTeX.Protocol;

/// <summary>
/// Conversions between the wire protocol's zero-based, end-exclusive UTF-8 byte
/// offsets and .NET's UTF-16 char (code unit) indices. Offsets that are out of
/// range, reversed, or that land inside a multi-byte scalar are rejected (return
/// <c>null</c>) rather than silently rounded, matching the Swift source's
/// scalar-boundary checks.
/// </summary>
public static class ByteOffsets
{
    /// <summary>
    /// Maps a UTF-8 byte range of <paramref name="text"/> to the equivalent UTF-16
    /// char range, or <c>null</c> if either offset is out of bounds, reversed, or
    /// not on a Unicode scalar boundary.
    /// </summary>
    public static (int Utf16Start, int Utf16End)? Utf16RangeForUtf8Bytes(string text, int startByte, int endByte)
    {
        if (startByte < 0 || endByte < startByte)
        {
            return null;
        }

        int? utf16Start = startByte == 0 ? 0 : null;
        int? utf16End = endByte == 0 ? 0 : null;
        int byteOffset = 0;
        int charOffset = 0;
        foreach (var rune in text.EnumerateRunes())
        {
            byteOffset += rune.Utf8SequenceLength;
            charOffset += rune.Utf16SequenceLength;
            if (byteOffset == startByte)
            {
                utf16Start = charOffset;
            }
            if (byteOffset == endByte)
            {
                utf16End = charOffset;
            }
        }

        return utf16Start is int s && utf16End is int e ? (s, e) : null;
    }

    /// <summary>
    /// Reverse mapping: the UTF-8 byte range corresponding to a UTF-16 char range
    /// (e.g. an editor selection), or <c>null</c> if either endpoint is out of
    /// bounds, reversed, or splits a surrogate pair.
    /// </summary>
    public static (int StartByte, int EndByte)? Utf8ByteRangeForUtf16Range(string text, int utf16Start, int utf16End)
    {
        if (utf16Start < 0 || utf16End < utf16Start || utf16End > text.Length)
        {
            return null;
        }

        int? startByte = utf16Start == 0 ? 0 : null;
        int? endByte = utf16End == 0 ? 0 : null;
        int byteOffset = 0;
        int charOffset = 0;
        foreach (var rune in text.EnumerateRunes())
        {
            byteOffset += rune.Utf8SequenceLength;
            charOffset += rune.Utf16SequenceLength;
            if (charOffset == utf16Start)
            {
                startByte = byteOffset;
            }
            if (charOffset == utf16End)
            {
                endByte = byteOffset;
            }
        }

        return startByte is int s && endByte is int e ? (s, e) : null;
    }

    /// <summary>
    /// Byte-for-byte comparison of two strings' UTF-8 encodings. .NET's ordinal
    /// string comparison already compares UTF-16 code units exactly (unlike
    /// Swift's default <c>==</c>, which performs Unicode canonical-equivalence
    /// normalization), so it gives the "did the underlying bytes change" answer
    /// the Swift source's <c>memcmp</c> fast path exists for, without an extra
    /// UTF-8 encode of either operand.
    /// </summary>
    public static bool SameBytes(string a, string b) => string.Equals(a, b, StringComparison.Ordinal);

    /// <summary>UTF-8 byte length of <paramref name="text"/>.</summary>
    public static int Utf8ByteCount(string text) => Encoding.UTF8.GetByteCount(text);
}
