use crate::{invalid, u16_at, u32_at, Error, FontResource, Result};
/// Exact unhinted design coordinates, signed 16.16 fixed point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlinePoint {
    pub x: i32,
    pub y: i32,
    pub on_curve: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleOutline {
    pub points: Vec<OutlinePoint>,
    /// Inclusive last point index of each closed quadratic contour.
    pub contour_ends: Vec<u16>,
    /// Raw signed font-unit bounding box [xmin,ymin,xmax,ymax].
    pub bounds: [i16; 4],
    /// Preserved, never executed by this decoder.
    pub instructions: Vec<u8>,
    pub overlap_simple: bool,
}
impl FontResource {
    pub fn simple_outline(&self, gid: u16) -> Result<SimpleOutline> {
        if gid as u32 >= self.descriptor().glyph_count {
            return Err(invalid("glyph ID outside font"));
        }
        let head = self.table(b"head").ok_or_else(|| invalid("head missing"))?;
        let loca = self.table(b"loca").ok_or_else(|| invalid("loca missing"))?;
        let glyf = self.table(b"glyf").ok_or_else(|| invalid("glyf missing"))?;
        let offset = |i| -> Result<usize> {
            Ok(if u16_at(head, 50)? == 1 {
                u32_at(loca, i * 4)? as usize
            } else {
                u16_at(loca, i * 2)? as usize * 2
            })
        };
        decode(
            glyf.get(offset(gid as usize)?..offset(gid as usize + 1)?)
                .ok_or_else(|| invalid("glyph extent"))?,
        )
    }
}
pub(crate) fn decode(data: &[u8]) -> Result<SimpleOutline> {
    let mut out = SimpleOutline {
        points: Vec::new(),
        contour_ends: Vec::new(),
        bounds: [0; 4],
        instructions: Vec::new(),
        overlap_simple: false,
    };
    if data.is_empty() {
        return Ok(out);
    }
    let contours = u16_at(data, 0)? as i16;
    if contours < 0 {
        return Err(Error::UnsupportedFont(
            "composite outline transforms not decoded".into(),
        ));
    }
    for (i, bound) in out.bounds.iter_mut().enumerate() {
        *bound = u16_at(data, 2 + i * 2)? as i16;
    }
    if out.bounds[0] > out.bounds[2] || out.bounds[1] > out.bounds[3] {
        return Err(invalid("inverted glyph bounding box"));
    }
    let mut at = 10;
    for _ in 0..contours {
        let end = u16_at(data, at)?;
        at += 2;
        if out.contour_ends.last().is_some_and(|p| *p >= end) {
            return Err(invalid("non-increasing contour endpoints"));
        }
        out.contour_ends.push(end);
    }
    // Empty glyphs may have only their header.
    if contours == 0 && at == data.len() {
        return Ok(out);
    }
    let instruction_count = u16_at(data, at)? as usize;
    at += 2;
    out.instructions = data
        .get(at..at + instruction_count)
        .ok_or_else(|| invalid("truncated simple instructions"))?
        .to_vec();
    at += instruction_count;
    let count = out.contour_ends.last().map_or(0, |end| *end as usize + 1);
    let mut flags = Vec::with_capacity(count);
    while flags.len() < count {
        let flag = *data
            .get(at)
            .ok_or_else(|| invalid("truncated point flags"))?;
        at += 1;
        if flag & 128 != 0 {
            return Err(invalid("reserved point flag"));
        }
        let repeat = if flag & 8 != 0 {
            let n = *data
                .get(at)
                .ok_or_else(|| invalid("truncated flag repeat"))? as usize;
            at += 1;
            n + 1
        } else {
            1
        };
        if flags.len() + repeat > count {
            return Err(invalid("point flag repeat overflow"));
        }
        flags.extend(std::iter::repeat_n(flag, repeat));
    }
    out.overlap_simple = flags.first().is_some_and(|flag| flag & 64 != 0);
    out.points = flags
        .iter()
        .map(|f| OutlinePoint {
            x: 0,
            y: 0,
            on_curve: f & 1 != 0,
        })
        .collect();
    for axis in 0..2 {
        let short = if axis == 0 { 2 } else { 4 };
        let same = if axis == 0 { 16 } else { 32 };
        let mut coordinate = 0i32;
        for (point, flag) in out.points.iter_mut().zip(&flags) {
            let delta = if flag & short != 0 {
                let v = *data
                    .get(at)
                    .ok_or_else(|| invalid("truncated short coordinate"))?
                    as i32;
                at += 1;
                if flag & same != 0 {
                    v
                } else {
                    -v
                }
            } else if flag & same != 0 {
                0
            } else {
                let v = u16_at(data, at)? as i16 as i32;
                at += 2;
                v
            };
            coordinate = coordinate
                .checked_add(delta)
                .ok_or_else(|| invalid("coordinate overflow"))?;
            if !(i16::MIN as i32..=i16::MAX as i32).contains(&coordinate) {
                return Err(invalid("coordinate outside signed font-unit bounds"));
            }
            let fixed = coordinate * 65536;
            if axis == 0 {
                point.x = fixed;
            } else {
                point.y = fixed;
            }
        }
    }
    // Preserve raw points: implied quadratic midpoints are a downstream path operation.
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn triangle() -> Vec<u8> {
        let mut d = vec![0, 1, 0, 0, 0, 0, 0, 100, 0, 100, 0, 2, 0, 0];
        d.extend([0x31, 0x33, 0x27, 100, 100, 100]);
        d
    }
    #[test]
    #[ignore = "requires explicitly selected installed font; no bundled font"]
    fn installed_simple_glyph_smoke() {
        let path = std::env::var("FLASHTEX_SMOKE_FONT").expect("explicit font path required");
        let bytes = std::fs::read(path).unwrap();
        let parsed = crate::parse(&bytes).unwrap();
        let table = |tag: &[u8; 4]| &bytes[parsed.tables[tag].clone()];
        let head = table(b"head");
        let loca = table(b"loca");
        let glyf = table(b"glyf");
        let offset = |i| {
            if u16_at(head, 50).unwrap() == 1 {
                u32_at(loca, i * 4).unwrap() as usize
            } else {
                u16_at(loca, i * 2).unwrap() as usize * 2
            }
        };
        let mut simple = 0;
        let mut composite = 0;
        for gid in 0..parsed.glyphs as usize {
            let data = &glyf[offset(gid)..offset(gid + 1)];
            if !data.is_empty() && (u16_at(data, 0).unwrap() as i16) < 0 {
                composite += 1;
                continue;
            }
            decode(data).unwrap_or_else(|e| panic!("gid {gid}: {e}"));
            simple += 1;
        }
        eprintln!(
            "font sha256={} simple/empty decoded={simple}, composite skipped={composite}",
            crate::sha256(&bytes)
        );
    }
    #[test]
    fn exact_coordinates_flags_and_contours() {
        let o = decode(&triangle()).unwrap();
        assert_eq!(o.contour_ends, vec![2]);
        assert_eq!(
            o.points,
            vec![
                OutlinePoint {
                    x: 0,
                    y: 0,
                    on_curve: true
                },
                OutlinePoint {
                    x: 6553600,
                    y: 0,
                    on_curve: true
                },
                OutlinePoint {
                    x: 0,
                    y: 6553600,
                    on_curve: true
                }
            ]
        );
    }
    #[test]
    fn truncated_and_repeated_flags() {
        let d = triangle();
        for n in 1..d.len() {
            assert!(decode(&d[..n]).is_err(), "{n}");
        }
        let mut d = triangle();
        d[14] = 0x39;
        d[15] = 255;
        assert!(decode(&d).is_err());
    }
    #[test]
    fn empty_composite_and_instructions() {
        assert!(decode(&[]).unwrap().points.is_empty());
        let mut d = triangle();
        d[0] = 255;
        d[1] = 255;
        assert!(matches!(decode(&d), Err(Error::UnsupportedFont(_))));
        let mut d = triangle();
        d[12] = 255;
        assert!(decode(&d).is_err());
    }
    #[test]
    fn offcurve_signed_delta_and_repeat() {
        let mut d = vec![0, 1, 255, 156, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0];
        d.extend([0x28, 1, 255, 156, 0, 0]);
        let o = decode(&d).unwrap();
        assert_eq!(o.points[0].x, -6553600);
        assert_eq!(o.points[1].x, -6553600);
        assert!(!o.points[1].on_curve);
    }
}
