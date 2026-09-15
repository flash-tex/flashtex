// name: ByteOffsetsCrossLanguageParityTests.cs
// purpose: Proves FlashTeX.Protocol.ByteOffsets (C#) and web/src/byte-offsets.ts
//   (TypeScript, the future WebView2 editor bridge's byte-offset conversion)
//   agree on the same shared fixture vectors in
//   src/FlashTeX.Editor/web/test/byte-offset-vectors.json. Byte-offset parity
//   across that native<->JS boundary is the single most bug-prone seam in the
//   editor integration (per the project's own design notes), so this fixture
//   is authoritative for both languages rather than duplicated by hand.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text.Json;
using FlashTeX.Protocol;
using Xunit;

namespace FlashTeX.Protocol.Tests;

public class ByteOffsetsCrossLanguageParityTests
{
    private static readonly JsonDocument Vectors = LoadVectors();

    private static JsonDocument LoadVectors()
    {
        string path = Path.Combine(AppContext.BaseDirectory, "fixtures", "byte-offset-vectors.json");
        return JsonDocument.Parse(File.ReadAllText(path));
    }

    public static IEnumerable<object[]> RangeVectorNames() =>
        Vectors.RootElement.GetProperty("rangeVectors").EnumerateArray()
            .Select(v => new object[] { v.GetProperty("name").GetString()! });

    public static IEnumerable<object[]> InvalidVectorNames() =>
        Vectors.RootElement.GetProperty("invalidVectors").EnumerateArray()
            .Select(v => new object[] { v.GetProperty("name").GetString()! });

    private static JsonElement FindVector(string collection, string name) =>
        Vectors.RootElement.GetProperty(collection).EnumerateArray()
            .First(v => v.GetProperty("name").GetString() == name);

    [Theory]
    [MemberData(nameof(RangeVectorNames))]
    public void Utf8ByteRangeForUtf16Range_MatchesTheSharedVector(string name)
    {
        JsonElement vector = FindVector("rangeVectors", name);
        string text = vector.GetProperty("text").GetString()!;
        int utf16Start = vector.GetProperty("utf16Start").GetInt32();
        int utf16End = vector.GetProperty("utf16End").GetInt32();
        int expectedUtf8Start = vector.GetProperty("utf8Start").GetInt32();
        int expectedUtf8End = vector.GetProperty("utf8End").GetInt32();

        (int StartByte, int EndByte)? result = ByteOffsets.Utf8ByteRangeForUtf16Range(text, utf16Start, utf16End);

        Assert.NotNull(result);
        Assert.Equal((expectedUtf8Start, expectedUtf8End), result!.Value);
    }

    [Theory]
    [MemberData(nameof(RangeVectorNames))]
    public void Utf16RangeForUtf8Bytes_MatchesTheSharedVector(string name)
    {
        JsonElement vector = FindVector("rangeVectors", name);
        string text = vector.GetProperty("text").GetString()!;
        int utf8Start = vector.GetProperty("utf8Start").GetInt32();
        int utf8End = vector.GetProperty("utf8End").GetInt32();
        int expectedUtf16Start = vector.GetProperty("utf16Start").GetInt32();
        int expectedUtf16End = vector.GetProperty("utf16End").GetInt32();

        (int Utf16Start, int Utf16End)? result = ByteOffsets.Utf16RangeForUtf8Bytes(text, utf8Start, utf8End);

        Assert.NotNull(result);
        Assert.Equal((expectedUtf16Start, expectedUtf16End), result!.Value);
    }

    [Theory]
    [MemberData(nameof(InvalidVectorNames))]
    public void InvalidVectors_AreRejectedInTheDirectionTheyName(string name)
    {
        JsonElement vector = FindVector("invalidVectors", name);
        string text = vector.GetProperty("text").GetString()!;
        string direction = vector.GetProperty("direction").GetString()!;

        switch (direction)
        {
            case "utf8ToUtf16":
                int startByte = vector.GetProperty("startByte").GetInt32();
                int endByte = vector.GetProperty("endByte").GetInt32();
                Assert.Null(ByteOffsets.Utf16RangeForUtf8Bytes(text, startByte, endByte));
                break;
            case "utf16ToUtf8":
                int utf16Start = vector.GetProperty("utf16Start").GetInt32();
                int utf16End = vector.GetProperty("utf16End").GetInt32();
                Assert.Null(ByteOffsets.Utf8ByteRangeForUtf16Range(text, utf16Start, utf16End));
                break;
            default:
                throw new ArgumentOutOfRangeException(nameof(direction), direction, "unknown fixture direction");
        }
    }
}
