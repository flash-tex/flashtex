// name: SignatureHelp.cs
// purpose: Signature help: while the caret sits inside an argument of `\cmd`,
//   report the command's argument pattern (from CompletionVocabulary, e.g.
//   `{num}{den}` or `[options]{class}`) with the current argument index and
//   the one-line description. Ported unit for unit from the pure model half
//   of apps/mac/Sources/FlashTeXMac/SignatureHelp.swift (`SignatureHelp.Info`,
//   `groups(of:)`, `Info.display` and `Info.info(in:caretUTF16:)`); Swift's
//   `NSString`/UTF-16 character scan maps directly onto .NET `string`/`char`
//   indexing, since .NET `char` is itself a UTF-16 code unit -- no byte-offset
//   conversion is needed here (contrast LaTeXSpellCheck.cs, which is also
//   UTF-16-native for the same reason).
//
//   Scope note: not ported: the `SignatureHelpPanel` (NSPanel/AppKit UI) and
//   `EditorIntelligence.CommandDocs` (a separate one-line-doc override table
//   that isn't part of this port's file list) -- the description here is
//   always the vocabulary entry's own `Detail`, whereas Swift prefers
//   `CommandDocs` when it has an override.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Editor;

/// <summary>Signature-help state for the command whose argument the caret sits in.</summary>
public readonly record struct SignatureHelpInfo(
    string Command,
    string Arguments,
    int ActiveArgument,
    string Description,
    int OpenerUtf16)
{
    /// <summary>Argument groups of <see cref="Arguments"/> in order, e.g. `["{num}", "{den}"]`.</summary>
    public IReadOnlyList<string> Groups => SignatureHelp.GroupsOf(Arguments);

    /// <summary>Display text `\cmd{num}{den}` (or `\cmd` when the pattern is
    /// unknown) and the UTF-16 range of the active group inside it (null when
    /// the caret is past the pattern's last group).</summary>
    public (string Text, Utf16Range? Active) Display
    {
        get
        {
            string text = "\\" + Command;
            Utf16Range? active = null;
            IReadOnlyList<string> groups = Groups;
            for (int i = 0; i < groups.Count; i++)
            {
                if (i == ActiveArgument)
                {
                    active = new Utf16Range(text.Length, groups[i].Length);
                }
                text += groups[i];
            }
            return (text, active);
        }
    }
}

public static class SignatureHelp
{
    /// <summary>Scan limit backwards from the caret (UTF-16 units of the caret's line).</summary>
    public const int ScanLimit = 2048;

    private const char Backslash = '\\';
    private const char LBrace = '{';
    private const char RBrace = '}';
    private const char LBracket = '[';
    private const char RBracket = ']';
    private const char Percent = '%';
    private const char Newline = '\n';

    /// <summary>Splits an argument pattern into its `{...}`/`[...]` groups.</summary>
    public static IReadOnlyList<string> GroupsOf(string arguments)
    {
        List<string> groups = new();
        var current = new System.Text.StringBuilder();
        int depth = 0;
        foreach (char c in arguments)
        {
            if ((c == LBrace || c == LBracket) && depth == 0)
            {
                current.Clear();
            }
            if (c is LBrace or LBracket)
            {
                depth += 1;
            }
            current.Append(c);
            if (c is RBrace or RBracket)
            {
                depth -= 1;
                if (depth == 0)
                {
                    groups.Add(current.ToString());
                    current.Clear();
                }
            }
        }
        return groups;
    }

    /// <summary>The command whose argument the caret is in, or null when the
    /// caret is not inside an unmatched `{`/`[` on its line, the enclosing
    /// group is not an argument of a control word, or nothing is known about
    /// that command.</summary>
    public static SignatureHelpInfo? Info(string text, int caretUtf16)
    {
        if (caretUtf16 < 0 || caretUtf16 > text.Length)
        {
            return null;
        }
        int lineStart = caretUtf16;
        while (lineStart > 0 && caretUtf16 - lineStart < ScanLimit && text[lineStart - 1] != Newline)
        {
            lineStart -= 1;
        }

        bool IsEscaped(int i)
        {
            int n = 0;
            int j = i - 1;
            while (j >= lineStart && text[j] == Backslash)
            {
                n += 1;
                j -= 1;
            }
            return n % 2 == 1;
        }

        // 1. The unmatched opener before the caret.
        int depth = 0;
        int? opener = null;
        int k = caretUtf16 - 1;
        while (k >= lineStart)
        {
            char c = text[k];
            if (c == Percent && !IsEscaped(k))
            {
                return null; // the caret is in a comment
            }
            if (c is RBrace or RBracket && !IsEscaped(k))
            {
                depth += 1;
            }
            else if (c is LBrace or LBracket && !IsEscaped(k))
            {
                if (depth == 0)
                {
                    opener = k;
                    break;
                }
                depth -= 1;
            }
            k -= 1;
        }
        if (opener is not int openerIndex)
        {
            return null;
        }
        for (int m = lineStart; m < openerIndex; m++)
        {
            if (text[m] == Percent && !IsEscaped(m))
            {
                return null; // commented out
            }
        }

        // 2. Closed argument groups between the command name and this opener.
        int closed = 0;
        int j = openerIndex - 1;
        while (true)
        {
            while (j >= lineStart && (text[j] == ' ' || text[j] == '\t'))
            {
                j -= 1;
            }
            if (j < lineStart || (text[j] != RBrace && text[j] != RBracket) || IsEscaped(j))
            {
                break;
            }
            int d = 0;
            int m = j;
            bool matched = false;
            while (m >= lineStart)
            {
                char c = text[m];
                if (c is RBrace or RBracket && !IsEscaped(m))
                {
                    d += 1;
                }
                else if (c is LBrace or LBracket && !IsEscaped(m))
                {
                    d -= 1;
                    if (d == 0)
                    {
                        matched = true;
                        break;
                    }
                }
                m -= 1;
            }
            if (!matched)
            {
                return null;
            }
            closed += 1;
            j = m - 1;
        }

        // 3. The control word: letters ending at j, preceded by `\`.
        int nameEnd = j + 1;
        int nameStart = nameEnd;
        while (nameStart > lineStart && IsAsciiLetter(text[nameStart - 1]))
        {
            nameStart -= 1;
        }
        if (nameStart >= nameEnd || nameStart <= lineStart || text[nameStart - 1] != Backslash || IsEscaped(nameStart - 1))
        {
            return null;
        }
        string name = text[nameStart..nameEnd];
        CompletionEntry? entry = CompletionVocabulary.ByName.GetValueOrDefault(name);
        if (entry is null)
        {
            return null;
        }
        return new SignatureHelpInfo(name, entry.Arguments, closed, entry.Detail, openerIndex);
    }

    private static bool IsAsciiLetter(char c) => (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
}
