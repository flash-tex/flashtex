# Move owned results into helper wire envelopes

Baseline helper on main f2ea364 creates a preview payload with json! containing a large Value, then creates an envelope containing that payload with json!. Pinned serde_json1.0.151 macro implementation (macros.rs279) routes arbitrary expressions through to_value(&$other); this copies each nested Value. The candidate builds small metadata objects and moves the owned compiler result/payload into them. Map representation, serialized field ordering, frame bounds, writer thread, output queue, durable receipt ordering and stale-result checks remain unchanged.

Alternating20pairs on one captured10,135,609-byte original compiler result: median legacy wrapping221.452ms, owned wrapping0.006284ms. Final serialization median27.426ms vs27.751ms. All40 serialized envelopes exactly equal. Input SHA256 2f525a17b95b4d8d8595116810d41ebaf9f432fa211c67f3b0dbbbacb207bdd3. Fixture setup/source clone is outside timed wrapping; legacy timing includes the copies and drops that production legacy wrapper performs. This is not a native app, compiler speed, or pixel parity measurement. Actual application benefit requires a new Mac bridge run.

Reproduce: cargo run --release --example wire_compare -- /path/to/captured-result.json. The example invokes the production wire helpers, compares the complete serialized bytes, alternates order, and records every sample. No LLM/provider calls.

35 controller tests passed, including real original compiler, large-result stdio, durable EOF/reopen, stale edit, approval, session rejection, stalled-output timeout/backpressure and project conflict cases; strict all-target Clippy passed. Cargo.lock merely reconciles already-integrated edit-ledger getrandom dependency; no new dependency added by this change.
