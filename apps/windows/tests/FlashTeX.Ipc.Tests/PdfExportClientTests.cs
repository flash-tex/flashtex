// name: PdfExportClientTests.cs
// purpose: End-to-end tests of PdfExportClient against the REAL flashtex-pdf and
//   flashtex-pdf-exact binaries (cargo build --release from crates/pdf): a
//   compile_result produced by the real flashtex-compiler round-tripped through
//   `flashtex-pdf --verify`, a checked-in rendering-v2 fixture round-tripped
//   through `flashtex-pdf-exact from-v2` against the Latin Modern font it
//   already vendors under apps/mac/Fonts, and a failed export (an output
//   directory that does not exist) reporting a clear error instead of throwing.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text;
using System.Text.Json;
using FlashTeX.Ipc;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;
using Xunit;
// NOTE: FlashTeX.Protocol.RenderingV2 is deliberately not `using`'d: it
// declares a `Path` record that collides with `System.IO.Path`, used
// throughout this file for temp-file paths. Referenced fully qualified.

namespace FlashTeX.Ipc.Tests;

public class PdfExportClientTests
{
    private const string PdfMagic = "%PDF-";

    [Fact]
    public async Task ExportViaCompileResultAsync_AgainstRealCompilerAndPdfBinaries_ProducesAValidPdf()
    {
        Assert.True(File.Exists(TestPaths.CompilerWorkerExePath),
            $"expected a release build of flashtex-compiler at '{TestPaths.CompilerWorkerExePath}'");
        Assert.True(File.Exists(TestPaths.PdfExePath),
            $"expected a release build of flashtex-pdf at '{TestPaths.PdfExePath}' (run `cargo build --release` from crates/pdf first)");

        CompileResult result = await CompileFixtureAsync();

        string outputPath = Path.Combine(Path.GetTempPath(), $"flashtex-pdf-export-test-{Guid.NewGuid():n}.pdf");
        try
        {
            var client = new PdfExportClient(TestPaths.PdfExePath, TestPaths.PdfExactExePath);
            PdfExportResult export = await client
                .ExportViaCompileResultAsync(result, outputPath, verify: true)
                .WaitAsync(TimeSpan.FromSeconds(30));

            Assert.True(export.Succeeded, $"expected a successful export; notes: {export.ErrorMessage}");
            AssertIsNonEmptyPdf(outputPath);
        }
        finally
        {
            File.Delete(outputPath);
        }
    }

    [Fact]
    public async Task ExportViaCompileResultAsync_WhenOutputDirectoryDoesNotExist_ReportsAClearErrorInsteadOfThrowing()
    {
        Assert.True(File.Exists(TestPaths.PdfExePath),
            $"expected a release build of flashtex-pdf at '{TestPaths.PdfExePath}'");

        CompileResult result = MinimalCompileResult();
        string missingDirectory = Path.Combine(Path.GetTempPath(), $"flashtex-does-not-exist-{Guid.NewGuid():n}");
        string outputPath = Path.Combine(missingDirectory, "out.pdf");

        var client = new PdfExportClient(TestPaths.PdfExePath, TestPaths.PdfExactExePath);
        PdfExportResult export = await client
            .ExportViaCompileResultAsync(result, outputPath, verify: false)
            .WaitAsync(TimeSpan.FromSeconds(30));

        Assert.False(export.Succeeded);
        Assert.False(string.IsNullOrWhiteSpace(export.ErrorMessage));
        Assert.False(File.Exists(outputPath));
    }

    [Fact]
    public async Task ExportExactFromDisplayListAsync_AgainstTheCheckedInFixtureAndBundledFont_ProducesAValidPdf()
    {
        Assert.True(File.Exists(TestPaths.PdfExactExePath),
            $"expected a release build of flashtex-pdf-exact at '{TestPaths.PdfExactExePath}' (run `cargo build --release` from crates/pdf first)");

        string fixturePath = Path.Combine(TestPaths.RepoRoot, "crates", "pdf", "tests", "fixtures", "v2-text-a-b.json");
        Assert.True(File.Exists(fixturePath), $"expected the checked-in fixture at '{fixturePath}'");

        // The fixture's single font (LMRoman12-Regular, e6be218a...) is vendored
        // byte-identically under apps/mac/Fonts (crates/pdf resolves by content hash).
        string fontDirectory = Path.Combine(TestPaths.RepoRoot, "apps", "mac", "Fonts");
        Assert.True(
            File.Exists(Path.Combine(fontDirectory, "lmroman12-regular.otf")),
            $"expected the vendored Latin Modern font under '{fontDirectory}'");

        byte[] fixtureJson = await File.ReadAllBytesAsync(fixturePath);
        FlashTeX.Protocol.RenderingV2.Envelope envelope =
            JsonSerializer.Deserialize<FlashTeX.Protocol.RenderingV2.Envelope>(fixtureJson, FlashTeXJson.Options)
            ?? throw new InvalidOperationException("fixture decoded to null");

        string outputPath = Path.Combine(Path.GetTempPath(), $"flashtex-pdf-exact-export-test-{Guid.NewGuid():n}.pdf");
        try
        {
            var client = new PdfExportClient(TestPaths.PdfExePath, TestPaths.PdfExactExePath);
            PdfExportResult export = await client
                .ExportExactFromDisplayListAsync(envelope.Payload, outputPath, fontDirectory)
                .WaitAsync(TimeSpan.FromSeconds(30));

            Assert.True(export.Succeeded, $"expected a successful export; notes: {export.ErrorMessage}");
            AssertIsNonEmptyPdf(outputPath);
        }
        finally
        {
            File.Delete(outputPath);
        }
    }

    private static async Task<CompileResult> CompileFixtureAsync()
    {
        string fixtureJson = await File.ReadAllTextAsync(
            Path.Combine(TestPaths.RepoRoot, "protocol", "fixtures", "compile-request.json"));
        Envelope<CompileRequest> fixture = JsonSerializer
            .Deserialize<Envelope<CompileRequest>>(fixtureJson, FlashTeXJson.Options)
            ?? throw new InvalidOperationException("compile-request fixture decoded to null");

        await using WorkerClient worker = WorkerClient.Start(TestPaths.CompilerWorkerExePath, _ => { });
        return await worker.CompileAsync(fixture.Id, fixture.Payload).WaitAsync(TimeSpan.FromSeconds(30));
    }

    private static CompileResult MinimalCompileResult()
    {
        var item = new PageItem.OfText(new PageItem.TextItem("Hi", 72, 700, 12, source: null));
        var page = new Page(1, 612, 792, new[] { (PageItem)item });
        return new CompileResult("proj", 1, Status.ok, new[] { page }, Array.Empty<Diagnostic>(), pdfPath: null);
    }

    private static void AssertIsNonEmptyPdf(string path)
    {
        Assert.True(File.Exists(path), $"expected an output file at '{path}'");
        byte[] bytes = File.ReadAllBytes(path);
        Assert.True(bytes.Length > 0, "expected a non-empty PDF");
        string magic = Encoding.ASCII.GetString(bytes, 0, Math.Min(PdfMagic.Length, bytes.Length));
        Assert.Equal(PdfMagic, magic);
    }
}
