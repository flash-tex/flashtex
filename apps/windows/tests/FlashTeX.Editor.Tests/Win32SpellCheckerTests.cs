// name: Win32SpellCheckerTests.cs
// purpose: Integration tests for FlashTeX.Editor.Win32SpellChecker against the
//   real Win32 OS spellchecking service (spellcheck.h's ISpellCheckerFactory /
//   ISpellChecker COM interfaces). These make live COM calls -- not mocks --
//   because the whole point of this class is that the OS-provided dictionary
//   actually answers; a mock would only prove the wrapper compiles, not that
//   the COM activation and marshaling are correct.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Runtime.Versioning;

namespace FlashTeX.Editor.Tests;

[SupportedOSPlatform("windows8.0")]
public sealed class WindowsSpellCheckFactAttribute : FactAttribute
{
    public WindowsSpellCheckFactAttribute()
    {
        if (Win32SpellChecker.TryCreate(out Win32SpellChecker? checker))
        {
            checker!.Dispose();
            return;
        }

        Skip = "The Windows spellcheck service is unavailable to this test host.";
    }
}

[SupportedOSPlatform("windows8.0")]
public class Win32SpellCheckerTests
{
    private static Win32SpellChecker Create() => new();

    [WindowsSpellCheckFact]
    public void MisspelledWordInProseGetsARealSuggestionFromWindows()
    {
        using var checker = Create();
        var errors = checker.Check("I would like to recieve your reply soon.");
        SpellingError error = Assert.Single(errors);
        Assert.Equal("recieve", "I would like to recieve your reply soon.".Substring(error.StartIndex, error.Length));
        Assert.Contains("receive", error.Suggestions, StringComparer.OrdinalIgnoreCase);
    }

    [WindowsSpellCheckFact]
    public void CorrectlySpelledTextHasNoErrors()
    {
        using var checker = Create();
        var errors = checker.Check("The quick brown fox jumps over the lazy dog.");
        Assert.Empty(errors);
    }

    [WindowsSpellCheckFact]
    public void EmptyTextHasNoErrors()
    {
        using var checker = Create();
        Assert.Empty(checker.Check(""));
    }

    [WindowsSpellCheckFact]
    public void CheckDocumentSkipsCommandArgumentsThatAreNotProse()
    {
        using var checker = Create();
        // "eq:mistke" is a \ref argument (skipArguments), never prose, so it is
        // never sent to the spellchecker even though it looks misspelled.
        string tex = "See \\ref{eq:mistke} for the paragrph below.";
        var errors = checker.CheckDocument(tex);
        Assert.Contains(errors, e => tex.Substring(e.StartIndex, e.Length) == "paragrph");
        Assert.DoesNotContain(errors, e => tex.Substring(e.StartIndex, e.Length).Contains("mistke"));
    }

    [WindowsSpellCheckFact]
    public void CheckDocumentRebasesRangesToTheWholeDocument()
    {
        using var checker = Create();
        string tex = "\\textbf{recieve}"; // \textbf's argument IS prose (not in SkipArguments).
        var errors = checker.CheckDocument(tex);
        SpellingError error = Assert.Single(errors);
        Assert.Equal("recieve", tex.Substring(error.StartIndex, error.Length));
    }

    [WindowsSpellCheckFact]
    public void IgnoreAndAddDoNotThrow()
    {
        using var checker = Create();
        checker.Ignore("flashtexignoredword");
        checker.Add("flashtexlearnedword");
    }

    [WindowsSpellCheckFact]
    public void MethodsThrowAfterDispose()
    {
        var checker = Create();
        checker.Dispose();
        Assert.Throws<ObjectDisposedException>(() => checker.Check("text"));
    }

    [WindowsSpellCheckFact]
    public void UnsupportedLanguageTagThrowsRatherThanSilentlyFailing()
    {
        using Win32SpellChecker _ = Create();
        Assert.ThrowsAny<Exception>(() => new Win32SpellChecker("xx-not-a-real-language"));
    }
}
