// name: CitationRenameTests.cs
// purpose: Unit tests for FlashTeX.Editor.CitationRename (key validation,
//   helper-error explanations, request shape, and plan parsing/validation),
//   ported behavior from apps/mac/Sources/FlashTeXMac/CitationRename.swift.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text.Json;

namespace FlashTeX.Editor.Tests;

public class CitationRenameTests
{
    // MARK: KeyProblem

    [Fact]
    public void KeyProblemRefusesEmptyKey()
    {
        Assert.Equal("the new key is empty", CitationRename.KeyProblem(""));
    }

    [Fact]
    public void KeyProblemAcceptsAnOrdinaryKey()
    {
        Assert.Null(CitationRename.KeyProblem("smith2024"));
    }

    [Theory]
    [InlineData("a b")]
    [InlineData("a\tb")]
    public void KeyProblemRefusesWhitespace(string key)
    {
        Assert.Contains("whitespace", CitationRename.KeyProblem(key));
    }

    [Theory]
    [InlineData("a{b")]
    [InlineData("a}b")]
    [InlineData("a(b")]
    [InlineData("a)b")]
    [InlineData("a\\b")]
    [InlineData("a\"b")]
    [InlineData("a%b")]
    [InlineData("a=b")]
    [InlineData("a#b")]
    [InlineData("a@b")]
    [InlineData("a,b")]
    public void KeyProblemRefusesEveryForbiddenCharacter(string key)
    {
        Assert.NotNull(CitationRename.KeyProblem(key));
    }

    [Fact]
    public void KeyProblemRefusesKeyExceedingMaxBytes()
    {
        string key = new string('a', CitationRename.MaxKeyBytes + 1);
        string? problem = CitationRename.KeyProblem(key);
        Assert.NotNull(problem);
        Assert.Contains((CitationRename.MaxKeyBytes + 1).ToString(), problem);
    }

    [Fact]
    public void KeyProblemAcceptsKeyExactlyAtMaxBytes()
    {
        string key = new string('a', CitationRename.MaxKeyBytes);
        Assert.Null(CitationRename.KeyProblem(key));
    }

    // MARK: Explain

    [Fact]
    public void ExplainMatchesKnownHelperErrorCodeAndKeepsTheOriginalMessage()
    {
        string explanation = CitationRename.Explain("RenameCollision: key already taken");
        Assert.Contains("already exists", explanation);
        Assert.Contains("RenameCollision: key already taken", explanation);
    }

    [Fact]
    public void ExplainFallsBackToRawMessageForUnknownCode()
    {
        Assert.Equal("helper: some unexpected failure", CitationRename.Explain("some unexpected failure"));
    }

    // MARK: BuildRequest

    [Fact]
    public void BuildRequestUsesTypedOldKeyVariantWhenNoSpanIsGiven()
    {
        var request = CitationRename.BuildRequest(
            new Dictionary<string, int> { ["main.tex"] = 1 }, membershipGeneration: 2, maxBytes: 4096,
            newName: "new1", oldName: "old1", span: null);

        Assert.Equal("plan_citation_rename", request.Operation);
        Assert.Equal("old1", request.OldName);
        Assert.Null(request.Path);
        Assert.Null(request.StartByte);
        Assert.Null(request.EndByte);
    }

    [Fact]
    public void BuildRequestUsesAtVariantWhenSpanIsGiven()
    {
        var request = CitationRename.BuildRequest(
            new Dictionary<string, int> { ["main.tex"] = 1 }, membershipGeneration: 2, maxBytes: 4096,
            newName: "new1", oldName: null, span: ("main.tex", 10, 15));

        Assert.Equal("plan_citation_rename_at", request.Operation);
        Assert.Null(request.OldName);
        Assert.Equal("main.tex", request.Path);
        Assert.Equal(10, request.StartByte);
        Assert.Equal(15, request.EndByte);
    }

    [Fact]
    public void BuildRequestDefaultsTypedOldKeyToEmptyStringWhenNull()
    {
        var request = CitationRename.BuildRequest(
            new Dictionary<string, int>(), membershipGeneration: 0, maxBytes: 4096, newName: "new1", oldName: null, span: null);
        Assert.Equal("", request.OldName);
    }

    // MARK: ParsePlan

    private static JsonElement ValidReply(string oldName = "smith2024", string newName = "smith2025", int generation = 7,
        int mainRevision = 3, (string File, int Rev, int Start, int End)[]? edits = null)
    {
        edits ??= [("main.tex", mainRevision, 100, 100 + oldName.Length)];
        var json = $$"""
        {
            "source_versions": { "main.tex": {{mainRevision}} },
            "membership_generation": {{generation}},
            "plan": {
                "schema": "flashtex.citation-rename-plan.v1",
                "kind": "citation_key_rename",
                "proposal_only": true,
                "requires_user_approval": true,
                "application_order": "reverse_byte_offset_per_document",
                "replacement": "{{newName}}",
                "snapshot": {
                    "project_id": "proj-1",
                    "generation": {{generation}},
                    "documents": [ { "file": "main.tex", "revision": {{mainRevision}} } ]
                },
                "rename": { "old_name": "{{oldName}}", "new_name": "{{newName}}" },
                "edits": [
                    {{string.Join(",", edits.Select(e => $$"""
                    { "file": "{{e.File}}", "revision": {{e.Rev}}, "start_byte": {{e.Start}}, "end_byte": {{e.End}},
                      "expected_text": "{{oldName}}", "replacement": "{{newName}}" }
                    """))}}
                ]
            }
        }
        """;
        return JsonDocument.Parse(json).RootElement;
    }

    [Fact]
    public void ParsePlanAcceptsAWellFormedReply()
    {
        var reply = ValidReply();
        var result = CitationRename.ParsePlan(reply, expectedOldName: "smith2024", newName: "smith2025");
        var accepted = Assert.IsType<CitationRename.PlanResult.Accepted>(result);
        Assert.Equal("smith2024", accepted.Plan.OldName);
        Assert.Equal("smith2025", accepted.Plan.NewName);
        Assert.Single(accepted.Plan.Edits);
        Assert.Equal("1 occurrence in 1 file", accepted.Plan.Summary);
    }

    [Fact]
    public void ParsePlanRefusesWhenSchemaIsWrong()
    {
        var json = ValidReply().GetRawText().Replace("flashtex.citation-rename-plan.v1", "some.other.schema");
        var result = CitationRename.ParsePlan(JsonDocument.Parse(json).RootElement, "smith2024", "smith2025");
        var refused = Assert.IsType<CitationRename.PlanResult.Refused>(result);
        Assert.Contains("schema is", refused.Reason);
    }

    [Fact]
    public void ParsePlanRefusesWhenExpectedOldNameDoesNotMatchPlan()
    {
        var result = CitationRename.ParsePlan(ValidReply(), expectedOldName: "otherKey", newName: "smith2025");
        var refused = Assert.IsType<CitationRename.PlanResult.Refused>(result);
        Assert.Contains("not “otherKey”", refused.Reason);
    }

    [Fact]
    public void ParsePlanAcceptsWithNoExpectedOldNameForAtVariant()
    {
        // The `_at` variant resolves the caret's key server-side; the caller passes null.
        var result = CitationRename.ParsePlan(ValidReply(), expectedOldName: null, newName: "smith2025");
        Assert.IsType<CitationRename.PlanResult.Accepted>(result);
    }

    [Fact]
    public void ParsePlanRefusesWhenOldAndNewKeyAreTheSame()
    {
        var reply = ValidReply(oldName: "same", newName: "same");
        var result = CitationRename.ParsePlan(reply, "same", "same");
        var refused = Assert.IsType<CitationRename.PlanResult.Refused>(result);
        Assert.Contains("old and new key are the same", refused.Reason);
    }

    [Fact]
    public void ParsePlanRefusesWhenSnapshotVersionsDifferFromReplyVersions()
    {
        string json = ValidReply().GetRawText().Replace("\"main.tex\": 3", "\"main.tex\": 4");
        // Only the top-level source_versions changed; the snapshot's still says 3 -> mismatch.
        var result = CitationRename.ParsePlan(JsonDocument.Parse(json).RootElement, "smith2024", "smith2025");
        var refused = Assert.IsType<CitationRename.PlanResult.Refused>(result);
        Assert.Contains("differ from the reply's versions", refused.Reason);
    }

    [Fact]
    public void ParsePlanRefusesWhenAnEditNamesARevisionOtherThanTheSnapshots()
    {
        var reply = ValidReply(mainRevision: 3, edits: [("main.tex", 5, 100, 100 + "smith2024".Length)]);
        var result = CitationRename.ParsePlan(reply, "smith2024", "smith2025");
        var refused = Assert.IsType<CitationRename.PlanResult.Refused>(result);
        Assert.Contains("not the snapshot's r3", refused.Reason);
    }

    [Fact]
    public void ParsePlanRefusesWhenAnEditsByteSpanIsTheWrongLengthForTheOldName()
    {
        var reply = ValidReply(edits: [("main.tex", 3, 100, 105)]); // "smith2024" is 9 bytes, not 5
        var result = CitationRename.ParsePlan(reply, "smith2024", "smith2025");
        var refused = Assert.IsType<CitationRename.PlanResult.Refused>(result);
        Assert.Contains("does not rename exactly", refused.Reason);
    }

    [Fact]
    public void ParsePlanRefusesWhenEditsOverlapOrGoBackwardsWithinAFile()
    {
        int len = "smith2024".Length;
        var reply = ValidReply(edits:
        [
            ("main.tex", 3, 100, 100 + len),
            ("main.tex", 3, 100 + len - 1, 100 + 2 * len - 1), // starts before the previous edit's end
        ]);
        var result = CitationRename.ParsePlan(reply, "smith2024", "smith2025");
        var refused = Assert.IsType<CitationRename.PlanResult.Refused>(result);
        Assert.Contains("overlaps or precedes the previous edit", refused.Reason);
    }

    [Fact]
    public void ParsePlanAcceptsMultipleNonOverlappingEditsInDocumentOrder()
    {
        int len = "smith2024".Length;
        var reply = ValidReply(edits:
        [
            ("main.tex", 3, 100, 100 + len),
            ("main.tex", 3, 100 + len + 10, 100 + 2 * len + 10),
        ]);
        var result = CitationRename.ParsePlan(reply, "smith2024", "smith2025");
        var accepted = Assert.IsType<CitationRename.PlanResult.Accepted>(result);
        Assert.Equal(2, accepted.Plan.Edits.Count);
        Assert.Equal("2 occurrences in 1 file", accepted.Plan.Summary);
    }

    [Fact]
    public void ParsePlanRefusesAReplyMissingSourceVersions()
    {
        var result = CitationRename.ParsePlan(JsonDocument.Parse("{}").RootElement, "old", "new");
        var refused = Assert.IsType<CitationRename.PlanResult.Refused>(result);
        Assert.Contains("no source_versions", refused.Reason);
    }
}
