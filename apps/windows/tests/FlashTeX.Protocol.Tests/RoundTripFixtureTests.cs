// name: RoundTripFixtureTests.cs
// purpose: Round-trip tests against the three shared wire-format fixtures in
//   protocol/fixtures/ — the primary correctness gate for this port. Each fixture
//   is deserialized into its DTO, re-serialized, and checked for semantic (not
//   textual) equivalence against the original, then deserialized again to confirm
//   the second hop reaches an equal object.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Text.Json;
using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Protocol.Tests;

public class RoundTripFixtureTests
{
    private static string FixturePath(string fileName) => Path.Combine(AppContext.BaseDirectory, "fixtures", fileName);

    [Fact]
    public void CompileRequestFixtureRoundTrips() => AssertRoundTrips<Envelope<CompileRequest>>("compile-request.json");

    [Fact]
    public void CompileResultFixtureRoundTrips() => AssertRoundTrips<Envelope<CompileResult>>("compile-result.json");

    [Fact]
    public void CaptureSubmissionFixtureRoundTrips() => AssertRoundTrips<Envelope<CaptureSubmit>>("capture-submission.json");

    private static void AssertRoundTrips<T>(string fileName)
    {
        string path = FixturePath(fileName);
        Assert.True(File.Exists(path), $"fixture not found at {path}");
        byte[] originalBytes = File.ReadAllBytes(path);

        var value = JsonSerializer.Deserialize<T>(originalBytes, FlashTeXJson.Options);
        Assert.NotNull(value);

        byte[] roundTripped = JsonSerializer.SerializeToUtf8Bytes(value, FlashTeXJson.Options);
        AssertJsonSemanticEquality(originalBytes, roundTripped);

        // deserialize -> serialize -> deserialize -> serialize is a stronger
        // guarantee than one hop: it also catches a converter that is lossy in only
        // one direction. The comparison is JSON-semantic rather than
        // Assert.Equal(value, second): most DTOs here store list-typed properties
        // as IReadOnlyList<T>, and the record-synthesized Equals falls back to
        // reference equality for that interface type (List<T> does not implement
        // IEquatable<T>), so two structurally identical instances built from
        // separate deserializations would otherwise compare unequal for reasons
        // unrelated to wire correctness.
        var second = JsonSerializer.Deserialize<T>(roundTripped, FlashTeXJson.Options);
        Assert.NotNull(second);
        byte[] roundTrippedAgain = JsonSerializer.SerializeToUtf8Bytes(second, FlashTeXJson.Options);
        AssertJsonSemanticEquality(roundTripped, roundTrippedAgain);
    }

    private static void AssertJsonSemanticEquality(byte[] originalBytes, byte[] roundTripped)
    {
        using var originalDocument = JsonDocument.Parse(originalBytes);
        using var roundTrippedDocument = JsonDocument.Parse(roundTripped);
        bool equal = JsonElementDeepEquals(originalDocument.RootElement, roundTrippedDocument.RootElement);
        Assert.True(
            equal,
            "round-tripped JSON differs from the original fixture."
                + $"\nOriginal:      {originalDocument.RootElement.GetRawText()}"
                + $"\nRound-tripped: {roundTrippedDocument.RootElement.GetRawText()}");
    }

    /// <summary>Structural JSON equality: key order and whitespace never matter.</summary>
    private static bool JsonElementDeepEquals(JsonElement a, JsonElement b)
    {
        if (a.ValueKind != b.ValueKind)
        {
            return false;
        }
        return a.ValueKind switch
        {
            JsonValueKind.Object => ObjectsDeepEqual(a, b),
            JsonValueKind.Array => ArraysDeepEqual(a, b),
            JsonValueKind.String => a.GetString() == b.GetString(),
            JsonValueKind.Number => NumbersDeepEqual(a, b),
            _ => true, // true/false/null: ValueKind equality above already decided it
        };
    }

    private static bool ObjectsDeepEqual(JsonElement a, JsonElement b)
    {
        var aProps = a.EnumerateObject().ToList();
        var bByName = b.EnumerateObject().ToDictionary(p => p.Name, p => p.Value);
        return aProps.Count == bByName.Count
            && aProps.All(p => bByName.TryGetValue(p.Name, out var bv) && JsonElementDeepEquals(p.Value, bv));
    }

    private static bool ArraysDeepEqual(JsonElement a, JsonElement b)
    {
        var aItems = a.EnumerateArray().ToList();
        var bItems = b.EnumerateArray().ToList();
        return aItems.Count == bItems.Count && aItems.Zip(bItems, JsonElementDeepEquals).All(matched => matched);
    }

    private static bool NumbersDeepEqual(JsonElement a, JsonElement b) =>
        a.GetRawText() == b.GetRawText() || a.GetDouble() == b.GetDouble();
}
