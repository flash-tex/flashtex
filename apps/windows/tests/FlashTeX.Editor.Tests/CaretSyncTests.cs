// name: CaretSyncTests.cs
// purpose: Unit tests for FlashTeX.Editor.CaretSync (source -> preview caret
//   containment and the sorted interval index), ported behavior from
//   apps/mac/Sources/FlashTeXMac/CaretSync.swift.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Editor.Tests;

public class CaretSyncTests
{
    private const string Path = "main.tex";

    private static PageItem TextItem(int start, int end, string? path = Path) =>
        new PageItem.OfText(new PageItem.TextItem("x", 0, 0, 10, path is null ? null : new SourceRange(path, start, end)));

    private static CompileResult Result(params Page[] pages) =>
        new("proj", 1, Status.ok, pages, [], null);

    [Theory]
    [InlineData(0, 5, 0, true)]
    [InlineData(0, 5, 4, true)]
    [InlineData(0, 5, 5, false)]
    [InlineData(0, 5, -1, false)]
    [InlineData(5, 5, 5, true)] // empty range contains only its own offset
    [InlineData(5, 5, 6, false)]
    [InlineData(5, 5, 4, false)]
    public void ContainsMatchesHalfOpenRangeWithEmptyRangeSpecialCase(int start, int end, int byteOffset, bool expected)
    {
        Assert.Equal(expected, CaretSync.Contains(start, end, byteOffset));
    }

    [Fact]
    public void ItemsContainingFindsOnlyItemsOfTheRequestedPathContainingTheByte()
    {
        var page1 = new Page(1, 100, 100, [TextItem(0, 5), TextItem(5, 10, "other.tex"), TextItem(10, 15)]);
        var page2 = new Page(2, 100, 100, [TextItem(0, 3)]);
        var result = Result(page1, page2);

        var hits = CaretSync.ItemsContaining(2, Path, result);
        Assert.Equal([(1, 0), (2, 0)], hits); // byte 2 is also inside page 2's [0,3) item

        var hits2 = CaretSync.ItemsContaining(12, Path, result);
        Assert.Equal([(1, 2)], hits2);

        // Byte 2 also exists on page 2, but under a different item (index 0 there).
        var hits3 = CaretSync.ItemsContaining(0, Path, result);
        Assert.Equal([(1, 0), (2, 0)], hits3);
    }

    [Fact]
    public void ItemsContainingSkipsNonTextItemsAndItemsWithNoSource()
    {
        var noSource = new PageItem.OfText(new PageItem.TextItem("x", 0, 0, 10, null));
        var rule = new PageItem.Unknown("rule", null, default);
        var page = new Page(1, 100, 100, [noSource, rule, TextItem(0, 5)]);
        var result = Result(page);

        Assert.Equal([(1, 2)], CaretSync.ItemsContaining(2, Path, result));
    }

    [Fact]
    public void IndicesByPageGroupsHitsByPageNumber()
    {
        var page = new Page(3, 100, 100, [TextItem(0, 5), TextItem(0, 5)]);
        var result = Result(page);

        var byPage = CaretSync.IndicesByPage(2, Path, result);
        Assert.Single(byPage);
        Assert.Equal(new HashSet<int> { 0, 1 }, byPage[3]);
    }

    [Theory]
    [InlineData("hello", 0, 0)]
    [InlineData("hello", 5, 5)]
    [InlineData("hello", 2, 2)]
    public void ByteOffsetOfCaretUtf16MapsAsciiOneToOne(string text, int caret, int expectedByte)
    {
        Assert.Equal(expectedByte, CaretSync.ByteOffsetOfCaretUtf16(caret, text));
    }

    [Theory]
    [InlineData(-1)]
    public void ByteOffsetOfCaretUtf16RefusesNegativeCaret(int caret)
    {
        Assert.Null(CaretSync.ByteOffsetOfCaretUtf16(caret, "hello"));
    }

    [Fact]
    public void ByteOffsetOfCaretUtf16RefusesCaretPastEndOfBuffer()
    {
        Assert.Null(CaretSync.ByteOffsetOfCaretUtf16(6, "hello"));
    }

    [Fact]
    public void ByteOffsetOfCaretUtf16RoundsDownACaretInsideASurrogatePair()
    {
        // U+1F600 (grinning face) is one surrogate pair (2 UTF-16 units), 4 UTF-8 bytes.
        // "a" then the emoji: text.Length == 3 (1 + 2).
        string text = "a😀";
        // Caret 1 sits exactly at the cluster boundary (valid): byte offset 1.
        Assert.Equal(1, CaretSync.ByteOffsetOfCaretUtf16(1, text));
        // Caret 2 splits the surrogate pair: rounds back to the cluster start (byte 1).
        Assert.Equal(1, CaretSync.ByteOffsetOfCaretUtf16(2, text));
        // Caret 3 (end of buffer) is the byte count of the whole string.
        Assert.Equal(5, CaretSync.ByteOffsetOfCaretUtf16(3, text));
    }

    [Fact]
    public void IndexMatchesLinearScanOnARandomizedFixture()
    {
        var random = new Random(1234);
        var items = new List<PageItem>();
        for (int i = 0; i < 500; i++)
        {
            int start = random.Next(0, 1000);
            int end = start + random.Next(0, 20);
            items.Add(TextItem(start, end));
        }
        var page = new Page(1, 100, 100, items);
        var result = Result(page);
        var index = new CaretSync.Index(result, Path);
        Assert.Equal(items.Count, index.Count);

        foreach (int byteOffset in new[] { 0, 1, 5, 10, 50, 500, 999, 1019 })
        {
            var expected = CaretSync.ItemsContaining(byteOffset, Path, result).OrderBy(h => h.Index).ToList();
            var actual = index.ItemsContaining(byteOffset).OrderBy(h => h.Index).ToList();
            Assert.Equal(expected, actual);
        }
    }

    [Fact]
    public void IndexHandlesNestedAndAdjacentRanges()
    {
        // [0,10) contains [2,4) and [4,6); a caret at 3 hits both the outer and the first inner.
        var page = new Page(1, 100, 100, [TextItem(0, 10), TextItem(2, 4), TextItem(4, 6)]);
        var result = Result(page);
        var index = new CaretSync.Index(result, Path);

        Assert.Equal([(1, 0), (1, 1)], index.ItemsContaining(3));
        Assert.Equal([(1, 0), (1, 2)], index.ItemsContaining(5));
        Assert.Equal([(1, 0)], index.ItemsContaining(7));
        Assert.Empty(index.ItemsContaining(10));
    }

    [Fact]
    public void IndexIndicesByPageMatchesNonIndexedConvenienceMethod()
    {
        var page = new Page(2, 100, 100, [TextItem(0, 5), TextItem(0, 5)]);
        var result = Result(page);
        var index = new CaretSync.Index(result, Path);

        var expected = CaretSync.IndicesByPage(2, Path, result);
        var actual = index.IndicesByPage(2);
        Assert.Equal(expected.Keys, actual.Keys);
        foreach (var key in expected.Keys)
        {
            Assert.Equal(expected[key], actual[key]);
        }
    }
}
