//! `vfpacket.c`, ported: the character packets of virtual fonts.

use super::with_state;
use crate::generated::Globals;

#[derive(Default, Clone)]
pub struct State {
    /// `vf_array`: per virtual font, the packet of each character
    /// `font_bc..=font_ec`.
    fonts: Vec<Vec<Vec<u8>>>,
    /// `packet_data_ptr`: the packet being read and the position in it.
    cur: (usize, usize, usize),
    /// `packet_array`: saved `(cur, vf_packet_length)` pairs.
    stack: Vec<((usize, usize, usize), i32)>,
}

// Checkpoint registration (crate::checkpoint): the state is cloned at a
// checkpoint and persisted with a snapshot.
crate::codec_struct!(State { fonts, cur, stack });

impl Globals {
    /// `newvfpacket`: room for the packets of font `f`.
    pub fn new_vf_packet(&mut self, f: i32) -> i32 {
        let n = (self.font_ec[f as usize] - self.font_bc[f as usize] + 1).max(0) as usize;
        with_state(|s| {
            s.vf.fonts.push(vec![Vec::new(); n]);
            s.vf.fonts.len() as i32 - 1
        })
    }

    /// `storepacket`: string `s` becomes the packet of character `c`.
    pub fn storepacket(&mut self, f: i32, c: i32, s: i32) {
        let data = self.str_bytes(s);
        let (base, bc) = (
            self.vf_packet_base[f as usize] as usize,
            self.font_bc[f as usize],
        );
        with_state(|st| st.vf.fonts[base][(c - bc) as usize] = data);
    }

    /// `startpacket`: read the packet of character `c` next.
    pub fn start_packet(&mut self, f: i32, c: i32) {
        let (base, bc) = (
            self.vf_packet_base[f as usize] as usize,
            self.font_bc[f as usize],
        );
        let i = (c - bc) as usize;
        let len = with_state(|st| {
            st.vf.cur = (base, i, 0);
            st.vf.fonts[base][i].len()
        });
        self.vf_packet_length = len as i32;
    }

    /// `packetbyte`: the next byte of the current packet.
    pub fn packet_byte(&mut self) -> i32 {
        self.vf_packet_length -= 1;
        with_state(|st| {
            let (f, c, p) = st.vf.cur;
            st.vf.cur.2 += 1;
            st.vf.fonts[f][c].get(p).copied().unwrap_or(0) as i32
        })
    }

    /// `pushpacketstate`.
    pub fn push_packet_state(&mut self) {
        let len = self.vf_packet_length;
        with_state(|st| {
            let cur = st.vf.cur;
            st.vf.stack.push((cur, len));
        });
    }

    /// `poppacketstate`.
    pub fn pop_packet_state(&mut self) {
        match with_state(|st| st.vf.stack.pop()) {
            Some((cur, len)) => {
                with_state(|st| st.vf.cur = cur);
                self.vf_packet_length = len;
            }
            None => self.pdftex_fail("packet stack empty, impossible to pop"),
        }
    }
}
