//! Paired bounded serialization experiment; never publishes frames or edits source.
#[path = "../src/output_buffer.rs"]
mod output_buffer;
use output_buffer::OutputBuffer;
use serde::Serialize;
use serde_json::{json, value::RawValue, Value};
use std::time::Instant;

fn encode<T: Serialize + ?Sized>(
    value: &T,
    buffered: bool,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let output = OutputBuffer::new(limit);
    if buffered {
        output_buffer::serialize(value, limit).map_err(|e| e.to_string())
    } else {
        let mut output = output;
        serde_json::to_writer(&mut output, value).map_err(|e| e.to_string())?;
        output.finish().map_err(|e| e.to_string())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("expected candidate JSON path")?;
    let value: Value = serde_json::from_reader(std::fs::File::open(path)?)?;
    let mut expected = serde_json::to_vec(&value)?;
    // Pre-encoding is deliberately outside the measured region. This comparison
    // isolates output cost only; it cannot establish raw transport validation cost.
    let raw = RawValue::from_string(String::from_utf8(expected.clone())?)?;
    expected.push(b'\n');
    let limit = 16 * 1024 * 1024;
    for buffered in [false, true] {
        assert_eq!(encode(&value, buffered, limit)?, expected);
        assert!(encode(&value, buffered, expected.len() - 1).is_err());
        assert_eq!(encode(&value, buffered, expected.len())?, expected);
        assert_eq!(encode(&raw, buffered, limit)?, expected);
        assert!(encode(&raw, buffered, expected.len() - 1).is_err());
        assert_eq!(encode(&raw, buffered, expected.len())?, expected);
    }
    let mut raw_observations = Vec::new();
    for pair in 0..10 {
        for use_raw in if pair % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let start = Instant::now();
            let bytes = if use_raw {
                encode(&raw, true, limit)?
            } else {
                encode(&value, true, limit)?
            };
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            assert_eq!(bytes, expected);
            raw_observations.push(json!({"pair":pair,"raw":use_raw,"ms":ms}));
        }
    }
    let mut observations = Vec::new();
    for pair in 0..10 {
        for buffered in if pair % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        } {
            let start = Instant::now();
            let bytes = encode(&value, buffered, limit)?;
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            assert_eq!(bytes, expected);
            observations.push(json!({"pair":pair,"buffered":buffered,"ms":ms}));
        }
    }
    println!(
        "{}",
        json!({"bytes":expected.len(),"exact_output":true,
        "exact_limit_and_overflow_checked":true,"buffer_capacity":8192,
        "observations":observations,"raw_observations":raw_observations,
        "raw_scope":"preencoded canonical body; parsing and validation excluded",
        "native_latency":"not measured"})
    );
    Ok(())
}
