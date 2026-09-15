// name: BridgeClientTests.cs
// purpose: End-to-end test of BridgeClient against the REAL flashtex-bridge
//   binary (cargo build --release from crates/bridge), driving the documented
//   document_open -> destination_pin -> capture_submit sequence (mirrored from
//   crates/bridge/tests/cli.rs) against the actual Windows engine process.
// author: Claude Sonnet 5
// date: 2026-09-13

using FlashTeX.Ipc;
using FlashTeX.Protocol.TransferV1;
using Xunit;

namespace FlashTeX.Ipc.Tests;

public class BridgeClientTests
{
    [Fact]
    public async Task DocumentOpen_DestinationPin_CaptureSubmit_RoundTripsAgainstTheRealBridgeBinary()
    {
        Assert.True(
            File.Exists(TestPaths.BridgeExePath),
            $"expected a release build of flashtex-bridge at '{TestPaths.BridgeExePath}' " +
            "(run `cargo build --release` from crates/bridge first)");

        DirectoryInfo store = Directory.CreateTempSubdirectory("flashtex-bridge-test-");
        try
        {
            var stderrLines = new List<string>();
            await using BridgeClient bridge = BridgeClient.Start(
                TestPaths.BridgeExePath, store.FullName, line => stderrLines.Add(line));

            await bridge
                .DocumentOpenAsync("open", new DocumentOpen("p", "main.tex", 1, "alpha"))
                .WaitAsync(TimeSpan.FromSeconds(30));

            Anchor anchor = await bridge
                .DestinationPinAsync("pin", new DestinationPin("fixture-anchor-1", "p", "main.tex", 1, 2, 2))
                .WaitAsync(TimeSpan.FromSeconds(30));
            Assert.True(anchor.IsValid, $"expected a valid anchor; stderr: {string.Join('\n', stderrLines)}");
            Assert.Equal("fixture-anchor-1", anchor.DestinationId);

            var image = new FlashTeX.Protocol.RuntimeV1.CaptureImage(
                "image/png",
                // A minimal 1x1 transparent PNG, matching protocol/fixtures/capture-submission.json.
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4//8/AAX+Av4N70a4AAAAAElFTkSuQmCC");
            var submit = new FlashTeX.Protocol.RuntimeV1.CaptureSubmit(
                "capture-1", "fixture-anchor-1", 1, image, "Faithfully transcribe the selected handwriting.");

            FlashTeX.Protocol.TransferV1.CaptureReceived received = await bridge
                .CaptureSubmitAsync("submit", submit)
                .WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal("capture-1", received.CaptureId);
            Assert.True(received.Durable);

            CaptureStatus status = await bridge
                .CaptureStatusAsync("status", new CaptureId("capture-1"))
                .WaitAsync(TimeSpan.FromSeconds(30));
            Assert.Equal("capture-1", status.CaptureId);
            Assert.False(status.Rejected);
        }
        finally
        {
            store.Delete(recursive: true);
        }
    }

    [Fact]
    public async Task CaptureStatus_ForAnUnknownCapture_ReturnsABridgeError()
    {
        Assert.True(File.Exists(TestPaths.BridgeExePath));

        DirectoryInfo store = Directory.CreateTempSubdirectory("flashtex-bridge-test-");
        try
        {
            await using BridgeClient bridge = BridgeClient.Start(TestPaths.BridgeExePath, store.FullName, _ => { });

            BridgeErrorException ex = await Assert.ThrowsAsync<BridgeErrorException>(async () =>
                await bridge.CaptureStatusAsync("status-unknown", new CaptureId("never-submitted"))
                    .WaitAsync(TimeSpan.FromSeconds(30)));
            Assert.Equal("status-unknown", ex.RequestId);
        }
        finally
        {
            store.Delete(recursive: true);
        }
    }
}
