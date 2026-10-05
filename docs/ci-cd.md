# CI/CD: build, test, release and publish FlashTeX

GitHub Actions workflows live in `.github/workflows/`, backed by scripts in
`scripts/` and `scripts/ci/` that also run locally. `ci.yml` is **tiered**: the
gate a branch must pass is proportional to what it changed.

| Piece | What it does |
|---|---|
| `ci.yml` | Tiered. **`merge_group` runs only the gate** (target ≤ 12 min with engine parity, ≤ 5 without): workspace build, the touched crates' fmt/clippy/tests, the licence boundary, the bundled inventory, the generated-table gates, trip, etrip and pdfTeX's regression tests on Linux, and the **new engine's parity gates** (lockstep, P-T1/P-T2 on the fixtures, the engine's tests; three shards on the NixOS PC) when the group touches engine-affecting paths. **After the merge** (push to `main`, and nightly): the Mac app, the iPad simulator, the Rust workspace on Linux and macOS, `render-pipeline`, `flashtex-cli`, `flashtex-xetex`, every macOS leg and the old engine's parity fixtures, fixed forward through the `main-red` issue (the PR's owner fixes within 2 h or the Commander reverts). **Pull requests** run the gate's set (engine parity on GitHub-hosted Ubuntu), plus the Mac app, the iPad and the old engine's fixtures only when they touch `apps/`. |
| `nightly.yml` | On a schedule (08:17 UTC) and on demand: the heavy suites. The parity scoreboard's `arxiv` (149 pinned e-prints) and `templates` tiers on a self-hosted Mac with TeX Live; the new engine's **T2** (LaTeX suites) and fixture-wide **incremental soundness** test on the NixOS PC; the macOS legs that left the merge queue; the whole workspace in the **debug** profile on both OSes, which nothing else covers; the tests of every crate in `scripts/rust-test-exclude.txt` and the clippy of every crate in `scripts/clippy-debt.txt`, without gating, so a list that can shrink is noticed within a day. |
| `release.yml` | On a `v*` tag or a manual run with a version: builds the helpers, packages `FlashTeX.app` into `FlashTeX.dmg` (signed + notarized when the secrets exist), tars the CLI tools for macOS arm64 and Linux x86_64, publishes the GitHub release with `SHA256SUMS`, then points the website at it. |
| `site.yml` | On every published (non-prerelease) release, on a push to `main` touching `site/**`, and on demand: re-renders the whole site from `site/` (`site/render.py`) and pushes it to `gh-pages`. This is what makes the download page and both installers reflect a release; see [How the website is updated](#how-the-website-is-updated). |
| `scripts/gate.sh` | The local half of the tiered gates: `scripts/gate.sh {quick|pr|full}` runs exactly the steps CI runs for that tier, scoped to the crates your branch touches. Run `pr` before you push. See [Tiers](#tiers-and-scriptsgatesh). |
| `scripts/check-license-boundary.sh` | The GPL/MIT boundary of `DESIGN.md` §3: nothing but `crates/flashtex-engine` may have it in its `cargo metadata` dependency graph, and nothing under `apps/ios` may reference a GPL crate or link Rust artifacts. Run by CI and by `gate.sh pr`. See [The licence boundary](#the-licence-boundary). |
| `scripts/ci/build-helpers.sh` | Builds the `flashtex` CLI and every helper `apps/mac/scripts/make-app.sh` bundles in release mode and prints `FLASHTEX_<NAME>=<path>` lines (the variables the app and its tests read; `FLASHTEX_CLI` is the CLI). |
| `scripts/ci/package-cli.sh` | Stages `flashtex` (the CLI), `flashtex-render`, `flashtex-compiler`, `flashtex-pdf`, `flashtex-pdf-exact` plus the pinned Latin Modern faces and TFM metrics into `flashtex-cli-<version>-<platform>.tar.gz` with a README (layout below). |
| `scripts/ci/update-site.sh` | A narrower, older path: rewrites just the version/checksum/date in `install.sh`, `download/index.html` and `index.html` on `gh-pages` in place and pushes; called directly by `release.yml`'s `publish` job. `site.yml`'s full re-render (above) also runs on the same `release: published` event and fully overwrites `gh-pages` from `site/` right after, so it is what actually determines the final published page; see the note in [How the website is updated](#how-the-website-is-updated). |

## Tiers and `scripts/gate.sh`

CI is tiered (`docs/design/engine-v2/DESIGN.md` §9, owner-approved 2026-09-29).
The point of the tiers is that the gate a branch has to pass is proportional to
what it changed, and that an agent can run that gate locally in the time it takes
to read the diff, so CI only confirms a result that is already known.

`scripts/gate.sh` is that local half. **One script, the same steps as CI**:

```sh
scripts/gate.sh quick     # what you changed: fmt, clippy, tests
scripts/gate.sh pr        # quick + parity fixtures + the licence boundary
scripts/gate.sh full      # the whole workspace in both profiles + the Mac app
scripts/gate.sh pr --list # print the steps without running them
```

| Tier | Steps | Where it runs in CI |
|---|---|---|
| `quick` | `rustfmt` over the `.rs` files the branch changed; `cargo clippy -p <crate> --all-targets --no-deps -- -D warnings` for each crate it touches; `cargo test -p <crate>` for each crate it touches. If the root `Cargo.toml`/`Cargo.lock` changed, also one `cargo check --workspace --all-targets` and one workspace-wide clippy — but no workspace tests ([why](#what-scoped-to-what-you-changed-means)) | the `quick` job, `ubuntu-latest` |
| `pr` | `quick`, plus the licence boundary (`scripts/check-license-boundary.sh`), the parity scoreboard self-tests, the parity **fixtures** tier against `tools/parity/baseline-fixtures.json`, and the bundled-inventory sha256 | the pull-request set: `quick`, `boundary`, `inventory`, `gates` (`parity-fixtures` only when a pull request touches `apps/`) |
| `full` | `pr`, plus `cargo build --workspace --all-targets` and `cargo test --workspace` in **both** the debug and the release profile, the same for the two standalone crates, the generated-table manifest, and `swift build && swift test` for the Mac app | after the merge (push to `main`, nightly): the post-merge set. The merge queue runs only the gate (`quick`, `build`, trip/etrip/regression, engine parity) |

Every step prints its own result and the run ends with a table of
`PASS`/`FAIL`/`WARN`/`SKIP` and seconds per step, so a slow gate can be read
rather than guessed at. Exit status is 1 if any step failed; `WARN` and `SKIP`
do not fail the run, and the table says why each one is what it is.

### What "scoped to what you changed" means

`quick` derives the crates it runs from

```sh
git diff --name-only origin/main...HEAD
```

(the merge-base diff: what your branch added, not what `main` added) plus your
uncommitted edits, because the point of a local gate is to catch the problem
before the commit. `--committed-only` drops the uncommitted half; `--base <ref>`
compares against something other than `origin/main`.

A path under `crates/<dir>/` maps to that directory's cargo package. A crate
that has its own `Cargo.lock` is one of the three the root workspace excludes
(`Cargo.toml`), and `gate.sh` runs cargo inside it rather than with `-p`.

A change to the root `Cargo.toml` or `Cargo.lock` (adding a dependency, say) can
break any root-workspace member, but it does **not** fan out into per-crate
clippy and tests for all of them — that took 41–45+ minutes on a hosted runner
and timed out the 45-minute `quick` job (PR #1183). Instead `quick` adds two
workspace-wide steps, each one cargo invocation sharing one build:

* `cargo check --workspace --all-targets --locked`, and
* `cargo clippy --workspace --all-targets --no-deps --locked -- -D warnings`,
  with the root-workspace crates in `scripts/clippy-debt.txt` `--exclude`d
  (they are not gating anyway; the check step still compiles all their targets).

Tests still run only for crates whose own files changed, exactly as without a
manifest change. **The whole workspace's tests are the merge queue's job**: the
`rust workspace` job runs `cargo test --workspace --release --locked
--no-fail-fast` (minus `scripts/rust-test-exclude.txt`, which it still runs,
non-gating) on Linux and macOS for every `merge_group` event and every push to
`main`, so a resolve change that breaks some crate's tests cannot land. The
standalone crates have their own lockfiles and are unaffected by the root one.

### fmt is a regression gate, not a reformat

A repo-wide `cargo fmt --all --check` cannot be a gate yet: at `7a1ed08aa` it
reports 288 unformatted files across 22 crates. Reformatting them is its own
lane, not a tax on every PR. So the fmt step checks only the `.rs` files the
branch changed, and it **fails only when the branch made a file worse** — the
file is unformatted now and was formatted (or did not exist) at the base.
Pre-existing debt in a file you touched is printed as a warning.

The check is `rustfmt --edition <the crate's edition> --emit stdout` over the
file's bytes, compared with the file. Feeding rustfmt the file on stdin is what
keeps it from following `mod` declarations into files the branch never touched.
Passing the crate's own edition matters: rustfmt uses it as the style edition,
and 2021 and 2024 sort `use` lists differently. Verified against `cargo fmt
--all --check` over a 60-file sample: identical verdicts for every file in the
root workspace.

### clippy, and `scripts/clippy-debt.txt`

`--no-deps` is the load-bearing flag. Without it, `cargo clippy -p X` lints every
path dependency of `X` too, so one crate's findings fail every crate downstream
of it — `crates/compiler` has a deny-by-default `never_loop`, which by itself
made clippy unusable for most of the workspace. With `--no-deps`, a crate is
answerable only for itself.

`scripts/clippy-debt.txt` lists the crates that do not pass today: measured per
crate on 2026-09-29 at `7a1ed08aa` with clippy 0.1.98, **20 of the 35
root-workspace members already pass**, 15 do not, and neither do the three
standalone crates. A listed crate still runs and its findings are still printed,
but it does not fail the gate; when it starts passing, the gate says so and the
line must be deleted. Same arrangement, and same rule, as
`scripts/rust-test-exclude.txt`: **the list may only shrink.**

### `scripts/rust-test-exclude.txt`

The crates whose tests are temporarily not gating, one package name per line with
its issue. `ci.yml` and `gate.sh` both read this file, so there is one list rather
than two that drift. An excluded crate still has to build, all targets; CI runs
its tests anyway without gating and warns when one passes again.

### Before you push

```sh
scripts/gate.sh pr
```

If it passes, the required CI set will pass, with two exceptions worth knowing:

* the parity **fixtures** baseline was recorded on macOS (`tools/parity/README.md`
  — `unicode-accents` paints CJK from system fonts), so `gate.sh` skips that step
  on Linux and CI runs it on a Mac;
* `--locked` is used everywhere, so a `Cargo.lock` that your change did not
  update is a failure in CI even if a plain `cargo build` succeeds locally.

If `gate.sh` refuses to start with "these directories are inside the root
workspace glob but have no `Cargo.toml`", delete them: they are stale `target/`
output for a crate that was removed from the repository, and because the root
workspace globs `crates/*`, they make *every* cargo command in the checkout fail
with `failed to load manifest for workspace member`.


## The licence boundary

`DESIGN.md` §3 is the whole licensing argument of the product: an **MIT** Mac app
and an **MIT** iPad companion around a **GPL-2-or-later** engine, which they
reach only by running its host process and speaking `display-list-v3` over a
socket. Nothing MIT links the engine.

That is exactly the kind of property that breaks silently in one `Cargo.toml`
line and is expensive to discover late, so `scripts/check-license-boundary.sh`
runs on every pull request (the `licence boundary` job) and inside
`scripts/gate.sh pr`. It takes seconds and needs no build.

**A — nothing but the engine may depend on the engine.** It reads
`cargo metadata`'s resolve graph — what actually links, not what a manifest says
— reverses it, and walks out from `flashtex-engine`. Any other package that can
reach it is a violation, reported with the dependency path that gets there.

It does this in **every** cargo workspace in the repository, not just the root
one: `Cargo.toml` excludes `crates/render-pipeline`, `crates/flashtex-cli` and
`crates/perf-bench`, and those are where the shipped CLI lives, so checking only
the root workspace would leave the blind spot exactly where it matters.

All dependency kinds count. A `dev-dependency` still links GPL code into a test
binary and a `build-dependency` still links it into a build script.

`crates/flashtex-engine` is created by another lane. Until it exists the check
says so and passes; it starts enforcing the moment that directory lands.

**B — `apps/ios` references no GPL crate and links no Rust artifacts.** Four
sub-checks over the Xcode/SwiftPM build inputs (and, for B1, the Swift sources)
— build products under `.build/` and `DerivedData/` are not repository content
and are skipped, and so are whole-line comments, because `Package.swift`
explains in a comment why it does *not* use `.unsafeFlags` and a check that
cannot tell a comment from a declaration is a check people switch off:

| | |
|---|---|
| B1 | no GPL crate named, by cargo name or Rust library name. The GPL set is the engine plus any crate whose own `LICENSE` file or `license` field says GPL, so a second GPL crate is covered the day it appears |
| B2 | no `.binaryTarget`, `.systemLibrary`, `linkedLibrary`, `linkerSetting`, `unsafeFlags`, `.xcframework`, `.dylib`/`.a`, `OTHER_LDFLAGS` naming a library, or `LIBRARY_SEARCH_PATHS` — the mechanisms by which a Swift target could link a Rust artifact at all |
| B3 | no build input shells out to `cargo`, `rustc`, `rustup`, `build-helpers.sh` or a `target/{debug,release}` path |
| B4 | no symlink under `apps/ios` resolves into `crates/`. `apps/ios` legitimately symlinks three Mac-owned *Swift* trees (`Package.swift` documents them); one pointing into `crates/` would pull Rust — one day engine — sources into the iPad target |

**`scripts/license-boundary-allow.txt`** exists for the one legitimate case: a
crate that is *itself* GPL-2-or-later, ships its own GPL `LICENSE`, and is not
linked into any MIT product target — a GPL host-process binary, if the engine
lane ever splits the host out of the library. It is empty today. Adding a line
is a licensing decision, not a way to make a red gate green.

The owner arranges the legal review of §3 before public release; development
proceeds meanwhile.

### Checking it by hand

```sh
scripts/check-license-boundary.sh          # exit 1 on a violation
scripts/check-license-boundary.sh --list   # what it checks, without running
```

Verified against planted violations (an engine crate, a member depending on it,
an `.xcconfig` naming `libflashtex_engine.a`, a `.binaryTarget`, a script calling
`cargo`, a symlink into `crates/`): each one is reported with its file, line and
dependency path, and the allow-list entry turns the dependency finding into an
`ok` line naming the file that permits it.

## `ci.yml`

### Which tier, and why

On 2026-09-27 a run on `main` took **225–275 minutes**, almost all of it macOS
jobs *queueing*: every pull request ran the whole matrix and ~90 branches
competed for GitHub-hosted macOS runners. Tiering fixed pull requests. It did
not fix the merge queue: on 2026-10-04 a `merge_group` run still took **45–130
minutes** wall clock, because every group ran the Mac app (19–24 min on hosted
`macos-26`), the Rust workspace (16–22 min, on the PC), engine parity (13–19
min, on the PC), `render-pipeline` (7–8), the iPad simulator (6–8) and the
workspace build (on the PC) — and up to 8 groups at a time queued for the PC's
three runners (`build` waited 30 minutes for one in run
[37169695929](https://github.com/flash-tex/flashtex/actions/runs/37169695929),
`rust workspace` 36). Owner, 2026-10-05: *"30–60 mins per merge is absolutely
unacceptable … can't we merge more stuff in and test later as needed?"*

So since 2026-10-05 the workflow decides, once, in a `plan` job, between three
tiers:

| Event | Tier | What runs |
|---|---|---|
| `merge_group` | **gate** | the [merge-queue gate](#the-merge-queue-gate) only |
| `workflow_dispatch` with `gate_only` | **gate** | the same, to measure it (engine parity always) |
| `push` to `main` | **all** (post-merge) | the [post-merge set](#after-the-merge-and-main-red), plus the gate's cheap jobs again |
| `workflow_dispatch` | **all** | everything |
| `pull_request` (any fork, any branch) | **pr** | the [pull-request set](#the-pull-request-set) |
| `push` to a branch other than `main` | **pr** | the same |

Parity stays gated before merge (parity > speed): the merge queue never lands
an engine change whose lockstep, P-T1/P-T2 or engine tests are red. What moved
out of the queue is everything a regression there can be *fixed forward* from:
the Mac app, the iPad companion, the Rust workspace on both systems, the
standalone crates and every macOS leg.

`plan` also **deduplicates**. An in-repo branch with an open pull request gets two
events for the same commit; the `pull_request` run is the one that carries the
required checks, so the `push` run stands down (`plan` asks
`repos/.../pulls?state=open&head=…`). A branch with no pull request still gets the
whole pr set from its push, which is what makes `git push` a useful signal on
its own. `plan` writes its decision into the run summary, so "why did this run
do nothing" is one click, not an investigation.

### The merge-queue gate

Target: a `merge_group` run's critical path **≤ 12 minutes when the group
touches the engine, ≤ 5 minutes otherwise**.

| Job | Runner | What |
|---|---|---|
| `plan` | ubuntu | tier, deduplication, runner choice, path filters |
| `gate checks and engine T0 (ubuntu-latest)` | ubuntu | **one job, six gates**, each a step that runs even if an earlier one failed: the licence boundary (`scripts/check-license-boundary.sh`), the bundled inventory (a sha256 comparison of two files), the generated tables (`scripts/check-generated.py` plus `gen_tables.py --check`), and the engine's T0 tests — trip, etrip and pdfTeX's regression tests, self-contained. One job since 2026-10-05: as six jobs of under two minutes each they took six of the Free plan's 20 concurrent hosted slots per gate run |
| `build (workspace, all targets)` | ubuntu | in the gate **`cargo check --workspace --all-targets --locked`** (pull requests: `cargo build`). Kept in the gate because `quick` builds only the crates a change touches, and this is what catches a change that breaks a crate it did not touch; `check` because `cargo build` took 5.0–6.6 min here, over the gate's 5, while code generation and linking are covered by the post-merge `rust workspace` jobs. The push to `main` runs `check` too, so the gate restores a warm check cache (`workspace-debug-check`) |
| `quick (touched crates)` | ubuntu | `scripts/gate.sh quick --committed-only` on the group's diff: fmt, clippy and tests of the crates it changed (and `flashtex-xetex` when the engine or `tools/web2rust` changed) |
| `engine parity (GitHub-hosted, …)` | ubuntu, five shards | only when the group touches engine-affecting paths; see [below](#the-new-engines-parity-gates) |

The engine-affecting paths (`engine_paths` in `plan`): `crates/flashtex-engine/`
and its path dependency `crates/display-list-v3/`; the root `Cargo.toml` and
`Cargo.lock` (it builds `--locked` in the root workspace); `tools/web2rust/`,
`tools/lockstep/`, `tools/parity/` and the two directories `parity.py` imports
(`tools/visual-oracle/`, `tools/real-world-corpus/`); `third_party/`;
`fixtures/`; `scripts/engine-parity.sh`, `.github/actions/texlive-2026/` and
`ci.yml` itself. A merge group's diff is cumulative (main to the group's head),
so once an engine change is in the queue every group behind it runs parity too —
which is what testing that exact tree requires. A failed or capped file list
counts as touching everything.

Nothing else runs in the queue: no Mac, no iPad, no Rust workspace, no
standalone crates, no macOS leg of anything, and the old engine's parity
fixtures (a macOS job) never.

### The pull-request set

Unchanged for reviewers, with one exception:

| Job | When |
|---|---|
| everything in the gate's table above | always (engine parity on GitHub-hosted Ubuntu, sharded the same way, when the change touches engine-affecting paths) |
| `mac app (swift build + test)`, `iPad companion (…)` | only when the change touches `apps/` (`apple` in `plan`) |
| `old engine: parity fixtures (GitHub-hosted macOS)` | only when the change touches `apps/` **and** a path those fixtures depend on |

The exception: **hosted macOS jobs run on a pull request only when it touches
`apps/`** (the Mac app, the iPad companion and their fonts). One hosted macOS job
used to wait 11–25 minutes for a runner on every pull request that touched the
old engine's paths, and since the Mac app and the iPad left the merge queue, a
pull request that touches `apps/` is now where Swift gets gated before the merge.
A pull request's `CI required` must be green before it can join the queue, so an
`apps/` change still cannot land with a red Mac app; everything else that could
break it (a helper crate, the bundled fonts) is caught after the merge.

`quick` needs `fetch-depth: 0`, because `gate.sh` scopes itself with
`git diff <base>...HEAD` and a shallow clone has no merge base. The base is the
pull request's base commit, or `HEAD~1` on `main`, or `origin/main` otherwise.

### Measured: the gate, 2026-10-05

Run time of each gate job (queueing excluded), from this lane's runs on
2026-10-05: the manual `gate_only` run
[37345026625](https://github.com/flash-tex/flashtex/actions/runs/37345026625)
(the gate exactly as `merge_group` runs it, NixOS shards) and the pull-request
run [37346450319](https://github.com/flash-tex/flashtex/actions/runs/37346450319)
(the same jobs, engine parity on GitHub-hosted shards). The PC was busy with
merge-queue groups still on the old workflow throughout.

| Job | GitHub-hosted | NixOS PC |
|---|---:|---:|
| `plan`, boundary, inventory, generated tables | ≤ 0.4 min each | — |
| `quick (touched crates)` (a workflow + script change: no crates) | 0.6–3.1 min | — |
| trip / etrip / pdfTeX regression (Linux) | 0.9–1.7 min | — |
| `build (workspace, all targets)` | 5.0–5.3 min (cache from a restore key) | — |
| engine parity, `lockstep` shard | 2.0–2.9 min (build 56 s, 1,466 cases in 4 slices: 22 s) | **2.7 min**, after waiting 54 min for a runner |
| engine parity, `P-T fixtures` shard | 3.5–5.6 min (build 87 s, 86/86 P-T1 and P-T2: 193 s) | **5.3 min** (build 58 s, parity 242 s), after waiting 7 min |
| engine parity, `engine tests` shard | 5.6–5.9 min | **9.8 min**, after waiting 30 min |
| **critical path** | **~6 min** | **9.8 min** of work, and 7–54 min of waiting for one of the PC's three runners |

So a `merge_group` run that touches the engine was bounded by its slowest
parity shard: about 6 minutes hosted, 9.8 on the busy PC (target ≤ 12) — before
the waits, which on the PC dwarfed the work. That is why the gate's parity moved
to hosted runners (the second change, below). One that does not touch it was
bounded by
`build`, about 5 minutes (target ≤ 5; `quick` grows with the crates a group
touches). Before, the same queue entries' slowest jobs were the Mac app (18.6–22
min of work) and the Rust workspace (11.8–12.2), and their wall clock was 20–48
minutes on 2026-10-04 (45–130 across the day), most of it waiting for the PC.
What remains is queueing: with three PC runners and three shards per engine
group, groups behind an engine change wait for runners; the hosted twin needs no
queue and stays inside the target (above).

**After the second change (same day): everything hosted, five shards, `cargo
check`.** Manual `gate_only` run
[37350251998](https://github.com/flash-tex/flashtex/actions/runs/37350251998),
the gate exactly as `merge_group` now runs it, all GitHub-hosted:

| Job | Run time |
|---|---:|
| `build (workspace, all targets)` (`cargo check`, cold check cache) | 1.5 min |
| `quick`, boundary, inventory, generated tables, trip, etrip, pdfTeX regression | 0.1–1.6 min each |
| engine parity, `lockstep` (build 87 s, 1,466/1,466 cases in 4 slices: 36 s) | 2.9 min |
| engine parity, `P-T fixtures 1/2`, `2/2` (build 81–83 s, 43/43 + 43/43 at P-T1 and P-T2: 91–97 s) | 3.9–4.4 min |
| engine parity, `engine tests 1/2`, `2/2` (147 s and 185 s) | 3.7–3.9 min |
| **critical path (slowest job's run time)** | **4.4 min** with engine parity, **1.6 min** without |

The run's wall clock was 8.8 minutes: its jobs waited 1–3 minutes each for a
runner. The organisation is on GitHub's Free plan, which runs at most **20
GitHub-hosted jobs at a time** across the repository, and a gate run is up to 15
jobs (9 without engine parity) — so with several merge groups in flight, the
binding constraint is that pool, not any one job. The levers, in order: fold the
sub-minute jobs (boundary, inventory, generated tables, trip, etrip, pdfTeX
regression) into one or two jobs; lower the queue's `max_entries_to_build` from
8; or a paid plan's larger pool.

History: on 2026-09-30 the then "fast set" of a scripts-and-docs pull request
took 3 min 11 s on Linux plus one GitHub-hosted macOS job that waited 11 minutes
for a runner (run
[36530915101](https://github.com/flash-tex/flashtex/actions/runs/36530915101)).
That queue is why no hosted macOS job runs on a pull request that does not touch
`apps/`.

### The old engine's parity fixtures

The old engine (`flashtex-cli`) is frozen (D13, fixes only), so almost no change
can move its fixtures tier, and the job needs a Mac
(`tools/parity/baseline-fixtures.json` was recorded on macOS, and
`unicode-accents` paints CJK from system fonts, so a Linux run would compare
against another host's numbers — the mistake `DESIGN.md` §8 forbids). `plan`
therefore computes `old_engine`:

* **`true` after the merge** — every push to `main`, and manual runs. On `main`
  a failure opens or updates the `main-red` issue like every
  [post-merge job](#after-the-merge-and-main-red).
* **Never in the merge queue.**
* **On a pull request or branch push, `true` only when the change touches
  `apps/` (the hosted-macOS rule) and a path the tier depends on.** A pull
  request's file list comes from the API; a failed call, or a list at the API's
  cap, counts as touching everything. The paths:
  * `crates/<name>/` for `flashtex-cli` and every path crate in its
    `cargo metadata` dependency graph, dev-dependencies included (21 crates,
    listed as `old_engine_crates` in `plan`);
  * the root `Cargo.toml` and `Cargo.lock` (the CLI builds in the root
    workspace, `--locked`);
  * `tools/parity/` and the two directories it imports from,
    `tools/visual-oracle/` and `tools/real-world-corpus/`;
  * `fixtures/` (the documents and their pdflatex references) and
    `apps/mac/Fonts/` (the font directory `parity.py` hands the CLI);
  * `.github/actions/parity-fixtures/` and `.github/workflows/ci.yml`.

`plan` runs before any checkout, so the crate list is written out by hand.
`scripts/ci/check-old-engine-paths.sh` compares it with `cargo metadata` and
fails if a crate is missing; the action runs it. A new path dependency can only
be added in a `Cargo.toml` already on the list, so the check runs exactly when
the list can go stale. `CI required` fails a run where `old_engine` is `true`
and neither of the two jobs passed; both skipped is a pass when it is `false`.

### The new engine's parity gates

Until 2026-09-30 no CI job checked the new engine's parity: the "parity
fixtures" job builds the *old* `flashtex-cli`, lockstep (T1), P-T1/P-T2, T2 and
the incremental soundness sweep were one-off lane runs, and the engine's TeX
Live tests printed "no TeX Live found; skipping" and passed (review 2026-09-30,
tracks 1 and 5). `scripts/engine-parity.sh` is now the gate, and CI runs it:

| Step | What |
|---|---|
| `build` | `cargo build --release -p flashtex-engine` (`flashtex-initex`), then `pdflatex.fmt` and `pdftex.fmt` from TeX Live's `.ini` files as fmtutil makes them |
| `lockstep` | `tools/lockstep`, every case (1,466 on 2026-10-05), against TeX Live's `pdftex`, in `--jobs` parallel slices |
| `parity` | the parity fixtures tier (83 documents) at P-T1 and P-T2 against TeX Live's pdfTeX 1.40.29; `--require-pt` fails unless every measured document passes both |
| `tests` | `cargo test --release -p flashtex-engine` with `FLASHTEX_REQUIRE_TEXLIVE=1`: a test that would skip for want of TeX Live **fails** |
| `t2`, `soundness` | nightly only (below) |

It needs TeX Live 2026 first on `PATH` (it refuses another pdfTeX, or another
TeX Live's `kpsewhich` earlier on `PATH`), `python3` and `qpdf`.

Shards run it, one job each, at once (2026-10-05; as one job it took 10–21
minutes on a busy PC — build 212 s, lockstep 499 s, parity 192 s, tests 348 s in
run [37322818362](https://github.com/flash-tex/flashtex/actions/runs/37322818362)):

| Shard | Steps (`scripts/engine-parity.sh …`) |
|---|---|
| `lockstep` | `build lockstep`; lockstep's ~1,500 cases split into `--jobs` interleaved slices run at once (`tools/lockstep/run.py --shard K/N`; it used to run them one after another) |
| `P-T fixtures 1/2`, `2/2` | `--shard 0/2 build parity`, `--shard 1/2 build parity`: every other fixture (`parity.py --shard`) |
| `engine tests 1/2`, `2/2` | `--shard 0/2 tests`, `--shard 1/2 tests`: every other test target of `flashtex-engine` (the library, the binaries, each `tests/*.rs`; the doc tests in slice 0), because cargo runs test binaries one after another |

`--shard K/N` is 0-based, as `parity.py`'s and `run.py`'s are. The N slices of a
step together are the whole step. As three shards (one each) the P-T fixtures
and engine tests shards took 5.4–5.9 min on hosted runners and set the critical
path; split in two, see the measurements above. The gate's critical path is the
slowest shard, not the sum. Two jobs carry the shards:

* **`engine parity (GitHub-hosted, <shard>)`**, five shards: every event that
  needs parity — the merge queue (since the second 2026-10-05 change: on the
  PC, up to 8 groups queued for three runners; hosted, the shards start at
  once), pull requests and every push to `main`. Described below.
* **`engine parity (NixOS, <shard>)`**, three shards (lockstep, P-T fixtures,
  engine tests), on the `flashtex-linux` runners (the owner's PC, TeX Live 2026
  first on their `PATH`): only a full manual run (`workflow_dispatch` without
  `gate_only`, PC switch on) — never a pull request (§9.3). Each shard keeps a **per-runner**
  `CARGO_TARGET_DIR` under `~/.cache/flashtex-ci/<runner>/` (the warm cache: a
  run rebuilds only what changed). Per runner, not one shared directory: cargo
  locks a target directory for a whole build, so shards sharing one would build
  in turn. No sccache: it is not on the PC, and the crate a change rebuilds is
  the engine itself, which sccache would miss anyway. The oracle's references
  are not kept: a cancelled run once cached a truncated oracle log, and every
  later run failed P-T1 on it. The fixtures shard runs at `-j 4`, because each
  parity worker holds up to ~2 GB while it compares a `\tracingall` log and
  each runner is a systemd service with `MemoryHigh=9G` (at `-j 8` the fixtures
  tier crawled for 14+ minutes); the lockstep shard at `-j 8` (small
  processes). `fail-fast`: a red shard frees the other two runners at once.
* The **GitHub-hosted** shards run on `ubuntu-latest` with TeX
  Live 2026 from the `texlive/texlive:latest-medium` image **pinned by digest**
  (`.github/actions/texlive-2026`), plus the two packages the fixtures read that
  scheme-medium lacks (`sansmathaccent`, `translations`), each pinned by the
  SHA-256 of its tlnet archive. The package list comes from `pdflatex -recorder`
  on every fixture, mapped through `texlive.tlpdb`. The tree and the oracle's
  references are kept in the Actions cache (the latter saved only by a green
  job, by each fixtures shard under its own key, falling back to any earlier
  oracle cache of the image; the three `build` shards share one Rust cache).
  The push to `main` matters for the caches: they are scoped by ref, a pull
  request or merge group can restore only its own and `main`'s, and without a
  `main` run every one would start cold (the TeX Live tree is 549 MB in the
  cache, the oracle's references 428 MB).

`CI required` fails a `merge_group` run in which `plan` set `engine` and
neither job passed in full (a matrix job passes only when every shard does), so
the new engine's parity is required in the merge queue whenever the group can
have changed it.

Measured as one unsharded job on 2026-09-30 (260 lockstep cases then; the PC was
shared with other lanes):

| Job (run) | TeX Live | build | lockstep | parity | tests | job total |
|---|---|---:|---:|---:|---:|---:|
| NixOS, `-j 4` ([36685138918](https://github.com/flash-tex/flashtex/actions/runs/36685138918)) | on the runner | 55 s | 51 s | 134 s | 70 s | **5 min 22 s** |
| hosted, cold caches ([36679362232](https://github.com/flash-tex/flashtex/actions/runs/36679362232), attempt 1) | 58 s (pull) | 94 s | 20 s | 259 s | 113 s | **10 min 2 s** |
| hosted, warm ([36679362232](https://github.com/flash-tex/flashtex/actions/runs/36679362232), attempt 2) | 7 s (cache) | 97 s | 20 s | 182 s | 109 s | **7 min 32 s** |
| hosted, warm ([36681366950](https://github.com/flash-tex/flashtex/actions/runs/36681366950)) | cache | 80 s | 10 s | 107 s | 63 s | **5 min 13 s** |
| hosted, cold caches ([36685138918](https://github.com/flash-tex/flashtex/actions/runs/36685138918)) | pull | 85 s | 19 s | 257 s | 106 s | **9 min 23 s** |

Every run: lockstep 260/260, P-T1 83/83, P-T2 83/83, all engine tests passing.
Lockstep has since grown to 1,466 cases, which is what pushed the single job
past 20 minutes and why it is sharded now.

### After the merge, and `main-red`

Since 2026-10-05 these run on every **push to `main`** (and on manual runs), not
in the merge queue:

| Job | Runner |
|---|---|
| `mac app (swift build + test)`, `iPad companion (…)` | hosted `macos-26` |
| `rust workspace (ubuntu-latest)` | ubuntu (GitHub-hosted since 2026-10-05: nobody waits on it, and on the PC it held a runner the gate needed) |
| `rust workspace (macos-15)` | self-hosted Mac, else hosted `macos-15` |
| `rust render-pipeline`, `rust flashtex-cli`, `rust flashtex-xetex` (both systems) | ubuntu; self-hosted Mac, else hosted `macos-15` |
| `gate checks and engine T0 (macos-15)` (trip, etrip, pdfTeX regression) | self-hosted Mac, else hosted `macos-15` |
| `old engine: parity fixtures (…)` | self-hosted Mac, else hosted `macos-15` |
| `engine parity (GitHub-hosted, …)` | ubuntu, three shards |

plus the gate's cheap jobs again. `nightly.yml` repeats the macOS legs and the
debug-profile workspace (`macos-legs`, `workspace-debug`).

**`main-red`.** On a push to `main`, the `main-red issue` job runs
`scripts/ci/main-red.sh` after every other job: if any job failed or timed
out, it opens an issue labelled **`main-red`** (or comments on the open one)
naming the run, the commit, the failed jobs and **the pull requests the push
merged** — the `Merge pull request #N` commits between the push's `before` and
`after`, each with its title, author and branch (the branch names the agent). A
fully green run on `main`'s *current tip* closes it; an older run finishing late
cannot, because runs on `main` overlap. `nightly.yml`'s failures comment on the
same issue (`--only 'macos|debug profile' --no-close`) but never close it: a
night covers a few legs, not everything. The label replaces `main-macos-red`
(macOS legs only, 2026-09-30 to 2026-10-05); the script closes a leftover
`main-macos-red` issue on the next green run.

**Policy.** A red `main` is fixed forward, and fast:

* **The owner of the merged pull request that broke it fixes forward within
  2 hours** — a fix through the merge queue like any other change. The issue
  names the candidate pull requests; when a push merged several, their owners
  sort out between them whose it is, from the failed job's log.
* **Otherwise the Commander reverts the merge** (a revert pull request through
  the queue), and the owner re-lands the change with its fix.
* The 2 hours run from the issue's comment for that run. A pull request that
  touches `apps/` already ran the Mac app and the iPad before the merge, so a
  red Mac app on `main` is most often a crate change underneath it.

Why moving them is safe enough: in the 48 hours before the macOS legs first
left the queue (2026-09-30; 165 CI runs), GitHub-hosted macOS legs passed 224
times and failed 0; self-hosted macOS legs failed 62 times, every one the
runner's environment, not the code. The Linux workspace, the Mac app and the
iPad move on the owner's direction (2026-10-05) that merge latency now costs
more than a fix-forward: the gate keeps every check that guards parity, and
`build` plus `quick` keep a change from landing without compiling or with its
own crates' tests red.

Manual runs (`workflow_dispatch` without `gate_only`) still run every job on
both systems.

### `CI required`, the one check to require

`ci-required` runs on every event, `needs` every other job, and fails if any of
them failed or was cancelled. A **skipped** job is fine: that is how a tier says
"not for this event".
Two gates are stricter, because a filter must not be able to skip them by
accident: in the merge queue, whenever `plan` set `engine` (the group touches
engine-affecting paths — or `plan`'s outputs are missing), one of the new
engine's two parity jobs must have passed with every shard; and whenever `plan`
set `old_engine` one of the old engine's two parity fixtures jobs must have
passed.

In the merge queue `CI required` therefore reflects the gate alone: every
post-merge job is `skipped` there, so it reports as soon as the slowest gate job
finishes. On a push to `main` it covers the post-merge set too (a red post-merge
job makes `main`'s commit status red as well as filing `main-red`).

Require *this* check in branch protection and in the merge queue, not the
individual job names. A required check that never runs blocks the queue forever,
and every job added, renamed or made conditional would otherwise mean another
branch-protection edit. The ruleset (`main: CI required + merge queue`) requires
exactly `CI required`, and the 2026-10-05 change keeps that name; the job names
it changed are not required anywhere: `engine parity (NixOS)` and `engine parity
(GitHub-hosted)` became one check per shard (`engine parity (NixOS, lockstep)`,
`… P-T fixtures)`, `… engine tests)`, and the same for `GitHub-hosted`), and
`main-macos-red issue` became `main-red issue`.

### Self-hosted Macs

`DESIGN.md` §9.3. The team's Apple Silicon Macs are idle most of the day and are
several times faster at the Swift and Rust suites than a hosted runner they have
to queue for. A **public** repository makes that a security question, and the
answer here has four parts:

1. **Fork pull requests never reach a self-hosted runner.** The job that uses
   `runs-on: [self-hosted, macOS, ARM64, flashtex]` carries the guard
   ```yaml
   if: github.event_name != 'pull_request' ||
       github.event.pull_request.head.repo.full_name == github.repository
   ```
   and `plan` will not select a self-hosted runner for a fork pull request
   either. Full-tier jobs are unreachable from `pull_request` altogether.
2. **Outside contributors need approval before any workflow runs.** The
   repository is set to
   `actions/permissions/fork-pr-contributor-approval` →
   `approval_policy=all_external_contributors`.
3. **A GitHub-hosted fallback is the default, not the emergency path.** The
   repository variable `FLASHTEX_SELFHOSTED_MAC` selects between them for the
   Macs, and `FLASHTEX_SELFHOSTED_LINUX` for the NixOS runners; while
   `FLASHTEX_SELFHOSTED_LINUX` is unset it follows `FLASHTEX_SELFHOSTED_MAC`
   (the single switch both shared until 2026-10-01). They are variables and not
   autodetection because `GITHUB_TOKEN` has no `administration` scope: no
   workflow can list the repository's runners. Until the Commander sets a switch
   to `1`, that side's jobs run on GitHub-hosted runners, so a runner can be
   registered and watched before anything depends on it, and one side being
   offline never holds the other back.
4. **Every job gets a fresh working directory.** See below.

### Installing a runner on another Mac

`scripts/ci/install-selfhosted-runner.sh` does the whole thing. It must be run
**by the Mac's own user, not with `sudo`** — it refuses as root, and installs a
per-user launchd agent, not a system daemon.

```sh
# 1. Check what it would do. Resolves the release, downloads it, verifies the
#    SHA-256 against the hash published in that release's notes, and stops.
scripts/ci/install-selfhosted-runner.sh --dry-run

# 2. Install and start it. `gh` must be authenticated as a user with admin on
#    the repository: the registration token comes from `gh api`.
scripts/ci/install-selfhosted-runner.sh

# 3. Verify, in this order.
(cd ~/Library/Application\ Support/flashtex-actions-runner/runner && ./svc.sh status)
launchctl list | grep actions.runner
gh api repos/flash-tex/flashtex/actions/runners \
  --jq '.runners[] | {name, status, busy, labels: [.labels[].name]}'

# 4. Only once a runner shows `online`, point the workflows at it. This moves
#    the Mac jobs only; if the NixOS runners are offline, first pin their own
#    switch to 0 (it follows this one while it is unset).
gh variable set FLASHTEX_SELFHOSTED_LINUX --repo flash-tex/flashtex --body 0
gh variable set FLASHTEX_SELFHOSTED_MAC --repo flash-tex/flashtex --body 1

# To remove it again (stops the agent, deregisters, deletes the install):
scripts/ci/install-selfhosted-runner.sh --uninstall
```

What the installer sets up, and why:

* **An exact, verified version.** It resolves a release of
  `github.com/actions/runner`, reads the SHA-256 that release publishes in its own
  notes, and aborts before unpacking anything if the download does not match. It
  then registers with `--disableupdate`, because a self-updating runner would
  replace the binary whose hash was verified. Moving to a new version is
  `--uninstall` and then `--version vX.Y.Z`.
* **Labels `self-hosted,macOS,ARM64,flashtex`**, matching `runs-on` in the
  workflows and `.github/actionlint.yaml`.
* **An ephemeral working directory per job.** The runner is persistent, but the
  `JOB_COMPLETED` hook removes the job's workspace and temp directory, and
  `JOB_STARTED` clears them again in case a previous job died without running its
  hook. Both hooks refuse to delete anything that is not under a `_work`
  directory. `_actions` and `_tool` — the runner's caches of downloaded actions
  and toolchains, which hold no repository state — are kept.
* **Cargo caches that survive the wipe**, in
  `~/Library/Caches/flashtex-actions-runner/{cargo,rustup}` via `CARGO_HOME` and
  `RUSTUP_HOME` in the runner's `.env`. That is where the time goes: the
  crates.io registry, the git checkouts, the toolchains.
* **No `CARGO_TARGET_DIR`.** It is no longer needed to keep two package sets
  apart -- `crates/render-pipeline/vendor/` is retired, so there is exactly one
  set of package names and one workspace target directory. Setting it would still
  be wrong here, because two concurrent jobs on the same machine would then share
  and lock one directory. Build caching comes from `Swatinem/rust-cache`.
* A runner registered **per repository**, so another repository cannot schedule
  work on the machine.

The residual risk is stated plainly: this is a machine on your network running
code from this repository, and anyone who can push a branch here can run commands
on it. Fork pull requests are excluded rather than sandboxed.

Set the machine to stay awake and logged in — a user agent stops at logout:

```sh
sudo pmset -a sleep 0 disablesleep 1      # on a Mac that is always on mains
```

### The NixOS PC (the gate's engine parity)

Owner, 2026-09-29: "any tests that can be run on the nixos pc should be run
there, as all other machines are laptops". The PC (AMD 7800X3D, 16 threads,
30 GB) runs **three** runner instances, `nixos-7800x3d`, `nixos-7800x3d-2` and
`nixos-7800x3d-3`, labelled `self-hosted, Linux, X64, flashtex-linux`, each a
per-user systemd service (`flashtex-actions-runner[-N]`) with its own runner
root, `_work` directory, `CARGO_HOME` and `RUSTUP_HOME` — never shared, because
a shared rustup broke concurrent builds on the Macs. Every job's `PATH` starts
with TeX Live 2026 (`~/texlive/2026/bin/x86_64-linux`), then the Nix profile
(qpdf, pdftoppm, git, python3).

All three instances run in one `flashtex.slice`, which caps the runners' combined work
(every build and test they start) at 20 GB of memory and, by owner request on
2026-10-04, **50% of the PC's CPU** (`CPUQuota=800%` on 16 threads; a hard limit). Set
`FLASHTEX_RUNNER_CPU_PERCENT` when installing, or change it live with
`systemctl --user set-property flashtex.slice CPUQuota=<N>%`.

Routing uses the same eligibility as the Macs but its own switch,
`FLASHTEX_SELFHOSTED_LINUX` (unset: it follows `FLASHTEX_SELFHOSTED_MAC`):
`plan`'s `selfhosted_linux` output is `true` for merge_group, push to main and
workflow_dispatch when that switch is `1` — never for `pull_request` or a
branch push. In `ci.yml` the PC now runs **only engine parity on a full manual
run** (`engine_nixos` in `plan`); nothing in the merge queue waits for it. Until
2026-10-05 it ran the workspace build, the Linux leg of the Rust workspace and
engine parity in every merge group, and up to 8 groups at a time queued for its
three runners (up to 50 minutes; with the slice's CPU cap, the three runners also
share 8 cores). Owner, 2026-09-30: hosted Linux is free for this public
repository, so every Linux job of `ci.yml` runs on `ubuntu-latest`. In `nightly.yml`
(schedule, workflow_dispatch) the Linux debug workspace, excluded crates, clippy
debt and the parity scoreboard's arxiv tier (its templates tier stays on a Mac;
see `nightly.yml` below). `plan` and `CI required` stay on hosted Ubuntu.

Two NixOS specifics: the engine's `build.rs` records the C++ runtime's directory
as an rpath when it lies outside `/usr` and `/lib` (without it every engine
binary failed to load `libstdc++.so.6`). (`gates` used to take Python 3.12
from nixpkgs there; it is hosted-only now and uses `actions/setup-python`.)

```sh
# From a Mac with gh (admin on the repository); the token never reaches a terminal.
scp scripts/ci/install-selfhosted-runner-nixos.sh kubar@nixos:
gh api -X POST repos/flash-tex/flashtex/actions/runners/registration-token --jq .token |
  ssh kubar@nixos 'IFS= read -r FLASHTEX_RUNNER_TOKEN; export FLASHTEX_RUNNER_TOKEN;
                   bash ~/install-selfhosted-runner-nixos.sh --instance 2'
# Re-provision an existing instance (hooks, .env, .path, unit) without re-registering:
ssh kubar@nixos 'bash ~/install-selfhosted-runner-nixos.sh --instance 1 --no-restart'
```

### Merge queue and branch protection on `main` — PREPARED, NOT APPLIED

`DESIGN.md` §9.2 asks for branch protection plus a GitHub merge queue on `main`.
**This lane deliberately did not apply it**, and neither should anyone until the
tiered workflow above is on `main`: a required check named `CI required` does not
exist on `main` yet, so requiring it would block every merge, including the merge
that would land the workflow.

Order matters:

1. Land the tiered `ci.yml` on `main`.
2. Confirm that a run on `main` produced a check named exactly `CI required`:
   ```sh
   gh run list --branch main --workflow CI --limit 1
   gh api "repos/flash-tex/flashtex/commits/$(git rev-parse origin/main)/check-runs" \
     --jq '.check_runs[].name' | sort -u
   ```
3. Only then apply the ruleset.

The whole configuration is one repository **ruleset**, because the merge queue
has no field in the classic branch-protection endpoint — it lives in rulesets
(and in the web UI). The exact file is committed as
[`.github/branch-protection/main-ruleset.json`](../.github/branch-protection/main-ruleset.json),
so what gets applied is reviewable rather than retyped:

```sh
# Apply (creates the ruleset):
gh api -X POST repos/flash-tex/flashtex/rulesets \
  --input .github/branch-protection/main-ruleset.json

# Verify:
gh api repos/flash-tex/flashtex/rulesets --jq '.[] | {id, name, enforcement}'
gh api "repos/flash-tex/flashtex/rulesets/<id>" \
  --jq '{name, enforcement, conditions, rules: [.rules[] | {type, parameters}]}'

# Change it later (same body, PUT to its id), or take it off again:
gh api -X PUT    "repos/flash-tex/flashtex/rulesets/<id>" --input .github/branch-protection/main-ruleset.json
gh api -X DELETE "repos/flash-tex/flashtex/rulesets/<id>"
```

What that body says, and why:

| Rule | Value | Why |
|---|---|---|
| `required_status_checks` | `CI required`, `strict_required_status_checks_policy: false` | one name to require (see above). **`strict` must be false** with a merge queue: the queue is what tests the branch against the tip, and "require branches to be up to date" fights it |
| `merge_queue` | `merge_method: MERGE`, `grouping_strategy: ALLGREEN`, `max_entries_to_build: 3`, `min_entries_to_merge: 1`, `min_entries_to_merge_wait_minutes: 5`, `check_response_timeout_minutes: 240` | §9.5: coherent landings, no stacking, **at most 3 branches in CI at a time**. `ALLGREEN` means a failing entry does not drag the ones behind it down with it. The response timeout has to exceed the full tier's slowest job plus GitHub-hosted macOS queueing; 60 minutes evicted green entries (#1193, #1199) while they waited for a runner. Lower it once self-hosted Macs run the queue |
| `pull_request` | `required_approving_review_count: 0` | a merge queue requires a pull request, and this repository's landings are agent-driven; the Commander raises this the day there are human reviewers to wait for |
| `deletion`, `non_fast_forward` | — | `main` cannot be deleted or force-pushed |

Commander decisions (2026-09-29):

* **`merge_method: MERGE`**, and `merge` is the only allowed method. `main` keeps
  its merge-commit history, and every commit keeps its own `Implementation-Agent` /
  `Commit-Executor` trailers (AGENTS.md provenance), which a squash would collapse.
  `required_linear_history` is therefore not set.
* **Bypass: repository admins (`RepositoryRole` 5, `always`).** It exists only so the
  Commander can write control files (`coordination/authority.json`) during a failover
  when CI itself is broken. Code and design changes go through the queue like
  everything else.

The repository already has an unrelated ruleset, `Simple protections`
(id `23198678`, `deletion` + `non_fast_forward`, with an empty `ref_name.include`
so it targets nothing). This body is a *new* ruleset and does not touch it;
consider deleting that one afterwards so there is a single place to look.

Finally, the setting this lane **did** apply, because it is a prerequisite for
self-hosted runners and blocks nothing:

```sh
gh api -X PUT repos/flash-tex/flashtex/actions/permissions/fork-pr-contributor-approval \
  -f approval_policy=all_external_contributors
gh api repos/flash-tex/flashtex/actions/permissions/fork-pr-contributor-approval
#   -> {"approval_policy":"all_external_contributors"}   (was "first_time_contributors")
```

### `nightly.yml`

Scheduled at 08:17 UTC, and `workflow_dispatch` with a `parity_tiers` input.
Concurrency is serialised (`cancel-in-progress: false`), so a run that overruns
into the next night makes the next one wait rather than doubling the load on one
Mac.

* **parity scoreboard (arxiv + templates)** — needs a real `pdflatex` and ~450 MB
  of fetched, hash-verified sources, and no GitHub image ships TeX Live, so it is
  self-hosted only, one leg per tier. **arxiv runs on the NixOS PC**: DESIGN §8's
  "never record host-dependent data on a different host" is kept because the
  tier has no committed baseline — the e-prints are SHA-pinned and the reference
  PDFs are made in the run by the PC's own pdflatex — unlike the fixtures tier,
  whose `baseline-fixtures.json` was recorded on macOS and so stays on macOS.
  **templates stays on a Mac**: its manifest pins 20 files of the Mac's MacTeX
  2026 tree by path and SHA-256, and on the PC's TeX Live 2026 snapshot 7 of them
  are missing or differ. Each leg's scores are its host's (system-font documents
  differ), so compare a leg's nightly scoreboards with each other only. Each leg
  has its own switch (arxiv `FLASHTEX_SELFHOSTED_LINUX`, templates
  `FLASHTEX_SELFHOSTED_MAC`); for a leg that is off, a companion job says so in
  the summary instead of leaving an empty run.
* **workspace, debug profile** (ubuntu × macos-15) — `ci.yml` tests *release*.
  Debug is the profile with `debug_assert!` and integer-overflow checks on, so an
  overflow the release build wraps silently is only ever caught here.
* **engine: LaTeX suites (T2)** and **engine: incremental soundness (every
  fixture)** — on the NixOS runners, through `scripts/engine-parity.sh build t2`
  and `scripts/engine-parity.sh soundness` (the `#[ignore]`d
  `every_fixture_edits_equal_scratch_compiles`: 83 documents, 171 compiles
  compared, 94 s on the PC), with `FLASHTEX_REQUIRE_TEXLIVE=1`. When
  `FLASHTEX_SELFHOSTED_LINUX` is not 1, a companion job says so.
  T2's baseline is TeX Live's own `pdftex` **on the runner's TeX Live**, run
  first in the same job (`tools/latex-suites/compare_failures.py`): the engine
  may fail no test the reference passes. `EXPECTED-FAILURES.txt` alone cannot
  be the gate there, because it matches the TeX Live `PINS.txt` names (LaTeX
  2025-11-01) and the PC has a newer one (LaTeX 2026-06-01, expl3 2026-09-09).
  Measured on the PC: the reference and the engine each fail the same 22 tests
  (1,509 of 1,531 executions pass), 13 of them outside `EXPECTED-FAILURES.txt`
  (`tikz-001`–`008`, `github-1398`, `m3graphics001`, `test`, `test-footnote`,
  `tlb-varioref-005`); about 24 minutes per run.
* **engine: T2, soundness and host memory (self-hosted Mac)** — the same three
  gates (`engine-parity.sh build t2`, `engine-parity.sh soundness`,
  `tools/incr-bench/mem_gate.sh`) one after the other in one job on a
  self-hosted Mac, used only on main while `FLASHTEX_SELFHOSTED_LINUX` is 0
  and `FLASHTEX_SELFHOSTED_MAC` is 1 (lane P5-BOARD-MAC, 2026-10-03). Every
  heavy Mac job (this one, the templates leg and `p5-scoreboard.yml`'s Mac
  route) is pinned to the runner labelled `flashtex-heavy` (mac-m1max-a-2);
  mac-m1max-a-1 stays general and serves the merge queue. They also share the
  concurrency group `flashtex-mac-heavy`, so only one runs at a time (the
  nightly waits for the 06:47 board), and this job `needs` the templates leg
  so the nightly never has two jobs pending in the group (a third pending job
  would cancel the earlier one). Its oracle is the Mac's MacTeX 2026: the same pdfTeX 1.40.29 as
  the PC but another snapshot (LaTeX 2025-11-01 on mac-m1max-a, the suites'
  `PINS.txt`, against the PC's 2026-06-01), so its T2 results are not the
  PC's. All Cargo output goes to one size-capped directory
  (`scripts/ci/mac-heavy-target.sh`). **T4 (`corpus-t4`) and the arxiv leg
  stay PC-only**: T4 runs most of a day with 8 workers and its ratchet
  baseline is bound to the PC's machine id and TeX Live, so a Mac would need
  its own baseline recorded first (an owner decision); the Mac's P5 board
  already measures the arxiv tier for both engines, so a second arxiv run
  would only double the Mac's load.
  **Advisory (owner decision):** on the Mac route the P5 board compiles
  third-party arXiv e-prints with pdfTeX and both engines **as the owner's
  user account** (the runners run under it, decision 6), and TeX can read any
  file that account can (`openin_any = a`). The job holds no write token and
  its checkout keeps no credentials, but the account's own files are within
  reach. A dedicated unprivileged runner user for the `flashtex-heavy` runner
  would be safer; whether to set one up is the owner's call.
* **macOS legs** — the release workspace, the standalone crates, trip and etrip
  on hosted macOS, which left the merge queue; a failed macOS job (this one or
  either leg of the debug workspace) comments on, or opens, `main-red`, and
  never closes it (only a green `ci.yml` run on `main`'s tip does).
* **excluded crates** and **clippy debt** — both lists run without gating and
  warn when an entry starts passing, so "the list may only shrink" is a fact the
  workflow reports rather than something someone has to go and measure.

`DESIGN.md` §8 also specifies T4 (a ~5,000-document corpus, nightly), T5 (30,000+
documents, weekly) and T6 (differential fuzzing, continuous). **None of those
three has an implementation in this repository yet**, so `nightly.yml` does not
pretend to run them; the `arxiv`/`templates` tiers are the breadth that exists
today. Each one joins this workflow in the lane that builds it.

### Validating a workflow change

```sh
brew install actionlint          # also pulls shellcheck, which it uses
actionlint                       # finds .github/workflows by itself
shellcheck scripts/gate.sh scripts/check-license-boundary.sh
```

`.github/actionlint.yaml` declares the `flashtex` runner label; without it every
`runs-on: [self-hosted, …]` is reported as an unknown label and the real findings
are lost in the noise.

### Why a main run must never be superseded

A run on `main` gets a concurrency group of its own (the key includes
`github.run_id`); only PR and branch runs share a key and collapse onto the
newest push. This is deliberate, and it was learned the hard way.

GitHub holds at most one in-progress and one *pending* run per concurrency
group, and cancels the pending one whenever a newer run joins the group.
`cancel-in-progress` never entered into it — it already evaluates to false for
a push. But with a mac job that queues for hours on the shared macOS pool and
a merge cadence measured in minutes, every push to main cancelled the run
queued behind the one in progress. **37 of the 40 runs on main before this
changed ended `cancelled`; exactly one completed.** Main's mac health was not
red-and-ignored, it was never measured.

That was the third distinct way this gate reported success it had not earned.
The full list, so nobody has to re-derive it:

1. **Swallowed failures.** `swift test` piped into `tail`, so the step's exit
   status came from `tail`: no number of failing tests could fail the job.
   Fixed by `set -o pipefail` (#490) — which immediately exposed 35 failures
   across 19 test cases that had been invisible.
2. **Truncated logs.** What the step prints is filtered and tailed to 400
   lines, so a failure could scroll out of the visible log entirely. The whole
   output is the `mac-swift-test-log` artifact; read that, not the step.
3. **Cancelled runs.** The above — most runs on main never reported at all.

The cost of the fix is real: runs on main no longer supersede each other, so a
burst of merges means a burst of concurrent macOS jobs on a pool that has
already been starved once (76 queued runs stalled a release for an hour). If
the pool becomes the binding constraint, the lever to reach for is the nine
`rust-workspace (macos-15)` and `rust-standalone (macos-15)` jobs — they
duplicate the `ubuntu-latest` ones — not restoring the cancellation, which buys
runner time by discarding the signal. (Before the workspace, each run
asked for 12 per-crate macOS Rust jobs. Now it asks for 3.)

### Generated tables

Seven committed tables come from TeX Live (or, for the Core 14 AFMs,
matplotlib's copy of Adobe's files). Each generator has a `--check` mode that
re-derives its output and compares it with the committed file. Exit 0 means up
to date, 1 means stale (a diff excerpt is printed), and 2 means it cannot be
checked here (a TeX tool or input is missing). `scripts/check-generated.py --run` runs
them all. On a Mac with MacTeX, every check returns 0 except `gen_tables.py`,
which needs Python 3.12. CI has no TeX, so it checks
`scripts/generated-manifest.json` instead: the SHA-256 of every generated file
and generator. A table edited by hand, or a generator changed without
regenerating, fails `gates`. After regenerating, run
`scripts/check-generated.py --update`. It re-derives everything first and
refuses to record a stale output.

## `release.yml`

1. **`version`** resolves the tag (`v0.2.0` from the pushed tag, or the
   `version` input of a manual run; a missing `v` is added).
2. **`macos`** (`macos-26`, arm64) builds the helpers, imports the signing
   certificate when configured (below), runs
   `apps/mac/scripts/make-app.sh --version <x.y.z> --dmg [--sign … --notarize flashtex-ci]`,
   then produces `FlashTeX.dmg`, `FlashTeX-<ver>-macos-arm64.dmg` (the same
   file under a versioned name) and `flashtex-cli-<ver>-macos-arm64.tar.gz`
   (using the hash-verified fonts/metrics staged inside the app bundle), and
   smoke-tests the extracted CLI with `bin/flashtex build` (no `FLASHTEX_*`
   in the environment, so the fonts must come from `share/flashtex`) and
   `flashtex-render --tex`. The temporary keychain is deleted afterwards.
3. **`linux`** (`ubuntu-latest`) builds `flashtex-cli`, `render-pipeline`,
   `compiler` and `pdf` individually; whatever builds is packaged as
   `flashtex-cli-<ver>-linux-x86_64.tar.gz` and the README inside lists any
   crate that did not build on Linux. A Linux failure does not block the
   release.
4. **`publish`** downloads both artifact sets, writes `SHA256SUMS`, creates
   the GitHub release (`gh release create … --generate-notes`, with a header
   describing the assets and the signing status; re-runs upload with
   `--clobber` instead), then runs `scripts/ci/update-site.sh <tag> <dmg-sha256>`
   which commits to `gh-pages` as `github-actions[bot]` and pushes.

### CLI tarball layout

```
flashtex-cli-<version>-<platform>/
  README.md
  bin/flashtex                the CLI: build / check / watch / supported / worker / fonts
  bin/flashtex-render         runtime-v1 worker (+ flashtex-compiler, flashtex-pdf,
                              flashtex-pdf-exact when built for the platform)
  share/flashtex/Fonts/       pinned Latin Modern OpenType faces + GUST licence
  share/flashtex/texmf/       rooted Latin Modern 2.004 TFM metrics + licence
  bin/Fonts -> ../share/flashtex/Fonts     relative links for the helpers'
  bin/texmf -> ../share/flashtex/texmf     older `<exe>/Fonts` discovery
```

`bin/flashtex` finds `share/flashtex/{Fonts,texmf}` relative to its own
location (`crates/render-pipeline/src/fonts.rs`, `Discovery`: after the
app-bundle `../Resources` and sibling `<exe>/Fonts` layouts, before the host
TeX Live directories), so the extracted directory works anywhere with no
environment and no TeX installation; `flashtex install-cli` symlinks the
binary into `/usr/local/bin` without breaking that. The Mac app also ships
the same binary as `Contents/MacOS/flashtex-cli` (it resolves
`Contents/Resources/{Fonts,texmf}`).

### Secrets (all optional)

| Secret | Purpose |
|---|---|
| `MAC_CERT_P12` | Base64 of a `.p12` export containing the "Developer ID Application: …" certificate and its private key. |
| `MAC_CERT_PASSWORD` | Password of that `.p12`. |
| `NOTARY_APPLE_ID`, `NOTARY_TEAM_ID`, `NOTARY_PASSWORD` | Apple ID, team ID and an app-specific password for `notarytool`; stored in the temporary keychain as profile `flashtex-ci` (`FLASHTEX_NOTARY_KEYCHAIN` tells `make-app.sh` where). |

Without `MAC_CERT_P12`/`MAC_CERT_PASSWORD` the app is ad-hoc signed and the
release notes say so (Gatekeeper then needs right-click > Open, or the site's
`install.sh`, which verifies the checksum and strips quarantine). With the
certificate but without the `NOTARY_*` trio the app is signed but not
notarized. Nothing in the workflow prints a secret; identities are matched by
name and the keychain is discarded after packaging.

Creating the `.p12` secret: export the Developer ID Application identity from
Keychain Access as `cert.p12`, then `base64 -i cert.p12 | pbcopy` and paste it
into the repository secret.

### Cutting a release

```sh
git checkout main && git pull
git tag v0.2.0
git push origin v0.2.0        # or: git push --tags
```

Or run the **Release** workflow manually from the Actions tab with
`version: v0.2.0`; the tag is created on that commit by `gh release create`.
The app version in `Info.plist` comes from the tag (`make-app.sh --version`),
so nothing in the repository needs editing for a release. The default version
`make-app.sh` uses when no `--version`/`APP_VERSION` is given is the last
released one; bump `DEFAULT_APP_VERSION` there when convenient.

### How the website is updated

The site is the `gh-pages` branch (`https://flash-tex.github.io/flashtex/`),
built from the templates in `site/`. Two mechanisms can write to it, in this
order on every published release:

1. **`release.yml`'s `publish` job → `scripts/ci/update-site.sh <version>
   <dmg-sha256>`.** A narrow, in-place rewrite: clones `gh-pages`, reads the
   current `VERSION=`/`SHA256=` out of `install.sh` and uses them as the
   anchor for every replacement (`install.sh`'s `VERSION=`/`SHA256=`; the
   version badge, date, release-notes link, `FlashTeX.dmg` links and checksum
   on `download/index.html`; the version line and download link on
   `index.html`), refuses to publish if the old version/checksum don't
   actually change, then commits and pushes. It does not know about
   `install-cli.sh` or the CLI tarballs at all — the site's two-install-path
   content and the `{{CLI_*}}` placeholders below come entirely from step 2.
2. **`site.yml`, triggered independently by the same `release: published`
   event.** Fully re-renders `site/` with `site/render.py` (below) and
   overwrites the whole of `gh-pages` with the result, then pushes.

Because both react to the same event, there is a short window where either
could push last; `site.yml`'s full render is a superset of what
`update-site.sh` writes (same anchors, plus the CLI-tarball/platform content),
so whichever finishes last leaves `gh-pages` fully correct either way — but if
`update-site.sh` ever changes independently, watch for it clobbering a
render-only change. `site.yml` also runs standalone (a push to `main` touching
`site/`, or `workflow_dispatch`), which `update-site.sh` never does.

**`site/render.py <out-dir> [--tag vX.Y.Z]`** copies `site/` into `<out-dir>`,
filling `{{TAG}}`, `{{VERSION}}`, `{{DATE}}`, `{{SIZE}}`, `{{SHA256}}` (the
`FlashTeX.dmg`/app path) and `{{CLI_MACOS_ARM64_SHA256}}`,
`{{CLI_MACOS_ARM64_SIZE}}`, `{{CLI_LINUX_X86_64_SHA256}}`,
`{{CLI_LINUX_X86_64_SIZE}}` (the CLI-tarball path) into `index.html`,
`download/index.html`, `install.sh` and `install-cli.sh`, reading the
per-file checksums from the release's `SHA256SUMS` asset. A
`{{#if HAS_LINUX_CLI}}…{{/if}}` block (and its `NO_LINUX_CLI` complement) is
kept only when that release actually has a Linux CLI tarball, so the download
page's Linux row and platform table degrade gracefully when the Linux build
failed. It waits (`--wait`, default 300s) for `FlashTeX.dmg`, `SHA256SUMS` and
the macOS CLI tarball to finish uploading — a `release: published` webhook can
arrive before `gh release create` finishes attaching every asset — and fails
outright if they never appear; a missing Linux tarball is not an error.

To rehearse `update-site.sh` without touching the real site:

```sh
git init --bare /tmp/site.git && git push /tmp/site.git origin/gh-pages:gh-pages
scripts/ci/update-site.sh v0.2.0 <sha256> --remote /tmp/site.git --no-push --keep
```

To rehearse the full render:

```sh
python3 site/render.py /tmp/flashtex-site --tag v0.2.0
open /tmp/flashtex-site/index.html
```

## Running the pieces locally

```sh
scripts/gate.sh pr                               # the gate to run before pushing
scripts/gate.sh full                             # everything the merge queue runs
scripts/engine-parity.sh                         # the new engine's gates (needs TeX Live 2026 first on PATH)
scripts/engine-parity.sh build t2                # T2, the LaTeX team's suites (nightly)
scripts/ci/build-helpers.sh                      # builds all helpers, prints FLASHTEX_* lines
set -a; source <(scripts/ci/build-helpers.sh --check); set +a   # just export the paths
cd apps/mac && swift build && CI=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_REVIEW_HISTORY_DIR=off swift test
apps/mac/scripts/make-app.sh --version 0.2.0 --dmg
scripts/ci/package-cli.sh 0.2.0 macos-arm64 dist
actionlint                                       # brew install actionlint
```
