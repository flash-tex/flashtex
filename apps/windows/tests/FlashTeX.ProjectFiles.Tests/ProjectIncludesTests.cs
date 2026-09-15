// name: ProjectIncludesTests.cs
// purpose: Unit tests for the ProjectIncludes lexical `\input`/`\include`
//   scanner, exercising the same matching rules as
//   apps/mac/Sources/FlashTeXMac/ProjectDocuments.swift's `ProjectIncludes`:
//   comment skipping, `\verb`/verbatim-environment skipping, bare `\input`
//   (no braces), nested braces in the argument, and non-literal detection.
// author: Claude Sonnet 5
// date: 2026-09-14

using Xunit;

namespace FlashTeX.ProjectFiles.Tests;

public class ProjectIncludesTests
{
    [Fact]
    public void Scan_FindsInputAndInclude_WithArgumentAndCommandSpans()
    {
        const string text = "before \\input{chapter1} middle \\include{chapter2} after";

        IReadOnlyList<ProjectIncludes.Reference> refs = ProjectIncludes.Scan(text);

        Assert.Equal(2, refs.Count);
        Assert.Equal(ProjectIncludes.Kind.Input, refs[0].Kind);
        Assert.Equal("chapter1", refs[0].Argument);
        Assert.Equal(text.IndexOf("\\input", StringComparison.Ordinal), refs[0].StartByte);
        Assert.Equal(text.IndexOf("{chapter1}", StringComparison.Ordinal) + "{chapter1}".Length, refs[0].EndByte);

        Assert.Equal(ProjectIncludes.Kind.Include, refs[1].Kind);
        Assert.Equal("chapter2", refs[1].Argument);
    }

    [Fact]
    public void Scan_SkipsACommentedOutInput()
    {
        const string text = "% \\input{ignored}\n\\input{real}\n";

        IReadOnlyList<ProjectIncludes.Reference> refs = ProjectIncludes.Scan(text);

        Assert.Single(refs);
        Assert.Equal("real", refs[0].Argument);
    }

    [Fact]
    public void Scan_SkipsInputInsideAVerbatimEnvironment()
    {
        const string text = "\\begin{verbatim}\n\\input{ignored}\n\\end{verbatim}\n\\input{real}\n";

        IReadOnlyList<ProjectIncludes.Reference> refs = ProjectIncludes.Scan(text);

        Assert.Single(refs);
        Assert.Equal("real", refs[0].Argument);
    }

    [Fact]
    public void Scan_SkipsInputInsideAVerbCommand()
    {
        const string text = "\\verb+\\input{ignored}+ \\input{real}";

        IReadOnlyList<ProjectIncludes.Reference> refs = ProjectIncludes.Scan(text);

        Assert.Single(refs);
        Assert.Equal("real", refs[0].Argument);
    }

    [Fact]
    public void Scan_AcceptsABareInputWithoutBraces()
    {
        const string text = "\\input chapter1 \\input{chapter2}";

        IReadOnlyList<ProjectIncludes.Reference> refs = ProjectIncludes.Scan(text);

        Assert.Equal(2, refs.Count);
        Assert.Equal("chapter1", refs[0].Argument);
        Assert.Equal("chapter2", refs[1].Argument);
    }

    [Fact]
    public void Scan_DoesNotAcceptABareIncludeWithoutBraces()
    {
        const string text = "\\include chapter1";

        IReadOnlyList<ProjectIncludes.Reference> refs = ProjectIncludes.Scan(text);

        Assert.Empty(refs);
    }

    [Fact]
    public void Scan_HandlesNestedBracesInTheArgument()
    {
        const string text = "\\input{a{nested}b}";

        IReadOnlyList<ProjectIncludes.Reference> refs = ProjectIncludes.Scan(text);

        Assert.Single(refs);
        Assert.Equal("a{nested}b", refs[0].Argument);
        Assert.True(refs[0].Literal);
    }

    [Theory]
    [InlineData("\\jobname", false)]
    [InlineData("chapter#1", false)]
    [InlineData("chapter1", true)]
    public void Scan_MarksArgumentsWithBackslashOrHashAsNonLiteral(string argument, bool expectedLiteral)
    {
        string text = $"\\input{{{argument}}}";

        IReadOnlyList<ProjectIncludes.Reference> refs = ProjectIncludes.Scan(text);

        Assert.Single(refs);
        Assert.Equal(expectedLiteral, refs[0].Literal);
    }

    [Fact]
    public void Scan_RespectsTheReferenceLimit()
    {
        string text = string.Concat(Enumerable.Repeat("\\input{a}\n", 10));

        IReadOnlyList<ProjectIncludes.Reference> refs = ProjectIncludes.Scan(text, limit: 3);

        Assert.Equal(3, refs.Count);
    }

    [Theory]
    [InlineData("chapters/one", "chapters/one")]
    [InlineData("./one", "one")]
    [InlineData("a/./b", "a/b")]
    [InlineData("a/b/../c", "a/c")]
    public void Normalize_CollapsesDotAndDotDotSegments(string raw, string expected)
    {
        Assert.Equal(expected, ProjectIncludes.Normalize(raw));
    }

    [Theory]
    [InlineData("/absolute")]
    [InlineData("~/home")]
    [InlineData("../escapes")]
    [InlineData("")]
    [InlineData("has\\backslash")]
    public void Normalize_RejectsInvalidPaths(string raw)
    {
        Assert.Throws<ProjectIncludes.PathFormatException>(() => ProjectIncludes.Normalize(raw));
    }

    [Fact]
    public void Candidates_AppendsTexExtensionUnlessAlreadyPresent()
    {
        Assert.Equal(["chapter1.tex", "chapter1"], ProjectIncludes.Candidates("chapter1"));
        Assert.Equal(["chapter1.tex"], ProjectIncludes.Candidates("chapter1.tex"));
    }
}
