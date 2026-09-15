// name: InsertionTests.cs
// purpose: Unit tests for FlashTeX.Editor.Insertion (anchor creation, rebasing
//   through trailing context, and insertion text formatting), ported behavior
//   from apps/mac/Sources/FlashTeXMac/Insertion.swift.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Editor.Tests;

public class InsertionTests
{
    private const string Path = "main.tex";

    [Fact]
    public void MakeAnchorCapturesByteOffsetAndTrailingContext()
    {
        var anchor = Insertion.MakeAnchor("a1", Path, "hello world", caretUtf16: 6, revision: 3);
        Assert.NotNull(anchor);
        Assert.Equal(6, anchor!.ByteOffset);
        Assert.Equal(3, anchor.Revision);
        Assert.Equal("world", anchor.ContextAfter);
    }

    [Fact]
    public void MakeAnchorTruncatesContextToContextLengthGraphemeClusters()
    {
        string text = new string('x', Insertion.ContextLength + 10);
        var anchor = Insertion.MakeAnchor("a1", Path, text, caretUtf16: 0, revision: 1);
        Assert.NotNull(anchor);
        Assert.Equal(Insertion.ContextLength, anchor!.ContextAfter.Length);
    }

    [Fact]
    public void MakeAnchorAtEndOfBufferHasEmptyContext()
    {
        var anchor = Insertion.MakeAnchor("a1", Path, "hello", caretUtf16: 5, revision: 1);
        Assert.NotNull(anchor);
        Assert.Equal(5, anchor!.ByteOffset);
        Assert.Equal("", anchor.ContextAfter);
    }

    [Fact]
    public void MakeAnchorRefusesOutOfRangeCaret()
    {
        Assert.Null(Insertion.MakeAnchor("a1", Path, "hello", caretUtf16: 6, revision: 1));
        Assert.Null(Insertion.MakeAnchor("a1", Path, "hello", caretUtf16: -1, revision: 1));
    }

    [Fact]
    public void ResolveIsExactWhenRevisionMatchesAndAnchorStillFitsInBuffer()
    {
        var anchor = new InsertionAnchor("a1", Path, ByteOffset: 5, Revision: 2, ContextAfter: "world");
        var resolution = Insertion.Resolve(anchor, "hello world", revision: 2);
        var exact = Assert.IsType<Insertion.Resolution.Exact>(resolution);
        Assert.Equal(5, exact.ByteOffset);
    }

    [Fact]
    public void ResolveNeedsReselectionWhenSameRevisionButAnchorPastEndOfShrunkBuffer()
    {
        var anchor = new InsertionAnchor("a1", Path, ByteOffset: 50, Revision: 2, ContextAfter: "world");
        var resolution = Insertion.Resolve(anchor, "short", revision: 2);
        var refusal = Assert.IsType<Insertion.Resolution.NeedsReselection>(resolution);
        Assert.Equal("anchor beyond end of buffer", refusal.Reason);
    }

    [Fact]
    public void ResolveKeepsAnEmptyContextAnchorAtTheEndOfANewBuffer()
    {
        // The anchor was at the end of the buffer (no trailing context); after an
        // edit at a different revision it should be rebased to the new end.
        var anchor = new InsertionAnchor("a1", Path, ByteOffset: 5, Revision: 1, ContextAfter: "");
        var resolution = Insertion.Resolve(anchor, "hello there", revision: 2);
        var rebased = Assert.IsType<Insertion.Resolution.Rebased>(resolution);
        Assert.Equal(11, rebased.ByteOffset);
    }

    [Fact]
    public void ResolveRebasesThroughAUniqueContextMatchAfterTextWasInsertedBefore()
    {
        // Original: "hello world", anchor at byte 6 ("world"), context "world".
        // Text is inserted before the anchor: "prefix hello world".
        var anchor = new InsertionAnchor("a1", Path, ByteOffset: 6, Revision: 1, ContextAfter: "world");
        var resolution = Insertion.Resolve(anchor, "prefix hello world", revision: 2);
        var rebased = Assert.IsType<Insertion.Resolution.Rebased>(resolution);
        Assert.Equal("prefix hello world".IndexOf("world", StringComparison.Ordinal), rebased.ByteOffset);
    }

    [Fact]
    public void ResolveNeedsReselectionWhenContextWasDeleted()
    {
        var anchor = new InsertionAnchor("a1", Path, ByteOffset: 6, Revision: 1, ContextAfter: "world");
        var resolution = Insertion.Resolve(anchor, "hello there", revision: 2);
        var refusal = Assert.IsType<Insertion.Resolution.NeedsReselection>(resolution);
        Assert.Equal("destination text was deleted or changed", refusal.Reason);
    }

    [Fact]
    public void ResolveNeedsReselectionWhenContextIsAmbiguousAndOldOffsetIsNotAmongTheMatches()
    {
        // "world" now occurs twice, and neither occurrence is at the anchor's old byte offset.
        var anchor = new InsertionAnchor("a1", Path, ByteOffset: 999, Revision: 1, ContextAfter: "world");
        var resolution = Insertion.Resolve(anchor, "world hello world", revision: 2);
        var refusal = Assert.IsType<Insertion.Resolution.NeedsReselection>(resolution);
        Assert.Equal("destination is ambiguous after edits", refusal.Reason);
    }

    [Fact]
    public void ResolvePrefersTheOldByteOffsetWhenContextIsAmbiguousButOneMatchIsAtIt()
    {
        // "world" occurs twice; the anchor's old offset (6) is exactly the first occurrence.
        var anchor = new InsertionAnchor("a1", Path, ByteOffset: 6, Revision: 1, ContextAfter: "world");
        var resolution = Insertion.Resolve(anchor, "hello world world", revision: 2);
        var rebased = Assert.IsType<Insertion.Resolution.Rebased>(resolution);
        Assert.Equal(6, rebased.ByteOffset);
    }

    [Fact]
    public void InsertionTextAddsLeadingNewlineWhenNotAtLineStart()
    {
        string text = "hello world";
        int byteOffset = 5; // mid "hello", not right after a newline and not at index 0
        Assert.Equal("\nBODY\n", Insertion.InsertionText("BODY", text, byteOffset));
    }

    [Fact]
    public void InsertionTextOmitsLeadingNewlineRightAfterANewline()
    {
        string text = "line one\nline two";
        int byteOffset = "line one\n".Length; // right after the newline: at line start
        Assert.Equal("BODY\n", Insertion.InsertionText("BODY", text, byteOffset));
    }

    [Fact]
    public void InsertionTextOmitsTrailingNewlineRightBeforeANewline()
    {
        string text = "line one\nline two";
        int byteOffset = "line one".Length; // right before the newline: at line end
        Assert.Equal("\nBODY", Insertion.InsertionText("BODY", text, byteOffset));
    }

    [Fact]
    public void InsertionTextOmitsBothNewlinesAtStartAndEndOfAnEmptyBuffer()
    {
        Assert.Equal("BODY", Insertion.InsertionText("  BODY  ", "", 0));
    }

    [Fact]
    public void InsertionTextTrimsTheProposalBody()
    {
        Assert.Equal("BODY", Insertion.InsertionText("\n  BODY  \n", "", 0));
    }
}
