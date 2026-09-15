// name: LayoutNegotiation.cs
// purpose: Per-request layout capability negotiation
//   (docs/contracts/runtime-v1-layout-capabilities.md). Ported from
//   apps/mac/Sources/FlashTeXProtocol/LayoutNegotiation.swift. A result is bound to
//   the capability set of its own request; the consumer never infers support from a
//   response and never lets a late response for a different request change the
//   renderer mode.
// author: Claude Sonnet 5
// date: 2026-09-13

using FlashTeX.Protocol.RuntimeV1;

namespace FlashTeX.Protocol;

/// <summary>
/// Capabilities a compile request carried, and the subset its result accepted.
/// </summary>
public sealed record LayoutNegotiation(IReadOnlyList<string> Requested, IReadOnlyList<string> Accepted)
{
    public LayoutNegotiation()
        : this(Array.Empty<string>(), Array.Empty<string>())
    {
    }

    public static readonly LayoutNegotiation Legacy = new();

    /// <summary>
    /// True when at least one capability was accepted: typed rules/font hints may
    /// appear and unknown primitives are reported instead of skipped.
    /// </summary>
    public bool IsNegotiated => Accepted.Count > 0;

    public bool RulesAccepted => Accepted.Contains(LayoutCapabilities.RulesV1);

    public bool FontHintsAccepted => Accepted.Contains(LayoutCapabilities.FontHintsV1);

    /// <summary>Requested but not accepted — reported explicitly, never guessed around.</summary>
    public IReadOnlyList<string> Missing => Requested.Where(r => !Accepted.Contains(r)).ToList();

    /// <summary>
    /// Checks a result against the capabilities its request carried. Returns a
    /// violation description (the result must be rejected) or <c>null</c>.
    /// Violations: an accepted capability that was not requested; a <c>rule</c>
    /// item without accepted <c>rules-v1</c>; a text <c>font</c> hint without
    /// accepted <c>font-hints-v1</c>. Acceptance is what the producer echoed — a
    /// shape the producer did not negotiate is rejected even if the client asked
    /// for it.
    /// </summary>
    public static string? Violation(CompileResult result, IReadOnlyList<string> requested)
    {
        var accepted = result.LayoutCapabilities ?? Array.Empty<string>();
        var extra = accepted.FirstOrDefault(c => !requested.Contains(c));
        if (extra is not null)
        {
            return $"accepted capability {extra} was not requested (requested: {Describe(requested)})";
        }

        var negotiation = new LayoutNegotiation(requested, accepted);
        foreach (var page in result.Pages)
        {
            var violation = ViolationInPage(page, negotiation, accepted);
            if (violation is not null)
            {
                return violation;
            }
        }
        return null;
    }

    private static string? ViolationInPage(Page page, LayoutNegotiation negotiation, IReadOnlyList<string> accepted)
    {
        foreach (var item in page.Items)
        {
            if (item is PageItem.OfRule && !negotiation.RulesAccepted)
            {
                return $"page {page.Number} contains a rule item but {LayoutCapabilities.RulesV1} was not accepted (accepted: {Describe(accepted)})";
            }
            if (item is PageItem.OfText { Item.Font: not null } && !negotiation.FontHintsAccepted)
            {
                return $"page {page.Number} text item carries a font hint but {LayoutCapabilities.FontHintsV1} was not accepted (accepted: {Describe(accepted)})";
            }
        }
        return null;
    }

    /// <summary>
    /// Source-aware error diagnostics for every unknown primitive kind. Empty on
    /// the legacy route, where unknown kinds are still skipped silently.
    /// </summary>
    public static IReadOnlyList<Diagnostic> UnsupportedPrimitiveDiagnostics(CompileResult result, LayoutNegotiation negotiation)
    {
        if (!negotiation.IsNegotiated)
        {
            return Array.Empty<Diagnostic>();
        }
        var diagnostics = new List<Diagnostic>();
        foreach (var page in result.Pages)
        {
            foreach (var item in page.Items)
            {
                if (item is PageItem.Unknown unknown)
                {
                    diagnostics.Add(DiagnosticFor(unknown, page.Number));
                }
            }
        }
        return diagnostics;
    }

    private static Diagnostic DiagnosticFor(PageItem.Unknown unknown, int pageNumber)
    {
        string at = unknown.Source is { } source
            ? $" at {source.Path} bytes {source.StartByte}..<{source.EndByte}"
            : " (no source mapping)";
        return new Diagnostic(
            Severity.error,
            $"unsupported layout primitive '{unknown.Kind}'{at} on page {pageNumber}",
            unknown.Source,
            "item not drawn; the rest of the page is shown");
    }

    /// <summary>"none" or the comma-joined set, for log and banner text.</summary>
    public static string Describe(IReadOnlyList<string> capabilities) =>
        capabilities.Count == 0 ? "none" : string.Join(", ", capabilities);
}
