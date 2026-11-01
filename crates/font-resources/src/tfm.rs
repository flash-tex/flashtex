//! Original bounded TFM decoding; 8-bit TeX encoding is not Unicode or a TrueType GID map.
use crate::{invalid, u16_at, u32_at, Result};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixWord(pub i32);
impl FixWord {
    /// Exact product in TeX points: numerator / 2^40. No rounding to scaled points.
    pub fn at_design_size(self, design: FixWord) -> i64 {
        self.0 as i64 * design.0 as i64
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterMetrics {
    pub width: FixWord,
    pub height: FixWord,
    pub depth: FixWord,
    pub italic: FixWord,
    pub tag: u8,
    pub remainder: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairAction {
    Kern(FixWord),
    Ligature {
        replacement: u8,
        retain_left: bool,
        retain_right: bool,
        advance: u8,
    },
}
#[derive(Debug, Clone)]
pub struct Tfm {
    pub checksum: u32,
    pub design_size: FixWord,
    pub source_sha256: String,
    chars: Vec<Option<CharacterMetrics>>,
    program: Vec<[u8; 4]>,
    kerns: Vec<FixWord>,
    parameters: Vec<FixWord>,
    pub boundary_character: Option<u8>,
    pub left_boundary_program: Option<usize>,
}
impl Tfm {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut n = [0usize; 12];
        for (i, n) in n.iter_mut().enumerate() {
            *n = u16_at(bytes, i * 2)? as usize;
            if *n >= 32768 {
                return Err(invalid("TFM length high bit"));
            }
        }
        let [lf, lh, bc, ec, nw, nh, nd, ni, nl, nk, ne, np] = n;
        if lh < 2
            || ec > 255
            || bc > ec + 1
            || nw == 0
            || nh == 0
            || nd == 0
            || ni == 0
            || nw > 256
            || nh > 16
            || nd > 16
            || ni > 64
            || ne > 256
        {
            return Err(invalid("TFM header counts"));
        }
        let count = ec + 1 - bc;
        if lf != 6 + lh + count + nw + nh + nd + ni + nl + nk + ne + np || bytes.len() != lf * 4 {
            return Err(invalid("TFM file/table length mismatch"));
        }
        let checksum = u32_at(bytes, 24)?;
        let design_size = FixWord(u32_at(bytes, 28)? as i32);
        if design_size.0 < 1 << 20 {
            return Err(invalid("TFM design size below one point"));
        }
        let mut at = (6 + lh) * 4;
        let infos = bytes[at..at + count * 4].as_chunks::<4>().0.to_vec();
        at += count * 4;
        let mut fixes = |count: usize, slant: bool| -> Result<Vec<FixWord>> {
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                let v = u32_at(bytes, at)? as i32;
                at += 4;
                if !(slant && i == 0) && !(-16777216..16777216).contains(&v) {
                    return Err(invalid("TFM dimension outside fix_word metric range"));
                }
                out.push(FixWord(v));
            }
            Ok(out)
        };
        let widths = fixes(nw, false)?;
        let heights = fixes(nh, false)?;
        let depths = fixes(nd, false)?;
        let italics = fixes(ni, false)?;
        if [widths[0], heights[0], depths[0], italics[0]]
            .iter()
            .any(|v| v.0 != 0)
        {
            return Err(invalid("TFM metric zero entry"));
        }
        let program = bytes[at..at + nl * 4].as_chunks::<4>().0.to_vec();
        at += nl * 4;
        let mut kerns = Vec::new();
        for _ in 0..nk {
            let v = u32_at(bytes, at)? as i32;
            at += 4;
            if !(-16777216..16777216).contains(&v) {
                return Err(invalid("TFM kern range"));
            }
            kerns.push(FixWord(v));
        }
        let recipes = bytes[at..at + ne * 4].as_chunks::<4>().0.to_vec();
        at += ne * 4;
        let mut parameters = Vec::new();
        for i in 0..np {
            let v = u32_at(bytes, at)? as i32;
            at += 4;
            if i != 0 && !(-16777216..16777216).contains(&v) {
                return Err(invalid("TFM parameter range"));
            }
            parameters.push(FixWord(v));
        }
        let mut chars = vec![None; 256];
        for (i, info) in infos.iter().enumerate() {
            let [w, hd, it, rem] = *info;
            let h = (hd >> 4) as usize;
            let d = (hd & 15) as usize;
            let italic = (it >> 2) as usize;
            if w as usize >= nw || h >= nh || d >= nd || italic >= ni {
                return Err(invalid("TFM character metric index"));
            }
            if w != 0 {
                chars[bc + i] = Some(CharacterMetrics {
                    width: widths[w as usize],
                    height: heights[h],
                    depth: depths[d],
                    italic: italics[italic],
                    tag: it & 3,
                    remainder: rem,
                });
            }
        }
        let exists = |c: u8| chars[c as usize].is_some();
        let boundary_character = program.first().filter(|p| p[0] == 255).map(|p| p[1]);
        let left_boundary_program = program
            .last()
            .filter(|p| p[0] == 255)
            .map(|p| p[2] as usize * 256 + p[3] as usize);
        for (i, p) in program.iter().enumerate() {
            let [skip, next, op, rem] = *p;
            if skip > 128 {
                if op as usize * 256 + rem as usize >= nl {
                    return Err(invalid("TFM ligature restart index"));
                }
                continue;
            }
            if skip < 128 && i + skip as usize + 1 >= nl {
                return Err(invalid("TFM ligature skip outside program"));
            }
            if !exists(next) && Some(next) != boundary_character {
                return Err(invalid("TFM ligature next character absent"));
            }
            if op >= 128 {
                if (op as usize - 128) * 256 + rem as usize >= nk {
                    return Err(invalid("TFM kern index"));
                }
            } else if !exists(rem) || op / 4 > ((op / 2) & 1) + (op & 1) {
                return Err(invalid("TFM ligature opcode/replacement"));
            }
        }
        for (code, ch) in chars.iter().enumerate() {
            if let Some(ch) = ch {
                match ch.tag {
                    1 => {
                        if ch.remainder as usize >= nl {
                            return Err(invalid("TFM character program index"));
                        }
                    }
                    2 => {
                        let mut seen = [false; 256];
                        let mut current = code;
                        loop {
                            if seen[current] {
                                return Err(invalid("TFM next-larger cycle"));
                            }
                            seen[current] = true;
                            let c = chars[current]
                                .ok_or_else(|| invalid("TFM next-larger character absent"))?;
                            if c.tag != 2 {
                                break;
                            }
                            current = c.remainder as usize;
                        }
                    }
                    3 if ch.remainder as usize >= ne => {
                        return Err(invalid("TFM extensible recipe index"));
                    }
                    _ => {}
                }
            }
        }
        for recipe in recipes {
            for (i, c) in recipe.into_iter().enumerate() {
                if (i == 3 || c != 0) && !exists(c) {
                    return Err(invalid("TFM extensible piece absent"));
                }
            }
        }
        Ok(Self {
            checksum,
            design_size,
            source_sha256: crate::sha256(bytes),
            chars,
            program,
            kerns,
            parameters,
            boundary_character,
            left_boundary_program,
        })
    }
    pub fn char_metrics(&self, code: u8) -> Option<CharacterMetrics> {
        self.chars[code as usize]
    }
    pub fn parameter(&self, one_based: usize) -> Option<FixWord> {
        one_based
            .checked_sub(1)
            .and_then(|i| self.parameters.get(i).copied())
    }
    pub fn pair_action(&self, left: u8, right: u8) -> Result<Option<PairAction>> {
        self.pair_action_budget(left, right, &mut 65536)
    }
    fn pair_action_budget(
        &self,
        left: u8,
        right: u8,
        budget: &mut usize,
    ) -> Result<Option<PairAction>> {
        let ch = self
            .char_metrics(left)
            .ok_or_else(|| invalid("TFM left character missing"))?;
        if self.char_metrics(right).is_none() && Some(right) != self.boundary_character {
            return Err(invalid("TFM right character missing"));
        }
        if ch.tag != 1 {
            return Ok(None);
        }
        self.program_action(ch.remainder as usize, right, true, budget)
    }
    fn program_action(
        &self,
        mut index: usize,
        right: u8,
        restart: bool,
        budget: &mut usize,
    ) -> Result<Option<PairAction>> {
        let first = *self
            .program
            .get(index)
            .ok_or_else(|| invalid("TFM program entry outside table"))?;
        if restart && first[0] > 128 {
            index = first[2] as usize * 256 + first[3] as usize;
        }
        for _ in 0..self.program.len() {
            *budget = budget
                .checked_sub(1)
                .ok_or_else(|| invalid("TFM ligature execution step budget"))?;
            let [skip, next, op, rem] = *self
                .program
                .get(index)
                .ok_or_else(|| invalid("TFM program index outside table"))?;
            if skip > 128 {
                return Ok(None);
            }
            if next == right {
                return Ok(Some(if op >= 128 {
                    PairAction::Kern(self.kerns[(op as usize - 128) * 256 + rem as usize])
                } else {
                    PairAction::Ligature {
                        replacement: rem,
                        retain_left: op & 2 != 0,
                        retain_right: op & 1 != 0,
                        advance: op / 4,
                    }
                }));
            }
            if skip >= 128 {
                return Ok(None);
            }
            index += skip as usize + 1;
        }
        Err(invalid("TFM pair program budget"))
    }
}
/// Input provenance refers to indices in the caller's encoded byte sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodedGlyph {
    pub code: u8,
    pub input_start: usize,
    pub input_end: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TfmItem {
    Glyph(EncodedGlyph),
    Kern(FixWord),
}
/// Caller controls explicit run-boundary suppression (for example no-boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryOptions {
    pub left: bool,
    pub right: bool,
}
impl Default for BoundaryOptions {
    fn default() -> Self {
        Self {
            left: true,
            right: true,
        }
    }
}
impl Tfm {
    /// Bounded execution on an explicit same-font encoded run. Implicit boundary
    /// sentinels never become glyph codes or output items.
    pub fn apply_ligatures_kerns(&self, input: &[u8]) -> Result<Vec<TfmItem>> {
        self.apply_ligatures_kerns_with_boundaries(input, BoundaryOptions::default())
    }
    pub fn apply_ligatures_kerns_with_boundaries(
        &self,
        input: &[u8],
        boundaries: BoundaryOptions,
    ) -> Result<Vec<TfmItem>> {
        if input.len() > 4096 {
            return Err(invalid("TFM input budget"));
        }
        if input.is_empty() {
            return Ok(Vec::new());
        }
        #[derive(Clone, Copy)]
        enum Item {
            Glyph(EncodedGlyph),
            Left,
            Right(u8),
            Kern(FixWord),
        }
        let mut out = Vec::with_capacity(input.len() + 2);
        if boundaries.left && self.left_boundary_program.is_some() {
            out.push(Item::Left);
        }
        for (i, &code) in input.iter().enumerate() {
            if self.char_metrics(code).is_none() {
                return Err(invalid("TFM input character missing"));
            }
            out.push(Item::Glyph(EncodedGlyph {
                code,
                input_start: i,
                input_end: i + 1,
            }));
        }
        if boundaries.right {
            if let Some(code) = self.boundary_character {
                out.push(Item::Right(code));
            }
        }
        let span = |item: Item| match item {
            Item::Glyph(g) => (g.input_start, g.input_end),
            Item::Left => (0, 0),
            Item::Right(_) => (input.len(), input.len()),
            Item::Kern(_) => unreachable!(),
        };
        let mut at = 0;
        let mut budget = 65536usize;
        for _ in 0..65536 {
            budget = budget
                .checked_sub(1)
                .ok_or_else(|| invalid("TFM ligature execution step budget"))?;
            if at + 1 >= out.len() {
                return Ok(out
                    .into_iter()
                    .filter_map(|item| match item {
                        Item::Glyph(g) => Some(TfmItem::Glyph(g)),
                        Item::Kern(k) => Some(TfmItem::Kern(k)),
                        _ => None,
                    })
                    .collect());
            }
            let left = out[at];
            let right = out[at + 1];
            let right_code = match right {
                Item::Glyph(g) => g.code,
                Item::Right(code) => code,
                _ => {
                    at += 1;
                    continue;
                }
            };
            let action = match left {
                Item::Glyph(g) => self.pair_action_budget(g.code, right_code, &mut budget)?,
                Item::Left => self.program_action(
                    self.left_boundary_program
                        .ok_or_else(|| invalid("TFM missing boundary entry"))?,
                    right_code,
                    false,
                    &mut budget,
                )?,
                _ => {
                    at += 1;
                    continue;
                }
            };
            match action {
                None => at += 1,
                Some(PairAction::Kern(amount)) => {
                    out.insert(at + 1, Item::Kern(amount));
                    at += 2;
                }
                Some(PairAction::Ligature {
                    replacement,
                    retain_left,
                    retain_right,
                    advance,
                }) => {
                    let (ls, le) = span(left);
                    let (rs, re) = span(right);
                    let mut replacements = Vec::with_capacity(3);
                    if retain_left {
                        replacements.push(left);
                    }
                    replacements.push(Item::Glyph(EncodedGlyph {
                        code: replacement,
                        input_start: ls.min(rs),
                        input_end: le.max(re),
                    }));
                    if retain_right {
                        replacements.push(right);
                    }
                    out.splice(at..at + 2, replacements);
                    at += advance as usize;
                }
            }
            if out.len() > 8192 {
                return Err(invalid("TFM output budget"));
            }
        }
        Err(invalid("TFM ligature execution step budget"))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let counts = [17u16, 2, 65, 66, 2, 1, 1, 1, 1, 1, 0, 0];
        let mut b = counts
            .into_iter()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<_>>();
        for word in [
            0x12345678u32,
            10 << 20,
            0x01000100,
            0x01000000,
            0,
            1 << 19,
            0,
            0,
            0,
            0x80428000,
            (-131072i32) as u32,
        ] {
            b.extend(word.to_be_bytes());
        }
        b
    }
    fn boundary_fixture(program: &[[u8; 4]], a_start: u8) -> Vec<u8> {
        let mut bytes = fixture();
        bytes.splice(60..64, program.iter().flatten().copied());
        bytes[0..2].copy_from_slice(&(17u16 + program.len() as u16 - 1).to_be_bytes());
        bytes[16..18].copy_from_slice(&(program.len() as u16).to_be_bytes());
        bytes[35] = a_start;
        bytes
    }
    #[test]
    fn implicit_boundaries_kern_and_suppression_preserve_input() {
        let bytes = boundary_fixture(
            &[
                [255, 250, 0, 0],
                [128, 250, 128, 0],
                [128, 65, 128, 0],
                [255, 0, 0, 2],
            ],
            1,
        );
        let t = Tfm::parse(&bytes).unwrap();
        let a = TfmItem::Glyph(EncodedGlyph {
            code: 65,
            input_start: 0,
            input_end: 1,
        });
        let k = TfmItem::Kern(FixWord(-131072));
        assert_eq!(t.apply_ligatures_kerns(b"A").unwrap(), vec![k, a, k]);
        assert_eq!(
            t.apply_ligatures_kerns_with_boundaries(
                b"A",
                BoundaryOptions {
                    left: false,
                    right: true
                }
            )
            .unwrap(),
            vec![a, k]
        );
        assert_eq!(
            t.apply_ligatures_kerns_with_boundaries(
                b"A",
                BoundaryOptions {
                    left: true,
                    right: false
                }
            )
            .unwrap(),
            vec![k, a]
        );
        assert_eq!(t.apply_ligatures_kerns(b"").unwrap(), vec![]);
        assert!(t.apply_ligatures_kerns(&[250]).is_err());
    }
    #[test]
    fn boundary_ligatures_keep_actual_byte_intervals() {
        let left = Tfm::parse(&boundary_fixture(&[[128, 65, 0, 66], [255, 0, 0, 0]], 0)).unwrap();
        let right =
            Tfm::parse(&boundary_fixture(&[[255, 250, 0, 0], [128, 250, 0, 66]], 1)).unwrap();
        let expected = vec![TfmItem::Glyph(EncodedGlyph {
            code: 66,
            input_start: 0,
            input_end: 1,
        })];
        assert_eq!(left.apply_ligatures_kerns(b"A").unwrap(), expected);
        assert_eq!(right.apply_ligatures_kerns(b"A").unwrap(), expected);
        let retained =
            Tfm::parse(&boundary_fixture(&[[255, 250, 0, 0], [128, 250, 6, 66]], 1)).unwrap();
        assert_eq!(retained.apply_ligatures_kerns(b"A").unwrap().len(), 2);
        // A real glyph with boundary's code remains a real glyph, not a sentinel.
        let actual =
            Tfm::parse(&boundary_fixture(&[[255, 66, 0, 0], [128, 66, 128, 0]], 1)).unwrap();
        assert_eq!(actual.apply_ligatures_kerns(b"AB").unwrap().len(), 3);
    }
    #[test]
    fn boundary_program_cycles_and_malformed_entries_fail_bounded() {
        let cycle = Tfm::parse(&boundary_fixture(&[[128, 65, 2, 65], [255, 0, 0, 0]], 0)).unwrap();
        assert!(cycle
            .apply_ligatures_kerns(b"A")
            .unwrap_err()
            .to_string()
            .contains("step budget"));
        assert!(Tfm::parse(&boundary_fixture(&[[128, 65, 0, 66], [255, 0, 0, 9]], 0)).is_err());
        assert!(Tfm::parse(&boundary_fixture(&[[128, 65, 0, 90], [255, 0, 0, 0]], 0)).is_err());
    }
    #[test]
    fn pinned_ec_lmr10_metrics_and_boundary_run() {
        let bytes = include_bytes!("../../font-engine/fixtures/tfm/ec-lmr10.tfm");
        assert_eq!(
            crate::sha256(bytes),
            "cd13479f463b9a575d053dd7bf0884daa46bfdeffe4b7f537c193861652ac9e5"
        );
        let t = Tfm::parse(bytes).unwrap();
        let fi = t.apply_ligatures_kerns(b"fi").unwrap();
        assert!(fi.contains(&TfmItem::Glyph(EncodedGlyph {
            code: 28,
            input_start: 0,
            input_end: 2
        })));
        assert!(t
            .apply_ligatures_kerns(b"AV")
            .unwrap()
            .contains(&TfmItem::Kern(FixWord(-116509))));
        eprintln!(
            "TFM boundary character={:?} leftprogram={:?} fi={:?}",
            t.boundary_character, t.left_boundary_program, fi
        );
    }
    #[test]
    fn run_kern_and_ligature_input_provenance() {
        let t = Tfm::parse(&fixture()).unwrap();
        assert_eq!(
            t.apply_ligatures_kerns(b"AB").unwrap(),
            vec![
                TfmItem::Glyph(EncodedGlyph {
                    code: 65,
                    input_start: 0,
                    input_end: 1
                }),
                TfmItem::Kern(FixWord(-131072)),
                TfmItem::Glyph(EncodedGlyph {
                    code: 66,
                    input_start: 1,
                    input_end: 2
                })
            ]
        );
        let mut bytes = fixture();
        bytes[62] = 0;
        bytes[63] = 66;
        let t = Tfm::parse(&bytes).unwrap();
        assert_eq!(
            t.apply_ligatures_kerns(b"AB").unwrap(),
            vec![TfmItem::Glyph(EncodedGlyph {
                code: 66,
                input_start: 0,
                input_end: 2
            })]
        );
    }
    #[test]
    fn run_retention_advance_and_loop_budget() {
        let mut bytes = fixture();
        bytes[62] = 5;
        bytes[63] = 65;
        let t = Tfm::parse(&bytes).unwrap();
        let out = t.apply_ligatures_kerns(b"AB").unwrap();
        assert_eq!(out.len(), 2);
        bytes[62] = 1;
        let t = Tfm::parse(&bytes).unwrap();
        assert!(t.apply_ligatures_kerns(b"AB").is_err());
        assert!(t.apply_ligatures_kerns(&vec![65; 4097]).is_err());
        assert!(t.apply_ligatures_kerns(b"Z").is_err());
    }
    #[test]
    fn run_boundary_semantics_not_silently_dropped() {
        let mut bytes = fixture();
        bytes[60] = 255;
        bytes[62] = 0;
        bytes[63] = 0;
        let t = Tfm::parse(&bytes).unwrap();
        assert_eq!(t.apply_ligatures_kerns(b"AB").unwrap().len(), 2);
    }
    #[test]
    fn exact_metrics_and_signed_kern() {
        let t = Tfm::parse(&fixture()).unwrap();
        assert_eq!(t.checksum, 0x12345678);
        assert_eq!(t.char_metrics(65).unwrap().width, FixWord(1 << 19));
        assert_eq!(
            t.pair_action(65, 66).unwrap(),
            Some(PairAction::Kern(FixWord(-131072)))
        );
        assert_eq!(FixWord(1 << 19).at_design_size(t.design_size), 5i64 << 40);
    }
    #[test]
    fn all_truncations_and_extra_bytes_fail() {
        let b = fixture();
        for i in 0..b.len() {
            assert!(Tfm::parse(&b[..i]).is_err(), "{i}");
        }
        let mut b = b;
        b.push(0);
        assert!(Tfm::parse(&b).is_err());
    }
    #[test]
    fn invalid_metric_program_indices() {
        for (at, value) in [(32, 2), (34, 5), (60, 0), (62, 129)] {
            let mut b = fixture();
            b[at] = value;
            assert!(Tfm::parse(&b).is_err(), "{at}");
        }
    }
    #[test]
    fn ligature_action_retains_semantics() {
        let mut b = fixture();
        b[62] = 0;
        b[63] = 66;
        let t = Tfm::parse(&b).unwrap();
        assert_eq!(
            t.pair_action(65, 66).unwrap(),
            Some(PairAction::Ligature {
                replacement: 66,
                retain_left: false,
                retain_right: false,
                advance: 0
            })
        );
        b[62] = 4;
        assert!(Tfm::parse(&b).is_err());
    }
}
