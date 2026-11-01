//! TFM loading scaled exactly as TeX does (tex.web §560–575).
//!
//! Only what text typesetting needs: per-character width/height/depth/italic in
//! scaled points, the ligature/kern program, the boundary character and the
//! font parameters (`\fontdimen`). Parameter 1 (slant) is kept unscaled in
//! units of 2^-16, exactly like TeX's `slant(f)`.

use std::fmt;

/// Scaled points (65536 sp = 1 pt).
pub type Scaled = i32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TfmError(pub String);

impl fmt::Display for TfmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bad TFM: {}", self.0)
    }
}

impl std::error::Error for TfmError {}

fn bad(msg: &str) -> TfmError {
    TfmError(msg.to_string())
}

/// Per-character dimensions after scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CharDims {
    pub width: Scaled,
    pub height: Scaled,
    pub depth: Scaled,
    pub italic: Scaled,
}

/// One instruction of a ligature/kern program (`skip`, `next`, `op`, `rem`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LigKernStep {
    pub skip: u8,
    pub next: u8,
    pub op: u8,
    pub rem: u8,
}

pub const STOP_FLAG: u8 = 128;
pub const KERN_FLAG: u8 = 128;

/// A TFM file loaded at a given size (`\font\x=name at size`).
#[derive(Debug, Clone)]
pub struct ScaledFont {
    pub name: String,
    pub size: Scaled,
    pub design_size: Scaled,
    chars: Vec<Option<CharDims>>,
    /// `Some(index into lig_kern)` for characters with `lig_tag`.
    lig_start: Vec<Option<usize>>,
    lig_kern: Vec<LigKernStep>,
    kerns: Vec<Scaled>,
    params: Vec<Scaled>,
    /// `font_bchar`: the right boundary character, if the font declares one.
    pub bchar: Option<u8>,
    /// `bchar_label`: start of the left-boundary program, if any.
    pub bchar_label: Option<usize>,
}

fn be16(b: &[u8], i: usize) -> Result<usize, TfmError> {
    b.get(i..i + 2)
        .map(|s| u16::from_be_bytes([s[0], s[1]]) as usize)
        .ok_or_else(|| bad("truncated header"))
}

impl ScaledFont {
    /// Loads `bytes` at `size` sp; `size <= 0` means the design size.
    pub fn from_bytes(name: &str, bytes: &[u8], size: Scaled) -> Result<Self, TfmError> {
        let mut n = [0usize; 12];
        for (k, v) in n.iter_mut().enumerate() {
            *v = be16(bytes, 2 * k)?;
        }
        let [lf, lh, bc, ec, nw, nh, nd, ni, nl, nk, ne, np] = n;
        if lf * 4 > bytes.len() || lh < 2 || ec > 255 || bc > ec + 1 {
            return Err(bad("inconsistent lengths"));
        }
        if lf != 6 + lh + (ec + 1 - bc) + nw + nh + nd + ni + nl + nk + ne + np {
            return Err(bad("lf mismatch"));
        }
        let word = |i: usize| -> [u8; 4] {
            let o = 4 * i;
            [bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]
        };
        let fix = |w: [u8; 4]| i32::from_be_bytes(w);
        let header = 6;
        let design = fix(word(header + 1)) >> 4; // TeX: z := design_size div 16 (in sp)
        let z_size = if size > 0 { size } else { design };
        let char_base = header + lh;
        let width_base = char_base + (ec + 1 - bc);
        let height_base = width_base + nw;
        let depth_base = height_base + nh;
        let italic_base = depth_base + nd;
        let lig_base = italic_base + ni;
        let kern_base = lig_base + nl;
        let exten_base = kern_base + nk;
        let param_base = exten_base + ne;

        // §572 store_scaled.
        let mut z = z_size;
        let mut alpha: i32 = 16;
        while z >= 0o40000000 {
            z /= 2;
            alpha += alpha;
        }
        let beta: i32 = 256 / alpha;
        let alpha = alpha * z;
        let store_scaled = |w: [u8; 4]| -> Result<Scaled, TfmError> {
            let (a, b, c, d) = (w[0] as i32, w[1] as i32, w[2] as i32, w[3] as i32);
            let sw = ((((d * z) / 256) + (c * z)) / 256 + (b * z)) / beta;
            match a {
                0 => Ok(sw),
                255 => Ok(sw - alpha),
                _ => Err(bad("fix_word out of range")),
            }
        };
        let scaled_table = |base: usize, count: usize| -> Result<Vec<Scaled>, TfmError> {
            (0..count).map(|i| store_scaled(word(base + i))).collect()
        };
        let widths = scaled_table(width_base, nw)?;
        let heights = scaled_table(height_base, nh)?;
        let depths = scaled_table(depth_base, nd)?;
        let italics = scaled_table(italic_base, ni)?;
        let kerns = scaled_table(kern_base, nk)?;
        let lig_kern: Vec<LigKernStep> = (0..nl)
            .map(|i| {
                let w = word(lig_base + i);
                LigKernStep {
                    skip: w[0],
                    next: w[1],
                    op: w[2],
                    rem: w[3],
                }
            })
            .collect();

        let mut chars = vec![None; 256];
        let mut lig_start = vec![None; 256];
        for c in bc..=ec {
            let w = word(char_base + c - bc);
            let wi = w[0] as usize;
            if wi == 0 {
                continue;
            }
            let hi = (w[1] >> 4) as usize;
            let di = (w[1] & 15) as usize;
            let ii = (w[2] >> 2) as usize;
            let tag = w[2] & 3;
            let rem = w[3] as usize;
            let get = |t: &Vec<Scaled>, i: usize| t.get(i).copied().ok_or_else(|| bad("index"));
            chars[c] = Some(CharDims {
                width: get(&widths, wi)?,
                height: get(&heights, hi)?,
                depth: get(&depths, di)?,
                italic: get(&italics, ii)?,
            });
            if tag == 1 {
                lig_start[c] = Some(rem);
            }
        }
        // §573: resolve lig_kern_restart for chars whose first instruction has skip > stop_flag.
        for start in lig_start.iter_mut().flatten() {
            let first = *lig_kern.get(*start).ok_or_else(|| bad("lig start"))?;
            if first.skip > STOP_FLAG {
                *start = 256 * first.op as usize + first.rem as usize;
            }
        }
        let mut bchar = None;
        let mut bchar_label = None;
        if nl > 0 {
            let first = lig_kern[0];
            if first.skip == 255 {
                bchar = Some(first.next);
            }
            let last = lig_kern[nl - 1];
            if last.skip == 255 {
                bchar_label = Some(256 * last.op as usize + last.rem as usize);
            }
        }
        let mut params = Vec::with_capacity(np.max(7));
        for k in 1..=np {
            let w = word(param_base + k - 1);
            if k == 1 {
                let mut sw = w[0] as i32;
                if sw > 127 {
                    sw -= 256;
                }
                let sw = ((sw * 256 + w[1] as i32) * 256 + w[2] as i32) * 16 + (w[3] as i32 >> 4);
                params.push(sw);
            } else {
                params.push(store_scaled(w)?);
            }
        }
        while params.len() < 7 {
            params.push(0);
        }
        Ok(ScaledFont {
            name: name.to_string(),
            size: z_size,
            design_size: design,
            chars,
            lig_start,
            lig_kern,
            kerns,
            params,
            bchar,
            bchar_label,
        })
    }

    pub fn char_dims(&self, c: u8) -> Option<CharDims> {
        self.chars[c as usize]
    }

    pub fn char_exists(&self, c: u8) -> bool {
        self.chars[c as usize].is_some()
    }

    pub fn width(&self, c: u8) -> Scaled {
        self.chars[c as usize].map_or(0, |d| d.width)
    }

    pub fn height(&self, c: u8) -> Scaled {
        self.chars[c as usize].map_or(0, |d| d.height)
    }

    pub fn depth(&self, c: u8) -> Scaled {
        self.chars[c as usize].map_or(0, |d| d.depth)
    }

    /// `\fontdimen k` (1-based). Parameter 1 is the unscaled slant.
    pub fn param(&self, k: usize) -> Scaled {
        self.params.get(k - 1).copied().unwrap_or(0)
    }

    pub fn slant(&self) -> Scaled {
        self.param(1)
    }
    pub fn space(&self) -> Scaled {
        self.param(2)
    }
    pub fn space_stretch(&self) -> Scaled {
        self.param(3)
    }
    pub fn space_shrink(&self) -> Scaled {
        self.param(4)
    }
    pub fn x_height(&self) -> Scaled {
        self.param(5)
    }
    pub fn quad(&self) -> Scaled {
        self.param(6)
    }
    pub fn extra_space(&self) -> Scaled {
        self.param(7)
    }

    /// Start index of `c`'s lig/kern program (already restarted), if it has one.
    pub fn lig_kern_start(&self, c: u8) -> Option<usize> {
        self.lig_start[c as usize]
    }

    pub fn step(&self, index: usize) -> Option<LigKernStep> {
        self.lig_kern.get(index).copied()
    }

    pub fn kern_value(&self, step: LigKernStep) -> Scaled {
        self.kerns
            .get(256 * (step.op as usize - KERN_FLAG as usize) + step.rem as usize)
            .copied()
            .unwrap_or(0)
    }

    /// Resolves a program index that may itself be a restart instruction.
    pub fn program_index(&self, index: usize) -> usize {
        match self.lig_kern.get(index) {
            Some(s) if s.skip > STOP_FLAG => 256 * s.op as usize + s.rem as usize,
            _ => index,
        }
    }
}
