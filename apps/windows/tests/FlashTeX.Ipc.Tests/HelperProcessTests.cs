// name: HelperProcessTests.cs
// purpose: End-to-end tests for FlashTeX.Ipc.HelperProcess against a real
//   child process (a tiny inline PowerShell script standing in for a Rust
//   helper), covering request/reply correlation by `id`, unsolicited frames,
//   process-exit fault propagation, and the oversized-line hard failure.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Text.Json;
using System.Text.Json.Serialization;
using FlashTeX.Ipc;
using Xunit;

namespace FlashTeX.Ipc.Tests;

public sealed record EchoRequest(
    [property: JsonPropertyName("id")] string Id,
    [property: JsonPropertyName("type")] string Type,
    [property: JsonPropertyName("payload")] string Payload);

public class HelperProcessTests
{
    private static readonly JsonSerializerOptions JsonOptions = new(JsonSerializerDefaults.Web);

    /// <summary>
    /// A trivial line-oriented child: echoes every stdin line back to stdout
    /// unchanged, standing in for a real Rust helper's request/reply behavior
    /// without depending on one being built for this test run.
    /// </summary>
    private static HelperProcessOptions EchoOptions(int maxLineBytes = 16 * 1024 * 1024) => new()
    {
        ExecutablePath = "powershell.exe",
        Arguments =
        [
            "-NoProfile", "-NonInteractive", "-Command",
            "$line = [Console]::In.ReadLine(); while ($null -ne $line) { [Console]::Out.WriteLine($line); [Console]::Out.Flush(); $line = [Console]::In.ReadLine() }",
        ],
        MaxLineBytes = maxLineBytes,
    };

    [Fact]
    public async Task SendAsync_CorrelatesReplyByMatchingId()
    {
        await using HelperProcess helper = HelperProcess.Start(EchoOptions(), _ => { });

        JsonDocument reply = await helper.SendAsync(
            "req-1",
            new EchoRequest("req-1", "ping", "hello"),
            JsonOptions).WaitAsync(TimeSpan.FromSeconds(10));

        Assert.Equal("req-1", reply.RootElement.GetProperty("id").GetString());
        Assert.Equal("hello", reply.RootElement.GetProperty("payload").GetString());
    }

    [Fact]
    public async Task SendAsync_RunsConcurrentRequestsWithoutCrossingReplies()
    {
        await using HelperProcess helper = HelperProcess.Start(EchoOptions(), _ => { });

        Task<JsonDocument> first = helper.SendAsync("a", new EchoRequest("a", "ping", "first"), JsonOptions);
        Task<JsonDocument> second = helper.SendAsync("b", new EchoRequest("b", "ping", "second"), JsonOptions);

        JsonDocument[] replies = await Task.WhenAll(first, second).WaitAsync(TimeSpan.FromSeconds(10));

        Assert.Equal("first", replies[0].RootElement.GetProperty("payload").GetString());
        Assert.Equal("second", replies[1].RootElement.GetProperty("payload").GetString());
    }

    [Fact]
    public async Task NullIdLine_IsPublishedOnUnsolicitedChannelNotMistakenForAReply()
    {
        // A child that speaks first with an id:null frame (mirroring
        // preview-controller's unprompted `update` frames) before this test
        // ever calls SendAsync, then behaves like the ordinary echo child.
        var options = new HelperProcessOptions
        {
            ExecutablePath = "powershell.exe",
            Arguments =
            [
                "-NoProfile", "-NonInteractive", "-Command",
                "[Console]::Out.WriteLine('{\"id\":null,\"type\":\"update\",\"payload\":\"server-pushed\"}'); " +
                "[Console]::Out.Flush(); " +
                "$line = [Console]::In.ReadLine(); while ($null -ne $line) { [Console]::Out.WriteLine($line); [Console]::Out.Flush(); $line = [Console]::In.ReadLine() }",
            ],
        };
        await using HelperProcess helper = HelperProcess.Start(options, _ => { });

        JsonDocument unsolicited = await helper.UnsolicitedLines.ReadAsync().AsTask().WaitAsync(TimeSpan.FromSeconds(10));
        Assert.Equal(JsonValueKind.Null, unsolicited.RootElement.GetProperty("id").ValueKind);
        Assert.Equal("server-pushed", unsolicited.RootElement.GetProperty("payload").GetString());

        // The channel carries only that one frame; a subsequent correlated
        // request still completes normally through the pending-request path.
        JsonDocument reply = await helper.SendAsync(
            "req-2", new EchoRequest("req-2", "ping", "after-unsolicited"), JsonOptions)
            .WaitAsync(TimeSpan.FromSeconds(10));
        Assert.Equal("after-unsolicited", reply.RootElement.GetProperty("payload").GetString());
    }

    [Fact]
    public async Task ProcessExit_FaultsAPendingRequest()
    {
        var options = new HelperProcessOptions
        {
            ExecutablePath = "powershell.exe",
            Arguments = ["-NoProfile", "-NonInteractive", "-Command", "exit 1"],
        };
        await using HelperProcess helper = HelperProcess.Start(options, _ => { });

        await Assert.ThrowsAsync<HelperProcessExitedException>(async () =>
            await helper.SendAsync("never-replied", new EchoRequest("never-replied", "ping", "x"), JsonOptions)
                .WaitAsync(TimeSpan.FromSeconds(10)));
    }

    [Fact]
    public async Task OversizedOutgoingLine_ThrowsWithoutWritingToTheChild()
    {
        await using HelperProcess helper = HelperProcess.Start(EchoOptions(maxLineBytes: 16), _ => { });

        await Assert.ThrowsAsync<HelperProtocolException>(async () =>
            await helper.SendAsync(
                "too-big",
                new EchoRequest("too-big", "ping", new string('x', 1024)),
                JsonOptions));
    }
}
