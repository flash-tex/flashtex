//! makeindex in-process (lane COLD-SPEED, Commander ruling 2026-10-04):
//! TeX Live 2026's makeindex ported to Rust (`crates/makeindex`, under the
//! MakeIndex Distribution Notice, linked here; DESIGN.md §3), run instead of
//! the user's TeX Live binary where the engine would start one:
//!
//! * restricted `\write18` (`system::runsystem`, DESIGN.md §4.5): a command
//!   texmfmp.c's `shell_cmd_is_allowed` passed whose program is `makeindex`
//!   (imakeidx, `\makeindex` with `[makeindex]`); the arguments are the
//!   words `/bin/sh -c` would make of the re-quoted command;
//! * the host's external tools (`host::external`, DESIGN.md §5.5): latexmk's
//!   makeindex rule.
//!
//! The port runs with the process's standard streams and working directory,
//! as the child would, and finds style files with the engine's own kpathsea
//! (`kpse_find_file(name, kpse_ist_format, 1)`), so neither needs a TeX Live
//! binary (DESIGN.md §4.4). `FLASHTEX_MAKEINDEX=external` restores the child
//! process (A/B measurement). The run stays an external effect and an L3
//! barrier exactly as before (`system::runsystem` records it first).

use crate::resolver::Format;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Whether makeindex runs in-process (the default) or as TeX Live's
/// program (`FLASHTEX_MAKEINDEX=external`).
pub fn in_process() -> bool {
    std::env::var("FLASHTEX_MAKEINDEX").map_or(true, |v| v != "external")
}

/// The arguments of `cmd` after the program name, if `cmd` runs makeindex
/// and nothing else: the words `/bin/sh -c cmd` would pass it.
///
/// `restricted` (allow 2): `cmd` is texmfmp.c's re-quoted command, the
/// program name followed by words that are runs of `QUOTE`-quoted segments
/// separated by blanks, never containing the quote (`shell_cmd_is_allowed`
/// refuses that). Otherwise (`-shell-escape`) only a plain command is
/// taken: the name `makeindex`, then words of characters the shell treats
/// literally, or single-quoted; anything else (`$`, `;`, `|`, `>`, a glob,
/// ...) is left to the shell.
pub fn command_args(cmd: &[u8], restricted: bool) -> Option<Vec<Vec<u8>>> {
    let is_blank = |c: u8| c == b' ' || c == b'\t';
    let quote = crate::os::SHELL_QUOTE;
    let mut words: Vec<Vec<u8>> = vec![];
    let mut cur: Option<Vec<u8>> = None;
    let mut i = 0;
    while i < cmd.len() {
        let c = cmd[i];
        if is_blank(c) {
            if let Some(w) = cur.take() {
                words.push(w);
            }
            i += 1;
        } else if c == quote || (!restricted && c == b'\'') {
            let end = cmd[i + 1..].iter().position(|&b| b == c)? + i + 1;
            cur.get_or_insert_with(Vec::new)
                .extend_from_slice(&cmd[i + 1..end]);
            i = end + 1;
        } else {
            let literal = c.is_ascii_alphanumeric() || b"._-/+=,:@%".contains(&c);
            if !restricted && !literal {
                return None;
            }
            cur.get_or_insert_with(Vec::new).push(c);
            i += 1;
        }
    }
    if let Some(w) = cur.take() {
        words.push(w);
    }
    if words.first().map(|w| &w[..]) != Some(b"makeindex") {
        return None;
    }
    Some(words.split_off(1))
}

/// The port's view of the engine: the process's streams, a directory for
/// relative names, and kpathsea through `find`.
pub struct EngineHost<'a> {
    pub cwd: Option<PathBuf>,
    pub stdout: &'a mut dyn Write,
    pub stderr: &'a mut dyn Write,
    pub stdin: Option<Vec<u8>>,
}

impl flashtex_makeindex::Host for EngineHost<'_> {
    fn find_ist(&mut self, name: &[u8]) -> Option<Vec<u8>> {
        let name = String::from_utf8_lossy(name).into_owned();
        let found = match &self.cwd {
            // A directory of its own (the external-tools runner's scratch
            // directory): kpathsea's `.` is the process's, so look there
            // first, as kpathsea would in that directory.
            Some(d) => [name.clone(), format!("{name}.ist")]
                .into_iter()
                .find(|n| d.join(n).is_file())
                .map(|n| {
                    if Path::new(&n).is_absolute() {
                        PathBuf::from(n)
                    } else {
                        PathBuf::from(format!("./{n}"))
                    }
                })
                .or_else(|| find_ist_kpathsea(&name)),
            None => find_ist_kpathsea(&name),
        }?;
        Some(path_bytes(&found))
    }
    fn in_name_ok(&mut self, name: &[u8]) -> bool {
        let name = String::from_utf8_lossy(name).into_owned();
        crate::system::with_resolver(|r| r.name_ok_silent(&name, false))
    }
    fn out_name_ok(&mut self, name: &[u8]) -> bool {
        let n = String::from_utf8_lossy(name).into_owned();
        let (ok, choice) = crate::system::with_resolver(|r| {
            let ok = r.name_ok_silent(&n, true);
            (
                ok,
                if ok {
                    None
                } else {
                    r.config_var("openout_any")
                },
            )
        });
        if !ok {
            let choice = choice.unwrap_or_else(|| "p".into());
            let m = flashtex_makeindex::not_writing_message(name, choice.as_bytes());
            let _ = self.stderr.write_all(&m);
        }
        ok
    }
    fn stdout(&mut self) -> &mut dyn Write {
        self.stdout
    }
    fn stderr(&mut self) -> &mut dyn Write {
        self.stderr
    }
    fn read_stdin(&mut self) -> Vec<u8> {
        match self.stdin.take() {
            Some(d) => d,
            None => {
                let mut v = vec![];
                let _ = std::io::Read::read_to_end(&mut std::io::stdin(), &mut v);
                v
            }
        }
    }
    fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }
}

/// `kpse_find_file(name, kpse_ist_format, 1)` with the engine's kpathsea.
fn find_ist_kpathsea(name: &str) -> Option<PathBuf> {
    crate::system::with_resolver(|r| r.find_ex(name, Format::Ist, true).0)
}

#[cfg(unix)]
fn path_bytes(p: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    p.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_bytes(p: &Path) -> Vec<u8> {
    p.to_string_lossy().as_bytes().to_vec()
}

/// Run makeindex on the process's streams and working directory, as the
/// child of `\write18` would run: its exit status, or -1 where the C
/// program would have been killed or never ended (what the shell route
/// reports for a child killed by a signal).
pub fn run_in_process(args: &[Vec<u8>]) -> i32 {
    let mut out = std::io::stdout();
    let mut err = std::io::stderr();
    let mut host = EngineHost {
        cwd: None,
        stdout: &mut out,
        stderr: &mut err,
        stdin: None,
    };
    let status = flashtex_makeindex::run(args, &mut host);
    if status < 0 {
        -1
    } else {
        status
    }
}

#[cfg(test)]
mod tests {
    use super::command_args;

    fn w(v: &[&str]) -> Vec<Vec<u8>> {
        v.iter().map(|s| s.as_bytes().to_vec()).collect()
    }

    #[cfg(unix)]
    #[test]
    fn restricted_commands_split_as_sh_does() {
        // What texmfmp.c's shell_cmd_is_allowed makes of imakeidx's call.
        assert_eq!(
            command_args(
                b"makeindex '-s' 'x.ist' '-o' 'notation.ind' 'notation.idx'",
                true
            ),
            Some(w(&["-s", "x.ist", "-o", "notation.ind", "notation.idx"]))
        );
        // `--x="a b"` became `'--x=''a b'`: one word.
        assert_eq!(
            command_args(b"makeindex '--x=''a b' 'q'", true),
            Some(w(&["--x=a b", "q"]))
        );
        assert_eq!(command_args(b"makeindex", true), Some(vec![]));
        assert_eq!(command_args(b"bibtex 'x'", true), None);
        assert_eq!(command_args(b"makeindex2 'x'", true), None);
    }

    #[test]
    fn unrestricted_commands_only_when_plain() {
        assert_eq!(
            command_args(b"makeindex -q main.idx", false),
            Some(w(&["-q", "main.idx"]))
        );
        assert_eq!(
            command_args(b"makeindex -s 'my style.ist' main", false),
            Some(w(&["-s", "my style.ist", "main"]))
        );
        for c in [
            &b"makeindex main.idx > log"[..],
            b"makeindex main.idx; rm x",
            b"makeindex $HOME/x.idx",
            b"makeindex *.idx",
            b"makeindex \"a b\"",
            b"makeindex ~/x.idx",
            b"/usr/bin/makeindex x.idx",
        ] {
            assert_eq!(
                command_args(c, false),
                None,
                "{}",
                String::from_utf8_lossy(c)
            );
        }
    }
}
