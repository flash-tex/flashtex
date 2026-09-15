// name: ProjectIncludes.cs
// purpose: Lexical `\input{path}`/`\include{path}` scanner and rooted-path
//   normalization for multi-file LaTeX projects, ported from
//   apps/mac/Sources/FlashTeXMac/ProjectDocuments.swift's `ProjectIncludes`
//   enum (itself a bounded lexical scan mirroring crates/project-files/src/scan.rs,
//   restricted to the two source-including commands). This is not macro
//   expansion: `\input{\jobname}` is reported as non-literal. Spans are
//   zero-based, end-exclusive UTF-8 byte offsets (runtime-v1 `source`
//   convention); callers convert to editor (UTF-16) coordinates explicitly via
//   FlashTeX.Protocol.ByteOffsets.
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text;
using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.ProjectFiles;

/// <summary>Scans LaTeX source for <c>\input</c>/<c>\include</c> commands, skipping comments, <c>\verb</c> and verbatim-like environments.</summary>
public static class ProjectIncludes
{
    public enum Kind
    {
        Input,
        Include,
    }

    /// <summary>
    /// One <c>\input</c>/<c>\include</c> found in a source text. Spans are
    /// zero-based, end-exclusive UTF-8 byte ranges (runtime-v1 <c>source</c>
    /// convention).
    /// </summary>
    /// <param name="Kind">Which command matched.</param>
    /// <param name="Argument">The referenced name as written (whitespace-trimmed).</param>
    /// <param name="StartByte">From the backslash through the closing brace (or the bare name).</param>
    /// <param name="EndByte">End of the whole command (exclusive).</param>
    /// <param name="ArgumentStartByte">Start of <paramref name="Argument"/> itself.</param>
    /// <param name="ArgumentEndByte">End of <paramref name="Argument"/> itself (exclusive).</param>
    /// <param name="Literal">False when the argument contains <c>\</c> or <c>#</c> and would need macro expansion.</param>
    public sealed record Reference(
        Kind Kind,
        string Argument,
        int StartByte,
        int EndByte,
        int ArgumentStartByte,
        int ArgumentEndByte,
        bool Literal)
    {
        /// <summary>The whole command's span as a runtime-v1 <see cref="SourceRange"/> (an empty <c>Path</c>: the caller knows which document it scanned).</summary>
        public SourceRange SourceRange => new(string.Empty, StartByte, EndByte);
    }

    /// <summary>Upper bound on references reported per document (discovery is bounded; the scan stops once the limit is reached).</summary>
    public const int MaxReferences = 64;

    /// <summary>Direct reads of an included document are bounded like the helper's ledger documents.</summary>
    public const int MaxDocumentBytes = 8 * 1024 * 1024;

    private static readonly HashSet<string> VerbatimEnvironments = new(StringComparer.Ordinal)
    {
        "verbatim", "verbatim*", "comment", "lstlisting", "minted", "Verbatim",
    };

    /// <summary>Scans <paramref name="text"/> for <c>\input</c>/<c>\include</c> in source order.</summary>
    public static IReadOnlyList<Reference> Scan(string text, int limit = MaxReferences)
    {
        var scanner = new Scanner(Encoding.UTF8.GetBytes(text), Math.Max(0, limit));
        scanner.Run();
        return scanner.Found;
    }

    /// <summary>An inclusive-exclusive byte span, e.g. the inside of a braced argument.</summary>
    private readonly record struct ByteSpan(int Start, int End);

    private sealed class Scanner(byte[] bytes, int limit)
    {
        private readonly byte[] _bytes = bytes;
        private int _pos;

        public List<Reference> Found { get; } = [];

        public void Run()
        {
            while (_pos < _bytes.Length && Found.Count < limit)
            {
                switch (_bytes[_pos])
                {
                    case (byte)'%':
                        SkipLine();
                        break;
                    case (byte)'\\':
                        HandleCommand();
                        break;
                    default:
                        _pos++;
                        break;
                }
            }
        }

        private static bool IsAlpha(byte b) => (b >= 0x41 && b <= 0x5A) || (b >= 0x61 && b <= 0x7A);

        private static bool IsSpace(byte b) => b is 0x20 or 0x09 or 0x0A or 0x0D or 0x0C;

        /// <summary>Length of the UTF-8 scalar starting at <paramref name="i"/> (1 for a continuation byte).</summary>
        private int CharLength(int i)
        {
            byte b = _bytes[i];
            if (b < 0x80)
            {
                return 1;
            }
            if ((b & 0xE0) == 0xC0)
            {
                return 2;
            }
            if ((b & 0xF0) == 0xE0)
            {
                return 3;
            }
            return (b & 0xF8) == 0xF0 ? 4 : 1;
        }

        private string Slice(int start, int end) => Encoding.UTF8.GetString(_bytes, start, end - start);

        private void SkipLine()
        {
            while (_pos < _bytes.Length && _bytes[_pos] != (byte)'\n')
            {
                _pos++;
            }
        }

        private void SkipWhitespace()
        {
            while (_pos < _bytes.Length && IsSpace(_bytes[_pos]))
            {
                _pos++;
            }
        }

        private void HandleCommand()
        {
            int start = _pos;
            _pos++;
            int nameStart = _pos;
            while (_pos < _bytes.Length && IsAlpha(_bytes[_pos]))
            {
                _pos++;
            }
            if (_pos == nameStart)
            {
                // Control symbol (`\%`, `\\`, `\{`): skip the symbol character.
                if (_pos < _bytes.Length)
                {
                    _pos += CharLength(_pos);
                }
                return;
            }
            string name = Slice(nameStart, _pos);
            switch (name)
            {
                case "verb":
                    SkipVerb();
                    break;
                case "begin":
                    MaybeSkipVerbatimEnvironment();
                    break;
                case "input":
                    HandleInputOrInclude(Kind.Input, start);
                    break;
                case "include":
                    HandleInputOrInclude(Kind.Include, start);
                    break;
            }
        }

        private void SkipVerb()
        {
            if (_pos < _bytes.Length && _bytes[_pos] == (byte)'*')
            {
                _pos++;
            }
            if (_pos >= _bytes.Length)
            {
                return;
            }
            int delimiterLength = CharLength(_pos);
            byte[] delimiter = _bytes[_pos..Math.Min(_pos + delimiterLength, _bytes.Length)];
            _pos += delimiterLength;
            _pos = Find(delimiter, _pos) is int found ? found + delimiterLength : _bytes.Length;
        }

        private void MaybeSkipVerbatimEnvironment()
        {
            int save = _pos;
            SkipWhitespace();
            if (BracedSpan() is not { } inner)
            {
                _pos = save;
                return;
            }
            string env = Slice(inner.Start, inner.End).Trim();
            if (!VerbatimEnvironments.Contains(env))
            {
                _pos = save;
                return;
            }
            byte[] marker = Encoding.ASCII.GetBytes($"\\end{{{env}}}");
            _pos = Find(marker, _pos) is int found ? found + marker.Length : _bytes.Length;
        }

        private int? Find(byte[] needle, int from)
        {
            if (needle.Length == 0 || needle.Length > _bytes.Length - from)
            {
                return null;
            }
            for (int i = from; i + needle.Length <= _bytes.Length; i++)
            {
                if (_bytes[i] != needle[0])
                {
                    continue;
                }
                bool match = true;
                for (int k = 1; k < needle.Length; k++)
                {
                    if (_bytes[i + k] != needle[k])
                    {
                        match = false;
                        break;
                    }
                }
                if (match)
                {
                    return i;
                }
            }
            return null;
        }

        /// <summary>
        /// If the next byte is <c>{</c>, consumes through the matching <c>}</c> and
        /// returns the inner span. Braces nest; a backslash escapes the next char.
        /// </summary>
        private ByteSpan? BracedSpan()
        {
            if (_pos >= _bytes.Length || _bytes[_pos] != (byte)'{')
            {
                return null;
            }
            int innerStart = _pos + 1;
            int depth = 0;
            int i = _pos;
            while (i < _bytes.Length)
            {
                byte c = _bytes[i];
                if (c == (byte)'\\')
                {
                    i++;
                    if (i < _bytes.Length)
                    {
                        i += CharLength(i);
                    }
                    continue;
                }
                if (c == (byte)'{')
                {
                    depth++;
                }
                else if (c == (byte)'}')
                {
                    depth--;
                    if (depth == 0)
                    {
                        _pos = i + 1;
                        return new ByteSpan(innerStart, i);
                    }
                }
                i++;
            }
            _pos = _bytes.Length; // unbalanced: consume to the end
            return null;
        }

        private void SkipOptionalArgument()
        {
            if (_pos >= _bytes.Length || _bytes[_pos] != (byte)'[')
            {
                return;
            }
            int i = _pos;
            while (i < _bytes.Length && _bytes[i] != (byte)']' && _bytes[i] != (byte)'\n')
            {
                i++;
            }
            if (i < _bytes.Length && _bytes[i] == (byte)']')
            {
                _pos = i + 1;
            }
        }

        private void HandleInputOrInclude(Kind kind, int start)
        {
            int afterName = _pos;
            SkipWhitespace();
            SkipOptionalArgument();
            SkipWhitespace();
            if (BracedSpan() is { } inner)
            {
                Push(kind, inner.Start, inner.End, start, _pos);
                return;
            }
            // Bare `\input name` (TeX primitive form); `\include` requires braces.
            if (kind == Kind.Input)
            {
                int nameStart = _pos;
                while (_pos < _bytes.Length)
                {
                    byte b = _bytes[_pos];
                    if (IsSpace(b) || b is (byte)'\\' or (byte)'%' or (byte)'{' or (byte)'}')
                    {
                        break;
                    }
                    _pos += CharLength(_pos);
                }
                if (_pos > nameStart)
                {
                    Push(kind, nameStart, _pos, start, _pos);
                    return;
                }
            }
            _pos = afterName;
        }

        private void Push(Kind kind, int argumentStart, int argumentEnd, int start, int end)
        {
            int a = argumentStart, b = argumentEnd;
            while (a < b && IsSpace(_bytes[a]))
            {
                a++;
            }
            while (b > a && IsSpace(_bytes[b - 1]))
            {
                b--;
            }
            if (a >= b)
            {
                return;
            }
            string argument = Slice(a, b);
            Found.Add(new Reference(kind, argument, start, end, a, b, !argument.Contains('\\') && !argument.Contains('#')));
        }
    }

    // MARK: rooted paths (mirror of crates/project-files ProjectPath::normalize)

    /// <summary>Thrown by <see cref="Normalize"/> when a raw <c>\input</c>/<c>\include</c> argument is not a valid project-relative path.</summary>
    public sealed class PathFormatException(string message) : Exception(message);

    /// <summary>
    /// Normalizes a project-relative path: forward slashes, no <c>.</c>/empty
    /// segments, <c>..</c> pops (an error when nothing is left), no backslash,
    /// colon, NUL or control characters, no leading <c>/</c> or <c>~</c>.
    /// </summary>
    public static string Normalize(string raw)
    {
        foreach (char c in raw)
        {
            if (c is '\\' or ':' or '\0' || char.IsControl(c))
            {
                throw new PathFormatException($"path contains forbidden character '{c}'");
            }
        }
        if (raw.StartsWith('/') || raw.StartsWith('~'))
        {
            throw new PathFormatException("path must be project-relative, not absolute");
        }
        var segments = new List<string>();
        foreach (string segment in raw.Split('/'))
        {
            switch (segment)
            {
                case "":
                case ".":
                    continue;
                case "..":
                    if (segments.Count == 0)
                    {
                        throw new PathFormatException("path escapes the project root via '..'");
                    }
                    segments.RemoveAt(segments.Count - 1);
                    break;
                default:
                    segments.Add(segment);
                    break;
            }
        }
        if (segments.Count == 0)
        {
            throw new PathFormatException("path is empty");
        }
        return string.Join('/', segments);
    }

    /// <summary>
    /// Rooted candidates for an <c>\input</c>/<c>\include</c> argument, in TeX's
    /// order: <c>name.tex</c> first, then the literal name (a name already
    /// ending in <c>.tex</c> is tried as written). References resolve against
    /// the project root, not the referencing file's directory (TeX
    /// working-directory rule).
    /// </summary>
    public static IReadOnlyList<string> Candidates(string argument)
    {
        string basePath = Normalize(argument);
        int lastSlash = basePath.LastIndexOf('/');
        string name = lastSlash >= 0 ? basePath[(lastSlash + 1)..] : basePath;
        if (name.EndsWith(".tex", StringComparison.Ordinal) && name.Length > 4)
        {
            return [basePath];
        }
        return [basePath + ".tex", basePath];
    }
}
