// name: NavigationTests.cs
// purpose: Tests for FlashTeX.Shell.Navigation: \begin/\end matching (forward,
//   backward, nested), \label/\ref matching (multiple refs, wrap-around),
//   byte-range rebasing across an edit before/after/overlapping the tracked
//   range, and the UTF-8-byte to UTF-16-char selection mapping (including
//   grapheme-cluster widening). Ported behavior-for-behavior from
//   apps/mac/Sources/FlashTeXMac/Navigation.swift's pure matching/rebasing
//   functions (the AppKit/ShellModel glue at the bottom of that file is out
//   of scope for this port).
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.RuntimeV1;
using FlashTeX.Shell;

namespace FlashTeX.Shell.Tests;

public class NavigationTests
{
    [Fact]
    public void CommandUses_FindsBeginEndLabelAndRefOccurrences()
    {
        const string text = "\\begin{align}\n\\label{eq:x}\nx = y\n\\end{align}\n\\ref{eq:x}";
        var uses = Navigation.CommandUses(text);

        Assert.Equal(new[] { "begin", "label", "end", "ref" }, uses.Select(u => u.Name));
        Assert.All(uses.Where(u => u.Name is "begin" or "end"), u => Assert.Equal("align", u.Arg));
        Assert.All(uses.Where(u => u.Name is "label" or "ref"), u => Assert.Equal("eq:x", u.Arg));
    }

    [Fact]
    public void CommandUses_SkipsCommandsInsideLineComments()
    {
        const string text = "% \\begin{align}\n\\begin{align}\\end{align}";
        var uses = Navigation.CommandUses(text);
        Assert.Equal(2, uses.Count);
    }

    [Fact]
    public void MatchingRange_BeginMatchesForwardToItsEnd()
    {
        const string text = "\\begin{align}\nx\n\\end{align}\n";
        var uses = Navigation.CommandUses(text);
        var begin = uses.Single(u => u.Name == "begin");
        var end = uses.Single(u => u.Name == "end");

        var result = Navigation.MatchingRange(text, begin.Range.Start);

        var found = Assert.IsType<Navigation.Target.Found>(result);
        Assert.Equal(end.Range, found.Range);
        Assert.Contains("Matched \\begin{align} → \\end{align}", found.Note);
    }

    [Fact]
    public void MatchingRange_EndMatchesBackwardToItsBegin()
    {
        const string text = "\\begin{align}\nx\n\\end{align}\n";
        var uses = Navigation.CommandUses(text);
        var begin = uses.Single(u => u.Name == "begin");
        var end = uses.Single(u => u.Name == "end");

        var result = Navigation.MatchingRange(text, end.Range.Start);

        var found = Assert.IsType<Navigation.Target.Found>(result);
        Assert.Equal(begin.Range, found.Range);
        Assert.Contains("Matched \\end{align} → \\begin{align}", found.Note);
    }

    [Fact]
    public void MatchingRange_NestedEnvironments_RespectDepth()
    {
        // begin(outer) begin(inner) end(inner) end(outer), same name so nesting
        // depth (not just name) decides which end/begin is the true counterpart.
        const string text = "\\begin{env}\\begin{env}\\end{env}\\end{env}";
        var uses = Navigation.CommandUses(text);
        Assert.Equal(4, uses.Count);
        var outerBegin = uses[0];
        var innerBegin = uses[1];
        var innerEnd = uses[2];
        var outerEnd = uses[3];

        var fromOuterBegin = Assert.IsType<Navigation.Target.Found>(Navigation.MatchingRange(text, outerBegin.Range.Start));
        Assert.Equal(outerEnd.Range, fromOuterBegin.Range);

        var fromInnerBegin = Assert.IsType<Navigation.Target.Found>(Navigation.MatchingRange(text, innerBegin.Range.Start));
        Assert.Equal(innerEnd.Range, fromInnerBegin.Range);

        var fromOuterEnd = Assert.IsType<Navigation.Target.Found>(Navigation.MatchingRange(text, outerEnd.Range.Start));
        Assert.Equal(outerBegin.Range, fromOuterEnd.Range);

        var fromInnerEnd = Assert.IsType<Navigation.Target.Found>(Navigation.MatchingRange(text, innerEnd.Range.Start));
        Assert.Equal(innerBegin.Range, fromInnerEnd.Range);
    }

    [Fact]
    public void MatchingRange_UnmatchedBegin_IsNotFound()
    {
        const string text = "\\begin{align}\nx\n";
        var uses = Navigation.CommandUses(text);
        var begin = uses.Single(u => u.Name == "begin");

        var result = Navigation.MatchingRange(text, begin.Range.Start);
        var notFound = Assert.IsType<Navigation.Target.NotFound>(result);
        Assert.Contains("has no matching \\end{align}", notFound.Reason);
    }

    [Fact]
    public void MatchingRange_LabelCyclesToTheNextReferenceAfterTheCaret()
    {
        const string text = "\\ref{eq1} A \\label{eq1} B \\ref{eq1} C \\ref{eq1}";
        var uses = Navigation.CommandUses(text);
        var label = uses.Single(u => u.Name == "label");
        var refs = uses.Where(u => u.Name == "ref").ToList();

        var result = Navigation.MatchingRange(text, label.Range.Start);

        var found = Assert.IsType<Navigation.Target.Found>(result);
        Assert.Equal(refs[1].Range, found.Range); // the ref immediately after the label, not the one before it
        Assert.Contains("Reference 2 of 3", found.Note);
    }

    [Fact]
    public void MatchingRange_LabelWrapsToTheFirstReferenceWhenCaretIsAfterTheLastOne()
    {
        const string text = "\\ref{eq1} \\ref{eq1} \\label{eq1}";
        var uses = Navigation.CommandUses(text);
        var label = uses.Single(u => u.Name == "label");
        var refs = uses.Where(u => u.Name == "ref").ToList();

        var result = Navigation.MatchingRange(text, label.Range.Start);

        var found = Assert.IsType<Navigation.Target.Found>(result);
        Assert.Equal(refs[0].Range, found.Range); // no ref after the label: wraps to the first one
        Assert.Contains("Reference 1 of 2", found.Note);
    }

    [Fact]
    public void MatchingRange_ReferenceJumpsToItsLabel()
    {
        const string text = "\\label{eq1} \\ref{eq1}";
        var uses = Navigation.CommandUses(text);
        var label = uses.Single(u => u.Name == "label");
        var reference = uses.Single(u => u.Name == "ref");

        var result = Navigation.MatchingRange(text, reference.Range.Start);

        var found = Assert.IsType<Navigation.Target.Found>(result);
        Assert.Equal(label.Range, found.Range);
    }

    [Fact]
    public void MatchingRange_MultiDocument_LabelInAnotherOpenDocumentIsFound()
    {
        var documents = new[]
        {
            new Document("main.tex", "\\ref{eq1}"),
            new Document("appendix.tex", "\\label{eq1}"),
        };
        var reference = Navigation.CommandUses(documents[0].Text).Single();

        var result = Navigation.MatchingRange(documents, "main.tex", reference.Range.Start);

        var found = Assert.IsType<Navigation.DocumentTarget.Found>(result);
        Assert.Equal("appendix.tex", found.Path);
    }

    [Fact]
    public void RebaseExactly_IdenticalText_MapsUnchangedWithNoNote()
    {
        const string text = "hello world";
        var result = Navigation.RebaseExactly(6, 11, text, text, "doc.tex");

        var mapped = Assert.IsType<Navigation.Rebased.Mapped>(result);
        Assert.Equal(6, mapped.Start);
        Assert.Equal(11, mapped.End);
        Assert.Null(mapped.Note);
    }

    [Fact]
    public void RebaseExactly_EditBeforeTheTrackedRange_ShiftsItByTheDelta()
    {
        const string baseline = "hello world";
        const string current = "XYZhello world";
        var start = baseline.IndexOf("world", StringComparison.Ordinal);
        var end = start + "world".Length;

        var result = Navigation.RebaseExactly(start, end, baseline, current, "doc.tex");

        var mapped = Assert.IsType<Navigation.Rebased.Mapped>(result);
        Assert.Equal(start + 3, mapped.Start);
        Assert.Equal(end + 3, mapped.End);
        Assert.NotNull(mapped.Note);
        Assert.Equal("world", current[mapped.Start..mapped.End]);
    }

    [Fact]
    public void RebaseExactly_EditAfterTheTrackedRange_LeavesItUnchanged()
    {
        const string baseline = "hello world";
        const string current = "hello worldXYZ";

        var result = Navigation.RebaseExactly(0, 5, baseline, current, "doc.tex");

        var mapped = Assert.IsType<Navigation.Rebased.Mapped>(result);
        Assert.Equal(0, mapped.Start);
        Assert.Equal(5, mapped.End);
        Assert.Null(mapped.Note);
    }

    [Fact]
    public void RebaseExactly_EditOverlappingTheTrackedRange_IsRefused()
    {
        const string baseline = "hello world";
        const string current = "hello there";
        var start = baseline.IndexOf("world", StringComparison.Ordinal);
        var end = start + "world".Length;

        var result = Navigation.RebaseExactly(start, end, baseline, current, "doc.tex");

        var refused = Assert.IsType<Navigation.Rebased.Refused>(result);
        Assert.Contains("overlap the edit", refused.Reason);
    }

    [Fact]
    public void EditorRange_SimpleAsciiSpan_MapsOneToOne()
    {
        const string text = "hello world";
        var result = Navigation.EditorRange(6, 11, text);

        var selected = Assert.IsType<Navigation.RangeMapping.Selected>(result);
        Assert.Equal(new Utf16Range(6, 11), selected.Range);
        Assert.Null(selected.WidenedFrom);
    }

    [Fact]
    public void EditorRange_OutOfBounds_IsRefused()
    {
        const string text = "hi";
        Assert.IsType<Navigation.RangeMapping.Refused>(Navigation.EditorRange(-1, 1, text));
        Assert.IsType<Navigation.RangeMapping.Refused>(Navigation.EditorRange(1, 0, text));
        Assert.IsType<Navigation.RangeMapping.Refused>(Navigation.EditorRange(0, 99, text));
    }

    [Fact]
    public void EditorRange_SpanInsideAComposedCharacterSequence_WidensToTheWholeCluster()
    {
        // 'e' + COMBINING ACUTE ACCENT (U+0301, 2 UTF-8 bytes) + 'x'.
        const string text = "e\u0301x";
        var accentByteStart = 1; // 'e' is byte 0; the combining mark starts at byte 1.
        var accentByteEnd = 3; // U+0301 is 2 bytes in UTF-8.

        var result = Navigation.EditorRange(accentByteStart, accentByteEnd, text);

        var selected = Assert.IsType<Navigation.RangeMapping.Selected>(result);
        Assert.Equal(new Utf16Range(0, 2), selected.Range); // widened to cover 'e' + the combining mark
        Assert.Equal(new Utf16Range(1, 2), selected.WidenedFrom);
    }
}
