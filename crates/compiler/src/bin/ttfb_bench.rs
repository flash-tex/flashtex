//! Time-to-first-response-byte on a pinned 500 KB fixture (FT-002 rev 9).
//!
//! The product question is not how long a compile takes in total; it is how long
//! the author waits before anything can be shown. Today the worker builds the
//! entire reply in memory and writes it in one go, so time to first byte equals
//! time to last byte. This measures the parts so the cost is known before
//! anything is optimised.
//!
//! COMPILER WORK ONLY. UI paint, scheduling and transport are outside this
//! crate; native paint parity and raw PDF byte equality remain separate gates
//! and are neither claimed nor estimated here.

use flashtex_compiler::protocol::handle_line;
use flashtex_font_engine::sha256;
use std::hint::black_box;
use std::io::Write;
use std::time::{Duration, Instant};

const SAMPLES: usize = 20;

/// Deterministic 500 KB fixture: identical bytes on every machine and run.
fn fixture() -> String {
    let mut out = String::with_capacity(520_000);
    out.push_str("\\documentclass{article}\n\\newcommand{\\proj}{FlashTeX}\n\\begin{document}\n");
    let mut n = 0usize;
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    while out.len() < 500_000 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        match (seed >> 33) % 4 {
            0 => out.push_str(&format!("\\section{{Section {n}}}\n")),
            1 => out.push_str(&format!(
                "Paragraph {n} of \\proj{{}} with $x^{{{n}}}$ maths.\n\n"
            )),
            2 => out.push_str(&format!(
                "Paragraph {n} discusses the result with enough words to wrap across a line.\n\n"
            )),
            _ => out.push_str(&format!("Displayed: $$\\frac{{a_{{{n}}}}}{{b}}$$\n\n")),
        }
        n += 1;
    }
    out.push_str("\\end{document}\n");
    out
}

fn request_for(text: &str, project: &str) -> String {
    use flashtex_compiler::json::{self, Value};
    let mut doc = Value::obj();
    doc.set("path", json::str_("main.tex"));
    doc.set("text", json::str_(text));
    let mut payload = Value::obj();
    payload.set("project_id", json::str_(project));
    payload.set("revision", Value::Num(1.0));
    payload.set("entry_path", json::str_("main.tex"));
    payload.set("documents", Value::Arr(vec![doc]));
    let mut env = Value::obj();
    env.set("protocol_version", Value::Num(1.0));
    env.set("id", json::str_("ttfb"));
    env.set("type", json::str_("compile"));
    env.set("payload", payload);
    json::write(&env)
}

fn percentile(samples: &mut [Duration], q: f64) -> f64 {
    samples.sort_unstable();
    let idx = (((samples.len() - 1) as f64) * q).round() as usize;
    samples[idx].as_secs_f64() * 1000.0
}

fn report(label: &str, mut s: Vec<Duration>) {
    println!(
        "  {label:<34} p50={:>9.3} ms  p95={:>9.3} ms  p99={:>9.3} ms",
        percentile(&mut s.clone(), 0.50),
        percentile(&mut s.clone(), 0.95),
        percentile(&mut s, 0.99)
    );
}

fn hex16(bytes: &[u8]) -> String {
    bytes.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn request(text: &str) -> String {
    request_for(text, "ttfb")
}

fn main() {
    let text = fixture();
    let line = request(&text);
    let binary = std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::read(p).ok())
        .map(|b| hex16(&sha256::digest(&b)))
        .unwrap_or_else(|| "unavailable".into());

    println!("FlashTeX time-to-first-response-byte");
    println!(
        "fixture:  {} bytes  sha256={}",
        text.len(),
        hex16(&sha256::digest(text.as_bytes()))
    );
    println!(
        "request:  {} bytes  sha256={}",
        line.len(),
        hex16(&sha256::digest(line.as_bytes()))
    );
    println!("binary:   sha256={binary}");
    println!("samples:  {SAMPLES}");
    println!();

    // COLD time to first byte: a document the worker has never seen. Each sample
    // uses a distinct project_id, because the session cache is keyed by project
    // and path — reusing one id measures warm reuse and flatters the number. An
    // earlier version of this benchmark made exactly that mistake, and it showed
    // as compile appearing slower than the whole reply, which is impossible.
    let mut cold = Vec::with_capacity(SAMPLES);
    let mut reply_len = 0usize;
    for i in 0..SAMPLES {
        let unique = request_for(&text, &format!("ttfb-cold-{i}"));
        let start = Instant::now();
        let reply = handle_line(black_box(&unique));
        cold.push(start.elapsed());
        reply_len = reply.len();
        black_box(reply);
    }
    report("COLD reply built (= TTFB today)", cold);

    // WARM: the same document recompiled, which is what an editing session does.
    let mut warm = Vec::with_capacity(SAMPLES);
    handle_line(&line);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        let reply = handle_line(black_box(&line));
        warm.push(start.elapsed());
        black_box(reply);
    }
    report("WARM reply built", warm);

    // Split the reply build into compile versus serialise, so the dominant part
    // is known rather than assumed. rev 7 cost two wrong fixes before measuring.
    {
        use flashtex_compiler::incremental::compile_full;
        use flashtex_compiler::layout::LayoutConstraints;
        let mut compile_only = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            let start = Instant::now();
            black_box(compile_full(black_box(&text), LayoutConstraints::default()));
            compile_only.push(start.elapsed());
        }
        report("  of which: compile", compile_only);
    }

    // Writing it out, measured separately so the split is visible.
    let reply = handle_line(&line);
    let mut write_samples = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let mut sink: Vec<u8> = Vec::with_capacity(reply.len() + 1);
        let start = Instant::now();
        sink.write_all(reply.as_bytes()).expect("write");
        sink.push(b'\n');
        write_samples.push(start.elapsed());
        black_box(sink);
    }
    report("serialised bytes -> writer", write_samples);

    println!();
    println!("  reply size: {reply_len} bytes");
    println!();
    println!("--- what this is, and is not ---");
    println!("measured:     compiler work only, request bytes in to reply bytes available");
    println!("NOT measured: UI paint, scheduling, IPC transport, PDF writing");
    println!("SEPARATE GATES, not claimed here: native paint parity, raw PDF byte equality");
}
