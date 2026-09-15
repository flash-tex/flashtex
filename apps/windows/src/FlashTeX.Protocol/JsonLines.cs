// name: JsonLines.cs
// purpose: Newline-delimited JSON (JSON Lines) framing helpers: encode one JSON
//   value as a single line, decode a stream of lines. Ported from
//   apps/mac/Sources/FlashTeXProtocol/JSONLines.swift. Strictly framing — the
//   process/pipe plumbing that uses this belongs to a separate, not-yet-built
//   FlashTeX.Ipc project.
// author: Claude Sonnet 5
// date: 2026-09-13

using System.Text.Json;
using System.Text.Json.Serialization;

namespace FlashTeX.Protocol
{
    /// <summary>Line-oriented JSON framing: one envelope per line, UTF-8, no embedded newlines.</summary>
    public static class JsonLines
    {
        private const byte NewlineByte = 0x0A;

        /// <summary>Serializes <paramref name="value"/> and appends a single trailing newline.</summary>
        public static byte[] EncodeLine<T>(T value, JsonSerializerOptions options)
        {
            byte[] json = JsonSerializer.SerializeToUtf8Bytes(value, options);
            byte[] line = new byte[json.Length + 1];
            json.CopyTo(line, 0);
            line[^1] = NewlineByte;
            return line;
        }

        /// <summary>Decodes a single complete line (without its trailing newline) as <typeparamref name="T"/>.</summary>
        public static T DecodeLine<T>(ReadOnlySpan<byte> line, JsonSerializerOptions options) =>
            JsonSerializer.Deserialize<T>(line, options) ?? throw new JsonException("line decoded to null");
    }

    /// <summary>
    /// Splits a byte stream into complete lines, keeping a partial trailing line.
    /// Scanning resumes where the previous append stopped, so a long unterminated
    /// line costs O(n) overall rather than O(n^2).
    /// </summary>
    public sealed class LineSplitter
    {
        private readonly List<byte> _buffer = new();
        private int _scanned;

        /// <summary>Appends <paramref name="data"/> and returns every newly completed line (without its trailing newline).</summary>
        public IReadOnlyList<byte[]> Append(ReadOnlySpan<byte> data)
        {
            _buffer.AddRange(data.ToArray());

            var lines = new List<byte[]>();
            int start = 0;
            for (int i = _scanned; i < _buffer.Count; i++)
            {
                if (_buffer[i] == 0x0A)
                {
                    lines.Add(_buffer.GetRange(start, i - start).ToArray());
                    start = i + 1;
                }
            }
            if (start > 0)
            {
                _buffer.RemoveRange(0, start);
            }
            _scanned = _buffer.Count;
            return lines;
        }

        public int PendingBytes => _buffer.Count;
    }
}

namespace FlashTeX.Protocol.RuntimeV1
{
    /// <summary>Envelope header only, for peeking at <c>protocol_version</c>/<c>type</c> before a typed decode.</summary>
    public sealed record EnvelopeHeader(
        [property: JsonPropertyName("protocol_version")] int ProtocolVersion,
        [property: JsonPropertyName("id")] string Id,
        [property: JsonPropertyName("type")] string Type);

    /// <summary>Runtime-v1's own minimal error payload (distinct from transfer-v1's <c>{code,message}</c> shape).</summary>
    public sealed record ErrorPayload([property: JsonPropertyName("message")] string Message);

    /// <summary>Line-framing helpers and constants specific to the runtime-v1 pipe.</summary>
    public static class JsonLinesProtocol
    {
        /// <summary>Default line size limit (bytes) for the runtime-v1 pipe; oversized lines are rejected by callers.</summary>
        public const int MaxLineBytes = 16 * 1024 * 1024;

        public static EnvelopeHeader ReadHeader(ReadOnlySpan<byte> line, JsonSerializerOptions options) =>
            JsonSerializer.Deserialize<EnvelopeHeader>(line, options) ?? throw new JsonException("envelope header decoded to null");

        public static Envelope<CompileRequest> CompileEnvelope(string id, CompileRequest request) =>
            new(Protocol.Version, id, "compile", request);
    }
}
