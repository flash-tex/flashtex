// name: EditorDiagnosticsTests.cs
// purpose: Unit tests for FlashTeX.Editor.EditorDiagnostics (byte-range
//   rebasing of diagnostics across edits, retained/carried marks, gap
//   categorization and grouping), ported behavior from
//   apps/mac/Sources/FlashTeXMac/EditorDiagnostics.swift.
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Editor.Tests;

public class EditorDiagnosticsTests
{
    private const string Path = "main.tex";

    private static CompileResult ResultWith(params Diagnostic[] diagnostics) =>
        new("proj", 1, Status.ok, [], diagnostics, null);

    private static Diagnostic Diag(int start, int end, string message = "boom", Severity severity = Severity.error, string? path = Path) =>
        new(severity, message, path is null ? null : new SourceRange(path, start, end), null);

    // MARK: rebasing across an edit, by position relative to the diagnostic's range

    [Fact]
    public void MarkKeepsExactRangeWhenTextIsUnchanged()
    {
        var result = ResultWith(Diag(5, 10));
        var report = EditorDiagnostics.MakeReport(result, null, Path, "0123456789ABCDEF", "0123456789ABCDEF");
        var mark = Assert.Single(report.Marks);
        Assert.Equal(5, mark.Range.Location);
        Assert.Equal(5, mark.Range.Length);
        Assert.Empty(report.StaleDiagnostics);
    }

    [Fact]
    public void MarkKeepsRangeWhenEditIsEntirelyBeforeIt()
    {
        // Original "AAAAABBBBB" (diagnostic bytes 5..10, "BBBBB"); insert "XX" at byte 0.
        var result = ResultWith(Diag(5, 10));
        var report = EditorDiagnostics.MakeReport(result, null, Path, "AAAAABBBBB", "XXAAAAABBBBB");
        var mark = Assert.Single(report.Marks);
        Assert.Equal(7, mark.Range.Location); // shifted right by the 2-byte insertion
        Assert.Equal(5, mark.Range.Length);
    }

    [Fact]
    public void MarkShiftsRangeWhenEditIsEntirelyAfterIt()
    {
        var result = ResultWith(Diag(0, 5));
        var report = EditorDiagnostics.MakeReport(result, null, Path, "AAAAABBBBB", "AAAAABBBBBXX");
        var mark = Assert.Single(report.Marks);
        Assert.Equal(0, mark.Range.Location);
        Assert.Equal(5, mark.Range.Length);
    }

    [Fact]
    public void MarkIsStaleWhenEditOverlapsTheStartOfItsRange()
    {
        // Diagnostic covers bytes 5..10 ("BBBBB"); an edit replaces bytes 3..7 ("AAB" -> "Z").
        var result = ResultWith(Diag(5, 10));
        var report = EditorDiagnostics.MakeReport(result, null, Path, "AAAAABBBBB", "AAAZBBB");
        Assert.Empty(report.Marks);
        var stale = Assert.Single(report.StaleDiagnostics);
        Assert.Equal("boom", stale.Message);
    }

    [Fact]
    public void MarkIsStaleWhenEditOverlapsTheEndOfItsRange()
    {
        // Diagnostic covers bytes 0..5 ("AAAAA"); an edit replaces bytes 3..7.
        var result = ResultWith(Diag(0, 5));
        var report = EditorDiagnostics.MakeReport(result, null, Path, "AAAAABBBBB", "AAAZBBB");
        Assert.Empty(report.Marks);
        Assert.Single(report.StaleDiagnostics);
    }

    [Fact]
    public void MarkIsStaleWhenEditFullyContainsItsRange()
    {
        // Diagnostic covers bytes 3..5 ("AA" inside "AAAAA"); edit replaces bytes 0..10 entirely.
        var result = ResultWith(Diag(3, 5));
        var report = EditorDiagnostics.MakeReport(result, null, Path, "AAAAABBBBB", "Z");
        Assert.Empty(report.Marks);
        Assert.Single(report.StaleDiagnostics);
    }

    [Fact]
    public void MarkIsStaleWhenItsRangeFullyContainsTheEdit()
    {
        // Diagnostic covers the whole original buffer's bytes 0..10; the edit replaces the middle (bytes 3..7).
        var result = ResultWith(Diag(0, 10));
        var report = EditorDiagnostics.MakeReport(result, null, Path, "AAAAABBBBB", "AAAZBBB");
        Assert.Empty(report.Marks);
        Assert.Single(report.StaleDiagnostics);
    }

    [Fact]
    public void MarksIgnoreDiagnosticsOfADifferentDocumentPath()
    {
        var result = ResultWith(Diag(0, 5, path: "other.tex"));
        var report = EditorDiagnostics.MakeReport(result, null, Path, "hello", "hello");
        Assert.Empty(report.Marks);
        Assert.Empty(report.StaleDiagnostics);
    }

    [Fact]
    public void MarksSkipDiagnosticsWithNoSource()
    {
        var result = ResultWith(Diag(0, 5, path: null));
        var report = EditorDiagnostics.MakeReport(result, null, Path, "hello", "hello");
        Assert.Empty(report.Marks);
        Assert.Empty(report.StaleDiagnostics);
    }

    [Fact]
    public void MarkRangeSnapsOutwardToGraphemeClusterBoundaries()
    {
        // Decomposed base letter + combining acute accent (U+0301) + "b": one
        // grapheme cluster (2 UTF-16 chars / 3 UTF-8 bytes), then "b" (1
        // char/byte). Byte range 0..1 is a valid scalar boundary (it names
        // only the base-letter scalar) but splits the combined cluster.
        string text = "e" + "́" + "b";
        var result = ResultWith(Diag(0, 1));
        var report = EditorDiagnostics.MakeReport(result, null, Path, text, text);
        var mark = Assert.Single(report.Marks);
        // Snapped outward to the whole cluster (chars 0..2), never splitting it.
        Assert.Equal(0, mark.Range.Location);
        Assert.Equal(2, mark.Range.Length);
    }

    [Fact]
    public void IdentityKeyIsStableAcrossRebasingSoAMarkAndAListRowAgree()
    {
        var result = ResultWith(Diag(5, 10));
        var report = EditorDiagnostics.MakeReport(result, "r1", Path, "AAAAABBBBB", "XXAAAAABBBBB");
        var mark = Assert.Single(report.Marks);
        Assert.Equal("r1#0@main.tex:5..<10", mark.Id);
        Assert.Equal(5, mark.OriginalSource.StartByte); // identity keeps the ORIGINAL span, not the rebased one
        Assert.Equal(7, mark.Range.Location); // but the drawn range is rebased
    }

    // MARK: gap categorization

    [Theory]
    [InlineData("this is not implemented", true)]
    [InlineData("\\foo is not supported by this compiler version", true)]
    [InlineData("\\in is not supported in math mode", true)]
    [InlineData("\\bar is not supported in the document preamble", true)]
    [InlineData("\\includegraphics is unsupported", true)]
    [InlineData("undefined control sequence \\foo", false)]
    public void IsGapDetectsCompilerGapPhrases(string message, bool expected)
    {
        Assert.Equal(expected, EditorDiagnostics.IsGap(message));
    }

    [Fact]
    public void CountsSeparatesGapsFromErrorsAndWarnings()
    {
        var diagnostics = new List<Diagnostic>
        {
            Diag(0, 1, "undefined control sequence", Severity.error),
            Diag(0, 1, "some warning", Severity.warning),
            Diag(0, 1, "this is not implemented", Severity.error),
        };
        var (errors, warnings, gaps) = EditorDiagnostics.Counts(diagnostics);
        Assert.Equal(1, errors);
        Assert.Equal(1, warnings);
        Assert.Equal(1, gaps);
    }

    // MARK: retained/carried marks across a failed follow-up

    [Fact]
    public void KeepsPreviousMarksIsTrueOnlyForAFailureWithNoPages()
    {
        Assert.True(EditorDiagnostics.KeepsPreviousMarks(new CompileResult("p", 1, Status.failed, [], [], null)));
        Assert.False(EditorDiagnostics.KeepsPreviousMarks(new CompileResult("p", 1, Status.ok, [], [], null)));
        Assert.False(EditorDiagnostics.KeepsPreviousMarks(new CompileResult("p", 1, Status.recovered, [], [], null)));
        var withPages = new CompileResult("p", 1, Status.failed, [new Page(1, 10, 10, [])], [], null);
        Assert.False(EditorDiagnostics.KeepsPreviousMarks(withPages));
    }

    [Fact]
    public void RetainedAfterKeepsPreviousRecordThroughAFailedFollowUp()
    {
        var ok = new CompileResult("p", 1, Status.ok, [], [], null);
        var retained = EditorDiagnostics.RetainedAfter(ok, "r1", new Dictionary<string, string> { [Path] = "text" }, previous: null);
        Assert.NotNull(retained);
        Assert.Equal("r1", retained!.ResultId);

        var failed = new CompileResult("p", 2, Status.failed, [], [], null);
        var afterFailure = EditorDiagnostics.RetainedAfter(failed, "r2", new Dictionary<string, string>(), retained);
        Assert.Same(retained, afterFailure); // retention never chains: still the original record
    }

    [Fact]
    public void MakeReportWithRetentionKeepsAndFlagsMarksFromTheLastSuccessfulResultOnAFailedFollowUp()
    {
        var successful = new CompileResult("p", 3, Status.ok, [], [Diag(0, 5)], null);
        var retained = new EditorDiagnostics.Retained("r3", successful, new Dictionary<string, string> { [Path] = "AAAAABBBBB" });
        var failed = new CompileResult("p", 4, Status.failed, [], [], null);

        var report = EditorDiagnostics.MakeReportWithRetention(failed, "r4", retained, Path, compiledText: "AAAAABBBBB", currentText: "AAAAABBBBB");

        var mark = Assert.Single(report.Marks);
        Assert.NotNull(mark.CarriedFrom);
        Assert.Equal(3, mark.CarriedFrom!.Revision);
        Assert.Equal(4, mark.CarriedFrom.FailedRevision);
        Assert.NotNull(report.CarriedFrom);
        Assert.Contains("kept from revision 3", report.StaleNote);
    }

    [Fact]
    public void MakeReportWithRetentionIncludesFreshSourcedDiagnosticsOfTheFailedResultAlongsideCarriedOnes()
    {
        var successful = new CompileResult("p", 3, Status.ok, [], [Diag(0, 5, "old error")], null);
        var retained = new EditorDiagnostics.Retained("r3", successful, new Dictionary<string, string> { [Path] = "AAAAABBBBB" });
        var failed = new CompileResult("p", 4, Status.failed, [], [Diag(6, 8, "fresh error")], null);

        var report = EditorDiagnostics.MakeReportWithRetention(failed, "r4", retained, Path, compiledText: "AAAAABBBBB", currentText: "AAAAABBBBB");

        Assert.Equal(2, report.Marks.Count);
        Assert.Contains(report.Marks, m => m.Message == "fresh error" && m.CarriedFrom == null);
        Assert.Contains(report.Marks, m => m.Message == "old error" && m.CarriedFrom != null);
    }

    [Fact]
    public void MakeReportWithRetentionIsPlainReportWhenLatestResultIsNotAFailureWithNoOutput()
    {
        var successful = new CompileResult("p", 3, Status.ok, [], [Diag(0, 5)], null);
        var retained = new EditorDiagnostics.Retained("r3", successful, new Dictionary<string, string> { [Path] = "AAAAABBBBB" });
        var latest = new CompileResult("p", 5, Status.ok, [], [Diag(0, 3, "new")], null);

        var report = EditorDiagnostics.MakeReportWithRetention(latest, "r5", retained, Path, compiledText: "AAAAABBBBB", currentText: "AAAAABBBBB");

        var mark = Assert.Single(report.Marks);
        Assert.Equal("new", mark.Message);
        Assert.Null(mark.CarriedFrom);
        Assert.Null(report.CarriedFrom);
    }

    // MARK: grouping

    [Fact]
    public void GroupsCollectsIdenticalSeverityAndMessageAndCountsThem()
    {
        var diagnostics = new List<Diagnostic>
        {
            Diag(10, 12, "dup"),
            Diag(0, 2, "dup"),
            Diag(0, 2, "unique", Severity.warning),
        };
        var groups = EditorDiagnostics.Groups(diagnostics);
        Assert.Equal(2, groups.Count);
        var dup = groups.Single(g => g.Message == "dup");
        Assert.Equal(2, dup.Count);
        Assert.Equal("2× dup", dup.Title);
        // Occurrences are ordered by byte offset within the same path: byte 0 before byte 10.
        Assert.Equal(1, dup.First);
    }

    [Fact]
    public void GroupTitleIsPlainMessageWhenThereIsOnlyOneOccurrence()
    {
        var groups = EditorDiagnostics.Groups([Diag(0, 2, "solo")]);
        Assert.Equal("solo", Assert.Single(groups).Title);
    }

    [Fact]
    public void GroupsOrdersUnsourcedDiagnosticsAfterSourcedOnesAndByOriginalIndexAmongThemselves()
    {
        var diagnostics = new List<Diagnostic>
        {
            Diag(0, 1, "x", path: null),
            Diag(5, 6, "x"),
        };
        var groups = EditorDiagnostics.Groups(diagnostics);
        var group = Assert.Single(groups);
        Assert.Equal(1, group.First); // the sourced occurrence (index 1) sorts before the unsourced one (index 0)
    }

    [Fact]
    public void OccurrenceLabelUsesLineNumberWhenTextIsKnownElseByteRange()
    {
        var diagnostics = new List<Diagnostic> { Diag(11, 13, "x") }; // byte 11 is on line 2 of "line1\nline2"
        var groups = EditorDiagnostics.Groups(diagnostics);
        var group = Assert.Single(groups);

        string withText = EditorDiagnostics.OccurrenceLabel(0, group, diagnostics, new Dictionary<string, string> { [Path] = "line1\nline2" });
        Assert.Equal("1 of 1: main.tex line 2", withText);

        string withoutText = EditorDiagnostics.OccurrenceLabel(0, group, diagnostics);
        Assert.Equal("1 of 1: main.tex bytes 11..<13", withoutText);
    }

    [Fact]
    public void OccurrenceLabelReportsNoSourceForAnUnsourcedDiagnostic()
    {
        var diagnostics = new List<Diagnostic> { Diag(0, 1, "x", path: null) };
        var group = Assert.Single(EditorDiagnostics.Groups(diagnostics));
        Assert.Equal("1 of 1: no source", EditorDiagnostics.OccurrenceLabel(0, group, diagnostics));
    }

    [Theory]
    [InlineData(0, "abc", 1)]
    [InlineData(1, "a\nb\nc", 1)]
    [InlineData(2, "a\nb\nc", 2)]
    [InlineData(4, "a\nb\nc", 3)]
    public void LineNumberCountsLineFeedsUpToTheOffset(int offset, string text, int expectedLine)
    {
        Assert.Equal(expectedLine, EditorDiagnostics.LineNumber(offset, text));
    }

    [Fact]
    public void LineNumberIsNullWhenOffsetIsOutOfRange()
    {
        Assert.Null(EditorDiagnostics.LineNumber(-1, "abc"));
        Assert.Null(EditorDiagnostics.LineNumber(100, "abc"));
    }

    [Fact]
    public void WithheldNoteSummarizesErrorsAndWarningsSeparately()
    {
        var result = ResultWith(Diag(3, 5, "e1", Severity.error), Diag(3, 5, "w1", Severity.warning));
        var report = EditorDiagnostics.MakeReport(result, null, Path, "AAAAABBBBB", "AAAZBBB");
        Assert.Equal(2, report.StaleCount);
        Assert.Contains("1 error and 1 warning", report.StaleNote);
    }
}
