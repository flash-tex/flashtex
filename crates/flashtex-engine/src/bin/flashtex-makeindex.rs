//! `flashtex-makeindex [makeindex's arguments]`: the makeindex port
//! (crates/makeindex) as a command, exactly as `\write18` runs it in-process
//! (src/makeindex.rs): the process's streams and working directory, style
//! files through the engine's kpathsea. It is what the oracle comparison in
//! docs/evidence/cold-speed-2026-10-04/makeindex/ runs against TeX Live's
//! `makeindex`.
//!
//! Exit status: makeindex's (0, or 1 after a fatal error); 139 where the C
//! program would have died of SIGSEGV; 125 where it would never have ended
//! (`flashtex_makeindex::LOOPS_FOREVER`), which the harness compares with
//! an oracle run that timed out.

fn main() {
    let args: Vec<Vec<u8>> = std::env::args_os()
        .skip(1)
        .map(|a| a.into_encoded_bytes())
        .collect();
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    let mut host = flashtex_engine::makeindex::EngineHost {
        cwd: None,
        stdout: &mut out,
        stderr: &mut err,
        stdin: None,
    };
    let status = flashtex_makeindex::run(&args, &mut host);
    std::process::exit(match status {
        flashtex_makeindex::LOOPS_FOREVER => 125,
        s if s < 0 => 128 - s,
        s => s,
    });
}
