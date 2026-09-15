// name: DocumentStatistics.cs
// purpose: A texcount-style word/statistics scan of one buffer: words in body
//   text, in section/chapter headers, and in `\caption{…}` arguments, plus the
//   number of inline and display math blocks. Ported from
//   apps/mac/Sources/FlashTeXMac/DocumentStatistics.swift. Comments,
//   verbatim-like environments, math and the preamble are excluded from every
//   word count; a command not on the skip list is transparent (its name
//   contributes no words, but its argument counts normally) — texcount's own
//   default for an unlisted command. This does not expand user macros. Pure
//   and synchronous; a caller that rescans on every keystroke should debounce
//   and scan off its UI thread itself (this port has no threading opinion,
//   mirroring the Swift source's own separation between the scanner and its
//   `WordCountModel` debounce wrapper, which belongs to a future WinUI shell
//   and is not ported here).
// author: Claude Sonnet 5
// date: 2026-09-14

using System.Text;

namespace FlashTeX.ProjectFiles;

public static class DocumentStatistics
{
    /// <summary>Word/block totals. <see cref="TotalWords"/> is what a status bar would show.</summary>
    public sealed record Counts(int BodyWords = 0, int HeaderWords = 0, int CaptionWords = 0, int InlineMath = 0, int DisplayMath = 0)
    {
        public int TotalWords => BodyWords + HeaderWords + CaptionWords;

        public static Counts operator +(Counts a, Counts b) => new(
            a.BodyWords + b.BodyWords,
            a.HeaderWords + b.HeaderWords,
            a.CaptionWords + b.CaptionWords,
            a.InlineMath + b.InlineMath,
            a.DisplayMath + b.DisplayMath);
    }

    /// <summary>One heading's span: everything from its title to the next heading (of any level) in the same document.</summary>
    /// <param name="DocumentPath">Set only when the aggregate covers more than one open document (see <see cref="Analyze(IReadOnlyList{ValueTuple{string, string}})"/>).</param>
    /// <param name="Level"><c>part</c>/<c>chapter</c> 0 … <c>paragraph</c>/<c>subparagraph</c> 4/5 (<see cref="DocumentOutline.SectionLevels"/>).</param>
    public sealed record SectionBreakdown(int Level, string Title, Counts Counts, string? DocumentPath = null);

    /// <summary>Result of scanning one document. <see cref="Truncated"/> is true when the document exceeded <see cref="MaxScannedBytes"/> and was not scanned (counts are all zero).</summary>
    public sealed record Result(Counts Counts, IReadOnlyList<SectionBreakdown> Sections, bool Truncated = false);

    /// <summary>Result of scanning an open document set.</summary>
    public sealed record AggregateResult(Counts Total, IReadOnlyList<SectionBreakdown> Sections, int DocumentCount, bool Truncated = false);

    /// <summary>Documents longer than this are not scanned (mirrors <see cref="DocumentOutline.MaxScannedUtf16"/>'s guard against a pathological rescan).</summary>
    public const int MaxScannedBytes = 4 * 1024 * 1024;

    /// <summary>Scans one document's full text.</summary>
    public static Result Analyze(string text)
    {
        if (Encoding.UTF8.GetByteCount(text) > MaxScannedBytes)
        {
            return new Result(new Counts(), [], Truncated: true);
        }
        var scanner = new Scanner(text);
        scanner.Run();
        return new Result(scanner.Total, scanner.Sections);
    }

    /// <summary>
    /// Scans every document in the open set and sums the totals. Section
    /// titles are tagged with their owning path only when there is more than
    /// one document, so a single-file project's popover stays unlabeled.
    /// </summary>
    public static AggregateResult Analyze(IReadOnlyList<(string Path, string Text)> documents)
    {
        Counts total = new();
        var sections = new List<SectionBreakdown>();
        bool truncated = false;
        bool multi = documents.Count > 1;
        foreach ((string path, string text) in documents)
        {
            Result r = Analyze(text);
            truncated = truncated || r.Truncated;
            total += r.Counts;
            sections.AddRange(multi ? r.Sections.Select(s => s with { DocumentPath = path }) : r.Sections);
        }
        return new AggregateResult(total, sections, documents.Count, truncated);
    }

    /// <summary>
    /// Word count of one UTF-16 range of <paramref name="text"/> (the editor
    /// selection), using the same scan — so a selection that happens to
    /// contain, say, a whole math block is excluded the same way the
    /// full-document count excludes it.
    /// </summary>
    public static int WordCount(string text, Utf16Range range)
    {
        if (range.Length <= 0 || range.Location < 0 || range.End > text.Length)
        {
            return 0;
        }
        return Analyze(text.Substring(range.Location, range.Length)).Counts.TotalWords;
    }

    // MARK: - Scanner

    private enum Bucket
    {
        Body,
        Header,
        Caption,
    }

    /// <summary>
    /// Commands whose argument(s) are never prose: skip the command name and
    /// every immediately-following <c>[…]</c>/<c>{…}</c> group (in the order
    /// LaTeX reads them), greedily, so a two-argument
    /// <c>\newcommand{\foo}[1]{…}</c> macro *definition* is skipped in full
    /// rather than counted as text.
    /// </summary>
    private static readonly IReadOnlySet<string> SkipCommands = new HashSet<string>(StringComparer.Ordinal)
    {
        "label", "ref", "eqref", "pageref", "cref", "Cref", "vref", "autoref", "nameref",
        "cite", "citep", "citet", "citeauthor", "citeyear", "citealt", "citealp",
        "footcite", "textcite", "parencite", "Citep", "Citet",
        "url", "path", "includegraphics", "input", "include", "subfile", "subimport", "import",
        "bibliography", "addbibresource", "bibliographystyle", "printbibliography", "bibitem",
        "usepackage", "RequirePackage", "documentclass",
        "newcommand", "renewcommand", "providecommand", "DeclareRobustCommand",
        "newenvironment", "renewenvironment", "DeclareMathOperator", "newcolumntype", "newtheorem",
        "setlength", "addtolength", "newlength", "setcounter", "newcounter",
        "pagestyle", "thispagestyle", "includepdf", "graphicspath",
        "hypersetup", "definecolor", "geometry", "usetikzlibrary",
        "hspace", "vspace", "hphantom", "vphantom", "phantom",
    };

    private static readonly IReadOnlySet<string> VerbatimEnvironments = new HashSet<string>(StringComparer.Ordinal)
    {
        "verbatim", "verbatim*", "Verbatim", "BVerbatim", "lstlisting", "minted", "alltt", "comment",
    };

    private static readonly IReadOnlySet<string> MathEnvironments = new HashSet<string>(StringComparer.Ordinal)
    {
        "equation", "equation*", "align", "align*", "alignat", "alignat*", "flalign", "flalign*",
        "gather", "gather*", "multline", "multline*", "eqnarray", "eqnarray*", "displaymath", "math",
    };

    /// <summary>
    /// Escaped-special-character commands (<c>\%</c>, <c>\&amp;</c>, …):
    /// resolved to the literal glyph for section-title display only; never
    /// counted as a word.
    /// </summary>
    private static readonly IReadOnlyDictionary<byte, char> SymbolGlyph = new Dictionary<byte, char>
    {
        [0x25] = '%', [0x24] = '$', [0x26] = '&', [0x23] = '#', [0x5F] = '_', [0x7B] = '{', [0x7D] = '}', [0x7E] = '~', [0x5E] = '^',
    };

    private sealed class MutableCounts
    {
        public int BodyWords;
        public int HeaderWords;
        public int CaptionWords;
        public int InlineMath;
        public int DisplayMath;

        public Counts ToImmutable() => new(BodyWords, HeaderWords, CaptionWords, InlineMath, DisplayMath);
    }

    private sealed class MutableSection(int level, string title)
    {
        public int Level { get; } = level;

        public string Title { get; } = title;

        public MutableCounts Counts { get; } = new();
    }

    /// <summary>
    /// One pass over <paramref name="text"/>'s UTF-8 bytes. ASCII structural
    /// characters (<c>\ % $ { }</c>) are matched a byte at a time; everything
    /// else (including multibyte UTF-8 continuation/lead bytes) is treated as
    /// ordinary word content, so accented/CJK text is counted without
    /// decoding scalars.
    /// </summary>
    private sealed class Scanner
    {
        private static readonly byte[] BeginDocument = Encoding.ASCII.GetBytes("\\begin{document}");
        private static readonly byte[] DoubleDollar = Encoding.ASCII.GetBytes("$$");
        private static readonly byte[] SingleDollar = Encoding.ASCII.GetBytes("$");
        private static readonly byte[] InlineMathEnd = Encoding.ASCII.GetBytes("\\)");
        private static readonly byte[] DisplayMathEnd = Encoding.ASCII.GetBytes("\\]");

        private readonly byte[] _bytes;
        private int _idx;
        private Bucket _currentBucket = Bucket.Body;
        private readonly List<Scope> _scopeStack = [];
        private string _titleBuffer = string.Empty;
        private readonly MutableCounts _total = new();
        private readonly List<MutableSection> _sections = [];
        private int? _currentSectionIndex;

        public Scanner(string text) => _bytes = Encoding.UTF8.GetBytes(text);

        public Counts Total => _total.ToImmutable();

        public IReadOnlyList<SectionBreakdown> Sections =>
            _sections.Select(s => new SectionBreakdown(s.Level, s.Title, s.Counts.ToImmutable())).ToList();

        private sealed class Scope
        {
            public required Bucket RestoreBucket { get; init; }

            public required bool EndsTitleCapture { get; init; }

            public int? Level { get; init; }
        }

        public void Run()
        {
            _idx = FindLiteral(BeginDocument, 0) is int found ? found + BeginDocument.Length : 0;
            while (_idx < _bytes.Length)
            {
                switch (_bytes[_idx])
                {
                    case (byte)'\\':
                        HandleBackslash();
                        break;
                    case (byte)'%':
                        SkipComment();
                        break;
                    case (byte)'$':
                        HandleDollar();
                        break;
                    case (byte)'{':
                        _scopeStack.Add(new Scope { RestoreBucket = _currentBucket, EndsTitleCapture = false });
                        _idx++;
                        break;
                    case (byte)'}':
                        _idx++;
                        CloseScope();
                        break;
                    default:
                        ScanWordRun();
                        break;
                }
            }
        }

        private void CloseScope()
        {
            if (_scopeStack.Count == 0)
            {
                return;
            }
            Scope top = _scopeStack[^1];
            _scopeStack.RemoveAt(_scopeStack.Count - 1);
            if (top.EndsTitleCapture)
            {
                FinalizeTitle(top.Level ?? 1);
            }
            _currentBucket = top.RestoreBucket;
        }

        private void ScanWordRun()
        {
            int start = _idx;
            while (_idx < _bytes.Length && !IsControl(_bytes[_idx]))
            {
                _idx++;
            }
            AddWords(WordsIn(start, _idx), _currentBucket);
            if (_currentBucket == Bucket.Header)
            {
                _titleBuffer += Encoding.UTF8.GetString(_bytes, start, _idx - start);
            }
        }

        // MARK: byte classification

        private static bool IsControl(byte b) => b is (byte)'\\' or (byte)'%' or (byte)'$' or (byte)'{' or (byte)'}';

        private static bool IsLetter(byte b) => (b >= 0x41 && b <= 0x5A) || (b >= 0x61 && b <= 0x7A);

        private static bool IsWhitespace(byte b) => b is 0x20 or 0x09 or 0x0A or 0x0D;

        private static bool IsWordByte(byte b) => (b >= 0x30 && b <= 0x39) || IsLetter(b) || b is 0x27 or 0x2D or 0x5F || b >= 0x80;

        private int WordsIn(int start, int end)
        {
            int count = 0;
            bool inWord = false;
            for (int k = start; k < end; k++)
            {
                bool isWord = IsWordByte(_bytes[k]);
                if (isWord && !inWord)
                {
                    count++;
                }
                inWord = isWord;
            }
            return count;
        }

        // MARK: counting

        private void AddWords(int n, Bucket bucket)
        {
            if (n <= 0)
            {
                return;
            }
            switch (bucket)
            {
                case Bucket.Body:
                    _total.BodyWords += n;
                    if (_currentSectionIndex is int i0)
                    {
                        _sections[i0].Counts.BodyWords += n;
                    }
                    break;
                case Bucket.Header:
                    _total.HeaderWords += n;
                    if (_currentSectionIndex is int i1)
                    {
                        _sections[i1].Counts.HeaderWords += n;
                    }
                    break;
                case Bucket.Caption:
                    _total.CaptionWords += n;
                    if (_currentSectionIndex is int i2)
                    {
                        _sections[i2].Counts.CaptionWords += n;
                    }
                    break;
            }
        }

        private void AddMath(int inlineCount = 0, int displayCount = 0)
        {
            _total.InlineMath += inlineCount;
            _total.DisplayMath += displayCount;
            if (_currentSectionIndex is int i)
            {
                _sections[i].Counts.InlineMath += inlineCount;
                _sections[i].Counts.DisplayMath += displayCount;
            }
        }

        private void FinalizeTitle(int level)
        {
            string cleaned = string.Join(' ', _titleBuffer.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries));
            _sections.Add(new MutableSection(level, cleaned.Length == 0 ? "(untitled)" : cleaned));
            _currentSectionIndex = _sections.Count - 1;
            _titleBuffer = string.Empty;
        }

        // MARK: raw scanning helpers

        private void SkipWhitespace()
        {
            while (_idx < _bytes.Length && IsWhitespace(_bytes[_idx]))
            {
                _idx++;
            }
        }

        private int? FindLiteral(byte[] pattern, int start)
        {
            if (pattern.Length == 0 || start > _bytes.Length - pattern.Length)
            {
                return null;
            }
            for (int i = start; i <= _bytes.Length - pattern.Length; i++)
            {
                if (_bytes[i] != pattern[0])
                {
                    continue;
                }
                bool match = true;
                for (int k = 1; k < pattern.Length; k++)
                {
                    if (_bytes[i + k] != pattern[k])
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

        private bool AdvancePastLiteral(byte[] pattern)
        {
            if (FindLiteral(pattern, _idx) is int found)
            {
                _idx = found + pattern.Length;
                return true;
            }
            _idx = _bytes.Length;
            return false;
        }

        private void SkipBalanced(byte open, byte close)
        {
            if (_idx >= _bytes.Length || _bytes[_idx] != open)
            {
                return;
            }
            int depth = 1;
            _idx++;
            while (_idx < _bytes.Length && depth > 0)
            {
                if (_bytes[_idx] == open)
                {
                    depth++;
                }
                else if (_bytes[_idx] == close)
                {
                    depth--;
                }
                _idx++;
            }
        }

        private void SkipArgumentsGreedy()
        {
            while (true)
            {
                SkipWhitespace();
                if (_idx >= _bytes.Length)
                {
                    return;
                }
                if (_bytes[_idx] == (byte)'[')
                {
                    SkipBalanced((byte)'[', (byte)']');
                }
                else if (_bytes[_idx] == (byte)'{')
                {
                    SkipBalanced((byte)'{', (byte)'}');
                }
                else
                {
                    return;
                }
            }
        }

        /// <summary>
        /// Reads the raw <c>{name}</c> right after (whitespace-tolerant) the
        /// cursor, consuming it, without treating its content as words or
        /// commands. Used for <c>\begin{name}</c>/<c>\end{name}</c>.
        /// </summary>
        private string ReadBraceGroupRaw()
        {
            SkipWhitespace();
            if (_idx >= _bytes.Length || _bytes[_idx] != (byte)'{')
            {
                return string.Empty;
            }
            _idx++;
            int start = _idx;
            int depth = 1;
            while (_idx < _bytes.Length && depth > 0)
            {
                if (_bytes[_idx] == (byte)'{')
                {
                    depth++;
                }
                else if (_bytes[_idx] == (byte)'}')
                {
                    depth--;
                }
                if (depth > 0)
                {
                    _idx++;
                }
            }
            string name = Encoding.UTF8.GetString(_bytes, start, _idx - start);
            if (_idx < _bytes.Length)
            {
                _idx++; // closing '}'
            }
            return name;
        }

        private void SkipComment()
        {
            while (_idx < _bytes.Length && _bytes[_idx] != (byte)'\n')
            {
                _idx++;
            }
            if (_idx < _bytes.Length)
            {
                _idx++; // the newline itself is ordinary whitespace
            }
        }

        private void HandleDollar()
        {
            if (_idx + 1 < _bytes.Length && _bytes[_idx + 1] == (byte)'$')
            {
                _idx += 2;
                AdvancePastLiteral(DoubleDollar);
                AddMath(displayCount: 1);
            }
            else
            {
                _idx++;
                AdvancePastLiteral(SingleDollar);
                AddMath(inlineCount: 1);
            }
        }

        private void HandleBackslash()
        {
            _idx++;
            if (_idx >= _bytes.Length)
            {
                return;
            }
            byte c = _bytes[_idx];
            if (c == (byte)'(')
            {
                _idx++;
                AdvancePastLiteral(InlineMathEnd);
                AddMath(inlineCount: 1);
                return;
            }
            if (c == (byte)'[')
            {
                _idx++;
                AdvancePastLiteral(DisplayMathEnd);
                AddMath(displayCount: 1);
                return;
            }
            if (!IsLetter(c))
            {
                _idx++; // one escaped symbol byte, e.g. \% \& \_ \\ \  \-
                if (_currentBucket == Bucket.Header && SymbolGlyph.TryGetValue(c, out char glyph))
                {
                    _titleBuffer += glyph;
                }
                return;
            }
            Dispatch(ReadCommandName());
        }

        private string ReadCommandName()
        {
            int start = _idx;
            while (_idx < _bytes.Length && IsLetter(_bytes[_idx]))
            {
                _idx++;
            }
            string name = Encoding.UTF8.GetString(_bytes, start, _idx - start);
            if (_idx < _bytes.Length && _bytes[_idx] == (byte)'*')
            {
                _idx++; // trailing '*', not part of the name
            }
            return name;
        }

        private void Dispatch(string name)
        {
            if (name is "begin" or "end")
            {
                DispatchEnvironment(name);
                return;
            }
            if (name == "verb")
            {
                HandleVerb();
                return;
            }
            if (name == "href")
            {
                HandleHref();
                return;
            }
            if (name == "caption")
            {
                HandleArgumentBucket(Bucket.Caption, level: null);
                return;
            }
            if (DocumentOutline.SectionLevels.TryGetValue(name, out int level))
            {
                HandleArgumentBucket(Bucket.Header, level);
                return;
            }
            if (SkipCommands.Contains(name))
            {
                SkipArgumentsGreedy();
            }
            // else: transparent — the command name contributes no words; any
            // following {…} is picked up by the generic brace handling above.
        }

        private void DispatchEnvironment(string name)
        {
            string env = ReadBraceGroupRaw();
            if (name == "end" && env == "document")
            {
                _idx = _bytes.Length;
                return;
            }
            if (name != "begin")
            {
                return; // a lone \end{env} names nothing further to skip
            }
            if (VerbatimEnvironments.Contains(env))
            {
                SkipArgumentsGreedy(); // e.g. \begin{lstlisting}[language=Python]
                AdvancePastLiteral(Encoding.UTF8.GetBytes($"\\end{{{env}}}"));
            }
            else if (MathEnvironments.Contains(env))
            {
                AdvancePastLiteral(Encoding.UTF8.GetBytes($"\\end{{{env}}}"));
                AddMath(displayCount: 1);
            }
        }

        /// <summary>
        /// <c>\section{…}</c> / <c>\caption{…}</c>-style commands: an optional
        /// <c>[short]</c> form is skipped, then the next <c>{…}</c> is
        /// captured into <paramref name="bucket"/> (and, for a heading, into a
        /// new <see cref="SectionBreakdown"/>) until its matching close brace.
        /// </summary>
        private void HandleArgumentBucket(Bucket bucket, int? level)
        {
            SkipWhitespace();
            if (_idx < _bytes.Length && _bytes[_idx] == (byte)'[')
            {
                SkipBalanced((byte)'[', (byte)']');
                SkipWhitespace();
            }
            if (_idx >= _bytes.Length || _bytes[_idx] != (byte)'{')
            {
                return;
            }
            _idx++;
            _scopeStack.Add(new Scope { RestoreBucket = _currentBucket, EndsTitleCapture = level is not null, Level = level });
            _currentBucket = bucket;
            if (level is not null)
            {
                _titleBuffer = string.Empty;
            }
        }

        /// <summary><c>\href[…]{url}{text}</c>: the URL is never text; the display text falls through to the generic brace handling right after.</summary>
        private void HandleHref()
        {
            SkipWhitespace();
            if (_idx < _bytes.Length && _bytes[_idx] == (byte)'[')
            {
                SkipBalanced((byte)'[', (byte)']');
                SkipWhitespace();
            }
            if (_idx < _bytes.Length && _bytes[_idx] == (byte)'{')
            {
                SkipBalanced((byte)'{', (byte)'}');
            }
        }

        private void HandleVerb()
        {
            if (_idx >= _bytes.Length)
            {
                return;
            }
            byte delimiter = _bytes[_idx];
            _idx++;
            while (_idx < _bytes.Length && _bytes[_idx] != delimiter)
            {
                _idx++;
            }
            if (_idx < _bytes.Length)
            {
                _idx++;
            }
        }
    }
}
