//! `vfpacket.c`, ported: the character packets of virtual fonts.

use super::shared::Shared;
use super::with_state;
use crate::generated::Globals;

/// The packets are written once, when a virtual font is loaded, and read
/// for every character of it that is shipped out; the read position
/// (`cur`, `stack`) changes with every one. `CState::vf` is copied on its
/// first write after a checkpoint (`Shared`), so the packets are shared
/// apart from it: that copy then takes the read position and two
/// reference counts, not every packet of every virtual font. (Held in the
/// state itself, the packets were copied after almost every checkpoint:
/// 265 MB in 5.4 M allocations on a 580-page book,
/// docs/evidence/mem-footprint-2026-10-04/.)
#[derive(Default, Clone)]
pub struct State {
    /// `vf_array`: per virtual font, the packet of each character
    /// `font_bc..=font_ec`.
    fonts: Shared<Vec<Shared<Vec<Vec<u8>>>>>,
    /// `packet_data_ptr`: the packet being read and the position in it.
    cur: (usize, usize, usize),
    /// `packet_array`: saved `(cur, vf_packet_length)` pairs.
    stack: Vec<((usize, usize, usize), i32)>,
}

// Checkpoint registration (crate::checkpoint): the state is cloned at a
// checkpoint and persisted with a snapshot. A `Shared` encodes as what it
// holds, so the encoding is that of the plain nested vectors.
crate::codec_struct!(State { fonts, cur, stack });

impl State {
    /// Whether `self` and `o` encode the same (`CState::same_as`), without
    /// encoding the packets when both share them.
    pub fn same_as(&self, o: &State) -> bool {
        self.cur == o.cur
            && self.stack == o.stack
            && (Shared::ptr_eq(&self.fonts, &o.fonts)
                || (self.fonts.len() == o.fonts.len()
                    && self
                        .fonts
                        .iter()
                        .zip(o.fonts.iter())
                        .all(|(a, b)| Shared::ptr_eq(a, b) || **a == **b)))
    }
}

impl Globals {
    /// `newvfpacket`: room for the packets of font `f`.
    pub fn new_vf_packet(&mut self, f: i32) -> i32 {
        let n = (self.font_ec[f as usize] - self.font_bc[f as usize] + 1).max(0) as usize;
        with_state(|s| {
            s.vf.fonts.push(Shared::new(vec![Vec::new(); n]));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::{Codec, Reader};

    fn enc<T: Codec>(x: &T) -> Vec<u8> {
        let mut w = vec![];
        x.enc(&mut w);
        w
    }

    fn sample() -> State {
        State {
            fonts: Shared::new(vec![
                Shared::new(vec![vec![1, 2, 3], vec![], vec![9]]),
                Shared::new(vec![vec![4; 40]]),
            ]),
            cur: (1, 0, 7),
            stack: vec![((0, 2, 1), 5)],
        }
    }

    /// The shared packets encode exactly as the plain nested vectors did,
    /// so a persisted S₀ reads the same either way.
    #[test]
    fn encoding_is_the_plain_vectors() {
        let s = sample();
        let plain: Vec<Vec<Vec<u8>>> =
            vec![vec![vec![1, 2, 3], vec![], vec![9]], vec![vec![4; 40]]];
        let mut want = enc(&plain);
        want.extend(enc(&s.cur));
        want.extend(enc(&s.stack));
        assert_eq!(enc(&s), want);
        let back = State::dec(&mut Reader::new(&want)).unwrap();
        assert_eq!(enc(&back), want);
        assert!(back.same_as(&s));
    }

    /// `same_as` is the encodings' equality, and a write copies only what
    /// it touches.
    #[test]
    fn same_as_is_encoding_equality() {
        let a = sample();
        let mut b = a.clone();
        assert!(a.same_as(&b));
        b.cur.2 += 1;
        assert!(!a.same_as(&b) && enc(&a) != enc(&b));
        let mut c = a.clone();
        c.fonts[1][0][3] = 5;
        assert!(!a.same_as(&c) && enc(&a) != enc(&c));
        // an equal copy that is not the same allocation
        let d = State::dec(&mut Reader::new(&enc(&a))).unwrap();
        assert!(a.same_as(&d));
        let mut e = a.clone();
        e.cur = (0, 0, 0);
        assert!(Shared::ptr_eq(&a.fonts, &e.fonts));
        e.fonts[0][1] = vec![8];
        assert!(Shared::ptr_eq(&a.fonts[1], &e.fonts[1]));
        assert!(!Shared::ptr_eq(&a.fonts[0], &e.fonts[0]));
        assert_eq!(a.fonts[0][1], Vec::<u8>::new());
    }
}
