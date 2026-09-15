// name: ByteOffsetsTests.cs
// purpose: Unit tests for the ported UTF-8 byte offset <-> UTF-16 char index
//   conversion helpers (FlashTeX.Protocol.ByteOffsets) — safety-critical logic
//   this port must get exactly right, per the port plan.
// author: Claude Sonnet 5
// date: 2026-09-13

namespace FlashTeX.Protocol.Tests;

public class ByteOffsetsTests
{
    [Fact]
    public void AsciiOnlyTextMapsBytesToCharsOneToOne()
    {
        const string text = "hello world";
        var range = ByteOffsets.Utf16RangeForUtf8Bytes(text, 6, 11);
        Assert.NotNull(range);
        Assert.Equal(6, range!.Value.Utf16Start);
        Assert.Equal(11, range.Value.Utf16End);
        Assert.Equal("world", text[range.Value.Utf16Start..range.Value.Utf16End]);

        var reverse = ByteOffsets.Utf8ByteRangeForUtf16Range(text, 0, 5);
        Assert.NotNull(reverse);
        Assert.Equal(0, reverse!.Value.StartByte);
        Assert.Equal(5, reverse.Value.EndByte);
    }

    [Fact]
    public void MultiByteLatinScalarMapsAsOneCharTwoBytes()
    {
        // "h", "é" (U+00E9, 2 UTF-8 bytes / 1 UTF-16 unit), "llo": byte offsets
        // 0,1,3,4,5,6; char offsets 0,1,2,3,4,5.
        const string text = "héllo";
        Assert.Equal(6, ByteOffsets.Utf8ByteCount(text));

        var accentRange = ByteOffsets.Utf16RangeForUtf8Bytes(text, 1, 3);
        Assert.NotNull(accentRange);
        Assert.Equal(1, accentRange!.Value.Utf16Start);
        Assert.Equal(2, accentRange.Value.Utf16End);
        Assert.Equal("é", text[1..2]);

        // Splitting the accented character's 2-byte encoding must be rejected.
        Assert.Null(ByteOffsets.Utf16RangeForUtf8Bytes(text, 1, 2));

        var reverse = ByteOffsets.Utf8ByteRangeForUtf16Range(text, 1, 2);
        Assert.NotNull(reverse);
        Assert.Equal(1, reverse!.Value.StartByte);
        Assert.Equal(3, reverse.Value.EndByte);
    }

    [Fact]
    public void CjkTextMapsThreeBytesPerChar()
    {
        // Each of "日","本","語" is 3 UTF-8 bytes and 1 UTF-16 code unit.
        const string text = "日本語";
        Assert.Equal(9, ByteOffsets.Utf8ByteCount(text));

        var middleChar = ByteOffsets.Utf16RangeForUtf8Bytes(text, 3, 6);
        Assert.NotNull(middleChar);
        Assert.Equal(1, middleChar!.Value.Utf16Start);
        Assert.Equal(2, middleChar.Value.Utf16End);
        Assert.Equal("本", text[1..2]);

        var reverse = ByteOffsets.Utf8ByteRangeForUtf16Range(text, 1, 2);
        Assert.NotNull(reverse);
        Assert.Equal(3, reverse!.Value.StartByte);
        Assert.Equal(6, reverse.Value.EndByte);

        // A byte offset landing inside a 3-byte sequence must be rejected.
        Assert.Null(ByteOffsets.Utf16RangeForUtf8Bytes(text, 4, 6));
    }

    [Fact]
    public void AstralSurrogatePairCharacterIsHandledAsOneUnit()
    {
        // U+1D11E MUSICAL SYMBOL G CLEF: 4 UTF-8 bytes, 2 UTF-16 code units
        // (a surrogate pair). Text is "A" + clef + "B".
        string clef = char.ConvertFromUtf32(0x1D11E);
        string text = "A" + clef + "B";
        Assert.Equal(6, ByteOffsets.Utf8ByteCount(text)); // 1 + 4 + 1
        Assert.Equal(4, text.Length); // 1 + 2 (surrogate pair) + 1

        var clefRange = ByteOffsets.Utf16RangeForUtf8Bytes(text, 1, 5);
        Assert.NotNull(clefRange);
        Assert.Equal(1, clefRange!.Value.Utf16Start);
        Assert.Equal(3, clefRange.Value.Utf16End);
        Assert.Equal(clef, text[1..3]);

        // A byte offset landing inside the astral character's 4-byte encoding must be rejected.
        Assert.Null(ByteOffsets.Utf16RangeForUtf8Bytes(text, 1, 4));
        Assert.Null(ByteOffsets.Utf16RangeForUtf8Bytes(text, 2, 5));

        // A UTF-16 index splitting the surrogate pair must also be rejected.
        Assert.Null(ByteOffsets.Utf8ByteRangeForUtf16Range(text, 1, 2));

        var reverse = ByteOffsets.Utf8ByteRangeForUtf16Range(text, 1, 3);
        Assert.NotNull(reverse);
        Assert.Equal(1, reverse!.Value.StartByte);
        Assert.Equal(5, reverse.Value.EndByte);
    }

    [Fact]
    public void OutOfRangeAndReversedOffsetsAreRejected()
    {
        const string text = "abc";
        Assert.Null(ByteOffsets.Utf16RangeForUtf8Bytes(text, -1, 2));
        Assert.Null(ByteOffsets.Utf16RangeForUtf8Bytes(text, 2, 1));
        Assert.Null(ByteOffsets.Utf16RangeForUtf8Bytes(text, 0, 10));
        Assert.Null(ByteOffsets.Utf8ByteRangeForUtf16Range(text, 0, 10));
    }

    [Fact]
    public void SameBytesComparesOrdinalNotCanonicalEquivalence()
    {
        Assert.True(ByteOffsets.SameBytes("hello", "hello"));
        Assert.False(ByteOffsets.SameBytes("hello", "hellp"));
        Assert.False(ByteOffsets.SameBytes("hello", "hello "));
    }
}
