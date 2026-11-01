//! `collab-v1` framing and the binary message bodies
//! (`docs/contracts/collab-v1.md` §3–§4). The JSON control bodies (`join`,
//! `awareness`, ...) are carried as opaque bytes here; the Swift core
//! decodes them.

use super::filemap::{BlobRef, FileId, FileKind, FileOp, FileOpKind};
use super::project::{DocRef, Section};
use super::text::TextOp;
use super::{Id, StateVector};

/// Largest frame (kind + body), 16 MiB.
pub const MAX_FRAME: usize = 1 << 24;

pub mod kind {
    pub const JOIN: u8 = 0x01;
    pub const JOIN_ACK: u8 = 0x02;
    pub const SYNC_REQUEST: u8 = 0x03;
    pub const SYNC_REPLY: u8 = 0x04;
    pub const UPDATE: u8 = 0x05;
    pub const ACK: u8 = 0x06;
    pub const AWARENESS: u8 = 0x07;
    pub const BLOB_WANT: u8 = 0x08;
    pub const BLOB_CHUNK: u8 = 0x09;
    pub const PREVIEW_SUBSCRIBE: u8 = 0x0A;
    pub const PREVIEW_FRAME: u8 = 0x0B;
    pub const COMPILE_REPORT: u8 = 0x0C;
    pub const LEAVE: u8 = 0x0D;
    pub const ERROR: u8 = 0x0E;

    /// Kinds whose body is a JSON object.
    pub fn is_json(k: u8) -> bool {
        matches!(
            k,
            JOIN | JOIN_ACK
                | ACK
                | AWARENESS
                | BLOB_WANT
                | PREVIEW_SUBSCRIBE
                | COMPILE_REPORT
                | LEAVE
                | ERROR
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    SyncRequest(Vec<(DocRef, StateVector)>),
    SyncReply(Vec<Section>),
    Update {
        seq: u64,
        sections: Vec<Section>,
    },
    BlobChunk {
        sha256: [u8; 32],
        offset: u64,
        data: Vec<u8>,
    },
    /// One display-list-v3 frame, verbatim.
    PreviewFrame(Vec<u8>),
    /// A JSON-bodied kind ([`kind::is_json`]); the bytes are not parsed.
    Json {
        kind: u8,
        body: Vec<u8>,
    },
    /// A kind this version does not know. Not fatal: answer with `error`.
    Unknown {
        kind: u8,
        body: Vec<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireError {
    EmptyFrame,
    FrameTooLarge,
    Truncated,
    BadVarint,
    BadUtf8,
    BadTag(u8),
    TrailingBytes,
    Invalid(&'static str),
}

// ----------------------------------------------------------------------
// Encoding
// ----------------------------------------------------------------------

#[derive(Default)]
struct W(Vec<u8>);

impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn var(&mut self, mut v: u64) {
        loop {
            let b = (v & 0x7f) as u8;
            v >>= 7;
            if v == 0 {
                self.0.push(b);
                return;
            }
            self.0.push(b | 0x80);
        }
    }
    fn bytes(&mut self, b: &[u8]) {
        self.var(b.len() as u64);
        self.0.extend_from_slice(b);
    }
    fn id(&mut self, id: Id) {
        self.var(id.replica);
        self.var(id.counter);
    }
    fn opt_id(&mut self, id: Option<Id>) {
        match id {
            None => self.u8(0),
            Some(id) => {
                self.u8(1);
                self.id(id);
            }
        }
    }
    fn doc(&mut self, d: DocRef) {
        match d {
            DocRef::FileMap => self.u8(0),
            DocRef::Text(f) => {
                self.u8(1);
                self.0.extend_from_slice(&f.0);
            }
        }
    }
    fn text_op(&mut self, op: &TextOp) {
        match op {
            TextOp::Insert {
                id,
                origin_left,
                origin_right,
                content,
            } => {
                self.u8(1);
                self.id(*id);
                self.opt_id(*origin_left);
                self.opt_id(*origin_right);
                self.bytes(content.as_bytes());
            }
            TextOp::Delete { id, target, len } => {
                self.u8(2);
                self.id(*id);
                self.id(*target);
                self.var(*len);
            }
        }
    }
    fn file_op(&mut self, op: &FileOp) {
        self.id(op.id);
        self.var(op.lamport);
        self.0.extend_from_slice(&op.file.0);
        match &op.kind {
            FileOpKind::Create { kind, path } => {
                self.u8(1);
                self.u8(*kind as u8);
                self.bytes(path.as_bytes());
            }
            FileOpKind::SetPath(p) => {
                self.u8(2);
                self.bytes(p.as_bytes());
            }
            FileOpKind::SetBlob(b) => {
                self.u8(3);
                self.0.extend_from_slice(&b.sha256);
                self.var(b.bytes);
                self.bytes(b.media_type.as_bytes());
            }
            FileOpKind::SetDeleted(d) => {
                self.u8(4);
                self.u8(u8::from(*d));
            }
        }
    }
    fn sections(&mut self, s: &[Section]) {
        self.var(s.len() as u64);
        for sec in s {
            match sec {
                Section::FileMap(ops) => {
                    self.doc(DocRef::FileMap);
                    self.var(ops.len() as u64);
                    ops.iter().for_each(|op| self.file_op(op));
                }
                Section::Text(f, ops) => {
                    self.doc(DocRef::Text(*f));
                    self.var(ops.len() as u64);
                    ops.iter().for_each(|op| self.text_op(op));
                }
            }
        }
    }
}

/// One complete frame: `u32` LE length, kind, body.
pub fn encode(msg: &Message) -> Vec<u8> {
    let mut w = W::default();
    let k = match msg {
        Message::SyncRequest(svs) => {
            w.var(svs.len() as u64);
            for (d, sv) in svs {
                w.doc(*d);
                w.var(sv.0.len() as u64);
                for (r, n) in &sv.0 {
                    w.var(*r);
                    w.var(*n);
                }
            }
            kind::SYNC_REQUEST
        }
        Message::SyncReply(s) => {
            w.sections(s);
            kind::SYNC_REPLY
        }
        Message::Update { seq, sections } => {
            w.var(*seq);
            w.sections(sections);
            kind::UPDATE
        }
        Message::BlobChunk {
            sha256,
            offset,
            data,
        } => {
            w.0.extend_from_slice(sha256);
            w.var(*offset);
            w.bytes(data);
            kind::BLOB_CHUNK
        }
        Message::PreviewFrame(b) => {
            w.0.extend_from_slice(b);
            kind::PREVIEW_FRAME
        }
        Message::Json { kind, body } | Message::Unknown { kind, body } => {
            w.0.extend_from_slice(body);
            *kind
        }
    };
    let mut out = Vec::with_capacity(w.0.len() + 5);
    out.extend_from_slice(&((w.0.len() + 1) as u32).to_le_bytes());
    out.push(k);
    out.extend_from_slice(&w.0);
    out
}

// ----------------------------------------------------------------------
// Decoding
// ----------------------------------------------------------------------

struct R<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> R<'a> {
    fn u8(&mut self) -> Result<u8, WireError> {
        let v = *self.b.get(self.i).ok_or(WireError::Truncated)?;
        self.i += 1;
        Ok(v)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], WireError> {
        if self.b.len() - self.i < n {
            return Err(WireError::Truncated);
        }
        let s = &self.b[self.i..self.i + n];
        self.i += n;
        Ok(s)
    }
    fn var(&mut self) -> Result<u64, WireError> {
        let mut v: u64 = 0;
        for shift in (0..70).step_by(7) {
            let b = self.u8()?;
            if shift == 63 && b > 1 {
                return Err(WireError::BadVarint);
            }
            v |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                if b == 0 && shift > 0 {
                    return Err(WireError::BadVarint); // overlong
                }
                return Ok(v);
            }
        }
        Err(WireError::BadVarint)
    }
    fn count(&mut self) -> Result<usize, WireError> {
        let n = self.var()?;
        // Every element takes at least one byte: a count beyond what is
        // left is a lie, refused before anything is allocated.
        if n > (self.b.len() - self.i) as u64 {
            return Err(WireError::Truncated);
        }
        Ok(n as usize)
    }
    fn string(&mut self) -> Result<String, WireError> {
        let n = self.count()?;
        let raw = self.take(n)?;
        String::from_utf8(raw.to_vec()).map_err(|_| WireError::BadUtf8)
    }
    fn id(&mut self) -> Result<Id, WireError> {
        Ok(Id::new(self.var()?, self.var()?))
    }
    fn opt_id(&mut self) -> Result<Option<Id>, WireError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(self.id()?)),
            t => Err(WireError::BadTag(t)),
        }
    }
    fn file_id(&mut self) -> Result<FileId, WireError> {
        Ok(FileId(self.take(16)?.try_into().unwrap()))
    }
    fn doc(&mut self) -> Result<DocRef, WireError> {
        match self.u8()? {
            0 => Ok(DocRef::FileMap),
            1 => Ok(DocRef::Text(self.file_id()?)),
            t => Err(WireError::BadTag(t)),
        }
    }
    fn text_op(&mut self) -> Result<TextOp, WireError> {
        match self.u8()? {
            1 => {
                let id = self.id()?;
                let origin_left = self.opt_id()?;
                let origin_right = self.opt_id()?;
                let content = self.string()?;
                if content.is_empty() {
                    return Err(WireError::Invalid("empty insert"));
                }
                Ok(TextOp::Insert {
                    id,
                    origin_left,
                    origin_right,
                    content,
                })
            }
            2 => {
                let id = self.id()?;
                let target = self.id()?;
                let len = self.var()?;
                if len == 0 {
                    return Err(WireError::Invalid("empty delete"));
                }
                Ok(TextOp::Delete { id, target, len })
            }
            t => Err(WireError::BadTag(t)),
        }
    }
    fn file_op(&mut self) -> Result<FileOp, WireError> {
        let id = self.id()?;
        let lamport = self.var()?;
        let file = self.file_id()?;
        let kind = match self.u8()? {
            1 => {
                let kind = match self.u8()? {
                    0 => FileKind::Text,
                    1 => FileKind::Blob,
                    t => return Err(WireError::BadTag(t)),
                };
                FileOpKind::Create {
                    kind,
                    path: self.string()?,
                }
            }
            2 => FileOpKind::SetPath(self.string()?),
            3 => FileOpKind::SetBlob(BlobRef {
                sha256: self.take(32)?.try_into().unwrap(),
                bytes: self.var()?,
                media_type: self.string()?,
            }),
            4 => match self.u8()? {
                0 => FileOpKind::SetDeleted(false),
                1 => FileOpKind::SetDeleted(true),
                t => return Err(WireError::BadTag(t)),
            },
            t => return Err(WireError::BadTag(t)),
        };
        Ok(FileOp {
            id,
            lamport,
            file,
            kind,
        })
    }
    fn sections(&mut self) -> Result<Vec<Section>, WireError> {
        let n = self.count()?;
        let mut out = Vec::new();
        for _ in 0..n {
            match self.doc()? {
                DocRef::FileMap => {
                    let m = self.count()?;
                    let mut ops = Vec::new();
                    for _ in 0..m {
                        ops.push(self.file_op()?);
                    }
                    out.push(Section::FileMap(ops));
                }
                DocRef::Text(f) => {
                    let m = self.count()?;
                    let mut ops = Vec::new();
                    for _ in 0..m {
                        ops.push(self.text_op()?);
                    }
                    out.push(Section::Text(f, ops));
                }
            }
        }
        Ok(out)
    }
    fn end(&self) -> Result<(), WireError> {
        if self.i == self.b.len() {
            Ok(())
        } else {
            Err(WireError::TrailingBytes)
        }
    }
}

/// Decode one frame from the start of `buf`. `Ok(None)`: not all of it has
/// arrived yet. `Ok(Some((msg, used)))`: one message and the bytes it took.
pub fn decode(buf: &[u8]) -> Result<Option<(Message, usize)>, WireError> {
    if buf.len() < 4 {
        return Ok(None);
    }
    let len = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
    if len == 0 {
        return Err(WireError::EmptyFrame);
    }
    if len > MAX_FRAME {
        return Err(WireError::FrameTooLarge);
    }
    if buf.len() < 4 + len {
        return Ok(None);
    }
    let k = buf[4];
    let body = &buf[5..4 + len];
    let mut r = R { b: body, i: 0 };
    let msg = match k {
        kind::SYNC_REQUEST => {
            let n = r.count()?;
            let mut svs = Vec::new();
            for _ in 0..n {
                let d = r.doc()?;
                let m = r.count()?;
                let mut sv = StateVector::default();
                for _ in 0..m {
                    let replica = r.var()?;
                    let next = r.var()?;
                    sv.set(replica, next);
                }
                svs.push((d, sv));
            }
            Message::SyncRequest(svs)
        }
        kind::SYNC_REPLY => Message::SyncReply(r.sections()?),
        kind::UPDATE => {
            let seq = r.var()?;
            Message::Update {
                seq,
                sections: r.sections()?,
            }
        }
        kind::BLOB_CHUNK => {
            let sha256 = r.take(32)?.try_into().unwrap();
            let offset = r.var()?;
            let n = r.count()?;
            Message::BlobChunk {
                sha256,
                offset,
                data: r.take(n)?.to_vec(),
            }
        }
        kind::PREVIEW_FRAME => {
            r.i = body.len();
            Message::PreviewFrame(body.to_vec())
        }
        k if kind::is_json(k) => {
            r.i = body.len();
            Message::Json {
                kind: k,
                body: body.to_vec(),
            }
        }
        k => {
            r.i = body.len();
            Message::Unknown {
                kind: k,
                body: body.to_vec(),
            }
        }
    };
    r.end()?;
    Ok(Some((msg, 4 + len)))
}
