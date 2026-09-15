// name: CaretSync.cs
// purpose: Source -> preview sync: which preview items the editor caret is
//   inside. Pure geometry/byte-range math (no UI dependency), ported from
//   apps/mac/Sources/FlashTeXMac/CaretSync.swift. Containment is exact in
//   runtime-v1 terms: an item with source `start_byte..<end_byte` contains the
//   caret byte `b` when `start_byte <= b < end_byte`; an empty range contains
//   only `b == start_byte`.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Globalization;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Editor;

/// <summary>
/// Ported from `CaretSync.swift`. The Swift source's `ShellModel`-keyed memoized
/// index cache (a Mac app-model concern, not portable/pure logic) is not part of
/// this port; only the pure containment/index logic is.
/// </summary>
public static class CaretSync
{
    /// <summary>Linear scan over every page. O(items); fine for one-off lookups and tests.</summary>
    public static IReadOnlyList<(int Page, int Index)> ItemsContaining(int byteOffset, string path, CompileResult result)
    {
        var hits = new List<(int Page, int Index)>();
        foreach (var page in result.Pages)
        {
            for (int index = 0; index < page.Items.Count; index++)
            {
                if (page.Items[index] is not PageItem.OfText { Item.Source: { } source } || source.Path != path)
                {
                    continue;
                }
                if (Contains(source.StartByte, source.EndByte, byteOffset))
                {
                    hits.Add((page.Number, index));
                }
            }
        }
        return hits;
    }

    /// <summary>Convenience for the preview: item indices grouped by page number.</summary>
    public static IReadOnlyDictionary<int, IReadOnlySet<int>> IndicesByPage(int byteOffset, string path, CompileResult result)
    {
        var output = new Dictionary<int, HashSet<int>>();
        foreach (var (page, index) in ItemsContaining(byteOffset, path, result))
        {
            if (!output.TryGetValue(page, out var set))
            {
                set = [];
                output[page] = set;
            }
            set.Add(index);
        }
        return output.ToDictionary(kv => kv.Key, kv => (IReadOnlySet<int>)kv.Value);
    }

    public static bool Contains(int start, int end, int byteOffset) => start <= byteOffset && byteOffset < Math.Max(end, start + 1);

    /// <summary>
    /// UTF-8 byte offset of an editor caret given in UTF-16 units. A caret that
    /// splits a UTF-16 surrogate pair is rounded down to the start of the
    /// grapheme cluster it falls in (via <see cref="StringInfo"/>, the .NET
    /// equivalent of the Swift source's `rangeOfComposedCharacterSequence`), or
    /// refused (null) when out of range.
    /// </summary>
    public static int? ByteOffsetOfCaretUtf16(int caretUtf16, string text)
    {
        if (caretUtf16 < 0 || caretUtf16 > text.Length)
        {
            return null;
        }
        int location = caretUtf16;
        if (caretUtf16 < text.Length && char.IsLowSurrogate(text[caretUtf16]))
        {
            location = GraphemeClusterStart(text, caretUtf16);
        }
        var range = ByteOffsets.Utf8ByteRangeForUtf16Range(text, location, location);
        return range?.StartByte;
    }

    /// <summary>The UTF-16 start of the grapheme cluster containing <paramref name="utf16Index"/>.</summary>
    private static int GraphemeClusterStart(string text, int utf16Index)
    {
        var enumerator = StringInfo.GetTextElementEnumerator(text);
        while (enumerator.MoveNext())
        {
            int start = enumerator.ElementIndex;
            int end = start + ((string)enumerator.Current).Length;
            if (utf16Index >= start && utf16Index < end)
            {
                return start;
            }
        }
        return utf16Index;
    }

    /// <summary>
    /// Items of one document, sorted by start byte, with a running maximum of
    /// end bytes so a containment query walks back only over entries whose span
    /// can still reach the byte. Build: O(n log n). Query: O(log n + m) where m
    /// is the number of entries visited.
    /// </summary>
    public sealed class Index
    {
        public readonly record struct Entry(int Start, int End, int Page, int ItemIndex);

        public string Path { get; }
        private readonly IReadOnlyList<Entry> _entries; // sorted by (Start, End, Page, ItemIndex)
        private readonly IReadOnlyList<int> _prefixMaxEnd; // prefixMaxEnd[i] = max(entries[0...i].End)

        /// <summary>Text items whose source names <paramref name="path"/>, in <paramref name="result"/>.</summary>
        public Index(CompileResult result, string path)
        {
            var entries = new List<Entry>();
            foreach (var page in result.Pages)
            {
                for (int itemIndex = 0; itemIndex < page.Items.Count; itemIndex++)
                {
                    if (page.Items[itemIndex] is not PageItem.OfText { Item.Source: { } source } || source.Path != path)
                    {
                        continue;
                    }
                    entries.Add(new Entry(source.StartByte, Math.Max(source.EndByte, source.StartByte + 1), page.Number, itemIndex));
                }
            }
            entries.Sort((a, b) =>
            {
                if (a.Start != b.Start)
                {
                    return a.Start.CompareTo(b.Start);
                }
                if (a.End != b.End)
                {
                    return a.End.CompareTo(b.End);
                }
                return a.Page != b.Page ? a.Page.CompareTo(b.Page) : a.ItemIndex.CompareTo(b.ItemIndex);
            });
            var prefix = new List<int>(entries.Count);
            int running = int.MinValue;
            foreach (var entry in entries)
            {
                running = Math.Max(running, entry.End);
                prefix.Add(running);
            }
            Path = path;
            _entries = entries;
            _prefixMaxEnd = prefix;
        }

        public int Count => _entries.Count;

        /// <summary>Hits in document order (page number, then item index).</summary>
        public IReadOnlyList<(int Page, int Index)> ItemsContaining(int byteOffset)
        {
            var hits = new List<Index.Entry>();
            // Upper bound: first entry whose start is > byteOffset.
            int lo = 0, hi = _entries.Count;
            while (lo < hi)
            {
                int mid = (lo + hi) >> 1;
                if (_entries[mid].Start <= byteOffset)
                {
                    lo = mid + 1;
                }
                else
                {
                    hi = mid;
                }
            }
            int i = lo - 1;
            while (i >= 0 && _prefixMaxEnd[i] > byteOffset)
            {
                if (_entries[i].End > byteOffset)
                {
                    hits.Add(_entries[i]);
                }
                i -= 1;
            }
            hits.Sort((a, b) => a.Page != b.Page ? a.Page.CompareTo(b.Page) : a.ItemIndex.CompareTo(b.ItemIndex));
            return hits.Select(e => (e.Page, e.ItemIndex)).ToList();
        }

        public IReadOnlyDictionary<int, IReadOnlySet<int>> IndicesByPage(int byteOffset)
        {
            var output = new Dictionary<int, HashSet<int>>();
            foreach (var (page, index) in ItemsContaining(byteOffset))
            {
                if (!output.TryGetValue(page, out var set))
                {
                    set = [];
                    output[page] = set;
                }
                set.Add(index);
            }
            return output.ToDictionary(kv => kv.Key, kv => (IReadOnlySet<int>)kv.Value);
        }
    }
}
