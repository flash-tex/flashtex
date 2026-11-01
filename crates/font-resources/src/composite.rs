use crate::{invalid, u16_at, u32_at, Result};
pub const MAX_COMPOSITE_DEPTH: usize = 32;
const MAX_EDGES: usize = 1_000_000;
/// Validates component graph and record extents, not outlines or bytecode.
pub(crate) fn validate(head: &[u8], loca: &[u8], glyf: &[u8], count: usize) -> Result<()> {
    let long = u16_at(head, 50)? == 1;
    let offset = |i| -> Result<usize> {
        Ok(if long {
            u32_at(loca, i * 4)? as usize
        } else {
            u16_at(loca, i * 2)? as usize * 2
        })
    };
    let mut graph = vec![Vec::new(); count];
    let mut edges = 0;
    for (gid, children) in graph.iter_mut().enumerate() {
        let data = glyf
            .get(offset(gid)?..offset(gid + 1)?)
            .ok_or_else(|| invalid("glyph range"))?;
        if data.is_empty() {
            continue;
        }
        let contours = u16_at(data, 0)? as i16;
        if contours >= 0 {
            continue;
        }
        if contours != -1 {
            return Err(invalid("unknown composite contour marker"));
        }
        let mut at = 10;
        let last_flags = loop {
            let flags = u16_at(data, at)?;
            let child = u16_at(data, at + 2)? as usize;
            at += 4;
            if child >= count {
                return Err(invalid("composite child GID outside font"));
            }
            edges += 1;
            if edges > MAX_EDGES {
                return Err(invalid("composite edge budget"));
            }
            children.push(child);
            let transforms = [8, 64, 128]
                .iter()
                .filter(|flag| flags & **flag != 0)
                .count();
            if transforms > 1 || flags & 0x1800 == 0x1800 {
                return Err(invalid("conflicting composite transforms"));
            }
            at += if flags & 1 != 0 { 4 } else { 2 };
            at += if flags & 8 != 0 {
                2
            } else if flags & 64 != 0 {
                4
            } else if flags & 128 != 0 {
                8
            } else {
                0
            };
            if at > data.len() {
                return Err(invalid("truncated composite arguments/transform"));
            }
            if flags & 32 == 0 {
                break flags;
            }
            if flags & 256 != 0 {
                return Err(invalid("instructions before final component"));
            }
        };
        if last_flags & 256 != 0 {
            let length = u16_at(data, at)? as usize;
            at += 2;
            if at + length > data.len() {
                return Err(invalid("truncated composite instructions"));
            }
        }
    }
    validate_graph(&graph)
}
fn validate_graph(graph: &[Vec<usize>]) -> Result<()> {
    fn visit(
        id: usize,
        graph: &[Vec<usize>],
        state: &mut [u8],
        heights: &mut [usize],
        depth: usize,
    ) -> Result<usize> {
        if depth > MAX_COMPOSITE_DEPTH {
            return Err(invalid("composite depth budget"));
        }
        if state[id] == 1 {
            return Err(invalid("composite glyph cycle"));
        }
        if state[id] == 2 {
            return Ok(heights[id]);
        }
        state[id] = 1;
        let mut height = 0;
        for &child in &graph[id] {
            height = height.max(1 + visit(child, graph, state, heights, depth + 1)?);
        }
        if height > MAX_COMPOSITE_DEPTH {
            return Err(invalid("composite depth budget"));
        }
        state[id] = 2;
        heights[id] = height;
        Ok(height)
    }
    let mut state = vec![0; graph.len()];
    let mut heights = vec![0; graph.len()];
    for id in 0..graph.len() {
        visit(id, graph, &mut state, &mut heights, 0)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input(child: u16, flags: u16) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let h = vec![0; 54];
        let mut g = vec![255, 255, 0, 0, 0, 0, 0, 0, 0, 0];
        g.extend(flags.to_be_bytes());
        g.extend(child.to_be_bytes());
        g.extend([0, 0]);
        (h, vec![0, 0, 0, 8, 0, 8], g)
    }
    #[test]
    fn composite_valid_and_bad_gid() {
        let (h, l, g) = input(1, 0);
        validate(&h, &l, &g, 2).unwrap();
        let (h, l, g) = input(2, 0);
        assert!(validate(&h, &l, &g, 2).is_err());
    }
    #[test]
    fn self_and_indirect_cycles() {
        let (h, l, g) = input(0, 0);
        assert!(validate(&h, &l, &g, 2).is_err());
        assert!(validate_graph(&[vec![1], vec![0]]).is_err());
    }
    #[test]
    fn bounded_depth_and_shared_subgraphs() {
        let mut graph = vec![vec![]; 34];
        for (i, row) in graph.iter_mut().enumerate().take(33) {
            row.push(i + 1);
        }
        assert!(validate_graph(&graph).is_err());
        graph[32].clear();
        validate_graph(&graph).unwrap();
        validate_graph(&[vec![], vec![0], vec![0, 1]]).unwrap();
    }
    #[test]
    fn transform_and_instruction_truncation() {
        for flags in [1, 8, 64, 128, 256, 32, 8 | 64] {
            let (h, l, g) = input(1, flags);
            assert!(validate(&h, &l, &g, 2).is_err(), "{flags}");
        }
    }
}
