// name: LaTeXSpellCheck.cs
// purpose: LaTeX-aware prose/non-prose tokenizer: which UTF-16 units of a LaTeX
//   source are prose that should be spellchecked, as opposed to command names,
//   math mode, comments, `\verb`/verbatim-like content, and the arguments of
//   reference/label/package/file/definition commands. Ported from the pure
//   `LaTeXProse` enum of apps/mac/Sources/FlashTeXMac/LaTeXSpellCheck.swift.
//
//   Scope note: only the tokenizer (`LaTeXProse`) is ported. The Swift file's
//   `LaTeXSpellChecker` (NSTextView/NSSpellChecker/NSLayoutManager integration:
//   debounced background checks, temporary spelling-underline attributes, the
//   right-click suggestions menu) is AppKit-specific UI/OS integration and is
//   not part of this port; a future Win32 `ISpellChecker` binding is separate,
//   later work that will consume the prose ranges this file produces.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Editor;

/// <summary>
/// Decides which UTF-16 ranges of a LaTeX source are prose. Ported unit for
/// unit from the Swift source's byte-class scan over `[UInt16]` (this port
/// uses .NET `char` arrays, which are UTF-16 code units, so the same
/// byte-for-byte logic applies without adaptation).
/// </summary>
public static class LaTeXProse
{
    /// <summary>Commands whose adjacent `[...]`/`{...}` arguments are all non-prose.</summary>
    public static readonly IReadOnlySet<string> SkipArguments = new HashSet<string>(StringComparer.Ordinal)
    {
        "label", "ref", "eqref", "pageref", "autoref", "cref", "Cref", "vref", "nameref", "labelcref",
        "cite", "citep", "citet", "citealp", "citealt", "citeauthor", "citeyear", "parencite", "textcite",
        "autocite", "footcite", "nocite", "bibitem",
        "usepackage", "RequirePackage", "documentclass", "LoadClass", "usetikzlibrary", "usepgfplotslibrary",
        "input", "include", "includeonly", "includegraphics", "includepdf", "bibliography", "bibliographystyle",
        "addbibresource", "graphicspath", "url", "path",
        "newcommand", "renewcommand", "providecommand", "newenvironment", "renewenvironment",
        "DeclareMathOperator", "DeclareRobustCommand", "newtheorem", "theoremstyle", "newlength", "newcounter",
        "setlength", "addtolength", "setcounter", "addtocounter", "numberwithin", "setlist", "newlist",
        "vspace", "hspace", "pagestyle", "thispagestyle", "pagenumbering", "hypersetup", "geometry",
        "lstset", "tikzset", "pgfplotsset", "captionsetup", "definecolor", "color", "fontsize", "linespread",
        "setstretch", "crefname", "Crefname", "ensuremath", "si", "SI", "num", "qty", "ang", "unit",
        "rule", "raisebox", "resizebox", "scalebox", "makebox", "framebox", "parbox",
        "begin", "end", // handled specially; listed so `\end{x}` is always skipped
    };

    /// <summary>Commands whose optional arguments and first `{...}` argument are non-prose (the second argument is text).</summary>
    public static readonly IReadOnlySet<string> SkipFirstArgument = new HashSet<string>(StringComparer.Ordinal)
    {
        "href", "textcolor", "colorbox", "hyperref",
    };

    /// <summary>Environments whose body is math.</summary>
    public static readonly IReadOnlySet<string> MathEnvironments = new HashSet<string>(StringComparer.Ordinal)
    {
        "equation", "equation*", "align", "align*", "alignat", "alignat*", "gather", "gather*",
        "multline", "multline*", "flalign", "flalign*", "eqnarray", "eqnarray*", "displaymath", "math",
        "dmath", "dmath*", "subequations",
    };

    /// <summary>Environments whose body is code or verbatim text.</summary>
    public static readonly IReadOnlySet<string> VerbatimEnvironments = new HashSet<string>(StringComparer.Ordinal)
    {
        "verbatim", "verbatim*", "Verbatim", "lstlisting", "minted", "comment", "filecontents",
        "filecontents*", "tikzpicture", "pgfpicture", "tikzcd", "forest",
    };

    /// <summary>Commands whose name is a letter but which put an accent on their argument (`\c{c}`, `\v{s}`).</summary>
    public static readonly IReadOnlySet<string> LetterAccents = new HashSet<string>(StringComparer.Ordinal) { "c", "v", "u", "H", "k", "r", "b", "d", "t" };

    private const char Backslash = '\\';
    private const char Percent = '%';
    private const char Dollar = '$';
    private const char LBrace = '{';
    private const char RBrace = '}';
    private const char LBracket = '[';
    private const char RBracket = ']';
    private const char Newline = '\n';
    private const char Star = '*';
    private const char EqualsSign = '=';

    public static bool IsLetter(char c) => (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');

    /// <summary>Letters of a prose word (non-ASCII units count as letters).</summary>
    public static bool IsWordUnit(char c) => IsLetter(c) || c >= 0x80;

    public static bool IsHSpace(char c) => c == ' ' || c == '\t';

    /// <summary>Control symbols that accent the next letter (`\'e`, `\"o`).</summary>
    public static bool IsSymbolAccent(char c) => c is '\'' or '"' or '`' or '^' or '~' or '=' or '.';

    // MARK: public (pure)

    /// <summary>Per-unit prose flags for UTF-16 <paramref name="units"/>.</summary>
    public static bool[] Mask(ReadOnlySpan<char> units)
    {
        var prose = new bool[units.Length];
        int i = 0;
        while (i < units.Length)
        {
            switch (units[i])
            {
                case Percent:
                    i = LineEnd(units, i);
                    break;
                case Dollar:
                    if (i + 1 < units.Length && units[i + 1] == Dollar)
                    {
                        i = SkipPast(units, i + 2, [Dollar, Dollar], blankLineEnds: false);
                    }
                    else
                    {
                        i = SkipPast(units, i + 1, [Dollar], blankLineEnds: true);
                    }
                    break;
                case Backslash:
                    i = ControlSequence(units, i, prose);
                    break;
                case LBrace or RBrace or LBracket or RBracket or '~' or '&' or '#' or '^' or '_':
                    i += 1;
                    break;
                default:
                    prose[i] = true;
                    i += 1;
                    break;
            }
        }
        return prose;
    }

    /// <summary>Maximal prose ranges of <paramref name="text"/> (UTF-16).</summary>
    public static IReadOnlyList<Utf16Range> ProseRanges(string text)
    {
        var flags = Mask(text);
        var result = new List<Utf16Range>();
        int? start = null;
        for (int i = 0; i < flags.Length; i++)
        {
            if (flags[i])
            {
                start ??= i;
            }
            else if (start is int s)
            {
                result.Add(new Utf16Range(s, i - s));
                start = null;
            }
        }
        if (start is int last)
        {
            result.Add(new Utf16Range(last, flags.Length - last));
        }
        return result;
    }

    /// <summary><paramref name="text"/> with every non-prose unit replaced by a space (newlines kept), so offsets are unchanged and no word spans a prose boundary.</summary>
    public static string MaskedText(string text)
    {
        char[] units = text.ToCharArray();
        var flags = Mask(units);
        for (int i = 0; i < units.Length; i++)
        {
            if (!flags[i] && units[i] != Newline)
            {
                units[i] = ' ';
            }
        }
        return new string(units);
    }

    // MARK: scanning

    private static int LineEnd(ReadOnlySpan<char> u, int i)
    {
        int k = i;
        while (k < u.Length && u[k] != Newline)
        {
            k += 1;
        }
        return k;
    }

    /// <summary>Whether a blank line (only spaces/tabs/CR) starts right after the newline at <paramref name="k"/>.</summary>
    private static bool BlankLineAfter(ReadOnlySpan<char> u, int k)
    {
        int j = k + 1;
        while (j < u.Length && (IsHSpace(u[j]) || u[j] == '\r'))
        {
            j += 1;
        }
        return j < u.Length && u[j] == Newline;
    }

    /// <summary>Index just past the first unescaped <paramref name="closer"/> at or after <paramref name="from"/> (the end of the text when missing; the blank line when <paramref name="blankLineEnds"/>).</summary>
    private static int SkipPast(ReadOnlySpan<char> u, int from, ReadOnlySpan<char> closer, bool blankLineEnds)
    {
        int k = from;
        while (k < u.Length)
        {
            if (u[k] == closer[0] && k + closer.Length <= u.Length && u.Slice(k, closer.Length).SequenceEqual(closer))
            {
                return k + closer.Length;
            }
            if (u[k] == Backslash)
            {
                k += 2;
                continue;
            }
            if (blankLineEnds && u[k] == Newline && BlankLineAfter(u, k))
            {
                return k;
            }
            k += 1;
        }
        return u.Length;
    }

    /// <summary>Index past the `{...}` or `[...]` group opening at <paramref name="k"/> (nesting and escapes respected; a blank line ends an unterminated group).</summary>
    private static int SkipGroup(ReadOnlySpan<char> u, int k)
    {
        char open = u[k];
        char close = open == LBrace ? RBrace : RBracket;
        int depth = 0;
        int j = k;
        while (j < u.Length)
        {
            char c = u[j];
            if (c == Backslash)
            {
                j += 2;
                continue;
            }
            if (c == open)
            {
                depth += 1;
            }
            else if (c == close)
            {
                depth -= 1;
                if (depth == 0)
                {
                    return j + 1;
                }
            }
            else if (c == Newline && BlankLineAfter(u, j))
            {
                return j;
            }
            j += 1;
        }
        return u.Length;
    }

    /// <summary>Skips every `[...]`/`{...}` group directly adjacent to <paramref name="from"/>.</summary>
    private static int SkipAdjacentGroups(ReadOnlySpan<char> u, int from)
    {
        int j = from;
        while (j < u.Length && (u[j] == LBrace || u[j] == LBracket))
        {
            j = SkipGroup(u, j);
        }
        return j;
    }

    private static void UnmarkWordBefore(int i, ReadOnlySpan<char> u, bool[] prose)
    {
        int k = i - 1;
        while (k >= 0 && IsWordUnit(u[k]))
        {
            prose[k] = false;
            k -= 1;
        }
    }

    /// <summary>Skips an accent's argument and the rest of the word it belongs to.</summary>
    private static int SkipAccentedWord(ReadOnlySpan<char> u, int from)
    {
        int j = from;
        if (j < u.Length && u[j] == LBrace)
        {
            j = SkipGroup(u, j);
        }
        else if (j < u.Length && IsWordUnit(u[j]))
        {
            j += 1;
        }
        while (j < u.Length)
        {
            if (IsWordUnit(u[j]))
            {
                j += 1;
                continue;
            }
            if (u[j] == Backslash && j + 1 < u.Length && IsSymbolAccent(u[j + 1]))
            {
                j = SkipAccentedWord(u, j + 2);
                continue;
            }
            break;
        }
        return j;
    }

    private static int ControlSequence(ReadOnlySpan<char> u, int i, bool[] prose)
    {
        if (i + 1 >= u.Length)
        {
            return u.Length;
        }
        char d = u[i + 1];
        if (!IsLetter(d))
        {
            switch (d)
            {
                case '(':
                    return SkipPast(u, i + 2, [Backslash, ')'], blankLineEnds: false);
                case LBracket:
                    return SkipPast(u, i + 2, [Backslash, RBracket], blankLineEnds: false);
                case Backslash:
                {
                    // Line break, optional `*` and `[dimension]`.
                    int j = i + 2;
                    if (j < u.Length && u[j] == Star)
                    {
                        j += 1;
                    }
                    if (j < u.Length && u[j] == LBracket)
                    {
                        j = SkipGroup(u, j);
                    }
                    return j;
                }
                default:
                    if (!IsSymbolAccent(d))
                    {
                        return i + 2; // \% \$ \& \, \  ...
                    }
                    UnmarkWordBefore(i, u, prose);
                    return SkipAccentedWord(u, i + 2);
            }
        }
        int nameEnd = i + 1;
        while (nameEnd < u.Length && IsLetter(u[nameEnd]))
        {
            nameEnd += 1;
        }
        string name = new(u[(i + 1)..nameEnd]);
        int after = nameEnd;
        if (after < u.Length && u[after] == Star)
        {
            after += 1;
        }
        switch (name)
        {
            case "verb":
            {
                if (after >= u.Length || IsLetter(u[after]) || IsHSpace(u[after]) || u[after] == Newline)
                {
                    return after;
                }
                char delimiter = u[after];
                int k = after + 1;
                while (k < u.Length && u[k] != delimiter && u[k] != Newline)
                {
                    k += 1;
                }
                return Math.Min(k + 1, u.Length);
            }
            case "begin":
            {
                if (after >= u.Length || u[after] != LBrace)
                {
                    return after;
                }
                int close = SkipGroup(u, after);
                int nameEndInEnv = close > after + 1 && close <= u.Length && u[close - 1] == RBrace ? close - 1 : close;
                string env = new(u[(after + 1)..nameEndInEnv]);
                if (MathEnvironments.Contains(env) || VerbatimEnvironments.Contains(env))
                {
                    return SkipPast(u, close, ("\\end{" + env + "}").ToCharArray(), blankLineEnds: false);
                }
                return SkipAdjacentGroups(u, close);
            }
            case "def" or "gdef" or "edef" or "xdef" or "let":
            {
                int k = after;
                while (k < u.Length && IsHSpace(u[k]))
                {
                    k += 1;
                }
                if (k < u.Length && u[k] == Backslash)
                {
                    k += 1;
                    while (k < u.Length && IsLetter(u[k]))
                    {
                        k += 1;
                    }
                }
                if (name == "let")
                {
                    return k;
                }
                while (k < u.Length && u[k] != LBrace && u[k] != Newline) // parameter text (#1#2)
                {
                    k += 1;
                }
                return k < u.Length && u[k] == LBrace ? SkipGroup(u, k) : k;
            }
        }
        if (LetterAccents.Contains(name) && after < u.Length && u[after] == LBrace)
        {
            UnmarkWordBefore(i, u, prose);
            return SkipAccentedWord(u, after);
        }
        if (SkipArguments.Contains(name))
        {
            int k = after;
            while (k < u.Length && IsHSpace(u[k]))
            {
                k += 1;
            }
            return k < u.Length && (u[k] == LBrace || u[k] == LBracket) ? SkipAdjacentGroups(u, k) : after;
        }
        if (SkipFirstArgument.Contains(name))
        {
            int k = after;
            while (k < u.Length && u[k] == LBracket)
            {
                k = SkipGroup(u, k);
            }
            if (name != "hyperref" && k < u.Length && u[k] == LBrace)
            {
                k = SkipGroup(u, k);
            }
            return k;
        }
        // Any other command: a `[key=value]` option list is not prose.
        if (after < u.Length && u[after] == LBracket)
        {
            int end = SkipGroup(u, after);
            if (u[after..Math.Min(end, u.Length)].Contains(EqualsSign))
            {
                return end;
            }
        }
        return after;
    }
}
