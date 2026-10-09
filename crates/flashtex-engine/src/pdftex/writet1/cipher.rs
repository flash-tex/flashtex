// Derived from pdfTeX's writet1.c (TeX Live 2026, pdfTeX 1.40.29),
// Copyright 1996-2023 Han The Thanh <thanh@pdftex.org>; GPL-2.0-or-later
// (crates/flashtex-engine/LICENSE). Modified for FlashTeX: ported to Rust
// line by line, then rewritten into this module on 2026-10-04
// (docs/design/engine-v2/REWRITE.md); git history dates each later change.

//! The Type 1 encryption (Adobe, *Type 1 Font Format*, chapter 7): one
//! cipher for the `eexec` part of a font, the same one with another key for
//! each charstring. writet1.c's `edecrypt`/`eencrypt` and
//! `cdecrypt`/`cencrypt`.

const C1: u16 = 52845;
const C2: u16 = 22719;

/// The cipher's running key `r`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cipher(u16);

impl Cipher {
    /// The key of the `eexec` part.
    pub const EEXEC: Cipher = Cipher(55665);
    /// The key of each charstring and subr.
    pub const CHARSTRING: Cipher = Cipher(4330);

    #[inline]
    pub fn decrypt(&mut self, cipher: u8) -> u8 {
        let plain = cipher ^ (self.0 >> 8) as u8;
        self.advance(cipher);
        plain
    }

    #[inline]
    pub fn encrypt(&mut self, plain: u8) -> u8 {
        let cipher = plain ^ (self.0 >> 8) as u8;
        self.advance(cipher);
        cipher
    }

    #[inline]
    fn advance(&mut self, cipher: u8) {
        self.0 = (u16::from(cipher).wrapping_add(self.0))
            .wrapping_mul(C1)
            .wrapping_add(C2);
    }
}

/// `cipher`-encrypted `plain`, from a fresh key.
pub(super) fn encrypt(mut key: Cipher, plain: impl IntoIterator<Item = u8>) -> Vec<u8> {
    plain.into_iter().map(|b| key.encrypt(b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decrypt_inverts_encrypt() {
        let plain: Vec<u8> = (0..=255).chain(0..=255).collect();
        let sealed = encrypt(Cipher::EEXEC, plain.iter().copied());
        let mut key = Cipher::EEXEC;
        let opened: Vec<u8> = sealed.iter().map(|&c| key.decrypt(c)).collect();
        assert_eq!(opened, plain);
    }

    #[test]
    fn matches_the_specification() {
        // Type 1 Font Format 7.2: the plaintext byte is c ^ (r >> 8), then
        // r = (c + r) * c1 + c2 (mod 65536); computed here in 32 bits.
        let mut key = Cipher::EEXEC;
        let plain: Vec<u8> = [0x2e, 0x4b, 0x0a, 0x9e]
            .iter()
            .map(|&c| key.decrypt(c))
            .collect();
        let mut r: u32 = 55665;
        let want: Vec<u8> = [0x2e_u8, 0x4b, 0x0a, 0x9e]
            .iter()
            .map(|&c| {
                let p = c ^ (r >> 8) as u8;
                r = ((c as u32 + r) * 52845 + 22719) & 0xffff;
                p
            })
            .collect();
        assert_eq!(plain, want);
    }
}
