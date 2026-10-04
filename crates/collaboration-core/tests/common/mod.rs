//! Shared test support for the `collab-v1` oracle: a seeded RNG, an
//! in-process network of peers with hostile delivery, and a tiny JSON
//! reader/writer for the fixture files (the crate has no dependencies).
#![allow(dead_code)]

use flashtex_collaboration_core::v1::wire::{self, Message};
use flashtex_collaboration_core::v1::{FileId, FileKind, Project, Section};

/// SplitMix64: tiny, seedable, the same in the Swift tests.
#[derive(Clone)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// Uniform in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    pub fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
    pub fn file_id(&mut self) -> FileId {
        let mut b = [0u8; 16];
        b[..8].copy_from_slice(&self.next().to_le_bytes());
        b[8..].copy_from_slice(&self.next().to_le_bytes());
        FileId(b)
    }
}

/// Pieces random text is made of: ASCII, LaTeX punctuation, two- three-
/// and four-byte scalars, and a combining mark (a grapheme that a
/// concurrent edit may split; a scalar never is).
pub const PIECES: &[&str] = &[
    "a", "b", "c", "x", "y", "z", " ", "\\", "{", "}", "\n", "é", "中", "😀", "e\u{301}",
];

pub fn random_text(rng: &mut Rng, max_pieces: usize) -> String {
    let n = 1 + rng.below(max_pieces);
    (0..n).map(|_| PIECES[rng.below(PIECES.len())]).collect()
}

pub const PATHS: &[&str] = &["main.tex", "a.tex", "ch/b.tex", "refs.bib"];

pub struct Msg {
    pub from: usize,
    pub to: usize,
    pub frame: Vec<u8>,
}

/// N peers exchanging `collab-v1` frames through a hostile network:
/// random order, duplicates, drops (repaired by state-vector sync), and
/// partitions. Every frame goes through the codec.
pub struct Sim {
    pub rng: Rng,
    pub peers: Vec<Project>,
    pub queue: Vec<Msg>,
    /// Every update frame broadcast, once each (the fixture's op log).
    pub frames: Vec<Vec<u8>>,
    pub partition: Option<Vec<bool>>,
    seq: u64,
}

impl Sim {
    pub fn new(seed: u64, n: usize) -> Self {
        let mut rng = Rng(seed);
        let mut ids: Vec<u64> = Vec::new();
        while ids.len() < n {
            let r = rng.next() | 1;
            if !ids.contains(&r) {
                ids.push(r);
            }
        }
        Self {
            rng,
            peers: ids.into_iter().map(Project::new).collect(),
            queue: Vec::new(),
            frames: Vec::new(),
            partition: None,
            seq: 0,
        }
    }

    pub fn broadcast(&mut self, from: usize, sections: Vec<Section>) {
        self.seq += 1;
        let frame = wire::encode(&Message::Update {
            seq: self.seq,
            sections,
        });
        for to in 0..self.peers.len() {
            if to != from {
                self.queue.push(Msg {
                    from,
                    to,
                    frame: frame.clone(),
                });
            }
        }
        self.frames.push(frame);
    }

    pub fn receive_frame(&mut self, to: usize, frame: &[u8]) {
        let (msg, used) = wire::decode(frame).expect("decodes").expect("complete");
        assert_eq!(used, frame.len());
        assert_eq!(wire::encode(&msg), frame, "canonical re-encoding");
        let sections = match msg {
            Message::Update { sections, .. } | Message::SyncReply(sections) => sections,
            other => panic!("unexpected {other:?}"),
        };
        let errors = self.peers[to].receive(&sections);
        assert!(errors.is_empty(), "peer {to} refused: {errors:?}");
    }

    fn deliverable(&self, m: &Msg) -> bool {
        match &self.partition {
            None => true,
            Some(side) => side[m.from] == side[m.to],
        }
    }

    /// Deliver one random in-flight message (maybe twice, maybe never).
    pub fn deliver_one(&mut self) -> bool {
        let candidates: Vec<usize> = (0..self.queue.len())
            .filter(|&i| self.deliverable(&self.queue[i]))
            .collect();
        if candidates.is_empty() {
            return false;
        }
        let i = candidates[self.rng.below(candidates.len())];
        let m = self.queue.swap_remove(i);
        if self.rng.chance(3) {
            return true; // dropped; the final sync repairs it
        }
        self.receive_frame(m.to, &m.frame);
        if self.rng.chance(8) {
            self.queue.push(m); // duplicated
        }
        true
    }

    pub fn text_files(&self, p: usize) -> Vec<FileId> {
        self.peers[p]
            .state_vectors()
            .into_iter()
            .filter_map(|(d, _)| match d {
                flashtex_collaboration_core::v1::DocRef::Text(f) => Some(f),
                _ => None,
            })
            .collect()
    }

    pub fn random_text_edit(&mut self, p: usize) {
        let files = self.text_files(p);
        if files.is_empty() {
            return;
        }
        let f = files[self.rng.below(files.len())];
        let len = self.peers[p].text(f).unwrap().len();
        let section = if len > 0 && self.rng.chance(35) {
            let pos = self.rng.below(len);
            let n = 1 + self.rng.below((len - pos).min(6));
            self.peers[p].delete(f, pos, n)
        } else {
            let pos = self.rng.below(len + 1);
            let t = random_text(&mut self.rng, 6);
            self.peers[p].insert(f, pos, &t)
        };
        if let Some(s) = section {
            self.broadcast(p, vec![s]);
        }
    }

    pub fn random_file_op(&mut self, p: usize) {
        let path = PATHS[self.rng.below(PATHS.len())];
        let known: Vec<FileId> = self.peers[p]
            .files()
            .files()
            .iter()
            .map(|e| e.file)
            .collect();
        let all = self.text_files(p);
        let roll = self.rng.below(4);
        let section = if roll == 0 || all.is_empty() {
            let id = self.rng.file_id();
            self.peers[p].file_op(|m| m.create(id, FileKind::Text, path))
        } else if roll == 1 && !known.is_empty() {
            let f = known[self.rng.below(known.len())];
            self.peers[p].file_op(|m| m.rename(f, path))
        } else {
            let f = all[self.rng.below(all.len())];
            let del = !self.peers[p].files().is_deleted(f).unwrap();
            self.peers[p].file_op(|m| m.set_deleted(f, del))
        };
        self.broadcast(p, vec![section.expect("local file op applies")]);
    }

    /// Deliver everything in flight, then repair drops with pairwise
    /// state-vector sync until every peer has everything.
    pub fn heal(&mut self) {
        self.partition = None;
        while self.deliver_one() {}
        for _round in 0..3 {
            for i in 0..self.peers.len() {
                for j in 0..self.peers.len() {
                    if i == j {
                        continue;
                    }
                    let req = self.peers[j].state_vectors();
                    let frame = wire::encode(&Message::SyncRequest(req));
                    let Message::SyncRequest(req) = wire::decode(&frame).unwrap().unwrap().0 else {
                        unreachable!()
                    };
                    let reply = wire::encode(&Message::SyncReply(self.peers[i].diff(&req)));
                    self.receive_frame(j, &reply);
                }
            }
        }
    }

    pub fn assert_converged(&self, ctx: &str) {
        let first = &self.peers[0];
        for (k, p) in self.peers.iter().enumerate() {
            assert_eq!(p.pending_len(), 0, "{ctx}: peer {k} still has pending ops");
            assert_eq!(
                p.state_vectors(),
                first.state_vectors(),
                "{ctx}: peer {k} state vectors"
            );
            for (d, _) in first.state_vectors() {
                if let flashtex_collaboration_core::v1::DocRef::Text(f) = d {
                    assert_eq!(
                        p.text(f).unwrap().text(),
                        first.text(f).unwrap().text(),
                        "{ctx}: peer {k} text differs"
                    );
                }
            }
            assert_eq!(
                p.files().files(),
                first.files().files(),
                "{ctx}: peer {k} file map"
            );
            assert_eq!(p.digest(), first.digest(), "{ctx}: peer {k} digest");
        }
    }
}

// ----------------------------------------------------------------------
// Fixture JSON: only what the fixtures use (objects, arrays, strings,
// non-negative integers).
// ----------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Obj(Vec<(String, Json)>),
    Arr(Vec<Json>),
    Str(String),
    Num(u64),
}

impl Json {
    pub fn get(&self, key: &str) -> &Json {
        match self {
            Json::Obj(kv) => {
                &kv.iter()
                    .find(|(k, _)| k == key)
                    .unwrap_or_else(|| panic!("no key {key}"))
                    .1
            }
            _ => panic!("not an object"),
        }
    }
    pub fn str(&self) -> &str {
        match self {
            Json::Str(s) => s,
            _ => panic!("not a string"),
        }
    }
    pub fn arr(&self) -> &[Json] {
        match self {
            Json::Arr(a) => a,
            _ => panic!("not an array"),
        }
    }
    pub fn num(&self) -> u64 {
        match self {
            Json::Num(n) => *n,
            _ => panic!("not a number"),
        }
    }

    /// The same value with every object's keys sorted (Swift's encoder
    /// may order them differently).
    pub fn sorted(&self) -> Json {
        match self {
            Json::Obj(kv) => {
                let mut kv: Vec<(String, Json)> =
                    kv.iter().map(|(k, v)| (k.clone(), v.sorted())).collect();
                kv.sort_by(|a, b| a.0.cmp(&b.0));
                Json::Obj(kv)
            }
            Json::Arr(a) => Json::Arr(a.iter().map(Json::sorted).collect()),
            other => other.clone(),
        }
    }

    pub fn write(&self, out: &mut String, indent: usize) {
        let pad = "  ".repeat(indent);
        match self {
            Json::Num(n) => out.push_str(&n.to_string()),
            Json::Str(s) => {
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                        c => out.push(c),
                    }
                }
                out.push('"');
            }
            Json::Arr(a) => {
                if a.iter().all(|j| matches!(j, Json::Num(_))) {
                    out.push('[');
                    for (i, j) in a.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        j.write(out, 0);
                    }
                    out.push(']');
                    return;
                }
                out.push_str("[\n");
                for (i, j) in a.iter().enumerate() {
                    out.push_str(&pad);
                    out.push_str("  ");
                    j.write(out, indent + 1);
                    if i + 1 < a.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&pad);
                out.push(']');
            }
            Json::Obj(kv) => {
                out.push_str("{\n");
                for (i, (k, v)) in kv.iter().enumerate() {
                    out.push_str(&pad);
                    out.push_str("  ");
                    Json::Str(k.clone()).write(out, 0);
                    out.push_str(": ");
                    v.write(out, indent + 1);
                    if i + 1 < kv.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&pad);
                out.push('}');
            }
        }
    }

    pub fn parse(src: &str) -> Json {
        let b: Vec<char> = src.chars().collect();
        let mut i = 0;
        let v = parse_value(&b, &mut i);
        skip_ws(&b, &mut i);
        assert_eq!(i, b.len(), "trailing JSON");
        v
    }
}

fn skip_ws(b: &[char], i: &mut usize) {
    while *i < b.len() && b[*i].is_whitespace() {
        *i += 1;
    }
}

fn parse_value(b: &[char], i: &mut usize) -> Json {
    skip_ws(b, i);
    match b[*i] {
        '{' => {
            *i += 1;
            let mut kv = Vec::new();
            loop {
                skip_ws(b, i);
                if b[*i] == '}' {
                    *i += 1;
                    return Json::Obj(kv);
                }
                if b[*i] == ',' {
                    *i += 1;
                    continue;
                }
                let Json::Str(k) = parse_value(b, i) else {
                    panic!("key")
                };
                skip_ws(b, i);
                assert_eq!(b[*i], ':');
                *i += 1;
                kv.push((k, parse_value(b, i)));
            }
        }
        '[' => {
            *i += 1;
            let mut a = Vec::new();
            loop {
                skip_ws(b, i);
                if b[*i] == ']' {
                    *i += 1;
                    return Json::Arr(a);
                }
                if b[*i] == ',' {
                    *i += 1;
                    continue;
                }
                a.push(parse_value(b, i));
            }
        }
        '"' => {
            *i += 1;
            let mut s = String::new();
            loop {
                let c = b[*i];
                *i += 1;
                match c {
                    '"' => return Json::Str(s),
                    '\\' => {
                        let e = b[*i];
                        *i += 1;
                        match e {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'r' => s.push('\r'),
                            'b' => s.push('\u{8}'),
                            'f' => s.push('\u{c}'),
                            'u' => {
                                let hex = |i: &mut usize| {
                                    let h: String = b[*i..*i + 4].iter().collect();
                                    *i += 4;
                                    u32::from_str_radix(&h, 16).unwrap()
                                };
                                let mut cp = hex(i);
                                if (0xd800..0xdc00).contains(&cp) {
                                    assert_eq!(b[*i], '\\');
                                    *i += 2;
                                    let lo = hex(i);
                                    cp = 0x10000 + ((cp - 0xd800) << 10) + (lo - 0xdc00);
                                }
                                s.push(char::from_u32(cp).unwrap());
                            }
                            other => s.push(other),
                        }
                    }
                    c => s.push(c),
                }
            }
        }
        c if c.is_ascii_digit() => {
            let start = *i;
            while *i < b.len() && b[*i].is_ascii_digit() {
                *i += 1;
            }
            Json::Num(b[start..*i].iter().collect::<String>().parse().unwrap())
        }
        c => panic!("unexpected {c:?} in JSON"),
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
