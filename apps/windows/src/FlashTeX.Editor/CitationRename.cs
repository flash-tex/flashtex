// name: CitationRename.cs
// purpose: Plan-parsing/validation logic for a reviewed citation-key rename
//   through a durable helper's read-only `plan_citation_rename` /
//   `plan_citation_rename_at` proposals (schema
//   `flashtex.citation-rename-plan.v1`, kind `citation_key_rename`). Ported
//   from apps/mac/Sources/FlashTeXMac/CitationRename.swift.
//
//   Scope note: only the plan/apply-adjacent PURE logic is ported here: new-key
//   validation (`KeyProblem`), the helper-error explanation table (`Explain`),
//   the request payload shape (`BuildRequest`) and, the bulk of the port, the
//   plan-reply parser/validator (`ParsePlan`) — which checks schema/kind,
//   snapshot generation and per-document versions, that the rename's
//   old/new name and application order match, and that every edit is
//   nonoverlapping and renames exactly the old key to the new one. This is the
//   same anchor/rebase discipline as `Insertion.cs`, applied to a whole plan.
//
//   NOT ported: the small review-window UI (`CitationRenamePanel`/
//   `CitationRenameBody`, WinUI-dependent future work per this port's
//   instructions); `previews(for:texts:)`, which depends on
//   `ProjectSearch.locate` (project-search infrastructure outside this port's
//   file list); `accessibilityLabel`, which depends on the Mac `DocumentKind`
//   model; and the network-calling `CitationRenameClient`
//   (`locateKeyAtCaret`/`planRename`/`applyRename`/`retryUncertain`), which
//   drives a durable helper process over a bespoke JSON-RPC protocol — that is
//   transport/client infrastructure, not the portable plan logic itself.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text.Json;
using FlashTeX.Protocol;

namespace FlashTeX.Editor;

public static class CitationRename
{
    public const string PlanSchema = "flashtex.citation-rename-plan.v1";
    public const string PlanKind = "citation_key_rename";
    public const string CommandPrefix = "citation-rename";

    /// <summary>The helper's key rule (project-index `plan_citation_rename`): 1…4096 bytes, no whitespace/control characters, none of <c>{}()\"%=#@,</c>.</summary>
    public const int MaxKeyBytes = 4096;

    private static readonly HashSet<char> ForbiddenKeyCharacters = ['{', '}', '(', ')', '\\', '"', '%', '=', '#', '@', ','];

    public const string NoHelperMessage = "Renaming a citation requires the durable helper (flashtex-preview-controller) to be attached and ready.";

    /// <summary>Why a typed key cannot be sent, mirroring the helper's rule so a refusal is explained before a round trip; the helper's check stays authoritative.</summary>
    public static string? KeyProblem(string key)
    {
        if (key.Length == 0)
        {
            return "the new key is empty";
        }
        int byteCount = ByteOffsets.Utf8ByteCount(key);
        if (byteCount > MaxKeyBytes)
        {
            return $"the new key is {byteCount} bytes; the helper accepts at most {MaxKeyBytes}";
        }
        foreach (char c in key)
        {
            if (char.IsWhiteSpace(c) || ForbiddenKeyCharacters.Contains(c) || char.IsControl(c))
            {
                string what = char.IsWhiteSpace(c) ? "whitespace" : c.ToString();
                return $"the key contains “{what}”, which a citation key cannot contain";
            }
        }
        return null;
    }

    private static readonly (string Code, string Hint)[] HelperErrorHints =
    [
        ("MissingBibliographyDefinition", "the key has no well-formed record in a declared bibliography source (a \\bibitem alone is not enough; declare the .bib through Document Kinds)"),
        ("AmbiguousCitationDefinition", "the key is defined more than once"),
        ("MalformedBibliographyDefinition", "the key's bibliography record is malformed"),
        ("RenameCollision", "the new key already exists in the project"),
        ("InvalidCitationKey", "the new key is not a valid citation key"),
        ("InvalidCitationRenamePlan", "the span is not a citation key the index knows, or the plan could not be built"),
        ("ReplacementPlanTooLarge", "the rename touches more text than the helper's plan limit"),
        ("unknown operation", "this helper build has no plan_citation_rename (needs crates/preview-controller with source plans)"),
        ("source snapshot changed", "the project changed since the snapshot; plan again"),
        ("unknown source path", "the caret's document is not part of the helper's project"),
    ];

    /// <summary>A short explanation for the helper's refusal codes (the helper's own message is always shown alongside; nothing is inferred from it).</summary>
    public static string Explain(string helperErrorMessage)
    {
        foreach (var (code, hint) in HelperErrorHints)
        {
            if (helperErrorMessage.Contains(code, StringComparison.Ordinal))
            {
                return $"{hint} (helper: {helperErrorMessage})";
            }
        }
        return $"helper: {helperErrorMessage}";
    }

    /// <summary>One rebased, byte-verified edit of a citation-rename plan.</summary>
    public sealed record ReplacementEdit(string Path, int Revision, int StartByte, int EndByte, string ExpectedText, string Replacement);

    /// <summary>The helper's proposal, bound to the exact snapshot it was computed on.</summary>
    public sealed record Plan(
        string OldName,
        string NewName,
        string ProjectId,
        IReadOnlyDictionary<string, int> SourceVersions,
        int MembershipGeneration,
        IReadOnlyList<ReplacementEdit> Edits)
    {
        /// <summary>Distinct paths touched, sorted (helper order is path then byte offset; nonoverlapping within a file).</summary>
        public IReadOnlyList<string> Paths => Edits.Select(e => e.Path).Distinct(StringComparer.Ordinal).OrderBy(p => p, StringComparer.Ordinal).ToList();

        public IReadOnlyList<ReplacementEdit> EditsIn(string path) => Edits.Where(e => e.Path == path).ToList();

        public string Summary => $"{Edits.Count} occurrence{(Edits.Count == 1 ? "" : "s")} in {Paths.Count} file{(Paths.Count == 1 ? "" : "s")}";

        public string Label => $"Rename citation “{OldName}” to “{NewName}”";
    }

    /// <summary>Result of parsing/validating a citation-rename plan reply: accepted, or refused with a reason.</summary>
    public abstract record PlanResult
    {
        private PlanResult()
        {
        }

        public sealed record Accepted(Plan Plan) : PlanResult;

        public sealed record Refused(string Reason) : PlanResult;
    }

    /// <summary>
    /// Parses and validates a citation-rename reply. <paramref name="expectedOldName"/>
    /// is required when the user typed it; for the `_at` variant the plan's own
    /// `rename.old_name` is the key (the helper resolved the span). Anything
    /// unexpected refuses the whole plan; nothing is inferred.
    /// </summary>
    public static PlanResult ParsePlan(JsonElement reply, string? expectedOldName, string newName)
    {
        PlanResult Refuse(string why) => new PlanResult.Refused("citation rename plan refused: " + why);

        if (!TryGetStringIntMap(reply, "source_versions", out var versions))
        {
            return Refuse("reply has no source_versions");
        }
        if (!TryGetInt(reply, "membership_generation", out int generation))
        {
            return Refuse("reply has no membership_generation");
        }
        if (!TryGetObject(reply, "plan", out var plan))
        {
            return Refuse("reply has no plan");
        }
        if (GetString(plan, "schema") != PlanSchema)
        {
            return Refuse($"schema is {GetString(plan, "schema") ?? "missing"}, not {PlanSchema}");
        }
        if (GetString(plan, "kind") != PlanKind)
        {
            return Refuse($"kind is {GetString(plan, "kind") ?? "missing"}, not {PlanKind}");
        }
        if (GetBool(plan, "proposal_only") != true || GetBool(plan, "requires_user_approval") != true)
        {
            return Refuse("plan is not marked proposal_only + requires_user_approval");
        }
        if (GetString(plan, "application_order") != "reverse_byte_offset_per_document")
        {
            return Refuse($"unknown application_order {GetString(plan, "application_order") ?? "missing"}");
        }

        if (!TryGetObject(plan, "snapshot", out var snapshot))
        {
            return Refuse("plan snapshot is malformed");
        }
        string? projectId = GetString(snapshot, "project_id");
        if (projectId is null || !TryGetInt(snapshot, "generation", out int snapGeneration) || !TryGetArray(snapshot, "documents", out var snapDocs))
        {
            return Refuse("plan snapshot is malformed");
        }
        if (snapGeneration != generation)
        {
            return Refuse($"plan generation {snapGeneration} is not the reply's {generation}");
        }
        var snapVersions = new Dictionary<string, int>(StringComparer.Ordinal);
        foreach (var doc in snapDocs.EnumerateArray())
        {
            string? file = GetString(doc, "file");
            if (file is null || !TryGetInt(doc, "revision", out int rev))
            {
                return Refuse("plan snapshot document is malformed");
            }
            snapVersions[file] = rev;
        }
        if (!MapsEqual(snapVersions, versions))
        {
            return Refuse("plan snapshot versions differ from the reply's versions");
        }

        if (!TryGetObject(plan, "rename", out var rename))
        {
            return Refuse("plan has no rename.old_name/new_name");
        }
        string? oldName = GetString(rename, "old_name");
        string? planNewName = GetString(rename, "new_name");
        if (oldName is null || planNewName is null)
        {
            return Refuse("plan has no rename.old_name/new_name");
        }
        if (expectedOldName is not null && expectedOldName != oldName)
        {
            return Refuse($"plan renames “{oldName}”, not “{expectedOldName}”");
        }
        if (planNewName != newName || GetString(plan, "replacement") != newName)
        {
            return Refuse($"plan new name is not “{newName}”");
        }
        if (oldName == newName)
        {
            return Refuse("old and new key are the same");
        }

        if (!TryGetArray(plan, "edits", out var rawEdits))
        {
            return Refuse("plan has no edits");
        }
        var edits = new List<ReplacementEdit>();
        var lastByPath = new Dictionary<string, int>(StringComparer.Ordinal);
        int oldNameByteCount = ByteOffsets.Utf8ByteCount(oldName);
        int i = 0;
        foreach (var e in rawEdits.EnumerateArray())
        {
            string? file = GetString(e, "file");
            string? expected = GetString(e, "expected_text");
            string? replacement = GetString(e, "replacement");
            if (file is null || !TryGetInt(e, "revision", out int rev) || !TryGetInt(e, "start_byte", out int start)
                || !TryGetInt(e, "end_byte", out int end) || expected is null || replacement is null)
            {
                return Refuse($"edit {i} is malformed");
            }
            if (!versions.TryGetValue(file, out int expectedRevision) || expectedRevision != rev)
            {
                string knownRevision = versions.TryGetValue(file, out int v) ? v.ToString() : "?";
                return Refuse($"edit {i} names {file} r{rev}, not the snapshot's r{knownRevision}");
            }
            if (start > end || end - start != oldNameByteCount || expected != oldName || replacement != newName)
            {
                return Refuse($"edit {i} ({file} {start}..<{end}) does not rename exactly “{oldName}” to “{newName}”");
            }
            if (lastByPath.TryGetValue(file, out int last) && start < last)
            {
                return Refuse($"edit {i} in {file} overlaps or precedes the previous edit");
            }
            lastByPath[file] = end;
            edits.Add(new ReplacementEdit(file, rev, start, end, expected, replacement));
            i += 1;
        }
        return new PlanResult.Accepted(new Plan(oldName, newName, projectId, snapVersions, generation, edits));
    }

    /// <summary>Operation name and fields of `plan_citation_rename` (typed old key) or `plan_citation_rename_at` (exact key span).</summary>
    public sealed record PlanRequest(
        string Operation,
        IReadOnlyDictionary<string, int> SourceVersions,
        int MembershipGeneration,
        int MaxBytes,
        string NewName,
        string? OldName,
        string? Path,
        int? StartByte,
        int? EndByte);

    /// <summary>Builds the request for either the typed-old-key or the exact-span variant, matching the helper's `plan_citation_rename`/`plan_citation_rename_at` payload shape.</summary>
    public static PlanRequest BuildRequest(
        IReadOnlyDictionary<string, int> sourceVersions, int membershipGeneration, int maxBytes, string newName,
        string? oldName, (string Path, int StartByte, int EndByte)? span)
    {
        if (span is { } s)
        {
            return new PlanRequest("plan_citation_rename_at", sourceVersions, membershipGeneration, maxBytes, newName,
                OldName: null, Path: s.Path, StartByte: s.StartByte, EndByte: s.EndByte);
        }
        return new PlanRequest("plan_citation_rename", sourceVersions, membershipGeneration, maxBytes, newName,
            OldName: oldName ?? "", Path: null, StartByte: null, EndByte: null);
    }

    // MARK: JSON helpers (System.Text.Json boundary parsing, analogous to the Swift source's `[String: Any]` reads)

    private static bool TryGetObject(JsonElement parent, string name, out JsonElement value)
    {
        if (parent.ValueKind == JsonValueKind.Object && parent.TryGetProperty(name, out value) && value.ValueKind == JsonValueKind.Object)
        {
            return true;
        }
        value = default;
        return false;
    }

    private static bool TryGetArray(JsonElement parent, string name, out JsonElement value)
    {
        if (parent.ValueKind == JsonValueKind.Object && parent.TryGetProperty(name, out value) && value.ValueKind == JsonValueKind.Array)
        {
            return true;
        }
        value = default;
        return false;
    }

    private static string? GetString(JsonElement parent, string name) =>
        parent.ValueKind == JsonValueKind.Object && parent.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.String
            ? value.GetString()
            : null;

    private static bool? GetBool(JsonElement parent, string name) =>
        parent.ValueKind == JsonValueKind.Object && parent.TryGetProperty(name, out var value) && value.ValueKind is JsonValueKind.True or JsonValueKind.False
            ? value.GetBoolean()
            : null;

    private static bool TryGetInt(JsonElement parent, string name, out int value)
    {
        if (parent.ValueKind == JsonValueKind.Object && parent.TryGetProperty(name, out var element) && element.ValueKind == JsonValueKind.Number
            && element.TryGetInt32(out value))
        {
            return true;
        }
        value = 0;
        return false;
    }

    private static bool TryGetStringIntMap(JsonElement parent, string name, out Dictionary<string, int> value)
    {
        value = new Dictionary<string, int>(StringComparer.Ordinal);
        if (parent.ValueKind != JsonValueKind.Object || !parent.TryGetProperty(name, out var map) || map.ValueKind != JsonValueKind.Object)
        {
            return false;
        }
        foreach (var property in map.EnumerateObject())
        {
            if (property.Value.ValueKind != JsonValueKind.Number || !property.Value.TryGetInt32(out int i))
            {
                return false;
            }
            value[property.Name] = i;
        }
        return true;
    }

    private static bool MapsEqual(IReadOnlyDictionary<string, int> a, IReadOnlyDictionary<string, int> b) =>
        a.Count == b.Count && a.All(kv => b.TryGetValue(kv.Key, out int v) && v == kv.Value);
}
