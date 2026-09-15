// name: PdfExportClient.cs
// purpose: One-shot process wrapper around the crates/pdf CLI tools
//   (flashtex-pdf, flashtex-pdf-exact), which are simple invoke-wait-read
//   command-line programs, not long-running JSON-Lines helpers. Turns a
//   runtime-v1 compile_result or a rendering-v2 display_list into a PDF file.
//   Every export writes to a sibling temp file first and publishes it to the
//   destination only after the tool exits 0 and the temp file is non-empty,
//   porting the durability property of apps/mac's ExportSession/RustPDFExport
//   (never leave a partial or corrupt PDF at the destination). See
//   crates/pdf/README.md for the exact CLI usage this wraps.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.ComponentModel;
using System.Diagnostics;
using System.Text;
using System.Text.Json;
using FlashTeX.Protocol;
using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Ipc;

// NOTE: FlashTeX.Protocol.RenderingV2 is deliberately not `using`'d here: it
// declares a `Path` record that collides with `System.IO.Path`, used heavily
// below for temp-file/sibling-path logic. RenderingV2 types are referenced
// fully qualified instead.

/// <summary>Outcome of one PDF export attempt.</summary>
public sealed record PdfExportResult
{
    private PdfExportResult(bool succeeded, string? errorMessage, string notes)
    {
        Succeeded = succeeded;
        ErrorMessage = errorMessage;
        Notes = notes;
    }

    /// <summary>True when the tool exited 0 and a non-empty file was published at the requested path.</summary>
    public bool Succeeded { get; }

    /// <summary>Populated only when <see cref="Succeeded"/> is false: a clear, user-facing failure message.</summary>
    public string? ErrorMessage { get; }

    /// <summary>The tool's combined stderr+stdout, trimmed. Non-empty on success can mean warnings were reported.</summary>
    public string Notes { get; }

    public static PdfExportResult Ok(string notes) => new(succeeded: true, errorMessage: null, notes);

    public static PdfExportResult Failed(string errorMessage) => new(succeeded: false, errorMessage, notes: errorMessage);
}

/// <summary>
/// Runs the one-shot `flashtex-pdf`/`flashtex-pdf-exact` CLI tools (crates/pdf)
/// against a real compile result or rendering-v2 display list. Each instance is
/// bound to the two executable paths (parallel to how <see cref="WorkerClient.Start"/>
/// takes an executable path); unlike <see cref="WorkerClient"/>/<see cref="BridgeClient"/>/
/// <see cref="EditLedgerClient"/> there is no persistent child process or JSON-Lines
/// framing here — every call spawns, feeds input, waits for exit, and reads the file
/// the tool wrote.
/// </summary>
public sealed class PdfExportClient
{
    private const int TimeoutSeconds = 60;
    private const string CompileResultEnvelopeId = "windows-pdf-export";
    private const string DisplayListEnvelopeId = "windows-pdf-export-exact";

    private static readonly Encoding Utf8NoBom = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false);

    private readonly string _flashTeXPdfPath;
    private readonly string _flashTeXPdfExactPath;

    /// <param name="flashTeXPdfExecutablePath">Path to the built `flashtex-pdf` binary.</param>
    /// <param name="flashTeXPdfExactExecutablePath">Path to the built `flashtex-pdf-exact` binary.</param>
    public PdfExportClient(string flashTeXPdfExecutablePath, string flashTeXPdfExactExecutablePath)
    {
        _flashTeXPdfPath = flashTeXPdfExecutablePath;
        _flashTeXPdfExactPath = flashTeXPdfExactExecutablePath;
    }

    /// <summary>
    /// Exports a runtime-v1 <c>compile_result</c> via the original Rust writer:
    /// `flashtex-pdf --out &lt;file&gt; [--verify]`, fed via stdin (crates/pdf/README.md
    /// confirms the input file argument may be omitted in favor of stdin). Always
    /// produces a white page, independent of any preview theme.
    /// </summary>
    public async Task<PdfExportResult> ExportViaCompileResultAsync(
        CompileResult result, string outputPath, bool verify, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(result);
        ArgumentException.ThrowIfNullOrWhiteSpace(outputPath);

        var envelope = new Envelope<CompileResult>(
            FlashTeX.Protocol.RuntimeV1.Protocol.Version, CompileResultEnvelopeId, "compile_result", result);
        byte[] input = JsonSerializer.SerializeToUtf8Bytes(envelope, FlashTeXJson.Options);

        string tempOutput = TemporarySibling(outputPath);
        var arguments = new List<string> { "--out", tempOutput };
        if (verify)
        {
            arguments.Add("--verify");
        }

        return await RunAndPublishAsync(_flashTeXPdfPath, arguments, input, tempOutput, outputPath, cancellationToken)
            .ConfigureAwait(false);
    }

    /// <summary>
    /// Exports a rendering-v2 display list via the exact writer: `flashtex-pdf-exact
    /// from-v2 &lt;file&gt; --out &lt;file&gt; [--font-dir DIR]`. The list is written to a
    /// temp file first (the README's `from-v2` subcommand takes it as a positional
    /// path, not stdin). <paramref name="fontDirectory"/> is resolved by the tool
    /// alongside its own defaults (`FLASHTEX_FONT_DIRS`, `FLASHTEX_LM_DIR`, the TeX
    /// Live Latin Modern directories); a real Windows caller should pass a directory
    /// that actually holds the fonts referenced by content hash (e.g. a bundled Fonts
    /// folder analogous to apps/mac/Fonts, once one exists on this platform) — pass
    /// null to rely solely on the tool's own discovery.
    /// </summary>
    public async Task<PdfExportResult> ExportExactFromDisplayListAsync(
        FlashTeX.Protocol.RenderingV2.DisplayList displayList, string outputPath, string? fontDirectory, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(displayList);
        ArgumentException.ThrowIfNullOrWhiteSpace(outputPath);

        string listPath = Path.Combine(Path.GetTempPath(), $"flashtex-display-list-{Guid.NewGuid():n}.json");
        try
        {
            var envelope = new FlashTeX.Protocol.RenderingV2.Envelope(DisplayListEnvelopeId, displayList);
            byte[] listJson = JsonSerializer.SerializeToUtf8Bytes(envelope, FlashTeXJson.Options);
            await File.WriteAllBytesAsync(listPath, listJson, cancellationToken).ConfigureAwait(false);

            string tempOutput = TemporarySibling(outputPath);
            var arguments = new List<string> { "from-v2", listPath, "--out", tempOutput };
            if (!string.IsNullOrEmpty(fontDirectory))
            {
                arguments.Add("--font-dir");
                arguments.Add(fontDirectory);
            }

            return await RunAndPublishAsync(_flashTeXPdfExactPath, arguments, stdinInput: null, tempOutput, outputPath, cancellationToken)
                .ConfigureAwait(false);
        }
        finally
        {
            DeleteQuietly(listPath);
        }
    }

    /// <summary>
    /// Runs the tool, and — only on a zero exit with a non-empty temp file — moves
    /// it over <paramref name="finalOutputPath"/>. `File.Move(overwrite: true)` already
    /// uses an atomic rename on Windows, so no destination is ever left partially
    /// written; any other outcome deletes the temp file and reports a clear message
    /// instead of throwing.
    /// </summary>
    private static async Task<PdfExportResult> RunAndPublishAsync(
        string executablePath,
        IReadOnlyList<string> arguments,
        byte[]? stdinInput,
        string tempOutputPath,
        string finalOutputPath,
        CancellationToken cancellationToken)
    {
        using var timeoutSource = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken);
        timeoutSource.CancelAfter(TimeSpan.FromSeconds(TimeoutSeconds));

        try
        {
            ProcessRunResult run = await RunProcessAsync(executablePath, arguments, stdinInput, timeoutSource.Token)
                .ConfigureAwait(false);
            return Publish(executablePath, run, tempOutputPath, finalOutputPath);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            DeleteQuietly(tempOutputPath);
            throw;
        }
        catch (OperationCanceledException)
        {
            DeleteQuietly(tempOutputPath);
            return PdfExportResult.Failed(
                $"{Path.GetFileName(executablePath)} did not finish within {TimeoutSeconds} s; terminated");
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or Win32Exception)
        {
            DeleteQuietly(tempOutputPath);
            return PdfExportResult.Failed($"could not run {Path.GetFileName(executablePath)}: {ex.Message}");
        }
    }

    private static PdfExportResult Publish(string executablePath, ProcessRunResult run, string tempOutputPath, string finalOutputPath)
    {
        string notes = (run.StandardError + run.StandardOutput).Trim();
        string toolName = Path.GetFileName(executablePath);
        if (run.ExitCode != 0)
        {
            DeleteQuietly(tempOutputPath);
            return PdfExportResult.Failed($"{toolName} exited {run.ExitCode}: {(notes.Length > 0 ? notes : "(no message)")}");
        }

        var tempInfo = new FileInfo(tempOutputPath);
        if (!tempInfo.Exists || tempInfo.Length == 0)
        {
            DeleteQuietly(tempOutputPath);
            return PdfExportResult.Failed($"{toolName} exited 0 but produced no output at '{tempOutputPath}'");
        }

        File.Move(tempOutputPath, finalOutputPath, overwrite: true);
        return PdfExportResult.Ok(notes);
    }

    /// <summary>Spawns the tool, drains both pipes concurrently, and returns its exit code and output.</summary>
    private static async Task<ProcessRunResult> RunProcessAsync(
        string executablePath, IReadOnlyList<string> arguments, byte[]? stdinInput, CancellationToken cancellationToken)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = executablePath,
            RedirectStandardInput = stdinInput is not null,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
            StandardOutputEncoding = Utf8NoBom,
            StandardErrorEncoding = Utf8NoBom,
        };
        foreach (string argument in arguments)
        {
            startInfo.ArgumentList.Add(argument);
        }

        using var process = new Process { StartInfo = startInfo };
        process.Start();
        await using CancellationTokenRegistration registration = cancellationToken.Register(
            static state => KillIfRunning((Process)state!), process);

        Task<string> stdoutTask = process.StandardOutput.ReadToEndAsync(cancellationToken);
        Task<string> stderrTask = process.StandardError.ReadToEndAsync(cancellationToken);
        if (stdinInput is not null)
        {
            await process.StandardInput.BaseStream.WriteAsync(stdinInput, cancellationToken).ConfigureAwait(false);
            process.StandardInput.Close();
        }

        await process.WaitForExitAsync(cancellationToken).ConfigureAwait(false);
        string stdout = await stdoutTask.ConfigureAwait(false);
        string stderr = await stderrTask.ConfigureAwait(false);
        return new ProcessRunResult(process.ExitCode, stdout, stderr);
    }

    private static void KillIfRunning(Process process)
    {
        try
        {
            if (!process.HasExited)
            {
                process.Kill(entireProcessTree: true);
            }
        }
        catch (InvalidOperationException)
        {
            // Already exited between the check and the kill; nothing to do.
        }
    }

    /// <summary>A sibling of <paramref name="path"/> in the same directory (so the final move is atomic on the same volume).</summary>
    private static string TemporarySibling(string path)
    {
        string? directory = Path.GetDirectoryName(path);
        string tempName = $".{Path.GetFileName(path)}.flashtex-export-{Guid.NewGuid():n}.tmp";
        return string.IsNullOrEmpty(directory) ? tempName : Path.Combine(directory, tempName);
    }

    private static void DeleteQuietly(string path)
    {
        try
        {
            if (File.Exists(path))
            {
                File.Delete(path);
            }
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // Best-effort cleanup only; the caller already has the primary failure to report.
        }
    }

    private readonly record struct ProcessRunResult(int ExitCode, string StandardOutput, string StandardError);
}
