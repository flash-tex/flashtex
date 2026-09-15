// name: Navigation.cs
// purpose: Pure, UTF-8-byte-exact LaTeX navigation logic ported from the
//   non-AppKit-specific half of apps/mac/Sources/FlashTeXMac/Navigation.swift:
//   \begin/\end and \label/\ref matching, and byte-range rebasing across
//   edits. The AppKit/NSRange glue at the bottom of the Swift file (the
//   ShellModel extension methods, the project-index helper probe, and the
//   NavigationCommands menu) is UI/IPC-layer and out of scope here; it will
//   be rebuilt against EditLedgerClient/PreviewControllerClient once those
//   land. Reuses FlashTeX.Protocol.ByteOffsets for UTF-8/UTF-16 conversion and
//   FlashTeX.Protocol.SourceMapping for the byte-range rebase algorithm,
//   rather than reimplementing either.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Shell;

/// <summary>Zero-based, end-exclusive UTF-8 byte range.</summary>
public readonly record struct ByteRange(int Start, int End);

/// <summary>Zero-based, end-exclusive UTF-16 char range (a .NET string index range).</summary>
public readonly record struct Utf16Range(int Start, int End)
{
    public int Length => End - Start;
}

/// <summary>
/// One <c>\name{arg}</c> occurrence: <see cref="Range"/> spans the backslash
/// through the closing brace, <see cref="ArgRange"/> the argument text.
/// </summary>
public readonly record struct CommandUse(string Name, string Arg, ByteRange Range, ByteRange ArgRange);

public static class Navigation
{
    private const byte Backslash = (byte)'\\';
    private const byte Percent = (byte)'%';
    private const byte Newline = (byte)'\n';
    private const byte OpenBrace = (byte)'{';
    private const byte CloseBrace = (byte)'}';

    /// <summary>Commands whose argument names a label they reference (as opposed to defining one).</summary>
    public static readonly IReadOnlySet<string> ReferenceCommands =
        new HashSet<string>(StringComparer.Ordinal) { "ref", "eqref", "pageref", "autoref" };

    private static readonly IReadOnlySet<string> InterestingCommands =
        new HashSet<string>(ReferenceCommands, StringComparer.Ordinal) { "begin", "end", "label" };

    private static bool IsAsciiLetter(byte b) => (b is >= (byte)'A' and <= (byte)'Z') || (b is >= (byte)'a' and <= (byte)'z');

    /// <summary>
    /// Every <c>\begin{…}</c>, <c>\end{…}</c>, <c>\label{…}</c>, and reference
    /// command with a complete braced argument, in document order. Commands
    /// inside a <c>%</c> comment (to the end of the line) are skipped;
    /// <c>\%</c> and <c>\\</c> are escapes. Ported byte-for-byte from
    /// <c>Navigation.commandUses(in:)</c>.
    /// </summary>
    public static IReadOnlyList<CommandUse> CommandUses(string text)
    {
        var bytes = Encoding.UTF8.GetBytes(text);
        var n = bytes.Length;
        var result = new List<CommandUse>();
        var i = 0;
        while (i < n)
        {
            var c = bytes[i];
            if (c == Percent)
            {
                while (i < n && bytes[i] != Newline)
                {
                    i++;
                }

                continue;
            }

            if (c != Backslash)
            {
                i++;
                continue;
            }

            var j = i + 1;
            while (j < n && IsAsciiLetter(bytes[j]))
            {
                j++;
            }

            if (j <= i + 1)
            {
                i = Math.Min(n, i + 2); // `\%`, `\\`, `\{`, or a trailing `\`
                continue;
            }

            var name = Encoding.UTF8.GetString(bytes, i + 1, j - i - 1);
            if (!InterestingCommands.Contains(name) || j >= n || bytes[j] != OpenBrace)
            {
                i = j;
                continue;
            }

            var k = j + 1;
            while (k < n && bytes[k] != CloseBrace && bytes[k] != OpenBrace && bytes[k] != Backslash && bytes[k] != Newline && bytes[k] != Percent)
            {
                k++;
            }

            if (k >= n || bytes[k] != CloseBrace)
            {
                i = j;
                continue;
            }

            var arg = Encoding.UTF8.GetString(bytes, j + 1, k - j - 1);
            result.Add(new CommandUse(name, arg, new ByteRange(i, k + 1), new ByteRange(j + 1, k)));
            i = k + 1;
        }

        return result;
    }

    /// <summary>Result of matching within a single document's text.</summary>
    public abstract record Target
    {
        private Target()
        {
        }

        public sealed record Found(ByteRange Range, string Note) : Target;

        public sealed record NotFound(string Reason) : Target;
    }

    /// <summary>Result of matching across a list of open documents.</summary>
    public abstract record DocumentTarget
    {
        private DocumentTarget()
        {
        }

        public sealed record Found(string Path, ByteRange Range, string Note) : DocumentTarget;

        public sealed record NotFound(string Reason) : DocumentTarget;
    }

    /// <summary>Single-document convenience wrapper over <see cref="MatchingRange(IReadOnlyList{Document}, string, int)"/>.</summary>
    public static Target MatchingRange(string text, int caretByte)
    {
        var result = MatchingRange(new[] { new Document(string.Empty, text) }, string.Empty, caretByte);
        return result switch
        {
            DocumentTarget.Found f => new Target.Found(f.Range, f.Note),
            DocumentTarget.NotFound nf => new Target.NotFound(nf.Reason),
            _ => throw new InvalidOperationException("unreachable DocumentTarget case"),
        };
    }

    /// <summary>
    /// Multi-document counterpart lookup. <c>\begin</c>/<c>\end</c> match
    /// within the active document. <c>\ref{X}</c> finds <c>\label{X}</c> in
    /// the active document first, then the other open documents in project
    /// order. <c>\label{X}</c> cycles through every reference to it: those
    /// after the caret in the active document, then the following documents,
    /// wrapping around. Ported from <c>Navigation.matchingRange(in:activePath:caretByte:)</c>.
    /// </summary>
    public static DocumentTarget MatchingRange(IReadOnlyList<Document> documents, string activePath, int caretByte)
    {
        var activeIndex = -1;
        for (var i = 0; i < documents.Count; i++)
        {
            if (documents[i].Path == activePath)
            {
                activeIndex = i;
                break;
            }
        }

        if (activeIndex < 0)
        {
            return new DocumentTarget.NotFound($"No open document named {activePath}.");
        }

        var uses = CommandUses(documents[activeIndex].Text);
        var hereIndex = FindHere(uses, caretByte);
        if (hereIndex < 0)
        {
            return new DocumentTarget.NotFound("Caret is not inside \\begin, \\end, \\label, or a \\ref-style command.");
        }

        var use = uses[hereIndex];
        var elsewhere = documents.Count > 1 ? " or any open document" : string.Empty;
        return use.Name switch
        {
            "begin" => MatchBegin(uses, hereIndex, use, activePath),
            "end" => MatchEnd(uses, hereIndex, use, activePath),
            "label" => MatchLabel(documents, activeIndex, uses, use, activePath),
            _ => MatchReference(documents, activeIndex, use, activePath, elsewhere),
        };
    }

    private static int FindHere(IReadOnlyList<CommandUse> uses, int caretByte)
    {
        for (var i = 0; i < uses.Count; i++)
        {
            if (uses[i].Range.Start == caretByte)
            {
                return i;
            }
        }

        for (var i = 0; i < uses.Count; i++)
        {
            if (uses[i].Range.Start <= caretByte && caretByte <= uses[i].Range.End)
            {
                return i;
            }
        }

        return -1;
    }

    private static DocumentTarget MatchBegin(IReadOnlyList<CommandUse> uses, int hereIndex, CommandUse use, string activePath)
    {
        var depth = 0;
        for (var i = hereIndex + 1; i < uses.Count; i++)
        {
            var other = uses[i];
            if (other.Arg != use.Arg)
            {
                continue;
            }

            if (other.Name == "begin")
            {
                depth++;
            }
            else if (other.Name == "end")
            {
                if (depth == 0)
                {
                    return new DocumentTarget.Found(activePath, other.Range,
                        $"Matched \\begin{{{use.Arg}}} → \\end{{{use.Arg}}} at byte {other.Range.Start}.");
                }

                depth--;
            }
        }

        return new DocumentTarget.NotFound($"\\begin{{{use.Arg}}} at byte {use.Range.Start} has no matching \\end{{{use.Arg}}}.");
    }

    private static DocumentTarget MatchEnd(IReadOnlyList<CommandUse> uses, int hereIndex, CommandUse use, string activePath)
    {
        var depth = 0;
        for (var i = hereIndex - 1; i >= 0; i--)
        {
            var other = uses[i];
            if (other.Arg != use.Arg)
            {
                continue;
            }

            if (other.Name == "end")
            {
                depth++;
            }
            else if (other.Name == "begin")
            {
                if (depth == 0)
                {
                    return new DocumentTarget.Found(activePath, other.Range,
                        $"Matched \\end{{{use.Arg}}} → \\begin{{{use.Arg}}} at byte {other.Range.Start}.");
                }

                depth--;
            }
        }

        return new DocumentTarget.NotFound($"\\end{{{use.Arg}}} at byte {use.Range.Start} has no matching \\begin{{{use.Arg}}}.");
    }

    private static DocumentTarget MatchLabel(IReadOnlyList<Document> documents, int activeIndex, IReadOnlyList<CommandUse> activeUses, CommandUse use, string activePath)
    {
        var refs = new List<(string Path, CommandUse Use)>();
        for (var offset = 0; offset < documents.Count; offset++)
        {
            var doc = documents[(activeIndex + offset) % documents.Count];
            var docUses = offset == 0 ? activeUses : CommandUses(doc.Text);
            foreach (var candidate in docUses)
            {
                if (ReferenceCommands.Contains(candidate.Name) && candidate.Arg == use.Arg)
                {
                    refs.Add((doc.Path, candidate));
                }
            }
        }

        if (refs.Count == 0)
        {
            var elsewhere = documents.Count > 1 ? " or any open document" : string.Empty;
            return new DocumentTarget.NotFound($"No reference to label {use.Arg} in this document{elsewhere}.");
        }

        var after = -1;
        for (var i = 0; i < refs.Count; i++)
        {
            if (refs[i].Path != activePath || refs[i].Use.Range.Start > use.Range.Start)
            {
                after = i;
                break;
            }
        }

        var chosenIndex = after >= 0 ? after : 0;
        var next = refs[chosenIndex];
        var place = next.Path == activePath ? string.Empty : $" in {next.Path}";
        return new DocumentTarget.Found(next.Path, next.Use.Range,
            $"Reference {chosenIndex + 1} of {refs.Count} to label {use.Arg} at byte {next.Use.Range.Start}{place}.");
    }

    private static DocumentTarget MatchReference(IReadOnlyList<Document> documents, int activeIndex, CommandUse use, string activePath, string elsewhere)
    {
        for (var offset = 0; offset < documents.Count; offset++)
        {
            var doc = documents[(activeIndex + offset) % documents.Count];
            var docUses = CommandUses(doc.Text);
            CommandUse? label = null;
            foreach (var candidate in docUses)
            {
                if (candidate.Name == "label" && candidate.Arg == use.Arg)
                {
                    label = candidate;
                    break;
                }
            }

            if (label is CommandUse found)
            {
                var place = doc.Path == activePath ? string.Empty : $" in {doc.Path}";
                return new DocumentTarget.Found(doc.Path, found.Range, $"Definition of {use.Arg}: \\label at byte {found.Range.Start}{place}.");
            }
        }

        return new DocumentTarget.NotFound($"No \\label{{{use.Arg}}} in this document{elsewhere}.");
    }

    /// <summary>Result of mapping a UTF-8 byte span onto a UTF-16 editor selection.</summary>
    public abstract record RangeMapping
    {
        private RangeMapping()
        {
        }

        /// <summary><paramref name="WidenedFrom"/> is set when the span was widened outward to a whole grapheme cluster.</summary>
        public sealed record Selected(Utf16Range Range, Utf16Range? WidenedFrom) : RangeMapping;

        public sealed record Refused(string Reason) : RangeMapping;
    }

    /// <summary>
    /// Exact UTF-16 selection for UTF-8 bytes <paramref name="start"/>..<paramref name="end"/>
    /// of <paramref name="text"/>. Refused when out of range, reversed, or
    /// inside a multi-byte scalar; widened outward to whole grapheme clusters
    /// (the units a text view selects and moves the caret by) so the
    /// selection never splits one. Ported from <c>Navigation.editorRange(start:end:in:path:)</c>.
    /// </summary>
    public static RangeMapping EditorRange(int start, int end, string text, string path = "")
    {
        var byteCount = ByteOffsets.Utf8ByteCount(text);
        var label = path.Length == 0 ? string.Empty : $" in {path}";
        if (start < 0 || end < start || end > byteCount)
        {
            return new RangeMapping.Refused($"Bytes {start}..{end} are not a valid range{label} (buffer is {byteCount} bytes).");
        }

        var mapped = ByteOffsets.Utf16RangeForUtf8Bytes(text, start, end);
        if (mapped is not (int s, int e))
        {
            return new RangeMapping.Refused($"Bytes {start}..{end}{label} start or end inside a multi-byte character; refusing to split it.");
        }

        var narrow = new Utf16Range(s, e);
        var widened = WidenToGraphemeClusters(text, narrow);
        return new RangeMapping.Selected(widened, widened == narrow ? null : narrow);
    }

    /// <summary>
    /// Widens <paramref name="range"/> outward so both ends sit on grapheme
    /// cluster boundaries (.NET's analogue of NSString's composed character
    /// sequences), matching Navigation.swift's guarantee that a selection
    /// never splits a base character and its combining marks. An empty range
    /// interior to a cluster moves to that cluster's start and stays empty.
    /// </summary>
    private static Utf16Range WidenToGraphemeClusters(string text, Utf16Range range)
    {
        if (text.Length == 0)
        {
            return range;
        }

        var boundaries = System.Globalization.StringInfo.ParseCombiningCharacters(text);
        if (boundaries.Length == 0)
        {
            return range;
        }

        if (range.Length == 0)
        {
            var at = range.Start < text.Length ? ClusterStart(boundaries, range.Start) : range.Start;
            return new Utf16Range(at, at);
        }

        var newStart = ClusterStart(boundaries, range.Start);
        var lastIncluded = range.End - 1;
        var lastClusterStart = ClusterStart(boundaries, lastIncluded);
        var newEnd = NextBoundaryOrEnd(boundaries, lastClusterStart, text.Length);
        return new Utf16Range(newStart, newEnd);
    }

    private static int ClusterStart(int[] boundaries, int index)
    {
        var result = boundaries[0];
        foreach (var boundary in boundaries)
        {
            if (boundary > index)
            {
                break;
            }

            result = boundary;
        }

        return result;
    }

    private static int NextBoundaryOrEnd(int[] boundaries, int clusterStart, int textLength)
    {
        foreach (var boundary in boundaries)
        {
            if (boundary > clusterStart)
            {
                return boundary;
            }
        }

        return textLength;
    }

    /// <summary>Result of rebasing a UTF-8 byte span across an edit.</summary>
    public abstract record Rebased
    {
        private Rebased()
        {
        }

        /// <summary><paramref name="Note"/> names the shift when there was one.</summary>
        public sealed record Mapped(int Start, int End, string? Note) : Rebased;

        public sealed record Refused(string Reason) : Rebased;
    }

    /// <summary>
    /// Maps bytes <paramref name="start"/>..<paramref name="end"/> of
    /// <paramref name="baseline"/> onto <paramref name="current"/> across the
    /// single changed region between them, refusing a span that overlaps the
    /// edit and verifying the mapped bytes still spell the baseline bytes so
    /// a wrong span can never be selected silently. Delegates the actual
    /// diffing/rebasing arithmetic to <see cref="FlashTeX.Protocol.SourceMapping"/>
    /// (already ported from the same Swift source) rather than duplicating it.
    /// Ported from <c>Navigation.rebaseExactly(start:end:from:to:path:)</c>.
    /// </summary>
    public static Rebased RebaseExactly(int start, int end, string baseline, string current, string path)
    {
        var outcome = SourceMapping.Rebase(start, end, baseline, current);
        switch (outcome)
        {
            case SourceMappingOutcome.Unchanged:
                return new Rebased.Mapped(start, end, null);
            case SourceMappingOutcome.OverlapsEdit:
                var region = SourceMapping.ComputeChangedRegion(baseline, current);
                return new Rebased.Refused(
                    $"bytes {start}..{end} of {path} overlap the edit at {region.StartByte}..{region.OldEndByte}, now {region.StartByte}..{region.NewEndByte}");
            case SourceMappingOutcome.Rebased rebased:
                var was = ByteOffsets.Utf16RangeForUtf8Bytes(baseline, start, end);
                var now = ByteOffsets.Utf16RangeForUtf8Bytes(current, rebased.Start, rebased.End);
                if (was is not (int ws, int we) || now is not (int ns, int ne) ||
                    !ByteOffsets.SameBytes(baseline[ws..we], current[ns..ne]))
                {
                    return new Rebased.Refused(
                        $"bytes {start}..{end} of {path} no longer spell the same text after rebasing to {rebased.Start}..{rebased.End}");
                }

                var note = rebased.Start != start ? $"rebased from {start}..{end} across edits" : null;
                return new Rebased.Mapped(rebased.Start, rebased.End, note);
            default:
                throw new InvalidOperationException("unreachable SourceMappingOutcome case");
        }
    }
}
