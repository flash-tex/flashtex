// name: Completion.cs
// purpose: Completion matching and ranking. Ported unit for unit from the
//   pure parts of apps/mac/Sources/FlashTeXMac/Completion.swift: `matchRank`
//   (0 exact, 1 prefix, 2 in-order-subsequence fuzzy match), `fuzzyFilter`
//   (strong matches win outright over fuzzy ones), and the vocabulary-ranking
//   half of `commandSuggestions` -- given the compiler's command vocabulary
//   and a typed prefix, rank the matching commands: the command spelled
//   exactly as typed first, then the rest of the vocabulary in table order,
//   falling back to fuzzy (in-order subsequence) matches only when nothing
//   starts with the prefix, so a typed prefix never has fuzzy rows mixed
//   under its own exact/prefix matches.
//
//   Scope note: this file ports only the matching/ranking algorithm and
//   signature lookup (see SignatureHelp.cs), not: Rust-metadata decoding or
//   project-index binding (`Metadata`, `commandSuggestions`'s declared-command
//   and "typed elsewhere in the document" sources, `environmentSuggestions`,
//   `referenceSuggestions`, `citationSuggestions`, `labelSuggestions`,
//   `packageSuggestions`, `fileSuggestions`, `wordSuggestions` -- all of which
//   need a document/project to scan, not just a vocabulary and a prefix); the
//   UTF-8 byte-level token/caret scanner (`token`, `caretToken`, `context`);
//   or snippet-insertion mechanics (`Suggestion.snippet` exists on the
//   vocabulary `CompletionEntry` but nothing here applies it) or any
//   AppKit/NSTableView/NSPanel UI (`CompletionSession`, `CompletionPanel`).
//   Those are separate, later work; a future caller supplies the prefix (from
//   whatever token scanner it uses) and consumes the ranked `Suggestion` list.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Editor;

/// <summary>What a <see cref="CompletionSuggestion"/> completes.</summary>
public enum CompletionKind
{
    Command,
}

/// <summary>One ranked completion candidate.</summary>
public readonly record struct CompletionSuggestion(string Label, string InsertText, CompletionKind Kind, string Detail);

/// <summary>Pure completion matching and ranking over the command vocabulary.</summary>
public static class Completion
{
    /// <summary>Matches the Swift source's `maxSuggestions`: the popup never
    /// shows more than this many rows.</summary>
    public const int MaxSuggestions = 12;

    /// <summary>Minimum typed length (in UTF-16 units) before the fuzzy
    /// subsequence fallback engages -- one or two characters fuzzy-match
    /// almost everything and would be noise, not a real narrowing.</summary>
    private const int MinFuzzyPrefixLength = 2;

    /// <summary>0 exact, 1 prefix, 2 subsequence (the typed characters appear
    /// in order, `sbs` in `subsection`), null no match. An empty prefix
    /// matches everything at rank 1 (a bare trigger lists the vocabulary).
    /// Subsequence matches need at least <see cref="MinFuzzyPrefixLength"/>
    /// typed characters.</summary>
    public static int? MatchRank(string candidate, string prefix)
    {
        if (prefix.Length == 0)
        {
            return 1;
        }
        if (candidate == prefix)
        {
            return 0;
        }
        if (candidate.StartsWith(prefix, StringComparison.Ordinal))
        {
            return 1;
        }
        if (prefix.Length < MinFuzzyPrefixLength)
        {
            return null;
        }
        int candidateIndex = 0;
        foreach (char p in prefix)
        {
            bool found = false;
            while (candidateIndex < candidate.Length)
            {
                char c = candidate[candidateIndex];
                candidateIndex += 1;
                if (c == p)
                {
                    found = true;
                    break;
                }
            }
            if (!found)
            {
                return null;
            }
        }
        return 2;
    }

    /// <summary>Items whose key matches <paramref name="prefix"/>: the exact
    /// and prefix matches (in the given order) when there are any; otherwise
    /// the subsequence matches, so a typed prefix never has fuzzy rows mixed
    /// under it.</summary>
    public static IReadOnlyList<T> FuzzyFilter<T>(IReadOnlyList<T> items, string prefix, Func<T, string> key)
    {
        List<T> strong = new();
        List<T> weak = new();
        foreach (T item in items)
        {
            switch (MatchRank(key(item), prefix))
            {
                case 0 or 1:
                    strong.Add(item);
                    break;
                case 2:
                    weak.Add(item);
                    break;
            }
        }
        return strong.Count > 0 ? strong : weak;
    }

    /// <summary>Ranked command-name completions for <paramref name="prefix"/>
    /// against <paramref name="supported"/> (default: the full compiler
    /// vocabulary, <see cref="CompletionVocabulary.Names"/>, in table order).
    /// Rank order: the command spelled exactly as typed first (e.g. `\sec`
    /// before `\section` when `sec` is itself a command), then every other
    /// prefix match in table order; when nothing starts with the prefix, the
    /// fuzzy (in-order subsequence) matches instead, capped at
    /// <see cref="MaxSuggestions"/>.</summary>
    public static IReadOnlyList<CompletionSuggestion> CommandSuggestions(string prefix, IReadOnlyList<string>? supported = null)
    {
        supported ??= CompletionVocabulary.Names;
        List<CompletionSuggestion> result = new();
        HashSet<string> offered = new(StringComparer.Ordinal);

        IEnumerable<string> exactFirst = supported.Contains(prefix, StringComparer.Ordinal)
            ? Enumerable.Repeat(prefix, 1).Concat(supported)
            : supported;
        foreach (string name in exactFirst)
        {
            if (!name.StartsWith(prefix, StringComparison.Ordinal) || !offered.Add(name))
            {
                continue;
            }
            result.Add(EntrySuggestion(name));
        }

        if (result.Count == 0 && prefix.Length >= MinFuzzyPrefixLength)
        {
            foreach (string name in supported)
            {
                if (result.Count >= MaxSuggestions)
                {
                    break;
                }
                if (offered.Contains(name) || MatchRank(name, prefix) != 2)
                {
                    continue;
                }
                offered.Add(name);
                result.Add(EntrySuggestion(name));
            }
        }

        return result.Count > MaxSuggestions ? result.GetRange(0, MaxSuggestions) : result;
    }

    private static CompletionSuggestion EntrySuggestion(string name)
    {
        CompletionEntry entry = CompletionVocabulary.ByName.GetValueOrDefault(name) ?? CompletionVocabulary.Generic(name);
        return new CompletionSuggestion(entry.Label, "\\" + name, CompletionKind.Command, entry.Detail);
    }
}
