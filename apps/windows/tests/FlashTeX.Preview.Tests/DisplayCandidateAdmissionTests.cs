// name: DisplayCandidateAdmissionTests.cs
// purpose: Unit tests for FlashTeX.Preview.DisplayCandidateAdmission, covering every
//   rejection reason and the accept/supersede path from
//   docs/contracts/runtime-v1-display-list-v2.md's "Helper boundary and currentness".
// author: Claude Sonnet 5
// date: 2026-09-14

using FlashTeX.Preview;

namespace FlashTeX.Preview.Tests;

public class DisplayCandidateAdmissionTests
{
    private static DisplayCandidateIdentity MakeIdentity(
        string sessionId = "s1", string projectId = "p1", string requestId = "r1", int revision = 1, int membershipGeneration = 1) =>
        new(sessionId, projectId, requestId, revision, membershipGeneration);

    private static AdmissionRejectionReason RejectionReasonOf(AdmissionResult result) =>
        Assert.IsType<AdmissionResult.Rejected>(result).Reason;

    [Fact]
    public void Admit_WithNoOutstandingRequest_IsRejected()
    {
        var admission = new DisplayCandidateAdmission();

        var result = admission.Admit(MakeIdentity());

        Assert.Equal(AdmissionRejectionReason.NoOutstandingRequest, RejectionReasonOf(result));
        Assert.Null(admission.Admitted);
    }

    [Fact]
    public void Admit_WrongSession_IsRejected()
    {
        var admission = new DisplayCandidateAdmission();
        admission.BeginRequest(MakeIdentity(sessionId: "current-session"));

        var result = admission.Admit(MakeIdentity(sessionId: "other-session"));

        Assert.Equal(AdmissionRejectionReason.WrongSession, RejectionReasonOf(result));
        Assert.Null(admission.Admitted);
    }

    [Fact]
    public void Admit_WrongProject_IsRejected()
    {
        var admission = new DisplayCandidateAdmission();
        admission.BeginRequest(MakeIdentity(projectId: "current-project"));

        var result = admission.Admit(MakeIdentity(projectId: "other-project"));

        Assert.Equal(AdmissionRejectionReason.WrongProject, RejectionReasonOf(result));
    }

    [Fact]
    public void Admit_OrphanedUncorrelatedRequestId_IsRejected()
    {
        var admission = new DisplayCandidateAdmission();
        admission.BeginRequest(MakeIdentity(requestId: "current-request"));

        // Same session/project, but the request id names no outstanding request: an
        // unsolicited/uncorrelated candidate per "duplicate, interleaved, unsolicited,
        // malformed or wrongly correlated siblings must not become render candidates".
        var result = admission.Admit(MakeIdentity(requestId: "some-other-request"));

        Assert.Equal(AdmissionRejectionReason.Orphaned, RejectionReasonOf(result));
    }

    [Fact]
    public void Admit_WrongMembershipGeneration_IsRejected()
    {
        var admission = new DisplayCandidateAdmission();
        admission.BeginRequest(MakeIdentity(membershipGeneration: 5));

        var result = admission.Admit(MakeIdentity(membershipGeneration: 4));

        Assert.Equal(AdmissionRejectionReason.WrongMembershipGeneration, RejectionReasonOf(result));
    }

    [Fact]
    public void Admit_StaleRevision_IsRejected()
    {
        var admission = new DisplayCandidateAdmission();
        admission.BeginRequest(MakeIdentity(revision: 10));

        // "Stale or cancelled work cannot acquire currentness by arriving later."
        var result = admission.Admit(MakeIdentity(revision: 9));

        Assert.Equal(AdmissionRejectionReason.StaleRevision, RejectionReasonOf(result));
    }

    [Fact]
    public void Admit_ValidCandidate_IsAcceptedAndBecomesAdmitted()
    {
        var admission = new DisplayCandidateAdmission();
        var expected = MakeIdentity();
        admission.BeginRequest(expected);

        var result = admission.Admit(expected);

        var accepted = Assert.IsType<AdmissionResult.Accepted>(result);
        Assert.Equal(expected, accepted.Identity);
        Assert.Equal(expected, admission.Admitted);
    }

    [Fact]
    public void Admit_ExactDuplicateOfAlreadyAdmittedCandidate_IsRejected()
    {
        var admission = new DisplayCandidateAdmission();
        var expected = MakeIdentity();
        admission.BeginRequest(expected);
        admission.Admit(expected);

        var result = admission.Admit(expected);

        Assert.Equal(AdmissionRejectionReason.Duplicate, RejectionReasonOf(result));
    }

    [Fact]
    public void Admit_SupersedingCandidateForSameRequest_IsAcceptedAndReplacesThePreviousOne()
    {
        var admission = new DisplayCandidateAdmission();
        admission.BeginRequest(MakeIdentity(revision: 1));
        var first = MakeIdentity(revision: 1);
        var firstResult = admission.Admit(first);
        Assert.IsType<AdmissionResult.Accepted>(firstResult);
        Assert.Equal(first, admission.Admitted);

        // A newer compile for the same outstanding request supersedes the admitted one —
        // "optional candidates may be replaced, evicted or refused".
        var superseding = MakeIdentity(revision: 2);
        var secondResult = admission.Admit(superseding);

        var accepted = Assert.IsType<AdmissionResult.Accepted>(secondResult);
        Assert.Equal(superseding, accepted.Identity);
        // "Retained old frames must not acquire new source authority": the old admission
        // is simply gone — Admitted now names only the superseding candidate.
        Assert.Equal(superseding, admission.Admitted);
        Assert.NotEqual(first, admission.Admitted);
    }

    [Fact]
    public void Admit_OlderCandidateArrivingAfterASupersedingOneWasAdmitted_IsRejectedAsStale()
    {
        var admission = new DisplayCandidateAdmission();
        admission.BeginRequest(MakeIdentity(revision: 1));
        admission.Admit(MakeIdentity(revision: 2));

        // The revision-1 candidate arrives late (e.g. reordered transport): it must not
        // acquire currentness now that revision 2 is admitted.
        var result = admission.Admit(MakeIdentity(revision: 1));

        Assert.Equal(AdmissionRejectionReason.StaleRevision, RejectionReasonOf(result));
        Assert.Equal(MakeIdentity(revision: 2), admission.Admitted);
    }

    [Fact]
    public void BeginRequest_ForANewRequest_ClearsThePreviouslyAdmittedCandidate()
    {
        var admission = new DisplayCandidateAdmission();
        admission.BeginRequest(MakeIdentity(requestId: "r1"));
        admission.Admit(MakeIdentity(requestId: "r1"));
        Assert.NotNull(admission.Admitted);

        admission.BeginRequest(MakeIdentity(requestId: "r2", revision: 1));

        Assert.Null(admission.Admitted);
        // The old request's candidate arriving late is now simply orphaned.
        var result = admission.Admit(MakeIdentity(requestId: "r1"));
        Assert.Equal(AdmissionRejectionReason.Orphaned, RejectionReasonOf(result));
    }

    [Fact]
    public void EndRequest_ClearsOutstandingRequestAndAdmittedCandidate()
    {
        var admission = new DisplayCandidateAdmission();
        admission.BeginRequest(MakeIdentity());
        admission.Admit(MakeIdentity());

        admission.EndRequest();

        Assert.Null(admission.Admitted);
        var result = admission.Admit(MakeIdentity());
        Assert.Equal(AdmissionRejectionReason.NoOutstandingRequest, RejectionReasonOf(result));
    }
}
