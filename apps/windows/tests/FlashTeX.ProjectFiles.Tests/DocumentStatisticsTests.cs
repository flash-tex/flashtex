// name: DocumentStatisticsTests.cs
// purpose: Unit tests for the DocumentStatistics texcount-style word/math
//   scanner, ported from apps/mac/Sources/FlashTeXMac/DocumentStatistics.swift.
//   Covers word-count exclusion rules: comments, math mode, verbatim
//   environments, the preamble, and reference/citation-style command
//   arguments — the cases the port's own doc comment calls out explicitly.
// author: Claude Sonnet 5
// date: 2026-09-14

using Xunit;

namespace FlashTeX.ProjectFiles.Tests;

public class DocumentStatisticsTests
{
    private const string PreambleAndBegin = "\\documentclass{article}\n\\begin{document}\n";
    private const string End = "\\end{document}\n";

    private static DocumentStatistics.Counts CountsOf(string body) =>
        DocumentStatistics.Analyze(PreambleAndBegin + body + End).Counts;

    [Fact]
    public void Analyze_CountsPlainBodyWords()
    {
        DocumentStatistics.Counts counts = CountsOf("one two three\n");

        Assert.Equal(3, counts.BodyWords);
        Assert.Equal(3, counts.TotalWords);
    }

    [Fact]
    public void Analyze_ExcludesThePreambleFromWordCounts()
    {
        // "documentclass" and "article" appear only before \begin{document}.
        DocumentStatistics.Counts counts = DocumentStatistics.Analyze(PreambleAndBegin + "one\n" + End).Counts;

        Assert.Equal(1, counts.BodyWords);
    }

    [Fact]
    public void Analyze_ExcludesCommentedOutText()
    {
        DocumentStatistics.Counts counts = CountsOf("one two % three four\nfive\n");

        Assert.Equal(3, counts.BodyWords);
    }

    [Theory]
    [InlineData("Inline math $x + y$ ends here.", 1, 0)]
    [InlineData("Display math \\[x + y\\] ends here.", 0, 1)]
    [InlineData("Double-dollar $$x + y$$ display.", 0, 1)]
    [InlineData("Paren math \\(x + y\\) inline.", 1, 0)]
    public void Analyze_CountsMathBlocksWithoutCountingTheirContentAsWords(string body, int expectedInline, int expectedDisplay)
    {
        DocumentStatistics.Counts counts = CountsOf(body + "\n");

        Assert.Equal(expectedInline, counts.InlineMath);
        Assert.Equal(expectedDisplay, counts.DisplayMath);
    }

    [Fact]
    public void Analyze_ExcludesMathEnvironmentContentButCountsOneDisplayBlock()
    {
        DocumentStatistics.Counts counts = CountsOf("before\n\\begin{align}\nx &= y + z \\\\\n\\end{align}\nafter\n");

        Assert.Equal(2, counts.BodyWords); // "before" and "after" only
        Assert.Equal(1, counts.DisplayMath);
    }

    [Fact]
    public void Analyze_ExcludesVerbatimEnvironmentContent()
    {
        DocumentStatistics.Counts counts = CountsOf("before\n\\begin{verbatim}\ncode here not prose\n\\end{verbatim}\nafter\n");

        Assert.Equal(2, counts.BodyWords); // "before" and "after" only
    }

    [Fact]
    public void Analyze_ExcludesReferenceAndCitationArguments()
    {
        DocumentStatistics.Counts counts = CountsOf("see \\label{sec:one}\\ref{sec:one}\\cite{knuth1986} for details\n");

        Assert.Equal(3, counts.BodyWords); // "see", "for", "details"
    }

    [Fact]
    public void Analyze_CountsSectionHeadingsSeparatelyFromBodyWords()
    {
        DocumentStatistics.Result result = DocumentStatistics.Analyze(PreambleAndBegin + "\\section{A Great Heading}\nbody text\n" + End);

        Assert.Equal(3, result.Counts.HeaderWords);
        Assert.Equal(2, result.Counts.BodyWords);
        Assert.Single(result.Sections);
        Assert.Equal("A Great Heading", result.Sections[0].Title);
        Assert.Equal(1, result.Sections[0].Level);
    }

    [Fact]
    public void Analyze_CountsCaptionWordsSeparately()
    {
        DocumentStatistics.Counts counts = CountsOf("\\begin{figure}\n\\caption{A helpful caption}\n\\end{figure}\n");

        Assert.Equal(3, counts.CaptionWords);
        Assert.Equal(0, counts.BodyWords);
    }

    [Fact]
    public void Analyze_TransparentCommandsCountTheirArgumentAsProse()
    {
        // \textbf is not on the skip list: its name contributes no words, but
        // the argument text counts normally (texcount's default for an
        // unlisted command).
        DocumentStatistics.Counts counts = CountsOf("\\textbf{bold words here}\n");

        Assert.Equal(3, counts.BodyWords);
    }

    [Fact]
    public void Analyze_TruncatesRatherThanScansAnOversizedDocument()
    {
        string huge = new string('a', DocumentStatistics.MaxScannedBytes + 1);

        DocumentStatistics.Result result = DocumentStatistics.Analyze(huge);

        Assert.True(result.Truncated);
        Assert.Equal(0, result.Counts.TotalWords);
    }

    [Fact]
    public void Analyze_OfMultipleDocuments_SumsTotalsAndTagsSectionsWithTheirPath()
    {
        var documents = new List<(string Path, string Text)>
        {
            ("main.tex", PreambleAndBegin + "\\section{Intro}\nfirst doc words\n" + End),
            ("chapter1.tex", "second doc has more words here\n"),
        };

        DocumentStatistics.AggregateResult aggregate = DocumentStatistics.Analyze(documents);

        Assert.Equal(2, aggregate.DocumentCount);
        Assert.Equal(3 + 6, aggregate.Total.BodyWords);
        Assert.Single(aggregate.Sections);
        Assert.Equal("main.tex", aggregate.Sections[0].DocumentPath);
    }

    [Fact]
    public void WordCount_OfASelectionExcludesMathTheSameWayFullDocumentCountingDoes()
    {
        const string text = "one two $x + y$ three";

        int count = DocumentStatistics.WordCount(text, new Utf16Range(0, text.Length));

        Assert.Equal(3, count); // "one", "two", "three" — the math content is not prose
    }
}
