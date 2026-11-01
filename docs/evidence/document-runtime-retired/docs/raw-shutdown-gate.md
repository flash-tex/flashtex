# Raw decoding cancellation and joined shutdown

The production decoder checks its stop flag after serde returns. Synchronous
`Decoder::drop` therefore joins after the current frame parse completes. Closing
a project invalidates owner state and delivery epochs without interrupting serde.
The input byte cap limits frame length; it does not guarantee elapsed parse time.

The new test-only gate acknowledges entry inside serde's recursive sequence
visitor and pauses on a channel. Each decoder installs its own gate into its
worker-local storage; no process-global hook exists. Production builds contain
neither the hook nor its channels. Tests start shutdown only after this explicit
acknowledgment, observe its stop flag, verify completion has not occurred, then
release parsing and require the joined terminal state. No sleep establishes entry.
The cancellation case similarly closes the project while parsing is paused,
then releases parsing and proves the obsolete result never becomes a preview.

Three separate-process observations at the unchanged 8-MiB framed limit:

| Case | Observation | Test-process peak |
| --- | --- | --- |
| Valid opaque string, joined shutdown | 55.99 ms from release to join | 11888 KiB |
| Malformed trailing delimiter | 43.58 ms from release to join | 11976 KiB |
| Cancel while paused | 0.008 ms owner update; 80.33 ms drain after release; 0.91 ms joined drop after drain | 20240 KiB |

Complete logs, binary digest and numeric observations are preserved in
`../benchmarks/raw-shutdown-gate`. Intentional gate waiting is excluded. These
are debug-test observations of long opaque strings, not worst-case JSON shapes,
allocator guarantees, native responsiveness, or calibrated performance claims.
No production change or cooperative parse-abort mechanism is justified by these
samples. Existing finite/depth checks, raw4/decoded1, and joined shutdown remain.

Run the ignored unit test
`decode_lane::tests::near_limit_shutdown_inside_serde_joins_after_explicit_release`
in a separate process, with `FLASHTEX_MALFORMED_TAIL=1` for the malformed variant.
Run `decode_cancellation_probe::cancellation_during_entered_serde_invalidates_source_ownership`
separately for owner cancellation. Both use `cargo test --lib <exact name> --
--ignored --exact --nocapture`; no producer build or cap change is needed.
