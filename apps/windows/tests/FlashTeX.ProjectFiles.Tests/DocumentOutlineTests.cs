// name: DocumentOutlineTests.cs
// purpose: Unit tests for the DocumentOutline sidebar scanner, ported from
//   apps/mac/Sources/FlashTeXMac/DocumentOutline.swift: sections, nested
//   environments, labels, comment stripping, and float/theorem captions.
// author: Claude Sonnet 5
// date: 2026-09-14

using Xunit;

namespace FlashTeX.ProjectFiles.Tests;

public class DocumentOutlineTests
{
    [Fact]
    public void Scan_FindsSectionsWithTheirLevel()
    {
        const string text = "\\chapter{Intro}\n\\section{Background}\n\\subsection{Details}\n";

        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);

        Assert.Equal(3, items.Count);
        Assert.All(items, i => Assert.Equal(DocumentOutline.Kind.Section, i.Kind));
        Assert.Equal([0, 1, 2], items.Select(i => i.Level));
        Assert.Equal(["Intro", "Background", "Details"], items.Select(i => i.Title));
    }

    [Fact]
    public void Scan_TracksEnvironmentNestingDepth()
    {
        const string text = "\\begin{figure}\n\\begin{center}\n\\end{center}\n\\end{figure}\n";

        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);
        List<DocumentOutline.Item> environments = items.Where(i => i.Kind == DocumentOutline.Kind.Environment).ToList();

        Assert.Equal(2, environments.Count);
        Assert.Equal("figure", environments[0].Title);
        Assert.Equal(0, environments[0].Level);
        Assert.Equal("center", environments[1].Title);
        Assert.Equal(1, environments[1].Level);
    }

    [Fact]
    public void Scan_TheDocumentEnvironmentItselfIsNotAnOutlineEntry()
    {
        const string text = "\\begin{document}\n\\section{One}\n\\end{document}\n";

        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);

        Assert.DoesNotContain(items, i => i.Kind == DocumentOutline.Kind.Environment && i.Title == "document");
    }

    [Fact]
    public void Scan_FindsLabels()
    {
        const string text = "\\section{One}\\label{sec:one}\n";

        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);

        DocumentOutline.Item label = Assert.Single(items, i => i.Kind == DocumentOutline.Kind.Label);
        Assert.Equal("sec:one", label.Title);
    }

    [Fact]
    public void Scan_SkipsCommentedOutCommands()
    {
        const string text = "% \\section{Ignored}\n\\section{Real}\n";

        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);

        DocumentOutline.Item section = Assert.Single(items);
        Assert.Equal("Real", section.Title);
    }

    [Fact]
    public void Scan_DoesNotTreatAnEscapedPercentAsAComment()
    {
        const string text = "\\section{100\\% done}\n";

        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);

        // The escaped '%' does not start a comment, but the section-title
        // group itself excludes '{'/'}'/newline only, so the match still
        // resolves to the text up to the first '}'.
        Assert.Single(items);
    }

    [Fact]
    public void Scan_AnnotatesAFigureWithItsCaption()
    {
        const string text = "\\begin{figure}\nplot here\n\\caption{Loss curves over time}\n\\end{figure}\n";

        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);

        DocumentOutline.Item figure = Assert.Single(items, i => i.Kind == DocumentOutline.Kind.Environment);
        Assert.Equal("Loss curves over time", figure.Caption);
        Assert.Equal("figure: Loss curves over time", figure.DisplayTitle);
    }

    [Fact]
    public void Scan_AnnotatesATheoremWithItsOptionalTitle()
    {
        const string text = "\\begin{theorem}[Fermat]\nstatement\n\\end{theorem}\n";

        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);

        DocumentOutline.Item theorem = Assert.Single(items, i => i.Kind == DocumentOutline.Kind.Environment);
        Assert.Equal("Fermat", theorem.Caption);
    }

    [Fact]
    public void Scan_RecognizesACustomTheoremEnvironmentDeclaredWithNewtheorem()
    {
        const string text = "\\newtheorem{lem}{Lemma}\n\\begin{lem}\nbody text here\n\\end{lem}\n";

        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);

        DocumentOutline.Item env = Assert.Single(items, i => i.Kind == DocumentOutline.Kind.Environment);
        Assert.Equal("body text here", env.Caption);
    }

    [Fact]
    public void Current_ReturnsTheLastSectionAtOrBeforeTheCaret()
    {
        const string text = "\\section{One}\nabc\n\\section{Two}\ndef\n";
        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);

        DocumentOutline.Item? current = DocumentOutline.Current(text.IndexOf("def", StringComparison.Ordinal), items);

        Assert.NotNull(current);
        Assert.Equal("Two", current!.Title);
    }

    [Fact]
    public void Counts_TalliesEachKind()
    {
        const string text = "\\section{One}\n\\label{a}\n\\begin{figure}\n\\end{figure}\n";
        IReadOnlyList<DocumentOutline.Item> items = DocumentOutline.Scan(text);

        IReadOnlyDictionary<DocumentOutline.Kind, int> counts = DocumentOutline.Counts(items);

        Assert.Equal(1, counts[DocumentOutline.Kind.Section]);
        Assert.Equal(1, counts[DocumentOutline.Kind.Label]);
        Assert.Equal(1, counts[DocumentOutline.Kind.Environment]);
    }
}
