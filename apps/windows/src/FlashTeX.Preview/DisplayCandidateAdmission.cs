// name: DisplayCandidateAdmission.cs
// purpose: Admission state machine for untrusted `display_candidate` frames from the
//   optional preview-controller helper route, implementing the correlation/staleness
//   rules in docs/contracts/runtime-v1-display-list-v2.md ("Helper boundary and
//   currentness"). A candidate is admitted only if it matches the consumer's own current
//   session/project/request/compile identity; nothing here proves paintability or
//   installation, only that the candidate is not stale/orphaned/wrongly correlated.
// author: Claude Sonnet 5
// date: 2026-09-14

namespace FlashTeX.Preview;

/// <summary>
/// The correlation identity a `display_candidate` must match: "Its session/project/
/// request/compile identity, membership generation and complete editor-version map must
/// match fresh caller-owned state" (contract doc, Helper boundary and currentness).
/// <see cref="MembershipGeneration"/> stands in for that fresh caller-owned state check in
/// this simplified port (the full editor-version map is out of scope here).
/// </summary>
public readonly record struct DisplayCandidateIdentity(
    string SessionId,
    string ProjectId,
    string RequestId,
    int CompileRevision,
    int MembershipGeneration);

/// <summary>Why a candidate was refused, matching a specific contract-doc sentence — see <see cref="DisplayCandidateAdmission.Admit"/>.</summary>
public enum AdmissionRejectionReason
{
    /// <summary>No outstanding request to correlate against yet (helper mode not enabled/awaited, or already resolved).</summary>
    NoOutstandingRequest,

    /// <summary>"session/project/request/compile identity ... must match fresh caller-owned state": wrong session.</summary>
    WrongSession,

    /// <summary>"session/project/request/compile identity ... must match fresh caller-owned state": wrong project.</summary>
    WrongProject,

    /// <summary>"Duplicate, interleaved, unsolicited, malformed or wrongly correlated siblings must not become render candidates": the request id names no outstanding request.</summary>
    Orphaned,

    /// <summary>"the complete editor-version map must match fresh caller-owned state": stale membership generation.</summary>
    WrongMembershipGeneration,

    /// <summary>"Stale or cancelled work cannot acquire currentness by arriving later.": older than the currently expected/admitted revision.</summary>
    StaleRevision,

    /// <summary>Exact repeat of the already-admitted candidate for this request.</summary>
    Duplicate,
}

/// <summary>Outcome of <see cref="DisplayCandidateAdmission.Admit"/>: closed hierarchy so callers must handle both cases.</summary>
public abstract record AdmissionResult
{
    private AdmissionResult()
    {
    }

    public sealed record Accepted(DisplayCandidateIdentity Identity) : AdmissionResult;

    public sealed record Rejected(AdmissionRejectionReason Reason, string Message) : AdmissionResult;
}

/// <summary>
/// Tracks the consumer's own current request identity and the currently admitted
/// candidate (if any), and decides whether an incoming `display_candidate` may become the
/// current render candidate. Not thread-safe by design: candidates arrive on one decode
/// stream and admission decisions must be applied in arrival order.
/// </summary>
public sealed class DisplayCandidateAdmission
{
    private DisplayCandidateIdentity? expectedRequest;
    private DisplayCandidateIdentity? admitted;

    /// <summary>The candidate currently admitted as the render candidate, or null if none has been (or the outstanding request has none yet).</summary>
    public DisplayCandidateIdentity? Admitted => this.admitted;

    /// <summary>
    /// Called by the consumer when it dispatches (or re-dispatches) a compile request with
    /// the helper route enabled: establishes fresh caller-owned state to correlate
    /// candidates against. Any previously admitted candidate stops being current — "stale
    /// or cancelled work cannot acquire currentness by arriving later" applies to the
    /// request that produced it too.
    /// </summary>
    public void BeginRequest(DisplayCandidateIdentity request)
    {
        this.expectedRequest = request;
        this.admitted = null;
    }

    /// <summary>Clears the outstanding request (e.g. the helper route was disabled, or the request resolved without a sibling).</summary>
    public void EndRequest()
    {
        this.expectedRequest = null;
        this.admitted = null;
    }

    /// <summary>
    /// Applies the correlation/staleness rules to <paramref name="candidate"/>. Accepting
    /// it makes it <see cref="Admitted"/>, replacing whatever was admitted before —
    /// "optional candidates may be replaced, evicted or refused" — but the old admitted
    /// value is simply gone, never returned again by <see cref="Admitted"/> ("retained old
    /// frames must not acquire new source authority").
    /// </summary>
    public AdmissionResult Admit(DisplayCandidateIdentity candidate)
    {
        if (this.expectedRequest is not { } expected)
        {
            return Reject(AdmissionRejectionReason.NoOutstandingRequest, "no outstanding request is awaiting a display candidate");
        }
        if (candidate.SessionId != expected.SessionId)
        {
            return Reject(AdmissionRejectionReason.WrongSession, $"candidate session '{candidate.SessionId}' does not match the current session '{expected.SessionId}'");
        }
        if (candidate.ProjectId != expected.ProjectId)
        {
            return Reject(AdmissionRejectionReason.WrongProject, $"candidate project '{candidate.ProjectId}' does not match the current project '{expected.ProjectId}'");
        }
        if (candidate.RequestId != expected.RequestId)
        {
            return Reject(AdmissionRejectionReason.Orphaned, $"candidate request '{candidate.RequestId}' does not correlate with the outstanding request '{expected.RequestId}'");
        }
        if (candidate.MembershipGeneration != expected.MembershipGeneration)
        {
            return Reject(AdmissionRejectionReason.WrongMembershipGeneration, $"candidate membership generation {candidate.MembershipGeneration} does not match the current generation {expected.MembershipGeneration}");
        }

        int baselineRevision = Math.Max(expected.CompileRevision, this.admitted?.CompileRevision ?? int.MinValue);
        if (candidate.CompileRevision < baselineRevision)
        {
            return Reject(AdmissionRejectionReason.StaleRevision, $"candidate revision {candidate.CompileRevision} is older than the current revision {baselineRevision}");
        }
        if (this.admitted is { } current && candidate.CompileRevision == baselineRevision && candidate == current)
        {
            return Reject(AdmissionRejectionReason.Duplicate, $"candidate for request '{candidate.RequestId}' revision {candidate.CompileRevision} was already admitted");
        }

        this.admitted = candidate;
        return new AdmissionResult.Accepted(candidate);
    }

    private static AdmissionResult Reject(AdmissionRejectionReason reason, string message) => new AdmissionResult.Rejected(reason, message);
}
