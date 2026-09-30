//! A blocking client for the engine host's Unix socket (spec §6).
//!
//! ```no_run
//! use flashtex_display_list::client::{Client, CompileRequest, Event};
//! let mut c = Client::connect("/tmp/flashtex.sock".as_ref()).unwrap();
//! c.compile(&CompileRequest::new(1, "/path/to/project", "main.tex")).unwrap();
//! while let Some(ev) = c.next_event().unwrap() {
//!     match ev {
//!         Event::Page(p) => println!("page {} with {} items", p.index, p.items.len()),
//!         Event::Done(d) => { println!("{}", d.to_string()); break }
//!         _ => {}
//!     }
//! }
//! ```

use crate::frame::{read_frame, write_frame};
use crate::json::{obj, s, Json};
use crate::page::{Page, StreamKind};
use crate::resource::{Font, Sources};
use crate::{kind, PROTOCOL, VERSION_MAJOR, VERSION_MINOR};
use std::io::{self, BufReader, BufWriter, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

/// One message from the host, decoded.
#[derive(Debug)]
pub enum Event {
    Started(Json),
    Font(Font),
    Image(Json),
    Page(Page),
    Form(Page),
    Sources(Sources),
    Diagnostic(Json),
    Done(Json),
    Error(Json),
    /// 3.1: which of an incremental client's pages are current and which
    /// stale (spec §6.4).
    Pages(Json),
    /// `diag-v1` (spec §6.7), for a client that accepted it.
    Diag(crate::diag::Diag),
    /// A kind this version does not know (a later minor version's): skip.
    Other(u8, Vec<u8>),
}

/// Decode one host frame.
pub fn decode_event(k: u8, body: Vec<u8>) -> Result<Event, String> {
    let json = |b: &[u8]| -> Result<Json, String> {
        Json::parse(std::str::from_utf8(b).map_err(|e| e.to_string())?)
    };
    Ok(match k {
        kind::STARTED => Event::Started(json(&body)?),
        kind::FONT => Event::Font(Font::decode(&body)?),
        kind::IMAGE => Event::Image(json(&body)?),
        kind::PAGE => Event::Page(Page::decode(StreamKind::Page, &body)?),
        kind::FORM => Event::Form(Page::decode(StreamKind::Form, &body)?),
        kind::SOURCES => Event::Sources(Sources::from_json(&json(&body)?)?),
        kind::DIAGNOSTIC => Event::Diagnostic(json(&body)?),
        kind::DONE => Event::Done(json(&body)?),
        kind::ERROR => Event::Error(json(&body)?),
        kind::PAGES => Event::Pages(json(&body)?),
        kind::DIAG => Event::Diag(crate::diag::Diag::decode(&body)?),
        _ => Event::Other(k, body),
    })
}

/// A `COMPILE` request (spec §6.3).
#[derive(Clone, Debug)]
pub struct CompileRequest {
    pub id: i64,
    /// The project directory (absolute); the engine runs there.
    pub root: String,
    /// The main file, relative to `root`.
    pub main: String,
    /// Format name (default `pdflatex`).
    pub format: String,
    /// `\write18`: "default" (texmf.cnf's, restricted in TeX Live), "off",
    /// "restricted" or "on".
    pub shell_escape: String,
    /// Where the PDF and log go (default: the host's choice, reported in
    /// `DONE`).
    pub output_dir: Option<String>,
    /// Job name (default: the main file's).
    pub jobname: Option<String>,
    /// Font keys the client already holds: their programs are not sent.
    pub have_fonts: Vec<String>,
    /// 3.1: keep the pages and resources of earlier compiles on this
    /// connection; only pages that changed are sent (spec §6.3).
    pub incremental: bool,
    /// 3.1: the page (0-based) the client shows: it is typeset first.
    pub viewport: Option<u32>,
    /// 3.1: files as the editor has them: (path relative to `root`, text).
    pub buffers: Vec<(String, String)>,
    /// 3.1: byte splices: (path, offset, bytes deleted, text inserted).
    pub edits: Vec<Edit>,
    /// A one-shot export run (the compressed PDF as pdflatex writes it).
    pub export: bool,
}

/// A splice of a file's bytes (spec §6.3, `edits`).
#[derive(Clone, Debug)]
pub struct Edit {
    pub path: String,
    pub offset: u64,
    pub delete: u64,
    pub insert: String,
}

impl CompileRequest {
    pub fn new(id: i64, root: &str, main: &str) -> Self {
        CompileRequest {
            id,
            root: root.into(),
            main: main.into(),
            format: "pdflatex".into(),
            shell_escape: "default".into(),
            output_dir: None,
            jobname: None,
            have_fonts: vec![],
            incremental: false,
            viewport: None,
            buffers: vec![],
            edits: vec![],
            export: false,
        }
    }

    pub fn to_json(&self) -> Json {
        let mut kv = vec![
            ("id".to_string(), Json::Int(self.id)),
            ("root".to_string(), s(&*self.root)),
            ("main".to_string(), s(&*self.main)),
            ("format".to_string(), s(&*self.format)),
            ("shell_escape".to_string(), s(&*self.shell_escape)),
        ];
        if let Some(d) = &self.output_dir {
            kv.push(("output_dir".into(), s(d.as_str())));
        }
        if let Some(j) = &self.jobname {
            kv.push(("jobname".into(), s(j.as_str())));
        }
        if !self.have_fonts.is_empty() {
            kv.push((
                "have_fonts".into(),
                Json::Arr(self.have_fonts.iter().map(|k| s(k.as_str())).collect()),
            ));
        }
        if self.incremental {
            kv.push(("incremental".into(), Json::Bool(true)));
        }
        if let Some(v) = self.viewport {
            kv.push(("viewport".into(), Json::Int(v as i64)));
        }
        if !self.buffers.is_empty() {
            kv.push((
                "buffers".into(),
                Json::Arr(
                    self.buffers
                        .iter()
                        .map(|(p, t)| obj([("path", s(p.as_str())), ("text", s(t.as_str()))]))
                        .collect(),
                ),
            ));
        }
        if !self.edits.is_empty() {
            kv.push((
                "edits".into(),
                Json::Arr(
                    self.edits
                        .iter()
                        .map(|e| {
                            obj([
                                ("path", s(e.path.as_str())),
                                ("offset", Json::Int(e.offset as i64)),
                                ("delete", Json::Int(e.delete as i64)),
                                ("insert", s(e.insert.as_str())),
                            ])
                        })
                        .collect(),
                ),
            ));
        }
        if self.export {
            kv.push(("export".into(), Json::Bool(true)));
        }
        Json::Obj(kv)
    }
}

/// A connection to the engine host.
pub struct Client {
    r: BufReader<UnixStream>,
    w: BufWriter<UnixStream>,
    /// The host's `HELLO`.
    pub hello: Json,
}

/// A handle that can cancel from another thread.
pub struct Canceller(UnixStream);

impl Canceller {
    pub fn cancel(&mut self, id: i64) -> io::Result<()> {
        let b = obj([("id", Json::Int(id))]).to_string();
        write_frame(&mut self.0, kind::CANCEL, b.as_bytes())?;
        self.0.flush()
    }
}

fn proto(e: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.into())
}

impl Client {
    /// Connect and exchange `HELLO`s; refuses a host of another major
    /// version.
    pub fn connect(path: &Path) -> io::Result<Client> {
        let stream = UnixStream::connect(path)?;
        Self::over(stream)
    }

    /// `connect`, accepting optional message families the host may offer
    /// (`HELLO.accept`, e.g. [`crate::diag::CAPABILITY`]).
    pub fn connect_accepting(path: &Path, accept: &[&str]) -> io::Result<Client> {
        let stream = UnixStream::connect(path)?;
        Self::over_accepting(stream, accept)
    }

    /// The same over an already connected stream.
    pub fn over(stream: UnixStream) -> io::Result<Client> {
        Self::over_accepting(stream, &[])
    }

    /// `over`, accepting optional message families.
    pub fn over_accepting(stream: UnixStream, accept: &[&str]) -> io::Result<Client> {
        crate::widen_socket_buffers(&stream);
        let mut c = Client {
            r: BufReader::with_capacity(1 << 20, stream.try_clone()?),
            w: BufWriter::new(stream),
            hello: Json::Null,
        };
        let hello = obj([
            ("protocol", s(PROTOCOL)),
            (
                "version",
                Json::Arr(vec![
                    Json::Int(VERSION_MAJOR as i64),
                    Json::Int(VERSION_MINOR as i64),
                ]),
            ),
            (
                "client",
                s(concat!("flashtex-display-list ", env!("CARGO_PKG_VERSION"))),
            ),
        ]);
        let hello = match hello {
            Json::Obj(mut kv) if !accept.is_empty() => {
                kv.push((
                    "accept".into(),
                    Json::Arr(accept.iter().map(|a| s(*a)).collect()),
                ));
                Json::Obj(kv)
            }
            h => h,
        };
        write_frame(&mut c.w, kind::C_HELLO, hello.to_string().as_bytes())?;
        c.w.flush()?;
        let (k, body) = read_frame(&mut c.r)?.ok_or_else(|| proto("host closed before HELLO"))?;
        let j = Json::parse(std::str::from_utf8(&body).map_err(|e| proto(e.to_string()))?)
            .map_err(proto)?;
        if k == kind::ERROR {
            return Err(proto(format!("host refused: {}", j)));
        }
        if k != kind::HELLO || j.str_field("protocol") != Some(PROTOCOL) {
            return Err(proto("not a display-list-v3 host"));
        }
        let major = j
            .get("version")
            .and_then(Json::as_array)
            .and_then(|a| a.first())
            .and_then(Json::as_i64);
        if major != Some(VERSION_MAJOR as i64) {
            return Err(proto(format!(
                "host speaks version {major:?}, not {VERSION_MAJOR}"
            )));
        }
        c.hello = j;
        Ok(c)
    }

    pub fn canceller(&self) -> io::Result<Canceller> {
        Ok(Canceller(self.w.get_ref().try_clone()?))
    }

    /// Send a `COMPILE` request. A request while another compile runs
    /// cancels that one first (its `DONE` says `cancelled`).
    pub fn compile(&mut self, req: &CompileRequest) -> io::Result<()> {
        write_frame(
            &mut self.w,
            kind::COMPILE,
            req.to_json().to_string().as_bytes(),
        )?;
        self.w.flush()
    }

    pub fn cancel(&mut self, id: i64) -> io::Result<()> {
        let b = obj([("id", Json::Int(id))]).to_string();
        write_frame(&mut self.w, kind::CANCEL, b.as_bytes())?;
        self.w.flush()
    }

    pub fn bye(&mut self) -> io::Result<()> {
        write_frame(&mut self.w, kind::BYE, b"{}")?;
        self.w.flush()
    }

    /// The connection's reader, for a caller that reads raw frames
    /// ([`crate::frame::read_frame`]) and decodes them itself.
    pub fn reader(&mut self) -> &mut BufReader<UnixStream> {
        &mut self.r
    }

    /// The next message, or `None` when the host closed the connection.
    pub fn next_event(&mut self) -> io::Result<Option<Event>> {
        match read_frame(&mut self.r)? {
            None => Ok(None),
            Some((k, body)) => decode_event(k, body).map(Some).map_err(proto),
        }
    }
}
