//! Original bounded virtual-font packet parser. Dimensions are raw signed VF fix units.
use crate::{invalid, tfm::FixWord, Result};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontDefinition {
    pub id: i32,
    pub checksum: u32,
    pub scale: FixWord,
    pub design_size: FixWord,
    pub area: Vec<u8>,
    pub name: Vec<u8>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Register {
    W,
    X,
    Y,
    Z,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Glyph {
        code: u32,
        advance: bool,
    },
    Rule {
        height: i32,
        width: i32,
        advance: bool,
    },
    Nop,
    Push,
    Pop,
    Right(i32),
    Down(i32),
    Register {
        register: Register,
        value: Option<i32>,
    },
    Font(i32),
    Special(Vec<u8>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub code: u32,
    pub width: FixWord,
    pub commands: Vec<Command>,
}
#[derive(Debug, Clone)]
pub struct VirtualFont {
    pub checksum: u32,
    pub design_size: FixWord,
    pub comment: Vec<u8>,
    pub source_sha256: String,
    fonts: BTreeMap<i32, FontDefinition>,
    packets: BTreeMap<u32, Packet>,
    first_font: Option<i32>,
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| invalid("VF length overflow"))?;
        let b = self
            .bytes
            .get(self.at..end)
            .ok_or_else(|| invalid("VF truncated data"))?;
        self.at = end;
        Ok(b)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn unsigned(&mut self, n: usize) -> Result<u32> {
        let mut v = 0u32;
        for b in self.take(n)? {
            v = (v << 8) | *b as u32;
        }
        Ok(v)
    }
    fn signed(&mut self, n: usize) -> Result<i32> {
        let v = self.unsigned(n)?;
        Ok(if n == 4 {
            v as i32
        } else {
            ((v << (32 - n * 8)) as i32) >> (32 - n * 8)
        })
    }
    fn dimension(&mut self, n: usize) -> Result<i32> {
        let v = self.signed(n)?;
        if !(-16777215..16777216).contains(&v) {
            return Err(invalid("VF dimension range"));
        }
        Ok(v)
    }
}
impl VirtualFont {
    pub fn fonts(&self) -> &BTreeMap<i32, FontDefinition> {
        &self.fonts
    }
    pub fn packet(&self, code: u32) -> Option<&Packet> {
        self.packets.get(&code)
    }
    pub fn first_font(&self) -> Option<i32> {
        self.first_font
    }
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(invalid("VF file budget"));
        }
        let mut r = Reader { bytes, at: 0 };
        if r.byte()? != 247 || r.byte()? != 202 {
            return Err(invalid("VF preamble/version"));
        }
        let length = r.byte()? as usize;
        let comment = r.take(length)?.to_vec();
        let checksum = r.unsigned(4)?;
        let design_size = FixWord(r.signed(4)?);
        if design_size.0 < 1 << 20 {
            return Err(invalid("VF design size"));
        }
        let mut fonts = BTreeMap::new();
        let mut packets = BTreeMap::new();
        let mut first_font = None;
        let mut total = 0;
        loop {
            let op = r.byte()?;
            match op {
                243..=246 => {
                    if !packets.is_empty() || fonts.len() >= 4096 {
                        return Err(invalid("VF late font definition/budget"));
                    }
                    let id = r.unsigned((op - 242) as usize)? as i32;
                    let checksum = r.unsigned(4)?;
                    let scale = FixWord(r.dimension(4)?);
                    let design_size = FixWord(r.signed(4)?);
                    if scale.0 <= 0 || design_size.0 < 1 << 20 {
                        return Err(invalid("VF font size"));
                    }
                    let a = r.byte()? as usize;
                    let n = r.byte()? as usize;
                    let area = r.take(a)?.to_vec();
                    let name = r.take(n)?.to_vec();
                    if name.is_empty() {
                        return Err(invalid("VF empty font name"));
                    }
                    if fonts
                        .insert(
                            id,
                            FontDefinition {
                                id,
                                checksum,
                                scale,
                                design_size,
                                area,
                                name,
                            },
                        )
                        .is_some()
                    {
                        return Err(invalid("VF duplicate font ID"));
                    }
                    first_font.get_or_insert(id);
                }
                0..=242 => {
                    if packets.len() >= 65536 {
                        return Err(invalid("VF packet count budget"));
                    }
                    let (length, code, width) = if op == 242 {
                        (
                            r.unsigned(4)? as usize,
                            r.unsigned(4)?,
                            FixWord(r.dimension(4)?),
                        )
                    } else {
                        (
                            op as usize,
                            r.byte()? as u32,
                            FixWord(r.unsigned(3)? as i32),
                        )
                    };
                    if length > 1024 * 1024 {
                        return Err(invalid("VF packet byte budget"));
                    }
                    let commands = parse_commands(r.take(length)?, &fonts, first_font)?;
                    total += commands.len();
                    if total > 1_000_000 {
                        return Err(invalid("VF total command budget"));
                    }
                    if packets
                        .insert(
                            code,
                            Packet {
                                code,
                                width,
                                commands,
                            },
                        )
                        .is_some()
                    {
                        return Err(invalid("VF duplicate packet"));
                    }
                }
                248 => {
                    if r.take(bytes.len() - r.at)?.iter().any(|b| *b != 248) {
                        return Err(invalid("VF postamble padding"));
                    }
                    break;
                }
                _ => return Err(invalid("VF outer opcode")),
            }
        }
        Ok(Self {
            checksum,
            design_size,
            comment,
            source_sha256: crate::sha256(bytes),
            fonts,
            packets,
            first_font,
        })
    }
}
fn parse_commands(
    bytes: &[u8],
    fonts: &BTreeMap<i32, FontDefinition>,
    mut font: Option<i32>,
) -> Result<Vec<Command>> {
    let mut r = Reader { bytes, at: 0 };
    let mut commands = Vec::new();
    let mut depth = 0;
    while r.at < bytes.len() {
        if commands.len() >= 100000 {
            return Err(invalid("VF packet command budget"));
        }
        let op = r.byte()?;
        let command = match op {
            0..=127 => Command::Glyph {
                code: op as u32,
                advance: true,
            },
            128..=131 => Command::Glyph {
                code: r.unsigned((op - 127) as usize)?,
                advance: true,
            },
            133..=136 => Command::Glyph {
                code: r.unsigned((op - 132) as usize)?,
                advance: false,
            },
            132 | 137 => Command::Rule {
                height: r.dimension(4)?,
                width: r.dimension(4)?,
                advance: op == 132,
            },
            138 => Command::Nop,
            141 => {
                depth += 1;
                if depth > 64 {
                    return Err(invalid("VF stack depth"));
                }
                Command::Push
            }
            142 => {
                if depth == 0 {
                    return Err(invalid("VF stack underflow"));
                }
                depth -= 1;
                Command::Pop
            }
            143..=146 => Command::Right(r.dimension((op - 142) as usize)?),
            147..=151 => Command::Register {
                register: Register::W,
                value: if op == 147 {
                    None
                } else {
                    Some(r.dimension((op - 147) as usize)?)
                },
            },
            152..=156 => Command::Register {
                register: Register::X,
                value: if op == 152 {
                    None
                } else {
                    Some(r.dimension((op - 152) as usize)?)
                },
            },
            157..=160 => Command::Down(r.dimension((op - 156) as usize)?),
            161..=165 => Command::Register {
                register: Register::Y,
                value: if op == 161 {
                    None
                } else {
                    Some(r.dimension((op - 161) as usize)?)
                },
            },
            166..=170 => Command::Register {
                register: Register::Z,
                value: if op == 166 {
                    None
                } else {
                    Some(r.dimension((op - 166) as usize)?)
                },
            },
            171..=234 => Command::Font((op - 171) as i32),
            235..=238 => Command::Font(r.unsigned((op - 234) as usize)? as i32),
            239..=242 => {
                let n = r.unsigned((op - 238) as usize)? as usize;
                Command::Special(r.take(n)?.to_vec())
            }
            _ => return Err(invalid("VF prohibited packet opcode")),
        };
        match &command {
            Command::Font(id) => {
                if !fonts.contains_key(id) {
                    return Err(invalid("VF missing font definition"));
                }
                font = Some(*id);
            }
            Command::Glyph { .. } if font.is_none() => {
                return Err(invalid("VF glyph without font"))
            }
            _ => {}
        }
        commands.push(command);
    }
    if depth != 0 {
        return Err(invalid("VF unbalanced stack"));
    }
    Ok(commands)
}
#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture(commands: &[u8], long: bool) -> Vec<u8> {
        let mut b = vec![247, 202, 0];
        for n in [0u32, 10 << 20] {
            b.extend(n.to_be_bytes());
        }
        b.extend([243, 0]);
        for n in [0u32, 1 << 20, 10 << 20] {
            b.extend(n.to_be_bytes());
        }
        b.extend([0, 1, b'f']);
        if long {
            b.push(242);
            for n in [commands.len() as u32, 65, 1 << 19] {
                b.extend(n.to_be_bytes());
            }
        } else {
            b.extend([commands.len() as u8, 65, 8, 0, 0]);
        }
        b.extend(commands);
        b.push(248);
        b
    }
    #[test]
    fn short_long_and_exact_commands() {
        for long in [false, true] {
            let v =
                VirtualFont::parse(&fixture(&[65, 143, 255, 141, 148, 2, 147, 142], long)).unwrap();
            assert_eq!(v.packet(65).unwrap().width, FixWord(1 << 19));
            assert_eq!(v.packet(65).unwrap().commands[1], Command::Right(-1));
        }
    }
    #[test]
    fn all_truncations_fail() {
        let b = fixture(&[65], true);
        for n in 0..b.len() {
            assert!(VirtualFont::parse(&b[..n]).is_err(), "{n}");
        }
    }
    #[test]
    fn invalid_opcodes_fonts_stack() {
        for commands in [
            vec![139],
            vec![172],
            vec![142],
            vec![141],
            vec![141; 65],
            vec![239, 255],
        ] {
            assert!(VirtualFont::parse(&fixture(&commands, true)).is_err());
        }
    }
    #[test]
    fn specials_preserved_not_ignored() {
        let v = VirtualFont::parse(&fixture(&[239, 2, b'p', b's'], false)).unwrap();
        assert_eq!(
            v.packet(65).unwrap().commands,
            vec![Command::Special(b"ps".to_vec())]
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    Glyph {
        local_font: i32,
        font_sha256: String,
        tfm_sha256: String,
        face_index: u32,
        glyph_id: u16,
        tfm_code: u8,
        x: crate::Coordinate,
        y: crate::Coordinate,
        scale: FixWord,
    },
    Rule {
        x: crate::Coordinate,
        y: crate::Coordinate,
        width: crate::Coordinate,
        height: crate::Coordinate,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpandedPacket {
    pub vf_sha256: String,
    pub tfm_sha256: String,
    pub character: u8,
    pub width: FixWord,
    pub placements: Vec<Placement>,
}
impl VirtualFont {
    /// Expands a single VF packet through explicitly bound physical fonts. Positions
    /// are exact fractions of the virtual font's size; no device rounding occurs.
    pub fn expand_packet(
        &self,
        code: u8,
        metrics: &crate::tfm::Tfm,
        resources: &BTreeMap<i32, crate::encoding::BoundTfmFont<'_>>,
    ) -> Result<ExpandedPacket> {
        use crate::{encoding::GlyphIdentity, Coordinate};
        if self.design_size != metrics.design_size
            || (self.checksum != 0 && metrics.checksum != 0 && self.checksum != metrics.checksum)
        {
            return Err(invalid("VF/TFM header identity mismatch"));
        }
        let packet = self
            .packet(code as u32)
            .ok_or_else(|| invalid("VF packet missing"))?;
        if metrics
            .char_metrics(code)
            .ok_or_else(|| invalid("VF TFM character missing"))?
            .width
            != packet.width
        {
            return Err(invalid("VF packet/TFM width mismatch"));
        }
        for (id, definition) in &self.fonts {
            let binding = resources
                .get(id)
                .ok_or_else(|| invalid("VF local font has no explicit resource binding"))?;
            if binding.design_size() != definition.design_size
                || (definition.checksum != 0
                    && binding.tfm().checksum != 0
                    && definition.checksum != binding.tfm().checksum)
            {
                return Err(invalid("VF local TFM header mismatch"));
            }
        }
        #[derive(Clone, Copy)]
        struct State {
            h: Coordinate,
            v: Coordinate,
            registers: [i32; 4],
        }
        let mut state = State {
            h: Coordinate::from_integer(0),
            v: Coordinate::from_integer(0),
            registers: [0; 4],
        };
        let mut stack = Vec::new();
        let mut font = self.first_font;
        let mut placements = Vec::new();
        for command in &packet.commands {
            match command {
                Command::Nop => {}
                Command::Push => stack.push(state),
                Command::Pop => state = stack.pop().ok_or_else(|| invalid("VF stack underflow"))?,
                Command::Font(id) => font = Some(*id),
                Command::Right(amount) => {
                    state.h = state.h.add(Coordinate::new(*amount as i128, 20)?)?
                }
                Command::Down(amount) => {
                    state.v = state.v.add(Coordinate::new(*amount as i128, 20)?)?
                }
                Command::Register { register, value } => {
                    let (index, horizontal) = match register {
                        Register::W => (0, true),
                        Register::X => (1, true),
                        Register::Y => (2, false),
                        Register::Z => (3, false),
                    };
                    if let Some(value) = value {
                        state.registers[index] = *value;
                    }
                    let amount = Coordinate::new(state.registers[index] as i128, 20)?;
                    if horizontal {
                        state.h = state.h.add(amount)?;
                    } else {
                        state.v = state.v.add(amount)?;
                    }
                }
                Command::Special(_) => {
                    return Err(crate::Error::UnsupportedFont(
                        "VF specials require an explicit handler; none installed".into(),
                    ))
                }
                Command::Rule {
                    height,
                    width,
                    advance,
                } => {
                    let w = Coordinate::new(*width as i128, 20)?;
                    let h = Coordinate::new(*height as i128, 20)?;
                    if *height > 0 && *width > 0 {
                        placements.push(Placement::Rule {
                            x: state.h,
                            y: state.v,
                            width: w,
                            height: h,
                        });
                    }
                    if *advance {
                        state.h = state.h.add(w)?;
                    }
                }
                Command::Glyph { code, advance } => {
                    let code = u8::try_from(*code).map_err(|_| {
                        crate::Error::UnsupportedFont(
                            "VF physical character exceeds explicit 8-bit TFM encoding".into(),
                        )
                    })?;
                    let id = font.ok_or_else(|| invalid("VF glyph has no font"))?;
                    let binding = resources
                        .get(&id)
                        .ok_or_else(|| invalid("VF resource binding missing"))?;
                    let (identity, metric) = binding.map_code(code)?;
                    let GlyphIdentity::Original(glyph_id) = identity else {
                        return Err(invalid("VF glyph resolves to explicit .notdef"));
                    };
                    let scale = self.fonts[&id].scale;
                    placements.push(Placement::Glyph {
                        local_font: id,
                        font_sha256: binding.font().descriptor().sha256.clone(),
                        tfm_sha256: binding.tfm().source_sha256.clone(),
                        face_index: binding.font().descriptor().face_index,
                        glyph_id,
                        tfm_code: code,
                        x: state.h,
                        y: state.v,
                        scale,
                    });
                    if *advance {
                        state.h = state.h.add(Coordinate::new(
                            metric.width.0 as i128 * scale.0 as i128,
                            40,
                        )?)?;
                    }
                }
            }
        }
        Ok(ExpandedPacket {
            vf_sha256: self.source_sha256.clone(),
            tfm_sha256: metrics.source_sha256.clone(),
            character: code,
            width: packet.width,
            placements,
        })
    }
}
