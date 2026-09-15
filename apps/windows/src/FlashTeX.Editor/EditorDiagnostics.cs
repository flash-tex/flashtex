// name: EditorDiagnostics.cs
// purpose: Turns a `compile_result`'s diagnostics into editor underline marks
//   for one document. Pure byte-range rebasing: a diagnostic's UTF-8 byte range
//   is rebased across edits made since the compile (`SourceMapping`, one
//   `ChangedRegion` per call) and dropped ("stale") when it overlaps the edited
//   region, so a mark is never drawn under the wrong text. Every mark carries
//   the exact identity of the diagnostic it came from (result id + index +
//   original byte span) so the underline and a diagnostics-list row can agree
//   on which diagnostic they describe even after the range has been rebased.
//   Ported from apps/mac/Sources/FlashTeXMac/EditorDiagnostics.swift.
//
//   Scope note: this file ports the rebasing engine, retained/carried-marks
//   logic, gap categorization and diagnostic grouping — the parts the porting
//   brief asks for ("byte-range rebasing of diagnostics across edits, with
//   identity keys"). It does NOT port `EditorDiagnostics.Explanation` /
//   `ExplanationClient` (a Rust-subprocess-backed catalogue reached over
//   stdio) or the `QuickFix` apply-suggestion flow, and it does not port
//   keyboard navigation (`navigationItems`/`step`, which depend on
//   `EditorDiagnosticNavigation`/`AccessibleEditorModel`, Mac-only
//   accessibility infrastructure not in this port's file list). Those are
//   separate, later work.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Globalization;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Editor;

/// <summary>A UTF-16 char range in an editor buffer (the .NET analogue of Foundation's `NSRange`).</summary>
public readonly record struct Utf16Range(int Location, int Length)
{
    public int End => Location + Length;
}

public static class EditorDiagnostics
{
    /// <summary>
    /// Stable identity of one diagnostic in one compile result. The byte span is
    /// the one the worker reported (in the compiled text), never the rebased
    /// one, so the identity survives edits that shift the mark.
    /// </summary>
    public readonly record struct Identity(string? ResultId, int DiagnosticIndex, SourceRange Source)
    {
        /// <summary>"id#index@path:start..&lt;end" — one token a list row and the underline can both compute.</summary>
        public string Key => $"{ResultId ?? "-"}#{DiagnosticIndex}@{Source.Path}:{Source.StartByte}..<{Source.EndByte}";
    }

    /// <summary>
    /// Why a mark is shown although it is not from the newest result: the newest
    /// result (<see cref="FailedRevision"/>) failed with no output, so the marks
    /// of the last result with output (<see cref="Revision"/>) are kept, rebased
    /// to the current text, and flagged — never cleared, never duplicated.
    /// </summary>
    public sealed record Carried(int Revision, int FailedRevision)
    {
        public string Line => $"kept from revision {Revision}: revision {FailedRevision} failed with no output";
    }

    public sealed record Mark
    {
        public required Identity Identity { get; init; }

        /// <summary>Range in the *current* editor text (rebased; snapped outward to grapheme cluster boundaries so an underline never splits one).</summary>
        public required Utf16Range Range { get; init; }

        public required Severity Severity { get; init; }

        public required string Message { get; init; }

        public string? Recovery { get; init; }

        /// <summary>
        /// Status of the result the diagnostic belongs to. A `recovered` result
        /// rendered provisionally, so its marks always show a recovery line —
        /// and stay errors; recovery never downgrades or hides one.
        /// </summary>
        public required Status ResultStatus { get; init; }

        /// <summary>Set once an explanation helper has answered for this result; not populated by this port (see file header).</summary>
        public string? Explanation { get; set; }

        /// <summary>Set when this mark was kept from an older result because the newest one failed with no output; null for marks of the result currently shown.</summary>
        public Carried? CarriedFrom { get; set; }

        public string Id => Identity.Key;

        public int DiagnosticIndex => Identity.DiagnosticIndex;

        /// <summary>The original byte span (for a list row's "bytes a..&lt;b" text).</summary>
        public SourceRange OriginalSource => Identity.Source;

        /// <summary>
        /// The recovery line shared by a tooltip, a list row and accessibility
        /// text: the worker's note, or "no provisional rendering" when the
        /// result is `recovered` but this diagnostic describes none.
        /// </summary>
        public string? RecoveryLine => EditorDiagnostics.RecoveryLine(Recovery, ResultStatus);

        public string ToolTip =>
            Message
            + (RecoveryLine is { } recoveryLine ? "\n↳ " + recoveryLine : "")
            + (Explanation is { } explanation ? "\n↳ " + explanation : "")
            + (CarriedFrom is { } carried ? "\n↳ " + carried.Line : "");

        public string SpokenDescription =>
            (Severity == Severity.error ? "Error: " : "Warning: ") + Message
            + (RecoveryLine is { } recoveryLine ? " — " + recoveryLine : "")
            + (Explanation is { } explanation ? " — " + explanation : "")
            + (CarriedFrom is { } carried ? " — " + carried.Line : "");
    }

    /// <summary>A diagnostic whose span overlaps the edit made since the compile: its underline is withheld until the next result.</summary>
    public readonly record struct Stale(Identity Identity, Severity Severity, string Message);

    /// <summary>One call's outcome: the drawable marks plus what was withheld.</summary>
    public sealed record Report
    {
        public required IReadOnlyList<Mark> Marks { get; init; }

        public required IReadOnlyList<Stale> StaleDiagnostics { get; init; }

        /// <summary>The edit the marks were rebased across (null: compiled text unknown or unchanged).</summary>
        public ChangedRegion? Edit { get; init; }

        /// <summary>Set when the marks were kept from an older result because the newest one failed with no output.</summary>
        public Carried? CarriedFrom { get; init; }

        public static Report Empty => new() { Marks = [], StaleDiagnostics = [] };

        public int StaleCount => StaleDiagnostics.Count;

        public IReadOnlySet<Identity> StaleIdentities => StaleDiagnostics.Select(s => s.Identity).ToHashSet();

        /// <summary>Footer/list caption: the carried-over line, the withheld count, or both; null when the marks are current and complete.</summary>
        public string? StaleNote => (CarriedFrom, WithheldNote) switch
        {
            (null, var withheld) => withheld,
            ({ } carried, null) => $"{Marks.Count} underline{(Marks.Count == 1 ? "" : "s")} {carried.Line}",
            ({ } carried, { } withheld) => $"{Marks.Count} underline{(Marks.Count == 1 ? "" : "s")} {carried.Line}; {withheld}",
        };

        /// <summary>The withheld count alone, null when nothing is withheld.</summary>
        public string? WithheldNote
        {
            get
            {
                if (StaleDiagnostics.Count == 0)
                {
                    return null;
                }
                int errors = StaleDiagnostics.Count(s => s.Severity == Severity.error);
                int warnings = StaleDiagnostics.Count - errors;
                string what = (errors, warnings) switch
                {
                    (0, var w) => $"{w} warning{(w == 1 ? "" : "s")}",
                    (var e, 0) => $"{e} error{(e == 1 ? "" : "s")}",
                    (var e, var w) => $"{e} error{(e == 1 ? "" : "s")} and {w} warning{(w == 1 ? "" : "s")}",
                };
                return $"{what} under edited text not underlined until the next compile";
            }
        }
    }

    // MARK: gap category (compiler "not implemented yet" vs. "the source is wrong")

    private static readonly string[] GapPhrases =
    [
        "not implemented",
        "not supported by this compiler version",
        "not supported in math mode",
        "not supported in the document preamble",
        "is unsupported",
    ];

    /// <summary>Whether a diagnostic says "FlashTeX does not do this yet" rather than "the source is wrong".</summary>
    public static bool IsGap(string message) => GapPhrases.Any(message.Contains);

    /// <summary>Errors and warnings that are not gaps, and the gaps, of <paramref name="diagnostics"/>.</summary>
    public static (int Errors, int Warnings, int Gaps) Counts(IReadOnlyList<Diagnostic> diagnostics)
    {
        int errors = 0, warnings = 0, gaps = 0;
        foreach (var d in diagnostics)
        {
            if (IsGap(d.Message))
            {
                gaps += 1;
            }
            else if (d.Severity == Severity.error)
            {
                errors += 1;
            }
            else
            {
                warnings += 1;
            }
        }
        return (errors, warnings, gaps);
    }

    /// <summary>Recovery line for a diagnostic of a result with <paramref name="status"/> (see <see cref="Mark.RecoveryLine"/>).</summary>
    public static string? RecoveryLine(string? recovery, Status status) =>
        recovery is not null ? "recovery: " + recovery : status == Status.recovered ? "no provisional rendering" : null;

    /// <summary>Identity of <c>result.Diagnostics[index]</c>, null when it has no source.</summary>
    public static Identity? IdentityFor(string? resultId, int index, CompileResult result)
    {
        if (index < 0 || index >= result.Diagnostics.Count || result.Diagnostics[index].Source is not { } source)
        {
            return null;
        }
        return new Identity(resultId, index, source);
    }

    /// <summary>Marks only (see <see cref="MakeReport"/> for the stale set).</summary>
    public static IReadOnlyList<Mark> Marks(CompileResult result, string? resultId, string path, string? compiledText, string currentText) =>
        MakeReport(result, resultId, path, compiledText, currentText).Marks;

    /// <summary>
    /// Marks plus the diagnostics withheld as stale. Cost is one byte diff of the
    /// two texts plus O(diagnostics) offset conversions; it never waits on a
    /// compile, so a keystroke can call it synchronously.
    /// </summary>
    public static Report MakeReport(CompileResult result, string? resultId, string path, string? compiledText, string currentText)
    {
        ChangedRegion? region = compiledText is not null && !ByteOffsets.SameBytes(compiledText, currentText)
            ? SourceMapping.ComputeChangedRegion(compiledText, currentText)
            : null;

        // Grapheme cluster boundaries of the current text, computed once for
        // every mark's outward-snap (the .NET equivalent of the Swift source's
        // per-index `String.Index(_:within:)` check).
        int[] clusterBoundaries = StringInfo.ParseCombiningCharacters(currentText);

        var marks = new List<Mark>(result.Diagnostics.Count);
        var stale = new List<Stale>();
        for (int index = 0; index < result.Diagnostics.Count; index++)
        {
            var diagnostic = result.Diagnostics[index];
            if (diagnostic.Source is not { } source || source.Path != path)
            {
                continue;
            }
            var identity = new Identity(resultId, index, source);
            int start = source.StartByte, end = source.EndByte;
            if (region is { } changedRegion)
            {
                var outcome = SourceMapping.Rebase(start, end, changedRegion);
                if (outcome is not global::FlashTeX.Protocol.SourceMappingOutcome.Rebased rebased)
                {
                    stale.Add(new Stale(identity, diagnostic.Severity, diagnostic.Message));
                    continue;
                }
                start = rebased.Start;
                end = rebased.End;
            }
            // Offsets inside a multi-byte scalar, reversed or out of range are
            // refused (null): a byte span that is not a valid slice of the
            // current buffer is not drawn anywhere.
            if (ClusterAlignedRange(currentText, start, end, clusterBoundaries) is not { } range)
            {
                continue;
            }
            marks.Add(new Mark
            {
                Identity = identity,
                Range = range,
                Severity = diagnostic.Severity,
                Message = diagnostic.Message,
                Recovery = diagnostic.Recovery,
                ResultStatus = result.Status,
            });
        }
        return new Report { Marks = marks, StaleDiagnostics = stale, Edit = region };
    }

    /// <summary>
    /// UTF-16 range for a UTF-8 byte span, widened outward to the grapheme
    /// clusters containing its ends so the range never splits one. Null when
    /// the bytes are not a valid scalar-aligned slice of the string.
    /// </summary>
    private static Utf16Range? ClusterAlignedRange(string text, int utf8Start, int utf8End, int[] clusterBoundaries)
    {
        if (ByteOffsets.Utf16RangeForUtf8Bytes(text, utf8Start, utf8End) is not (int start, int end))
        {
            return null;
        }
        start = SnapBackward(clusterBoundaries, start);
        if (end < text.Length)
        {
            end = SnapForward(clusterBoundaries, end);
        }
        return new Utf16Range(start, end - start);
    }

    private static int SnapBackward(int[] boundaries, int index)
    {
        if (index <= 0)
        {
            return index;
        }
        int position = Array.BinarySearch(boundaries, index);
        if (position >= 0)
        {
            return index; // already a cluster boundary
        }
        int insertion = ~position; // boundaries[insertion] is the smallest entry > index
        return insertion > 0 ? boundaries[insertion - 1] : 0;
    }

    private static int SnapForward(int[] boundaries, int index)
    {
        int position = Array.BinarySearch(boundaries, index);
        if (position >= 0)
        {
            return index; // already a cluster boundary
        }
        int insertion = ~position; // boundaries[insertion] is the smallest entry > index
        return insertion < boundaries.Length ? boundaries[insertion] : index;
    }

    // MARK: - Partial output: a failed follow-up keeps the last marks, flagged

    /// <summary>The last result that produced output, remembered so a later `failed` result with no pages does not clear the underlines.</summary>
    public sealed record Retained(string? ResultId, CompileResult Result, IReadOnlyDictionary<string, string> CompiledDocuments);

    /// <summary>
    /// True when <paramref name="result"/> is a failure that produced no
    /// output, so the marks of the last result with output are kept (flagged)
    /// rather than cleared. Any result with pages, and any `ok`/`recovered`
    /// result, replaces them.
    /// </summary>
    public static bool KeepsPreviousMarks(CompileResult result) => result.Status == Status.failed && result.Pages.Count == 0;

    /// <summary>
    /// The <see cref="Retained"/> record to remember after binding
    /// <paramref name="result"/>: the result itself when it produced output,
    /// otherwise the previous record (retention never chains through failures).
    /// </summary>
    public static Retained? RetainedAfter(CompileResult result, string? resultId, IReadOnlyDictionary<string, string> compiledDocuments, Retained? previous) =>
        KeepsPreviousMarks(result) ? previous : new Retained(resultId, result, compiledDocuments);

    /// <summary>
    /// <see cref="MakeReport"/> that survives a failed follow-up: when
    /// <paramref name="latest"/> is a failure with no output and
    /// <paramref name="retained"/> holds an earlier result with output, the
    /// marks are the retained result's, rebased from the text it was compiled
    /// from to <paramref name="currentText"/> and flagged carried; any sourced
    /// diagnostic of the failed result itself is a fresh, unflagged mark
    /// alongside them. Otherwise this is exactly <see cref="MakeReport"/>.
    /// </summary>
    public static Report MakeReportWithRetention(CompileResult latest, string? resultId, Retained? retained, string path, string? compiledText, string currentText)
    {
        var fresh = MakeReport(latest, resultId, path, compiledText, currentText);
        if (!KeepsPreviousMarks(latest) || retained is null || KeepsPreviousMarks(retained.Result))
        {
            return fresh;
        }
        retained.CompiledDocuments.TryGetValue(path, out var retainedCompiledText);
        var kept = MakeReport(retained.Result, retained.ResultId, path, retainedCompiledText, currentText);
        var carried = new Carried(retained.Result.Revision, latest.Revision);
        var keptMarks = kept.Marks.Select(m => m with { CarriedFrom = carried }).ToList();
        return kept with
        {
            Marks = [.. fresh.Marks, .. keptMarks],
            StaleDiagnostics = [.. fresh.StaleDiagnostics, .. kept.StaleDiagnostics],
            CarriedFrom = carried,
            Edit = kept.Edit ?? fresh.Edit,
        };
    }

    // MARK: - Identical diagnostics grouped (count + per-occurrence jump)

    /// <summary>Diagnostics of one result with the same severity and message, in the order of their first occurrence.</summary>
    public sealed record Group(Severity Severity, string Message, string? Recovery, IReadOnlyList<int> Occurrences)
    {
        public int Count => Occurrences.Count;

        /// <summary>Index of the first occurrence (document order).</summary>
        public int First => Occurrences[0];

        public string Id => $"{Severity}:{Message}";

        /// <summary>"12x \in is not supported in math mode", or just the message for one.</summary>
        public string Title => Count > 1 ? $"{Count}× {Message}" : Message;
    }

    /// <summary>Groups <paramref name="diagnostics"/> by (severity, message). <paramref name="documentOrder"/> orders occurrences across documents (unknown paths after known ones).</summary>
    public static IReadOnlyList<Group> Groups(IReadOnlyList<Diagnostic> diagnostics, IReadOnlyList<string>? documentOrder = null)
    {
        var order = documentOrder ?? [];
        int Rank(string path)
        {
            int i = order.ToList().IndexOf(path);
            return i < 0 ? order.Count : i;
        }

        var keyOrder = new List<string>();
        var members = new Dictionary<string, List<int>>();
        for (int i = 0; i < diagnostics.Count; i++)
        {
            string key = $"{diagnostics[i].Severity}:{diagnostics[i].Message}";
            if (!members.TryGetValue(key, out var list))
            {
                list = [];
                members[key] = list;
                keyOrder.Add(key);
            }
            list.Add(i);
        }

        bool Before(int a, int b)
        {
            var sourceA = diagnostics[a].Source;
            var sourceB = diagnostics[b].Source;
            if (sourceA is null && sourceB is null)
            {
                return a < b;
            }
            if (sourceA is null)
            {
                return false;
            }
            if (sourceB is null)
            {
                return true;
            }
            if (sourceA.Path != sourceB.Path)
            {
                int rankA = Rank(sourceA.Path), rankB = Rank(sourceB.Path);
                return rankA != rankB ? rankA < rankB : string.CompareOrdinal(sourceA.Path, sourceB.Path) < 0;
            }
            return sourceA.StartByte != sourceB.StartByte ? sourceA.StartByte < sourceB.StartByte : a < b;
        }

        var groups = new List<Group>();
        foreach (var key in keyOrder)
        {
            var sorted = members[key];
            sorted.Sort((a, b) => Before(a, b) ? -1 : Before(b, a) ? 1 : 0);
            var recoveries = sorted.Select(i => diagnostics[i].Recovery).Distinct().ToList();
            groups.Add(new Group(diagnostics[sorted[0]].Severity, diagnostics[sorted[0]].Message,
                recoveries.Count == 1 ? diagnostics[sorted[0]].Recovery : null, sorted));
        }
        groups.Sort((a, b) => Before(a.First, b.First) ? -1 : Before(b.First, a.First) ? 1 : 0);
        return groups;
    }

    public static IReadOnlyList<Group> Groups(CompileResult result, IReadOnlyList<string>? documentOrder = null) => Groups(result.Diagnostics, documentOrder);

    /// <summary>The source of occurrence <paramref name="k"/> (zero-based, document order) of <paramref name="group"/>, null when out of range or unsourced.</summary>
    public static SourceRange? Occurrence(int k, Group group, IReadOnlyList<Diagnostic> diagnostics)
    {
        if (k < 0 || k >= group.Occurrences.Count || group.Occurrences[k] >= diagnostics.Count)
        {
            return null;
        }
        return diagnostics[group.Occurrences[k]].Source;
    }

    public static SourceRange? Occurrence(int k, Group group, CompileResult result) => Occurrence(k, group, result.Diagnostics);

    /// <summary>"3 of 12: main.tex line 41" (from <paramref name="texts"/>, the compiled text, when known; else "bytes a..&lt;b"); "3 of 12: no source" when unsourced.</summary>
    public static string OccurrenceLabel(int k, Group group, IReadOnlyList<Diagnostic> diagnostics, IReadOnlyDictionary<string, string>? texts = null)
    {
        string prefix = $"{k + 1} of {group.Count}: ";
        if (Occurrence(k, group, diagnostics) is not { } source)
        {
            return prefix + "no source";
        }
        if (texts is not null && texts.TryGetValue(source.Path, out var text) && LineNumber(source.StartByte, text) is { } line)
        {
            return prefix + $"{source.Path} line {line}";
        }
        return prefix + $"{source.Path} bytes {source.StartByte}..<{source.EndByte}";
    }

    public static string OccurrenceLabel(int k, Group group, CompileResult result, IReadOnlyDictionary<string, string>? texts = null) =>
        OccurrenceLabel(k, group, result.Diagnostics, texts);

    /// <summary>One-based line containing byte <paramref name="offset"/> of <paramref name="text"/> (LF-counted); null when out of range.</summary>
    public static int? LineNumber(int offset, string text)
    {
        int totalBytes = ByteOffsets.Utf8ByteCount(text);
        if (offset < 0 || offset > totalBytes)
        {
            return null;
        }
        byte[] bytes = System.Text.Encoding.UTF8.GetBytes(text);
        int line = 1;
        for (int i = 0; i < offset; i++)
        {
            if (bytes[i] == 0x0A)
            {
                line += 1;
            }
        }
        return line;
    }
}
