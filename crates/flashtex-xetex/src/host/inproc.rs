//! The host's resident engine (`crate::resident`): runs of the engine on an
//! engine thread of this process, instead of a child process per run
//! (docs/design/xetex/INCREMENTAL.md §5 step 2).
//!
//! The engine's display list comes to the compile through a socket pair, so
//! a run's pages go through `compile::forward` as a child's do (the
//! client's fonts and accepted items), and are sent as they are shipped.
//! A cancel sets the run's flag, which the engine reads between commands,
//! at macro calls, lines and pages (`flashtex_poll`).
//!
//! The engine's runtime is per process (`flashtex_engine::system`), so
//! there is one engine thread in the process, and the connections' runs
//! take turns on it ([`Shared::run`]).
//!
//! `FLASHTEX_UNICODE_INPROCESS=0` runs every compile in a child process
//! instead, with hot spares (`spare.rs`), as before.

use crate::fontmgr::FontCatalog;
use crate::resident::{Engine, Fonts, Report, Spec};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

/// Whether compiles run in this process (the default).
pub fn enabled() -> bool {
    std::env::var("FLASHTEX_UNICODE_INPROCESS").map_or(true, |v| v != "0")
}

/// Work for the engine thread: one run, and where its report goes; or a
/// save of S₀ to its file (`Engine::save_s0`).
enum Req {
    Run {
        spec: Spec,
        reply: mpsc::Sender<Report>,
    },
    SaveS0(PathBuf, PathBuf),
    /// Wait for the S₀ writers, then answer.
    Flush(mpsc::Sender<()>),
}

/// The engine thread, which every connection shares.
pub struct Shared {
    tx: Mutex<Option<mpsc::Sender<Req>>>,
    /// Held for a whole run: one run at a time.
    turn: Mutex<()>,
    /// Requests sent and not yet taken by the engine thread: a save of S₀
    /// in progress stops for them (`Engine::save_pending`).
    queued: Arc<AtomicUsize>,
}

impl Default for Shared {
    fn default() -> Self {
        Shared {
            tx: Mutex::new(None),
            turn: Mutex::new(()),
            queued: Arc::new(AtomicUsize::new(0)),
        }
    }
}

/// How long the engine thread waits for another request before it makes
/// an asked-for save of S₀ (a keystroke typed right after a compile comes
/// first; the save itself stops for a request that comes while it runs).
/// `FLASHTEX_UNICODE_SAVE_IDLE_MS` sets it (tests).
fn save_when_idle() -> std::time::Duration {
    let ms = std::env::var("FLASHTEX_UNICODE_SAVE_IDLE_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);
    std::time::Duration::from_millis(ms)
}

type Dirs = Vec<(PathBuf, Option<super::spare::Ident>)>;

/// The fonts' catalog, kept while the font directories are unchanged (the
/// rule `spare.rs` keeps a spare's catalog by, which is fontconfig's own):
/// the directories are read first, then the catalog made from them.
///
/// The directories are walked once, and after that each one's identity is
/// checked (`spare::font_dirs_unchanged`: a status call a directory, no
/// listing): the walk itself took 4-12 ms on macOS, at every keystroke of a
/// document that names a font.
#[derive(Default)]
struct KeptFonts {
    /// The last walk of the directories.
    walked: Option<Dirs>,
    /// The catalog made from them, and the walk it was made after.
    kept: Option<(Dirs, Arc<FontCatalog>)>,
}

impl KeptFonts {
    /// The font directories as they are now.
    fn now(&mut self) -> &Dirs {
        let same = self
            .walked
            .as_ref()
            .is_some_and(|w| super::spare::font_dirs_unchanged(w));
        if !same {
            self.walked = Some(super::spare::font_dirs());
        }
        self.walked.as_ref().expect("walked above")
    }
}

impl Fonts for KeptFonts {
    fn catalog(&mut self) -> Arc<FontCatalog> {
        let dirs = self.now().clone();
        if let Some((d, c)) = &self.kept {
            if *d == dirs {
                return c.clone();
            }
        }
        let c = Arc::new(FontCatalog::for_lookup());
        self.kept = Some((dirs, c.clone()));
        c
    }

    fn dirs(&mut self) -> [u64; 2] {
        let d = format!("{:?}", self.now());
        flashtex_engine::persist::hash128(d.as_bytes())
    }
}

/// The engine thread's scheduling: as the pdfTeX host's engine thread
/// (`FLASHTEX_HOST_QOS=default` leaves it), user-interactive on macOS, so
/// that a keystroke's run is not put on the efficiency cores.
fn interactive_qos() {
    #[cfg(target_os = "macos")]
    {
        if std::env::var("FLASHTEX_HOST_QOS").as_deref() == Ok("default") {
            return;
        }
        extern "C" {
            fn pthread_set_qos_class_self_np(qos_class: u32, relative_priority: i32) -> i32;
        }
        // <sys/qos.h>: QOS_CLASS_USER_INTERACTIVE
        const QOS_CLASS_USER_INTERACTIVE: u32 = 0x21;
        // SAFETY: sets the calling thread's own scheduling class.
        unsafe {
            pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0);
        }
    }
}

impl Shared {
    /// The engine thread's queue, started on first use.
    fn sender(&self) -> Result<mpsc::Sender<Req>, String> {
        let mut tx = self.tx.lock().unwrap();
        if let Some(t) = tx.as_ref() {
            return Ok(t.clone());
        }
        let (t, rx) = mpsc::channel::<Req>();
        let queued = self.queued.clone();
        std::thread::Builder::new()
            .name("unicode-engine".into())
            // the recursion TeX may need (the pdfTeX host's engine thread's)
            .stack_size(512 << 20)
            .spawn(move || {
                interactive_qos();
                let clock = flashtex_engine::host::pin_clock_from_env();
                let mut engine = Engine::new(clock, Some(Box::new(KeptFonts::default())));
                let waiting = || queued.load(Ordering::Acquire) > 0;
                let idle = save_when_idle();
                let save = |engine: &mut Engine, stop: &dyn Fn() -> bool| match engine
                    .save_pending(stop)
                {
                    Ok(Some(n)) if debug() => eprintln!("flashtex-host-unicode: {n}"),
                    Ok(_) => {}
                    Err(e) => eprintln!("flashtex-host-unicode: saving S0: {e}"),
                };
                loop {
                    // an asked-for save of S₀ is made when no request comes
                    // for a while, and stops for one that comes meanwhile
                    let req = if engine.has_pending_save() {
                        match rx.recv_timeout(idle) {
                            Ok(r) => r,
                            Err(mpsc::RecvTimeoutError::Timeout) => {
                                save(&mut engine, &waiting);
                                continue;
                            }
                            Err(mpsc::RecvTimeoutError::Disconnected) => {
                                save(&mut engine, &|| false);
                                break;
                            }
                        }
                    } else {
                        match rx.recv() {
                            Ok(r) => r,
                            Err(_) => break,
                        }
                    };
                    queued.fetch_sub(1, Ordering::AcqRel);
                    match req {
                        Req::Run { spec, reply } => {
                            let rep = engine.run(spec);
                            let _ = reply.send(rep);
                        }
                        Req::SaveS0(p, root) => match engine.save_s0(&p, &root) {
                            Ok(Some(n)) if debug() => eprintln!("flashtex-host-unicode: {n}"),
                            Ok(_) => {}
                            Err(e) => eprintln!("flashtex-host-unicode: saving S0: {e}"),
                        },
                        Req::Flush(done) => {
                            // (the host ends: the asked-for save is made
                            // now, whole)
                            save(&mut engine, &|| false);
                            engine.flush_writes();
                            let _ = done.send(());
                        }
                    }
                }
            })
            .map_err(|e| format!("the engine thread: {e}"))?;
        *tx = Some(t.clone());
        Ok(t)
    }

    /// Run `spec` on the engine thread; `during` runs on this thread while
    /// the engine does (it reads the display list). Runs take turns.
    pub fn run<T>(&self, spec: Spec, during: impl FnOnce() -> T) -> Result<(Report, T), String> {
        let _turn = self.turn.lock().unwrap_or_else(|p| p.into_inner());
        let tx = self.sender()?;
        let (reply, rx) = mpsc::channel();
        self.send(&tx, Req::Run { spec, reply })
            .map_err(|_| "the engine thread has ended".to_string())?;
        let t = during();
        let rep = rx
            .recv()
            .map_err(|_| "the engine thread has ended".to_string())?;
        Ok((rep, t))
    }
}

/// Save the engine's S₀ to `path` if it is new (`Engine::save_s0`), after
/// the runs queued before it.
impl Shared {
    pub fn save_s0(&self, path: PathBuf, root: PathBuf) {
        if let Ok(tx) = self.sender() {
            let _ = self.send(&tx, Req::SaveS0(path, root));
        }
    }

    /// Wait until every S₀ asked to be saved is written (a host about to
    /// end: its process takes the writers with it).
    pub fn flush(&self) {
        let Some(tx) = self.tx.lock().unwrap().clone() else {
            return;
        };
        let (done, rx) = mpsc::channel();
        if self.send(&tx, Req::Flush(done)).is_ok() {
            let _ = rx.recv();
        }
    }

    /// Send `req` to the engine thread, counted as waiting until it takes it.
    fn send(&self, tx: &mpsc::Sender<Req>, req: Req) -> Result<(), ()> {
        self.queued.fetch_add(1, Ordering::AcqRel);
        tx.send(req).map_err(|_| {
            self.queued.fetch_sub(1, Ordering::AcqRel);
        })
    }
}

fn debug() -> bool {
    std::env::var_os("FLASHTEX_UNICODE_DEBUG").is_some()
}

/// A new run's cancel flag.
pub fn cancel_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}
