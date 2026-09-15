// name: LaTeXSpellCheckTests.cs
// purpose: Unit tests for FlashTeX.Editor.LaTeXProse (the LaTeX-aware
//   prose/non-prose tokenizer), ported behavior from the `LaTeXProse` enum of
//   apps/mac/Sources/FlashTeXMac/LaTeXSpellCheck.swift.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Editor.Tests;

public class LaTeXSpellCheckTests
{
    private static string Ranges(string text) =>
        string.Join(",", LaTeXProse.ProseRanges(text).Select(r => $"{r.Location}-{r.End}"));

    // MARK: plain prose and control sequences

    [Fact]
    public void PlainProseIsOneRange()
    {
        Assert.Equal("0-11", Ranges("hello world"));
    }

    [Fact]
    public void CommandNameIsNotProseButTextAroundItIs()
    {
        // "see \ref{fig} here": "\ref{fig}" is skipped whole (skipArguments), "see " and " here" are prose.
        string text = "see \\ref{fig} here";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("see ", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" here", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void UnknownCommandWithNoArgumentsLeavesFollowingTextAsProse()
    {
        string text = "\\textbf hello";
        var ranges = LaTeXProse.ProseRanges(text);
        var range = Assert.Single(ranges);
        Assert.Equal(" hello", text[range.Location..range.End]);
    }

    [Fact]
    public void EscapedSpecialCharacterIsNotProseButSurroundingTextIs()
    {
        // "\%" is an escaped percent sign (not a comment starter), 2 units skipped.
        string text = "50\\%off";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("50", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal("off", text[ranges[1].Location..ranges[1].End]);
    }

    // MARK: comments

    [Fact]
    public void CommentRunsToEndOfLineAndIsNotProse()
    {
        // The scan stops right at the newline (never consuming it), and the
        // newline unit itself falls through to the tokenizer's default "plain
        // text" case, so it joins the following text into one prose range.
        string text = "before % a comment here\nafter";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("before ", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal("\nafter", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void MaskedTextReplacesNonProseWithSpacesButKeepsOffsetsAndNewlines()
    {
        string text = "a % comment\nb";
        string masked = LaTeXProse.MaskedText(text);
        Assert.Equal(text.Length, masked.Length);
        Assert.Equal('a', masked[0]);
        Assert.Equal('\n', masked[11]); // the newline itself is never replaced, even though it ends a masked comment
        Assert.Equal('b', masked[12]);
        Assert.All(masked[1..11], c => Assert.Equal(' ', c)); // "% comment" is fully masked to spaces
    }

    // MARK: math mode

    [Theory]
    [InlineData("inline $x + y$ done", "inline ", " done")]
    [InlineData("before \\(x+y\\) after", "before ", " after")]
    public void InlineMathIsNotProse(string text, string before, string after)
    {
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal(before, text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(after, text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void DisplayMathDollarDollarIsNotProse()
    {
        string text = "before $$x+y$$ after";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("before ", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" after", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void BracketDisplayMathIsNotProse()
    {
        string text = "before \\[x+y\\] after";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("before ", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" after", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void MathEnvironmentBodyIsNotProse()
    {
        string text = "before \\begin{equation}x+y\\end{equation} after";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("before ", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" after", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void NonMathEnvironmentBodyIsProse()
    {
        string text = "\\begin{quote}words here\\end{quote}";
        var ranges = LaTeXProse.ProseRanges(text);
        var range = Assert.Single(ranges);
        Assert.Equal("words here", text[range.Location..range.End]);
    }

    // MARK: verbatim-like content

    [Fact]
    public void VerbCommandBodyIsNotProseRegardlessOfDelimiter()
    {
        string text = "code \\verb|int x = 1;| done";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("code ", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" done", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void VerbatimEnvironmentBodyIsNotProse()
    {
        string text = "before \\begin{verbatim}raw <text> here\\end{verbatim} after";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("before ", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" after", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void TikzPictureEnvironmentBodyIsNotProse()
    {
        string text = "\\begin{tikzpicture}\\draw (0,0) -- (1,1);\\end{tikzpicture}";
        Assert.Empty(LaTeXProse.ProseRanges(text));
    }

    // MARK: reference/label/package/file/definition command arguments

    [Theory]
    [InlineData("cite")]
    [InlineData("label")]
    [InlineData("ref")]
    [InlineData("usepackage")]
    [InlineData("includegraphics")]
    [InlineData("bibliography")]
    public void SkipArgumentCommandsHideTheirBracketedArguments(string command)
    {
        string text = $"see \\{command}{{arg1}}[opt] more";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("see ", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" more", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void HrefHidesUrlButSecondArgumentIsProse()
    {
        string text = "\\href{https://x.test}{link text} more";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("link text", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" more", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void HyperrefHidesOnlyItsBracketedArgumentsNotAFollowingBraceGroup()
    {
        // Unlike href/textcolor/colorbox, hyperref's own first `{...}` is prose (it names a label, but the tokenizer treats it uniformly as text unless bracketed).
        string text = "\\hyperref[sec:x]{jump here} more";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("jump here", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" more", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void NewcommandDefinitionBodyIsSkipped()
    {
        string text = "\\newcommand{\\foo}[1]{bar #1} prose";
        var ranges = LaTeXProse.ProseRanges(text);
        var range = Assert.Single(ranges);
        Assert.Equal(" prose", text[range.Location..range.End]);
    }

    [Fact]
    public void DefCommandSkipsControlSequenceParameterTextAndBody()
    {
        string text = "\\def\\foo#1{bar} prose";
        var ranges = LaTeXProse.ProseRanges(text);
        var range = Assert.Single(ranges);
        Assert.Equal(" prose", text[range.Location..range.End]);
    }

    [Fact]
    public void UnknownCommandWithKeyValueOptionsHidesTheOptionListOnly()
    {
        // A generic command's `[key=value]` option list is not prose (contains '='); braces are unaffected.
        string text = "\\mycmd[scale=2]{prose here} after";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("prose here", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" after", text[ranges[1].Location..ranges[1].End]);
    }

    [Fact]
    public void UnknownCommandWithNonKeyValueBracketArgumentIsNotSkipped()
    {
        // No '=' inside the brackets: this is ordinary text, so it stays prose (only the command name itself is hidden).
        string text = "\\mycmd[plain] prose";
        var ranges = LaTeXProse.ProseRanges(text);
        Assert.Equal(2, ranges.Count);
        Assert.Equal("plain", text[ranges[0].Location..ranges[0].End]);
        Assert.Equal(" prose", text[ranges[1].Location..ranges[1].End]);
    }

    // MARK: accents

    [Fact]
    public void SymbolAccentHidesItsArgumentAndUnmarksThePrecedingPartialWord()
    {
        // "caf\'e" -> "café": the accent command (`\'`) and the letter it
        // accents are not prose, and the "caf" already marked prose before the
        // backslash is unmarked (it belongs to the same accented word), so
        // only the following " nice" remains.
        string text = "caf\\'e nice";
        var range = Assert.Single(LaTeXProse.ProseRanges(text));
        Assert.Equal(" nice", text[range.Location..range.End]);
    }

    [Fact]
    public void LetterAccentCommandHidesItsBraceArgumentAndUnmarksPrecedingWord()
    {
        // "Hru\v{s}ka": "\v{s}" accents "s"; "Hru" (already scanned as prose) is unmarked.
        string text = "Hru\\v{s}ka done";
        var ranges = LaTeXProse.ProseRanges(text);
        // Everything from the start of "Hru" through "ka" is non-prose (the accented word), leaving only " done".
        var range = Assert.Single(ranges);
        Assert.Equal(" done", text[range.Location..range.End]);
    }

    // MARK: mixed realistic document

    [Fact]
    public void MixedDocumentFlagsOnlyProseAcrossCommandsMathCommentsAndVerbatim()
    {
        string text = "Intro text. \\cite{key1} continues with $x^2$ math, then\n" +
                       "% a full comment line\n" +
                       "more prose \\verb|code| and \\begin{equation}y=1\\end{equation} end.";
        string masked = LaTeXProse.MaskedText(text);
        Assert.DoesNotContain("cite", masked);
        Assert.DoesNotContain("key1", masked);
        Assert.DoesNotContain("x^2", masked);
        Assert.DoesNotContain("a full comment line", masked);
        Assert.DoesNotContain("code", masked);
        Assert.DoesNotContain("y=1", masked);
        Assert.Contains("Intro text.", masked);
        Assert.Contains("continues with", masked);
        Assert.Contains("more prose", masked);
        Assert.Contains("end.", masked);
        Assert.Equal(text.Length, masked.Length);
    }

    [Fact]
    public void EmptyDocumentHasNoProseRanges()
    {
        Assert.Empty(LaTeXProse.ProseRanges(""));
    }

    [Fact]
    public void EntireDocumentOfOnlyPunctuationIsNotProse()
    {
        Assert.Empty(LaTeXProse.ProseRanges("{}[]~&#^_"));
    }
}
