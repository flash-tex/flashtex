//! What dominates the host's heap on a long document (DESIGN.md §15.2's
//! memory budget): the same compiles as the host's typing loop, in one
//! process, then the heap measured while each part is released in turn.
//!
//! ```sh
//! mem_probe --fonts DIR --doc DIR [--keys N] [--marker EDITMID] [--evict AGE]
//!           [--done-pdf on|off]
//! ```
//!
//! After a cold compile, the conversion of every page (the connection's
//! tables) and N seeded keystrokes at the marker, each followed by
//! `comemo::evict(AGE)` as the host does, it prints one JSON line: the
//! heap in use (macOS `malloc_zone_statistics`, all zones) and the
//! resident size, before and after releasing, in this order, the retained
//! documents, the connection's tables, comemo's caches (`evict(0)`), the
//! World (sources, files) and the fonts. A part's share is the heap it
//! frees; what two parts share is freed with the later one.
//!
//! macOS only (the heap statistics); the host itself is portable.

use std::path::PathBuf;

use flashtex_typst_host::convert::{self, ClientCaps, Positions, Tables};
use flashtex_typst_host::seeded;
use flashtex_typst_host::world::{FontOptions, Fonts, HostWorld};

#[repr(C)]
#[derive(Default)]
struct MallocStats {
    blocks_in_use: u32,
    size_in_use: usize,
    max_size_in_use: usize,
    size_allocated: usize,
}

extern "C" {
    fn malloc_zone_statistics(zone: *mut std::ffi::c_void, stats: *mut MallocStats);
    fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
}

/// (heap in use, heap allocated from the system) in MB.
fn heap() -> (f64, f64) {
    let mut s = MallocStats::default();
    // SAFETY: a null zone asks for every zone's statistics combined.
    unsafe { malloc_zone_statistics(std::ptr::null_mut(), &mut s) };
    (
        s.size_in_use as f64 / 1048576.0,
        s.size_allocated as f64 / 1048576.0,
    )
}

fn rss_mb() -> f64 {
    let out = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<f64>()
        .unwrap_or(f64::NAN)
        / 1024.0
}

fn snap(label: &str, out: &mut Vec<String>) {
    let (used, alloc) = heap();
    out.push(format!(
        "{{\"after\":{label:?},\"heap_in_use_mb\":{used:.1},\"heap_allocated_mb\":{alloc:.1},\"rss_mb\":{:.1}}}",
        rss_mb()
    ));
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut fonts_dir, mut doc) = (None, None);
    let (mut keys, mut marker, mut age) = (40usize, "EDITMID".to_string(), 3usize);
    let mut done_pdf = false;
    while let Some(a) = args.next() {
        let v = args.next().expect("a value");
        match a.as_str() {
            "--fonts" => fonts_dir = Some(PathBuf::from(v)),
            "--doc" => doc = Some(PathBuf::from(v)),
            "--keys" => keys = v.parse().unwrap(),
            "--marker" => marker = v,
            "--evict" => age = v.parse().unwrap(),
            // Also the standard compile and typst-pdf's whole export after
            // each keystroke, as the host does while a page is INCOMPLETE
            // (DONE.pdf, spec §11.9).
            "--done-pdf" => done_pdf = v == "on",
            _ => panic!("unknown argument {a}"),
        }
    }
    let doc = doc.expect("--doc");
    let main_path = doc.join("main.typ");
    let original = std::fs::read_to_string(&main_path).unwrap();
    let mut out = vec![];
    snap("start", &mut out);
    let fonts = Fonts::load(&FontOptions {
        paths: vec![fonts_dir.expect("--fonts")],
        system: false,
    });
    snap("fonts scanned", &mut out);
    let mut world = HostWorld::new(&doc, "main.typ", &fonts).unwrap();
    let mut tables = Tables::new();
    let caps = ClientCaps {
        minor: 3,
        opentype_programs: true,
        program_refs: true,
        program_budget: None,
    };
    let cold = seeded::standard(&world)
        .output
        .output
        .expect("the document compiles");
    for i in 0..cold.pages().len() {
        convert::page(&world, &cold, i, &mut tables, caps, &[], Positions::Frame).unwrap();
    }
    comemo::evict(age);
    let pages = cold.pages().len();
    snap("cold compile, every page converted", &mut out);
    let mut prev = Some(cold);
    let (mut same_frames, mut compared) = (0usize, 0usize);
    let mut evict_ms = vec![];
    let mut text = original.clone();
    for k in 0..keys {
        let off = text.find(marker.as_str()).expect("marker");
        text.insert(off, (b'a' + (k % 26) as u8) as char);
        std::fs::write(&main_path, &text).unwrap();
        world.reset();
        tables.begin_compile();
        let c = seeded::compile(&world, prev.as_ref());
        let d = c.output.output.expect("the edit compiles");
        // Pages whose frame is the previous document's own (the same item
        // list): what a hash could skip.
        if let Some(p) = &prev {
            for (a, b) in d.pages().iter().zip(p.pages()) {
                if std::ptr::eq(a.frame.items().as_slice(), b.frame.items().as_slice()) {
                    same_frames += 1;
                }
            }
            compared += d.pages().len();
        }
        // The host converts the pages that changed; the edited one at least.
        let i = d.pages().len() / 2;
        convert::page(&world, &d, i, &mut tables, caps, &[], Positions::Frame).unwrap();
        prev = Some(d);
        if done_pdf {
            let std = seeded::standard(&world).output.output.expect("compiles");
            typst_pdf::pdf(&std, &typst_pdf::PdfOptions::default()).expect("exports");
        }
        let t = std::time::Instant::now();
        comemo::evict(age);
        evict_ms.push(t.elapsed().as_secs_f64() * 1e3);
    }
    snap("keystrokes (steady state)", &mut out);
    // Free pages the allocator holds, returned to the system.
    let t = std::time::Instant::now();
    // SAFETY: a null zone relieves every zone; goal 0: as much as possible.
    let freed = unsafe { malloc_zone_pressure_relief(std::ptr::null_mut(), 0) };
    snap(
        &format!(
            "malloc_zone_pressure_relief ({:.0} MB in {:.1} ms)",
            freed as f64 / 1048576.0,
            t.elapsed().as_secs_f64() * 1e3
        ),
        &mut out,
    );
    drop(prev);
    snap("documents released", &mut out);
    drop(tables);
    snap("connection tables released", &mut out);
    comemo::evict(0);
    snap("comemo caches cleared", &mut out);
    drop(world);
    snap("World released", &mut out);
    drop(fonts);
    snap("fonts released", &mut out);
    std::fs::write(&main_path, original).unwrap();
    println!(
        "{{\"doc\":{:?},\"pages\":{pages},\"keys\":{keys},\"marker\":{marker:?},\"evict\":{age},\"done_pdf\":{done_pdf},\"evict_ms_p50\":{:.1},\"evict_ms_max\":{:.1},\"same_frames\":{same_frames},\"pages_compared\":{compared},\"steps\":[{}]}}",
        doc.display().to_string(),
        {
            evict_ms.sort_by(f64::total_cmp);
            evict_ms[evict_ms.len() / 2]
        },
        evict_ms.last().unwrap(),
        out.join(",")
    );
}
