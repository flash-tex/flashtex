//! A development driver: the port on the process's streams, with style
//! files found through `kpsewhich` (the engine's `flashtex-makeindex`
//! binary uses the engine's own kpathsea instead).

fn main() {
    let args: Vec<Vec<u8>> = std::env::args_os()
        .skip(1)
        .map(|a| a.into_encoded_bytes())
        .collect();
    let mut host = flashtex_makeindex::ProcessHost::new(|name: &[u8]| {
        let out = std::process::Command::new("kpsewhich")
            .arg("-progname=makeindex")
            .arg("-format=ist")
            .arg(String::from_utf8_lossy(name).as_ref())
            .output()
            .ok()?;
        let mut p = out.stdout;
        while p.last() == Some(&b'\n') {
            p.pop();
        }
        (!p.is_empty()).then_some(p)
    });
    let status = flashtex_makeindex::run(&args, &mut host);
    std::process::exit(match status {
        flashtex_makeindex::LOOPS_FOREVER => 125,
        s if s < 0 => 128 - s,
        s => s,
    });
}
