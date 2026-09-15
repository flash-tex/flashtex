// name: SourceMapping.cs
// purpose: Rebases UTF-8 byte ranges from the text a `compile_result` was produced
//   for onto the current editor text. Ported from
//   apps/mac/Sources/FlashTeXProtocol/SourceMapping.swift. Not wire data: no JSON
//   attributes needed here.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Text;
using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Protocol;

/// <summary>
/// Rebase result: ranges entirely before the changed region keep their offsets,
/// ranges entirely after it shift by the length delta, and anything overlapping
/// the changed region is refused. Multiple edits collapse into one region, which
/// is conservative — it never maps a range onto different text.
/// </summary>
public abstract record SourceMappingOutcome
{
    private SourceMappingOutcome()
    {
    }

    public sealed record Unchanged : SourceMappingOutcome;

    public sealed record Rebased(int Start, int End) : SourceMappingOutcome;

    public sealed record OverlapsEdit : SourceMappingOutcome;
}

/// <summary>
/// The single byte region that differs between <c>Old</c> and <c>New</c>: bytes
/// <c>StartByte..OldEndByte</c> of <c>Old</c> were replaced by <see cref="Replacement"/>
/// (= <c>New</c> bytes <c>StartByte..NewEndByte</c>). Computed from the common
/// prefix/suffix, then widened outward so both ends sit on UTF-8 scalar boundaries.
/// Identical strings yield an empty region at the end of the text.
/// </summary>
public sealed record ChangedRegion(int StartByte, int OldEndByte, int NewEndByte, string Replacement);

public static class SourceMapping
{
    public static SourceMappingOutcome Rebase(int start, int end, string oldText, string newText)
    {
        if (ByteOffsets.SameBytes(oldText, newText))
        {
            return new SourceMappingOutcome.Unchanged();
        }
        return Rebase(start, end, ComputeChangedRegion(oldText, newText));
    }

    /// <summary>
    /// Rebases across an already computed region (callers mapping many ranges
    /// across the same edit compute <see cref="ComputeChangedRegion"/> once).
    /// </summary>
    public static SourceMappingOutcome Rebase(int start, int end, ChangedRegion region)
    {
        int delta = region.NewEndByte - region.OldEndByte;
        if (end <= region.StartByte)
        {
            return new SourceMappingOutcome.Rebased(start, end);
        }
        if (start >= region.OldEndByte)
        {
            return new SourceMappingOutcome.Rebased(start + delta, end + delta);
        }
        return new SourceMappingOutcome.OverlapsEdit();
    }

    public static ChangedRegion ComputeChangedRegion(string oldText, string newText)
    {
        byte[] oldBytes = Encoding.UTF8.GetBytes(oldText);
        byte[] newBytes = Encoding.UTF8.GetBytes(newText);
        return ComputeChangedRegion(oldBytes, newBytes);
    }

    private static bool IsContinuationByte(byte b) => (b & 0xC0) == 0x80;

    private static ChangedRegion ComputeChangedRegion(byte[] o, byte[] n)
    {
        int prefix = 0;
        while (prefix < o.Length && prefix < n.Length && o[prefix] == n[prefix])
        {
            prefix++;
        }
        while (prefix > 0 && ((prefix < o.Length && IsContinuationByte(o[prefix])) || (prefix < n.Length && IsContinuationByte(n[prefix]))))
        {
            prefix--;
        }

        int suffix = 0;
        while (suffix < o.Length - prefix && suffix < n.Length - prefix && o[o.Length - 1 - suffix] == n[n.Length - 1 - suffix])
        {
            suffix++;
        }
        // The suffix bytes are identical in both strings, so one boundary check covers both.
        while (suffix > 0 && IsContinuationByte(o[o.Length - suffix]))
        {
            suffix--;
        }

        int oldEnd = o.Length - suffix;
        int newEnd = n.Length - suffix;
        string replacement = Encoding.UTF8.GetString(n, prefix, newEnd - prefix);
        return new ChangedRegion(prefix, oldEnd, newEnd, replacement);
    }

    /// <summary>
    /// Rebases and additionally checks that the mapped bytes still spell
    /// <paramref name="expectedText"/> when one is known (an item's <c>text</c>).
    /// </summary>
    public static SourceRange? Rebase(SourceRange range, string oldText, string newText, string? expectedText)
    {
        var outcome = Rebase(range.StartByte, range.EndByte, oldText, newText);
        switch (outcome)
        {
            case SourceMappingOutcome.Unchanged:
                return range;
            case SourceMappingOutcome.Rebased rebased:
                var mapped = range with { StartByte = rebased.Start, EndByte = rebased.End };
                if (expectedText is not null)
                {
                    var mappedRange = ByteOffsets.Utf16RangeForUtf8Bytes(newText, mapped.StartByte, mapped.EndByte);
                    if (mappedRange is not (int s, int e) || newText[s..e] != expectedText)
                    {
                        return null;
                    }
                }
                return mapped;
            default:
                return null;
        }
    }
}
