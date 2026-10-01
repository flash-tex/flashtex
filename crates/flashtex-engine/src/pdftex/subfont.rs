//! `subfont.c`, ported: subfont map entries (`name@sfd@`), which split a
//! TrueType font with more than 256 glyphs (CJK, Unicode) into TeX fonts
//! of 256 characters, one per subfont of a subfont definition file
//! (`.sfd`, `kpse_sfd_format`).
//!
//! An entry `prefix@Unicode@ ... <font.ttf` becomes one entry per subfont
//! `infix` of `Unicode.sfd`, named `prefix` + `infix`, whose characters are
//! the character codes the subfont lists (writettf.rs finds their glyphs
//! through the font's `cmap`).

use super::cfmt;
use super::fonts::Fonts;
use super::mapfile::{FmEntry, Mode, F_SUBFONT};
use super::output::set_cur_file_name;
use crate::generated::Globals;
use crate::resolver::Format;

/// `SFD_BUF_SIZE` (`SMALL_BUF_SIZE`).
const SFD_BUF_SIZE: usize = 256;

/// A subfont: its infix and the character code of each of its 256
/// characters (-1: unassigned).
pub type Subfont = (Vec<u8>, Vec<i32>);

/// The reader of an `.sfd` file (`sfd_file`, `sfd_line`).
struct Sfd {
    data: Vec<u8>,
    pos: usize,
    eof: bool,
    line: Vec<u8>,
}

impl Sfd {
    /// `getc(sfd_file)`.
    fn getc(&mut self) -> i32 {
        match self.data.get(self.pos) {
            Some(&b) => {
                self.pos += 1;
                b as i32
            }
            None => {
                self.eof = true;
                -1
            }
        }
    }
}

impl Globals {
    /// `sfd_getline`: the next line that is neither empty nor a comment
    /// (`#`), through `append_char_to_buf` and `append_eol`.
    fn sfd_getline(&mut self, f: &mut Sfd, expect_eof: bool) {
        loop {
            if f.eof {
                if expect_eof {
                    return;
                }
                self.pdftex_fail("unexpected end of file");
            }
            f.line.clear();
            loop {
                let mut c = f.getc();
                // append_char_to_buf(c, p, sfd_line, SFD_BUF_SIZE)
                if c == 9 {
                    c = 32;
                }
                if c == 13 || c == -1 {
                    c = 10;
                }
                if c != b' ' as i32 || f.line.last().is_some_and(|&l| l != 32) {
                    if f.line.len() + 1 > SFD_BUF_SIZE {
                        self.pdftex_fail("buffer overflow at file subfont.c, line 95");
                    }
                    f.line.push(c as u8);
                }
                if c == 10 {
                    break;
                }
            }
            // append_eol(p, sfd_line, SFD_BUF_SIZE)
            if f.line.len() + 2 > SFD_BUF_SIZE {
                self.pdftex_fail("buffer overflow at file subfont.c, line 97");
            }
            let n = f.line.len();
            if n > 1 && f.line[n - 1] != 10 {
                f.line.push(10);
            }
            let n = f.line.len();
            if n > 2 && f.line[n - 2] == 32 {
                f.line[n - 2] = 10;
                f.line.pop();
            }
            if f.line.len() < 2 || f.line.first() == Some(&b'#') {
                continue;
            }
            return;
        }
    }

    /// `read_sfd`: the subfonts of `sfd_name`, read once per run; None if
    /// the file cannot be opened (with a warning).
    fn read_sfd(&mut self, st: &mut Fonts, sfd_name: &[u8]) -> Option<Vec<Subfont>> {
        if let Some(s) = st.map.sfd_tree.get(sfd_name) {
            return Some(s.clone());
        }
        set_cur_file_name(Some(sfd_name));
        let data = self
            .open_input_named(sfd_name, Format::Sfd)
            .and_then(|p| std::fs::read(p).ok());
        let Some(data) = data else {
            self.pdftex_warn("cannot open SFD file for reading");
            set_cur_file_name(None);
            return None;
        };
        // (it prints; a map file parse that reads one is not replayed from
        // the map cache)
        super::note_printed();
        self.tex_printf(b"{");
        // cur_file_name is still the name asked for, not the path
        self.tex_printf(sfd_name);
        let mut f = Sfd {
            data,
            pos: 0,
            eof: false,
            line: Vec::new(),
        };
        // C prepends each subfont to a list
        let mut subfonts: Vec<Subfont> = Vec::new();
        // read_sfd's locals, which keep their values from line to line
        let (mut n, mut i, mut j): (i32, i64, i64) = (0, 0, 0);
        while !f.eof {
            self.sfd_getline(&mut f, true);
            if f.line.first() == Some(&10) {
                break; // empty line indicating eof
            }
            let infix = cfmt::scan_word_n(&f.line, &mut n);
            let mut charcodes = vec![-1i32; 256];
            let mut p = (n.max(0) as usize).min(f.line.len()); // skip to the next word
            let mut k: i64 = 0;
            loop {
                let c = f.line.get(p).copied().unwrap_or(0);
                if c == b'\\' {
                    // continue on next line
                    self.sfd_getline(&mut f, false);
                    p = 0;
                    continue;
                } else if c == 0 {
                    break; // end of subfont
                }
                if cfmt::scan_long_n(&f.line[p..], &mut i, &mut n) == 0 {
                    self.pdftex_fail(&format!(
                        "invalid token:\n{}",
                        String::from_utf8_lossy(&f.line[p..])
                    ));
                }
                p = (p + n.max(0) as usize).min(f.line.len());
                let c = f.line.get(p).copied().unwrap_or(0);
                if c == b':' {
                    // offset
                    k = i;
                    p += 1;
                } else if c == b'_' {
                    // range
                    let rest = f.line.get(p + 1..).unwrap_or(&[]).to_vec();
                    if cfmt::scan_long_n(&rest, &mut j, &mut n) == 0 {
                        self.pdftex_fail(&format!(
                            "invalid token:\n{}",
                            String::from_utf8_lossy(&f.line[p..])
                        ));
                    }
                    if i > j || k + (j - i) > 255 {
                        self.pdftex_fail(&format!(
                            "invalid range:\n{}",
                            String::from_utf8_lossy(&f.line[p..])
                        ));
                    }
                    while i <= j {
                        if (0..256).contains(&k) {
                            charcodes[k as usize] = i as i32;
                        }
                        k += 1;
                        i += 1;
                    }
                    p = (p + n.max(0) as usize + 1).min(f.line.len());
                } else {
                    // codepoint (C writes past the array beyond 255)
                    if (0..256).contains(&k) {
                        charcodes[k as usize] = i as i32;
                    }
                    k += 1;
                }
            }
            subfonts.insert(0, (infix, charcodes));
        }
        self.tex_printf(b"}");
        set_cur_file_name(None);
        st.map.sfd_tree.insert(sfd_name.to_vec(), subfonts.clone());
        Some(subfonts)
    }

    /// `handle_subfont_fm` (subfont.c): if `fm` is a subfont entry
    /// (`prefix@sfd@`) whose `.sfd` file can be read, register one entry
    /// per subfont in its place and answer true.
    pub(super) fn handle_subfont_fm(&mut self, st: &mut Fonts, fm: &FmEntry, mode: Mode) -> bool {
        let p = &fm.tfm_name;
        let Some(q) = p.iter().position(|&c| c == b'@') else {
            return false;
        };
        let Some(r) = p[q + 1..]
            .iter()
            .position(|&c| c == b'@')
            .map(|i| i + q + 1)
        else {
            return false;
        };
        // prefix or sfd name is empty, or the second '@' is not the last char
        if q == 0 || r <= q + 1 || r != p.len() - 1 {
            return false;
        }
        let mut buf = p[q + 1..r].to_vec(); // sfd name
        if let Some(z) = buf.iter().position(|&b| b == 0) {
            buf.truncate(z);
        }
        if buf.len() + 4 > SFD_BUF_SIZE {
            self.pdftex_fail("buffer overflow at file subfont.c, line 196");
        }
        buf.extend_from_slice(b".sfd");
        let Some(sfd) = self.read_sfd(st, &buf) else {
            return false;
        };
        // at this point we know fm is a subfont
        let typ = fm.typ | F_SUBFONT;
        // set default values for PidEid
        let (pid, eid) = if fm.pid == -1 {
            (3, 1)
        } else {
            (fm.pid, fm.eid)
        };
        for (infix, charcodes) in sfd {
            let mut name = p[..q].to_vec();
            name.extend_from_slice(&infix);
            let mut fm2 = FmEntry::new();
            fm2.tfm_name = name;
            fm2.ff_name = fm.ff_name.clone();
            fm2.typ = typ;
            fm2.pid = pid;
            fm2.eid = eid;
            fm2.subfont = Some(charcodes);
            self.avl_do_entry(st, fm2, mode); // dropped if it fails
        }
        true
    }
}
