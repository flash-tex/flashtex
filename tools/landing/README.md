# Landing tools (Commander)

These are the scripts the Commander (`daniel-parent`) uses to land PRs through main's merge queue. They're kept here so any lane can see what is running on its PR, and so a successor Commander can rerun them.

- `land.sh <pr> <sha> <gates|-> <modes|->` runs a chain. First, the head is unchanged and the `CI required` check on the head (the `pull_request` run) is green. Next, one sweep is dispatched per boxmemo mode (throttled to `MAXSW` active sweeps, default 8). Then it waits for `AFTER` to merge, optionally retargets the base, and turns on auto-merge. It never pushes. The last line of `$LAND_STATE/land/<pr>.log` is the status: `DONE`, `CI-RED`, `SWEEP-RED`, `HEAD-MOVED` or `TIMEOUT`.
- `fixmerge.sh <fix> <parent>` merges a stacked fix PR into its parent once the fix's `pull_request` CI is green, then relands the parent.

Rules the scripts follow:
- Judge CI by the `pull_request` run. The `push` run on the same SHA skips the mac job.
- A queued PR locks its head branch. Dequeue a parent before merging a fix into it, but never dequeue a PR that touches `.github/workflows/`: d-q222's token can't re-enqueue it.
- A waived sweep result is always named on the PR along with its issue (e.g. #2145).
