//! The hand-written side of `changes/undefined.ch`: where TeX Live's xetex
//! reads through a null pointer and crashes (DESIGN.md section 13), the port
//! stops as a fatal error and names the case with this line. It is printed
//! character by character, not interned, so that the string pool stays
//! TeX Live's (#2458).

use crate::generated::Globals;

const NOTE: &str = "FlashTeX: internal error avoided (TeX Live xetex crashes here)";

impl Globals {
    /// `undefined_null_read_note`: print [`NOTE`] where the selector points.
    pub fn undefined_null_read_note(&mut self) {
        for b in NOTE.bytes() {
            self.print_char(i32::from(b));
        }
    }
}
