// name: DocumentOutline.cs
// purpose: Sidebar Outline scanner: sectioning commands, `\begin{...}`
//   environments and `\label{...}`s in document order, ported from
//   apps/mac/Sources/FlashTeXMac/DocumentOutline.swift. A cheap lexical regex
//   scan, not a TeX parser (mirrors the accessibility rotor's own approach);
//   comments are skipped (an unescaped `%` ends the scanned part of a line).
//   Positions are UTF-16 char indices/lengths: .NET string indices are already
//   UTF-16 code units, matching Swift's NSString/NSRange convention directly,
//   so no FlashTeX.Protocol.ByteOffsets conversion is needed in this file.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text.RegularExpressions;

namespace FlashTeX.ProjectFiles;

/// <summary>A UTF-16 char range: the .NET analog of Foundation's <c>NSRange</c>, used wherever this port mirrors Mac code addressed in UTF-16.</summary>
public readonly record struct Utf16Range(int Location, int Length)
{
    public int End => Location + Length;
}

/// <summary>Sections, environments and labels of one buffer, for the sidebar's Outline.</summary>
public static class DocumentOutline
{
    public enum Kind
    {
        Section,
        Environment,
        Label,
    }

    public static string Title(this Kind kind) => kind switch
    {
        Kind.Section => "Sections",
        Kind.Environment => "Environments",
        Kind.Label => "Labels",
        _ => throw new ArgumentOutOfRangeException(nameof(kind)),
    };

    /// <param name="Kind">Section, environment or label.</param>
    /// <param name="Command">Section command name (<c>section</c>, <c>subsection</c>…), environment name, or <c>"label"</c>.</param>
    /// <param name="Title">The braced argument: heading text, environment name, or the label key.</param>
    /// <param name="Level">Indentation level: chapter 0, section 1, subsection 2, …; environments nest by their <c>\begin</c> depth; labels 0.</param>
    /// <param name="Utf16">UTF-16 range of the whole command (selecting it reveals the item in the editor).</param>
    /// <param name="Line">1-based line of the command's start.</param>
    /// <param name="Caption">For a float or theorem-like environment: its <c>\caption{…}</c>, the <c>\begin{thm}[title]</c> optional title, or the first words of its body.</param>
    public sealed record Item(
        Kind Kind,
        string Command,
        string Title,
        int Level,
        Utf16Range Utf16,
        int Line,
        string? Caption = null)
    {
        public string Id => $"{Kind}:{Utf16.Location}";

        /// <summary>Sidebar text: the caption when there is one ("figure: Loss curves").</summary>
        public string DisplayTitle => Caption is { } caption ? $"{Title}: {caption}" : Title;
    }

    /// <summary>Environments whose caption/title is shown in the outline: floats and the standard theorem-like names (plus every <c>\newtheorem{name}</c> of the buffer).</summary>
    public static readonly IReadOnlySet<string> FloatEnvironments = new HashSet<string>(StringComparer.Ordinal)
    {
        "figure", "figure*", "table", "table*", "algorithm", "listing", "wrapfigure", "wraptable", "subfigure",
    };

    public static readonly IReadOnlySet<string> TheoremEnvironments = new HashSet<string>(StringComparer.Ordinal)
    {
        "theorem", "lemma", "proposition", "corollary", "definition", "remark", "example", "proof", "claim", "conjecture",
        "exercise", "problem", "solution", "axiom", "notation", "assumption", "fact", "observation",
    };

    /// <summary>Sectioning commands and their level (chapter 0 … paragraph 4).</summary>
    public static readonly IReadOnlyDictionary<string, int> SectionLevels = new Dictionary<string, int>(StringComparer.Ordinal)
    {
        ["part"] = 0,
        ["chapter"] = 0,
        ["section"] = 1,
        ["subsection"] = 2,
        ["subsubsection"] = 3,
        ["paragraph"] = 4,
        ["subparagraph"] = 5,
    };

    /// <summary>Documents longer than this are not scanned for the outline (the scan is linear; the sidebar re-scans after edits settle).</summary>
    public const int MaxScannedUtf16 = 2 * 1024 * 1024;

    // Groups: 1 sectioning name, 2 its argument; 3 begin env; 4 label key; 5 end env.
    private static readonly Regex Pattern = new(
        "\\\\(" + string.Join('|', SectionLevels.Keys.OrderBy(k => k, StringComparer.Ordinal)) + ")\\*?(?:\\[[^\\]\\n]*\\])?\\{([^{}\\n]*)\\}"
        + "|\\\\begin\\{([A-Za-z*]+)\\}"
        + "|\\\\label\\{([^{}\\n]*)\\}"
        + "|\\\\end\\{([A-Za-z*]+)\\}",
        RegexOptions.Compiled);

    private static readonly Regex NewtheoremPattern = new(@"\\newtheorem\*?\{([A-Za-z*]+)\}", RegexOptions.Compiled);

    /// <summary>
    /// The current item for follow-caret highlighting: the last sectioning
    /// command at or before <paramref name="caret"/>, else the innermost
    /// environment whose <c>\begin</c> is at or before it.
    /// </summary>
    public static Item? Current(int caret, IReadOnlyList<Item> items)
    {
        Item? section = null;
        foreach (Item item in items)
        {
            if (item.Kind == Kind.Section && item.Utf16.Location <= caret)
            {
                section = item;
            }
        }
        if (section is not null)
        {
            return section;
        }
        for (int i = items.Count - 1; i >= 0; i--)
        {
            if (items[i].Kind == Kind.Environment && items[i].Utf16.Location <= caret)
            {
                return items[i];
            }
        }
        return null;
    }

    /// <summary>All items of <paramref name="text"/> in document order.</summary>
    public static IReadOnlyList<Item> Scan(string text)
    {
        if (text.Length > MaxScannedUtf16)
        {
            return [];
        }

        List<int> lineStarts = ComputeLineStarts(text);
        var items = new List<Item>();
        int environmentDepth = 0;
        for (int lineIndex = 0; lineIndex < lineStarts.Count; lineIndex++)
        {
            int lineStart = lineStarts[lineIndex];
            int lineEnd = lineIndex + 1 < lineStarts.Count ? lineStarts[lineIndex + 1] : text.Length;
            int searchableLength = CommentFreeLength(text, lineStart, lineEnd);
            if (searchableLength == 0)
            {
                continue;
            }

            string searchable = text.Substring(lineStart, searchableLength);
            foreach (Match match in Pattern.Matches(searchable))
            {
                ScanMatch(match, lineStart, lineStarts, items, ref environmentDepth);
            }
        }

        AnnotateCaptions(items, text);
        return items;
    }

    private static List<int> ComputeLineStarts(string text)
    {
        var lineStarts = new List<int> { 0 };
        for (int i = 0; i < text.Length; i++)
        {
            if (text[i] == '\n' && i + 1 < text.Length)
            {
                lineStarts.Add(i + 1);
            }
        }
        return lineStarts;
    }

    /// <summary>Length of the line's content before an unescaped <c>%</c> (the whole line when there is none).</summary>
    private static int CommentFreeLength(string text, int lineStart, int lineEnd)
    {
        bool prevBackslash = false;
        for (int k = lineStart; k < lineEnd; k++)
        {
            char c = text[k];
            if (c == '%' && !prevBackslash)
            {
                return k - lineStart;
            }
            prevBackslash = c == '\\' && !prevBackslash;
        }
        return lineEnd - lineStart;
    }

    private static void ScanMatch(Match match, int lineStart, List<int> lineStarts, List<Item> items, ref int environmentDepth)
    {
        int location = lineStart + match.Index;
        int line = LineNumber(lineStarts, location);
        var range = new Utf16Range(location, match.Length);

        if (match.Groups[1].Success && match.Groups[2].Success)
        {
            string name = match.Groups[1].Value;
            string argument = match.Groups[2].Value.Trim();
            items.Add(new Item(Kind.Section, name, argument, SectionLevels.GetValueOrDefault(name, 1), range, line));
        }
        else if (match.Groups[3].Success)
        {
            string env = match.Groups[3].Value;
            // The document environment is the page itself, not an outline entry.
            if (env != "document")
            {
                items.Add(new Item(Kind.Environment, "begin", env, environmentDepth, range, line));
                environmentDepth++;
            }
        }
        else if (match.Groups[4].Success)
        {
            items.Add(new Item(Kind.Label, "label", match.Groups[4].Value, 0, range, line));
        }
        else if (match.Groups[5].Success && match.Groups[5].Value != "document")
        {
            environmentDepth = Math.Max(0, environmentDepth - 1);
        }
    }

    private static int LineNumber(List<int> lineStarts, int location)
    {
        int line = 0;
        foreach (int start in lineStarts)
        {
            if (start > location)
            {
                break;
            }
            line++;
        }
        return line;
    }

    /// <summary>
    /// Fills <see cref="Item.Caption"/> for floats and theorem-like environments:
    /// the first <c>\caption{…}</c> before the matching <c>\end</c>, else the
    /// <c>[title]</c> right after <c>\begin{…}</c>, else the body's first words
    /// (≤ 60 characters).
    /// </summary>
    private static void AnnotateCaptions(List<Item> items, string text)
    {
        var theoremNames = new HashSet<string>(TheoremEnvironments, StringComparer.Ordinal);
        foreach (Match m in NewtheoremPattern.Matches(text))
        {
            theoremNames.Add(m.Groups[1].Value);
        }

        for (int i = 0; i < items.Count; i++)
        {
            if (items[i].Kind != Kind.Environment)
            {
                continue;
            }
            string name = items[i].Title;
            bool isFloat = FloatEnvironments.Contains(name);
            bool isTheorem = theoremNames.Contains(name);
            if (!isFloat && !isTheorem)
            {
                continue;
            }

            int start = items[i].Utf16.End;
            int endMarkerIndex = text.IndexOf($"\\end{{{name}}}", start, StringComparison.Ordinal);
            int bodyEnd = endMarkerIndex < 0 ? text.Length : endMarkerIndex;
            if (bodyEnd < start)
            {
                continue;
            }
            string body = text[start..bodyEnd];

            string? caption = isFloat ? CaptionFromFloatBody(text, start, body) : CaptionFromTheoremBody(start, body, text);
            if (caption is not null)
            {
                items[i] = items[i] with { Caption = caption };
            }
        }
    }

    private static string? CaptionFromFloatBody(string text, int bodyStart, string body)
    {
        const string captionCommand = "\\caption{";
        int captionIndex = body.IndexOf(captionCommand, StringComparison.Ordinal);
        if (captionIndex < 0)
        {
            return null;
        }
        int openBrace = bodyStart + captionIndex + captionCommand.Length - 1;
        return BracedArgument(text, openBrace) is { } argument ? Summarize(argument) : null;
    }

    private static string? CaptionFromTheoremBody(int bodyStart, string body, string text)
    {
        if (bodyStart < text.Length && text[bodyStart] == '[')
        {
            int closeIndex = body.IndexOf(']');
            return closeIndex >= 0 ? Summarize(body[1..closeIndex]) : null;
        }
        string words = body.Trim();
        return words.Length == 0 ? null : Summarize(words);
    }

    /// <summary>The balanced <c>{…}</c> argument starting at <paramref name="openBrace"/>, or null when unclosed.</summary>
    private static string? BracedArgument(string text, int openBrace)
    {
        if (openBrace >= text.Length || text[openBrace] != '{')
        {
            return null;
        }
        int depth = 0;
        int i = openBrace;
        while (i < text.Length)
        {
            char c = text[i];
            if (c == '\\')
            {
                i += 2;
                continue;
            }
            if (c == '{')
            {
                depth++;
            }
            else if (c == '}')
            {
                depth--;
                if (depth == 0)
                {
                    return text[(openBrace + 1)..i];
                }
            }
            i++;
        }
        return null;
    }

    /// <summary>One line, whitespace collapsed, ≤ 60 characters with an ellipsis.</summary>
    private static string Summarize(string s)
    {
        string flat = string.Join(' ', s.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries));
        return flat.Length > 60 ? string.Concat(flat.AsSpan(0, 59), "…") : flat;
    }

    /// <summary>Items of one kind, in document order.</summary>
    public static IReadOnlyList<Item> ItemsOfKind(Kind kind, IReadOnlyList<Item> items) =>
        items.Where(i => i.Kind == kind).ToList();

    /// <summary>Counts per kind for the sidebar headers.</summary>
    public static IReadOnlyDictionary<Kind, int> Counts(IReadOnlyList<Item> items)
    {
        var counts = new Dictionary<Kind, int>();
        foreach (Item item in items)
        {
            counts[item.Kind] = counts.GetValueOrDefault(item.Kind) + 1;
        }
        return counts;
    }
}
