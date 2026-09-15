// name: Insertion.cs
// purpose: Pure byte-offset anchor/rebase logic for tracking where a
//   capture/insertion should land as the document is edited around it. Ported
//   from apps/mac/Sources/FlashTeXMac/Insertion.swift.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Globalization;
using FlashTeX.Protocol;

namespace FlashTeX.Editor;

/// <summary>
/// A Mac-pinned insertion destination (`destination_id` in the contract). Stores
/// the UTF-8 byte offset in a document at a given editor revision so a later
/// proposal can be rebased or rejected honestly.
/// </summary>
public sealed record InsertionAnchor(string Id, string Path, int ByteOffset, int Revision, string ContextAfter);

/// <summary>Pure insertion logic, kept out of any view model for testing.</summary>
public static class Insertion
{
    /// <summary>Number of grapheme clusters (Swift `Character`s) of trailing context stored with an anchor.</summary>
    public const int ContextLength = 24;

    public static InsertionAnchor? MakeAnchor(string id, string path, string text, int caretUtf16, int revision)
    {
        var byteRange = ByteOffsets.Utf8ByteRangeForUtf16Range(text, caretUtf16, caretUtf16);
        if (byteRange is not (int startByte, int _))
        {
            return null;
        }
        string contextAfter = TakeGraphemeClusters(text, caretUtf16, ContextLength);
        return new InsertionAnchor(id, path, startByte, revision, contextAfter);
    }

    /// <summary>How an anchor resolved against the current buffer.</summary>
    public abstract record Resolution
    {
        private Resolution()
        {
        }

        public sealed record Exact(int ByteOffset) : Resolution;

        public sealed record Rebased(int ByteOffset) : Resolution;

        public sealed record NeedsReselection(string Reason) : Resolution;
    }

    /// <summary>
    /// Resolves an anchor against the current buffer. Exact if the revision is
    /// unchanged; otherwise re-finds the stored trailing context. Ambiguous or
    /// missing context requires reselection rather than guessing.
    /// </summary>
    public static Resolution Resolve(InsertionAnchor anchor, string text, int revision)
    {
        int textUtf8Count = ByteOffsets.Utf8ByteCount(text);
        if (revision == anchor.Revision)
        {
            return anchor.ByteOffset <= textUtf8Count
                ? new Resolution.Exact(anchor.ByteOffset)
                : new Resolution.NeedsReselection("anchor beyond end of buffer");
        }
        if (anchor.ContextAfter.Length == 0)
        {
            // Anchor was at end of buffer; keep it at end if the buffer still ends the same way.
            return new Resolution.Rebased(textUtf8Count);
        }
        var matches = new List<int>();
        int searchFrom = 0;
        while (searchFrom <= text.Length)
        {
            int found = text.IndexOf(anchor.ContextAfter, searchFrom, StringComparison.Ordinal);
            if (found < 0)
            {
                break;
            }
            var range = ByteOffsets.Utf8ByteRangeForUtf16Range(text, found, found);
            if (range is (int startByte, int _))
            {
                matches.Add(startByte);
            }
            // Advance by one grapheme cluster (as the Swift source's
            // `text.index(after: r.lowerBound)` does), not one UTF-16 unit, so a
            // match beginning mid-surrogate-pair/combining-mark cluster is never
            // reported as a second, spurious occurrence.
            searchFrom = found < text.Length ? found + StringInfo.GetNextTextElement(text, found).Length : found + 1;
        }
        return matches.Count switch
        {
            1 => new Resolution.Rebased(matches[0]),
            0 => new Resolution.NeedsReselection("destination text was deleted or changed"),
            _ => matches.Contains(anchor.ByteOffset)
                ? new Resolution.Rebased(anchor.ByteOffset)
                : new Resolution.NeedsReselection("destination is ambiguous after edits"),
        };
    }

    /// <summary>Text to insert: proposal LaTeX, trimmed, with surrounding newlines when the insertion point is not already at a line boundary.</summary>
    public static string InsertionText(string latex, string text, int byteOffset)
    {
        string body = latex.Trim();
        var utf16 = ByteOffsets.Utf16RangeForUtf8Bytes(text, byteOffset, byteOffset);
        int index = utf16?.Utf16Start ?? text.Length;
        bool atLineStart = index == 0 || text[index - 1] == '\n';
        bool atLineEnd = index == text.Length || text[index] == '\n';
        return (atLineStart ? "" : "\n") + body + (atLineEnd ? "" : "\n");
    }

    /// <summary>The first <paramref name="count"/> grapheme clusters of <paramref name="text"/> starting at UTF-16 index <paramref name="startUtf16"/>.</summary>
    private static string TakeGraphemeClusters(string text, int startUtf16, int count)
    {
        if (startUtf16 >= text.Length)
        {
            return "";
        }
        string tail = text[startUtf16..];
        var enumerator = StringInfo.GetTextElementEnumerator(tail);
        var builder = new System.Text.StringBuilder();
        int taken = 0;
        while (taken < count && enumerator.MoveNext())
        {
            builder.Append((string)enumerator.Current);
            taken += 1;
        }
        return builder.ToString();
    }
}
