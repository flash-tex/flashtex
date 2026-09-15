// name: CompletionTests.cs
// purpose: Unit tests for FlashTeX.Editor.Completion (matching/ranking) and
//   FlashTeX.Editor.CompletionVocabulary (the embedded compiler inventory),
//   ported behavior from apps/mac/Sources/FlashTeXMac/Completion.swift.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Security.Cryptography;

namespace FlashTeX.Editor.Tests;

public class CompletionTests
{
    // MARK: vocabulary

    [Fact]
    public void VocabularyLoadsFromTheEmbeddedResource()
    {
        Assert.NotEmpty(CompletionVocabulary.Names);
        Assert.Contains("section", CompletionVocabulary.Names);
        Assert.Contains("alpha", CompletionVocabulary.Names);
    }

    [Fact]
    public void BundledInventoryMatchesTheMacCopy()
    {
        // apps/windows/src/FlashTeX.Editor/Resources/supported-latex.json must
        // stay byte-identical to apps/mac/Sources/FlashTeXMac/Resources/supported-latex.json
        // (both are refreshed from the same `flashtex-compiler --supported json`
        // output; see the <EmbeddedResource> comment in FlashTeX.Editor.csproj).
        string? repoRoot = FindRepoRoot(AppContext.BaseDirectory);
        Assert.True(repoRoot is not null, "could not locate the repository root from the test output directory");
        string macCopyPath = Path.Combine(repoRoot!, "apps", "mac", "Sources", "FlashTeXMac", "Resources", "supported-latex.json");
        Assert.True(File.Exists(macCopyPath), $"Mac bundled copy not found at {macCopyPath}");

        using Stream embedded = typeof(CompletionVocabulary).Assembly
            .GetManifestResourceStream("FlashTeX.Editor.Resources.supported-latex.json")!;
        byte[] embeddedHash = SHA256.HashData(embedded);
        byte[] macHash = SHA256.HashData(File.ReadAllBytes(macCopyPath));
        Assert.Equal(Convert.ToHexString(macHash), Convert.ToHexString(embeddedHash));
    }

    /// <summary>Walks up from <paramref name="start"/> looking for the
    /// repository root (the directory containing both `apps` and `.git`).</summary>
    private static string? FindRepoRoot(string start)
    {
        var dir = new DirectoryInfo(start);
        for (int i = 0; dir is not null && i < 12; i++, dir = dir.Parent)
        {
            if (Directory.Exists(Path.Combine(dir.FullName, "apps")) && Directory.Exists(Path.Combine(dir.FullName, ".git")))
            {
                return dir.FullName;
            }
        }
        return null;
    }

    [Fact]
    public void VocabularyNamesAreUniqueAndByNameAgrees()
    {
        Assert.Equal(CompletionVocabulary.Names.Count, CompletionVocabulary.Names.Distinct().Count());
        foreach (string name in CompletionVocabulary.Names)
        {
            Assert.Equal(name, CompletionVocabulary.ByName[name].Name);
        }
    }

    [Fact]
    public void SectionEntryHasItsDocumentedArgumentShape()
    {
        CompletionEntry entry = CompletionVocabulary.ByName["section"];
        Assert.Equal("{...}", entry.Arguments);
        Assert.Equal(CompletionMode.Text, entry.Mode);
        Assert.Equal("\\section{...}", entry.Label);
    }

    [Fact]
    public void MathSymbolEntryCarriesItsGlyph()
    {
        CompletionEntry entry = CompletionVocabulary.ByName["alpha"];
        Assert.Equal(CompletionMode.Math, entry.Mode);
        Assert.Equal("α", entry.Glyph);
        Assert.StartsWith("math · ", entry.Detail);
    }

    [Fact]
    public void GenericEntryDescribesAnUncataloguedName()
    {
        CompletionEntry entry = CompletionVocabulary.Generic("mycommand");
        Assert.Equal("mycommand", entry.Name);
        Assert.Equal("supported by this compiler", entry.Description);
    }

    // MARK: matchRank

    [Fact]
    public void EmptyPrefixMatchesEverythingAtPrefixRank()
    {
        Assert.Equal(1, Completion.MatchRank("section", ""));
    }

    [Fact]
    public void ExactMatchRanksZero()
    {
        Assert.Equal(0, Completion.MatchRank("section", "section"));
    }

    [Fact]
    public void PrefixMatchRanksOne()
    {
        Assert.Equal(1, Completion.MatchRank("subsection", "sub"));
    }

    [Fact]
    public void InOrderSubsequenceMatchRanksTwo()
    {
        Assert.Equal(2, Completion.MatchRank("subsection", "sbs"));
    }

    [Fact]
    public void OutOfOrderCharactersDoNotMatch()
    {
        Assert.Null(Completion.MatchRank("subsection", "bsu"));
    }

    [Fact]
    public void SingleCharacterNonPrefixNeverFuzzyMatches()
    {
        // Swift: subsequence matches need at least two typed characters.
        Assert.Null(Completion.MatchRank("subsection", "u"));
    }

    [Fact]
    public void NoOverlapDoesNotMatch()
    {
        Assert.Null(Completion.MatchRank("section", "xyz"));
    }

    // MARK: fuzzyFilter

    [Fact]
    public void FuzzyFilterPrefersStrongMatchesOverFuzzyOnes()
    {
        string[] items = { "subsection", "subsubsection", "abstract" };
        var result = Completion.FuzzyFilter(items, "sub", s => s);
        Assert.Equal(new[] { "subsection", "subsubsection" }, result);
    }

    [Fact]
    public void FuzzyFilterFallsBackToFuzzyWhenNoStrongMatch()
    {
        string[] items = { "subsection", "abstract" };
        var result = Completion.FuzzyFilter(items, "sbs", s => s);
        Assert.Equal(new[] { "subsection" }, result);
    }

    // MARK: command suggestions: ranking against the vocabulary

    [Fact]
    public void PrefixMatchingMultipleCommandsRanksThemInTableOrder()
    {
        var result = Completion.CommandSuggestions("sub");
        var labels = result.Select(s => s.InsertText).ToList();
        Assert.Contains("\\subsection", labels);
        Assert.Contains("\\subsubsection", labels);
        // Table order: subsection is declared before subsubsection in the inventory.
        Assert.True(labels.IndexOf("\\subsection") < labels.IndexOf("\\subsubsection"));
    }

    [Fact]
    public void ExactNameMatchIsListedFirst()
    {
        // "sub" is not itself a full command name, but "section" is a full
        // command name and also a prefix-match target for "sec": it must come
        // before any other command starting with "sec".
        var result = Completion.CommandSuggestions("section");
        Assert.Equal("\\section", result[0].InsertText);
    }

    [Fact]
    public void EmptyPrefixListsTheWholeSuppliedVocabularyInOrder()
    {
        var supported = new[] { "alpha", "beta", "gamma" };
        var result = Completion.CommandSuggestions("", supported);
        Assert.Equal(new[] { "\\alpha", "\\beta", "\\gamma" }, result.Select(s => s.InsertText));
    }

    [Fact]
    public void PrefixMatchingNothingReturnsEmpty()
    {
        var result = Completion.CommandSuggestions("zzzznotacommand");
        Assert.Empty(result);
    }

    [Fact]
    public void FuzzyFallbackOnlyAppliesWhenNoPrefixMatchExists()
    {
        // "sbs" matches nothing as a prefix, but subsequence-matches "subsection".
        var result = Completion.CommandSuggestions("sbs");
        Assert.Contains(result, s => s.InsertText == "\\subsection");
    }

    [Fact]
    public void ResultsAreCappedAtMaxSuggestions()
    {
        // "a" as a prefix matches dozens of commands in the real vocabulary.
        var result = Completion.CommandSuggestions("a");
        Assert.True(result.Count <= Completion.MaxSuggestions);
    }

    [Fact]
    public void UnknownSupportedNameGetsAGenericDetail()
    {
        var result = Completion.CommandSuggestions("mycommand", new[] { "mycommand" });
        var suggestion = Assert.Single(result);
        Assert.Equal("supported by this compiler", suggestion.Detail);
    }
}
