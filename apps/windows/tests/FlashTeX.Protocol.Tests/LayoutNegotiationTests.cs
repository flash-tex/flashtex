// name: LayoutNegotiationTests.cs
// purpose: Tests for the layout-capability list validation bounds
//   (docs/contracts/runtime-v1-layout-capabilities.md): at most 16 entries, each at
//   most 64 UTF-8 bytes, nonempty, and duplicate-free.
// author: Claude Sonnet 5
// date: 2026-09-13

using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Protocol.Tests;

public class LayoutNegotiationTests
{
    [Fact]
    public void ValidCapabilityListPassesWithoutThrowing()
    {
        var exception = Record.Exception(() => LayoutCapabilities.Validate(new[] { LayoutCapabilities.RulesV1, LayoutCapabilities.FontHintsV1 }));
        Assert.Null(exception);
    }

    [Fact]
    public void EmptyCapabilityListPassesWithoutThrowing()
    {
        var exception = Record.Exception(() => LayoutCapabilities.Validate(Array.Empty<string>()));
        Assert.Null(exception);
    }

    [Fact]
    public void MoreThanSixteenEntriesIsRejected()
    {
        var capabilities = Enumerable.Range(0, LayoutCapabilities.MaxCount + 1).Select(i => $"cap-{i}").ToArray();
        Assert.Throws<RuntimeV1DecodeException>(() => LayoutCapabilities.Validate(capabilities));
    }

    [Fact]
    public void ExactlySixteenEntriesIsAccepted()
    {
        var capabilities = Enumerable.Range(0, LayoutCapabilities.MaxCount).Select(i => $"cap-{i}").ToArray();
        var exception = Record.Exception(() => LayoutCapabilities.Validate(capabilities));
        Assert.Null(exception);
    }

    [Fact]
    public void EntryLongerThanSixtyFourUtf8BytesIsRejected()
    {
        string tooLong = new('a', LayoutCapabilities.MaxBytes + 1);
        Assert.Throws<RuntimeV1DecodeException>(() => LayoutCapabilities.Validate(new[] { tooLong }));
    }

    [Fact]
    public void EntryOfExactlySixtyFourUtf8BytesIsAccepted()
    {
        string exact = new('a', LayoutCapabilities.MaxBytes);
        var exception = Record.Exception(() => LayoutCapabilities.Validate(new[] { exact }));
        Assert.Null(exception);
    }

    [Fact]
    public void MultiByteEntryIsMeasuredInUtf8BytesNotChars()
    {
        // "é" is 1 UTF-16 char but 2 UTF-8 bytes, so 33 of them are 66 bytes (> 64).
        string tooLongInBytes = new('é', 33);
        Assert.Throws<RuntimeV1DecodeException>(() => LayoutCapabilities.Validate(new[] { tooLongInBytes }));
    }

    [Fact]
    public void EmptyCapabilityStringIsRejected()
    {
        Assert.Throws<RuntimeV1DecodeException>(() => LayoutCapabilities.Validate(new[] { "" }));
    }

    [Fact]
    public void DuplicateCapabilityIsRejected()
    {
        Assert.Throws<RuntimeV1DecodeException>(() => LayoutCapabilities.Validate(new[] { LayoutCapabilities.RulesV1, LayoutCapabilities.RulesV1 }));
    }

    [Fact]
    public void CompileRequestConstructorEnforcesTheSameBounds()
    {
        var documents = new[] { new Document("main.tex", "hello") };
        var tooMany = Enumerable.Range(0, LayoutCapabilities.MaxCount + 1).Select(i => $"cap-{i}").ToArray();
        Assert.Throws<RuntimeV1DecodeException>(() => new CompileRequest("p", 1, "main.tex", documents, tooMany));
    }
}
