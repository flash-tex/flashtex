// name: Win32SpellChecker.cs
// purpose: Wraps the Win32 OS spellchecking service (spellcheck.h's
//   ISpellCheckerFactory/ISpellChecker COM interfaces, available since
//   Windows 8 -- the same service Mail, Edge and Notepad use) behind a plain
//   C# API, and composes it with LaTeXProse.ProseRanges (LaTeXSpellCheck.cs)
//   so only prose byte ranges of a LaTeX document are ever checked. The COM
//   surface itself is generated at build time by Microsoft.Windows.CsWin32
//   from NativeMethods.txt into internal `Windows.Win32.*` types; this class
//   is the only place in the project that touches them.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Runtime.InteropServices;
using System.Runtime.Versioning;
using Windows.Win32; // brings the CsWin32-generated string-overload extension methods into scope
using Windows.Win32.Foundation;
using Windows.Win32.Globalization;
using Windows.Win32.System.Com;

namespace FlashTeX.Editor;

/// <summary>One spelling error: the UTF-16 range of the misspelled word and
/// its replacement suggestions (empty when the corrective action is not a
/// suggestion/replacement one, e.g. "delete this text").</summary>
public readonly record struct SpellingError(int StartIndex, int Length, IReadOnlyList<string> Suggestions);

/// <summary>Thin wrapper over one native <c>ISpellChecker</c>. One instance
/// owns one COM spell checker for one language and must be disposed to
/// release it.</summary>
[SupportedOSPlatform("windows8.0")]
public sealed class Win32SpellChecker : IDisposable
{
    public const string DefaultLanguageTag = "en-US";

    private readonly ISpellChecker _checker;
    private bool _disposed;

    /// <summary>Creates a spell checker for <paramref name="languageTag"/> (a
    /// <a href="http://tools.ietf.org/html/bcp47">BCP47</a> tag, e.g.
    /// "en-US"). Throws <see cref="COMException"/> when the OS has no
    /// registered spell-check provider for that language.</summary>
    public Win32SpellChecker(string languageTag = DefaultLanguageTag)
    {
        ArgumentException.ThrowIfNullOrEmpty(languageTag);
        var factory = (ISpellCheckerFactory)new SpellCheckerFactory();
        try
        {
            _checker = factory.CreateSpellChecker(languageTag);
        }
        finally
        {
            Marshal.ReleaseComObject(factory);
        }
    }

    /// <summary>
    /// Attempts to acquire the Windows spellcheck service without treating its
    /// absence as an editor failure. The service is optional on Windows (and
    /// can be disabled by enterprise policy or unavailable in sandboxed test
    /// hosts), so callers should use this capability probe when spellcheck is
    /// an enhancement rather than a required operation.
    /// </summary>
    public static bool TryCreate(
        out Win32SpellChecker? checker,
        string languageTag = DefaultLanguageTag)
    {
        try
        {
            checker = new Win32SpellChecker(languageTag);
            return true;
        }
        catch (COMException)
        {
            checker = null;
            return false;
        }
        catch (UnauthorizedAccessException)
        {
            checker = null;
            return false;
        }
    }

    /// <summary>Spellchecks <paramref name="text"/> as one unit, with no
    /// LaTeX-awareness; ranges are UTF-16 offsets into <paramref name="text"/>.
    /// Prefer <see cref="CheckDocument"/> for LaTeX source, so command names,
    /// math and reference arguments are never flagged.</summary>
    public IReadOnlyList<SpellingError> Check(string text)
    {
        ThrowIfDisposed();
        if (text.Length == 0)
        {
            return Array.Empty<SpellingError>();
        }
        List<SpellingError> errors = new();
        IEnumSpellingError enumerator = _checker.Check(text);
        try
        {
            while (true)
            {
                HRESULT hr = enumerator.Next(out ISpellingError error);
                if (hr.Value != 0)
                {
                    break; // S_FALSE (or any non-S_OK): no more errors.
                }
                try
                {
                    errors.Add(ReadError(text, error));
                }
                finally
                {
                    Marshal.ReleaseComObject(error);
                }
            }
        }
        finally
        {
            Marshal.ReleaseComObject(enumerator);
        }
        return errors;
    }

    /// <summary>Spellchecks only the prose ranges of a LaTeX
    /// <paramref name="text"/> (<see cref="LaTeXProse.ProseRanges"/>), so
    /// command names, math mode, comments, verbatim content and
    /// reference/citation/file arguments are never checked. Ranges in the
    /// result are UTF-16 offsets into the whole document.</summary>
    public IReadOnlyList<SpellingError> CheckDocument(string text)
    {
        ThrowIfDisposed();
        List<SpellingError> errors = new();
        foreach (Utf16Range range in LaTeXProse.ProseRanges(text))
        {
            string segment = text.Substring(range.Location, range.Length);
            foreach (SpellingError error in Check(segment))
            {
                errors.Add(error with { StartIndex = error.StartIndex + range.Location });
            }
        }
        return errors;
    }

    /// <summary>Treats <paramref name="word"/> as correctly spelled for the
    /// rest of this session only (not persisted by the OS).</summary>
    public void Ignore(string word)
    {
        ThrowIfDisposed();
        _checker.Ignore(word);
    }

    /// <summary>Learns <paramref name="word"/> as correctly spelled; the OS
    /// persists this across sessions.</summary>
    public void Add(string word)
    {
        ThrowIfDisposed();
        _checker.Add(word);
    }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }
        _disposed = true;
        Marshal.ReleaseComObject(_checker);
    }

    private void ThrowIfDisposed() => ObjectDisposedException.ThrowIf(_disposed, this);

    private SpellingError ReadError(string text, ISpellingError error)
    {
        int start = checked((int)error.StartIndex);
        int length = checked((int)error.Length);
        IReadOnlyList<string> suggestions = error.CorrectiveAction switch
        {
            CORRECTIVE_ACTION.CORRECTIVE_ACTION_GET_SUGGESTIONS => ReadSuggestions(text.Substring(start, length)),
            CORRECTIVE_ACTION.CORRECTIVE_ACTION_REPLACE => new[] { error.Replacement.ToString() ?? string.Empty },
            _ => Array.Empty<string>(),
        };
        return new SpellingError(start, length, suggestions);
    }

    private IReadOnlyList<string> ReadSuggestions(string word)
    {
        IEnumString enumerator = _checker.Suggest(word);
        try
        {
            return ReadAllStrings(enumerator);
        }
        finally
        {
            Marshal.ReleaseComObject(enumerator);
        }
    }

    /// <summary>Drains a COM <c>IEnumString</c> one element at a time (a
    /// single-slot buffer, so <c>pceltFetched</c> may be null per the
    /// <c>IEnumString::Next</c> contract), freeing each string the enumerator
    /// allocated with the COM task allocator.</summary>
    private static unsafe List<string> ReadAllStrings(IEnumString enumerator)
    {
        List<string> results = new();
        Span<PWSTR> single = stackalloc PWSTR[1];
        while (true)
        {
            HRESULT hr = enumerator.Next(single, null);
            if (hr.Value != 0)
            {
                break;
            }
            PWSTR value = single[0];
            string? text = value.ToString();
            if (text is not null)
            {
                results.Add(text);
            }
            Marshal.FreeCoTaskMem((IntPtr)value.Value);
        }
        return results;
    }
}
