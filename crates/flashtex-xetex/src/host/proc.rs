//! The host's children: each engine run and each external tool.
//!
//! - **One process group per child.** It is started as a group of its own
//!   (`setpgid(0, 0)`), so a cancel kills the group (`killpg`): the
//!   engine together with what `\write18` started, a tool together with
//!   what it started.
//! - **A lifeline.** The pdfTeX host runs its engine in its own process; this
//!   host runs it as a child, which must not outlive the host however the
//!   host ends (an exit, SIGTERM, SIGKILL, a crash). The host holds the
//!   write end of one pipe for its whole life (close-on-exec, so no child
//!   holds it); every child gets the read end (`FLASHTEX_LIFELINE_FD`). The
//!   kernel closes the write end when the host ends, the read returns end
//!   of file, and the child kills its own process group
//!   ([`watch_lifeline`]). An engine child watches it itself; a tool,
//!   which is not FlashTeX's program, runs under this program invoked with
//!   [`RUN_TOOL`], which watches for it.

use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The environment variable naming the lifeline's read end in a child.
pub const LIFELINE_ENV: &str = "FLASHTEX_LIFELINE_FD";

/// This program's first argument when it runs a tool for the host:
/// `flashtex-host-unicode --run-tool PROGRAM ARG...`.
pub const RUN_TOOL: &str = "--run-tool";

/// The lifeline and this program's path: what starting a child needs.
pub struct Children {
    exe: PathBuf,
    read: OwnedFd,
    // kept open for the host's whole life; its close is the signal
    _write: OwnedFd,
}

fn pipe_cloexec() -> std::io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: `fds` has room for the two descriptors pipe(2) writes.
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    for fd in fds {
        // SAFETY: a descriptor pipe(2) just returned.
        unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    }
    // SAFETY: both descriptors are new and owned by nothing else.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

impl Children {
    pub fn new(exe: &Path) -> std::io::Result<Children> {
        let (read, write) = pipe_cloexec()?;
        Ok(Children {
            exe: exe.to_path_buf(),
            read,
            _write: write,
        })
    }

    /// Sets `cmd` up as a child: a process group of its own, the lifeline's
    /// read end kept open across `exec` and named in the environment.
    pub fn prepare(&self, cmd: &mut Command) {
        let fd: RawFd = self.read.as_raw_fd();
        cmd.process_group(0).env(LIFELINE_ENV, fd.to_string());
        // SAFETY: fcntl(2) is async-signal-safe, and only this descriptor
        // (the child's copy) changes.
        unsafe {
            cmd.pre_exec(move || {
                if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    /// A command that runs tool `prog` under this program's watch
    /// ([`run_tool`]), prepared as a child.
    pub fn tool(&self, prog: &Path) -> Command {
        let mut c = Command::new(&self.exe);
        c.arg(RUN_TOOL).arg(prog);
        self.prepare(&mut c);
        c
    }
}

/// Kills process group `pgid` (a child and everything it started).
pub fn kill_group(pgid: u32) {
    // SAFETY: killpg(2) has no memory-safety preconditions.
    unsafe { libc::killpg(pgid as libc::pid_t, libc::SIGKILL) };
}

/// In a child the host started: a thread that kills this process group
/// when the host ends (the lifeline's end of file). Nothing when the
/// environment names no lifeline (a run of `xelatex` from a shell).
pub fn watch_lifeline() {
    let Some(fd) = std::env::var(LIFELINE_ENV)
        .ok()
        .and_then(|v| v.parse::<RawFd>().ok())
    else {
        return;
    };
    // the engine's own children are not the host's
    std::env::remove_var(LIFELINE_ENV);
    // SAFETY: fcntl(2) on a descriptor number; failure is handled.
    unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) };
    std::thread::spawn(move || {
        let mut b = [0u8; 1];
        loop {
            // SAFETY: `b` is a valid one-byte buffer.
            let n = unsafe { libc::read(fd, b.as_mut_ptr().cast(), 1) };
            if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            if n <= 0 {
                break;
            }
        }
        // the host is gone: this group (this process and what it started)
        // SAFETY: kill(2) has no memory-safety preconditions.
        unsafe { libc::kill(0, libc::SIGKILL) };
    });
}

/// `--run-tool PROGRAM ARG...`: run the tool in this process group, under
/// the lifeline's watch; this program's exit status is the tool's (a
/// signal's death is 128 + its number, as a shell says it).
pub fn run_tool(args: &[String]) -> i32 {
    watch_lifeline();
    let Some((prog, rest)) = args.split_first() else {
        return 2;
    };
    match Command::new(prog).args(rest).status() {
        Ok(s) => {
            use std::os::unix::process::ExitStatusExt;
            s.code().unwrap_or_else(|| 128 + s.signal().unwrap_or(0))
        }
        Err(e) => {
            eprintln!("flashtex-host-unicode: {prog}: {e}");
            127
        }
    }
}
