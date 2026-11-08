//! Independent structural checks on a finished PDF byte string.
//!
//! These read the file back the way a viewer's first pass would: header,
//! `startxref`, the cross-reference table, and each `N 0 obj` header at the
//! offset the table claims. They exist so the writer's tests catch the classic
//! wrong-offset bug rather than trusting the writer's own bookkeeping. They are
//! also handy for the CLI's `--verify` flag.

/// What the cross-reference pass found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structure {
    /// Number of in-use objects (the free object 0 is excluded).
    pub object_count: usize,
    pub xref_offset: usize,
    /// Object numbers of every stream object, in file order.
    pub stream_objects: Vec<usize>,
}

/// One `Tj` as written by this crate's writer, recovered from a content stream.
///
/// An item with several font runs yields several placements sharing the same
/// `x`/`y` (the item's `Td`); `continues` is true for every run after the first.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// Font resource name without the slash, e.g. `F1` or `F2`.
    pub font: String,
    pub font_size: f64,
    pub x: f64,
    pub y: f64,
    pub continues: bool,
    /// Raw string bytes after undoing the writer's escapes.
    pub bytes: Vec<u8>,
}

/// One filled rectangle (`x y w h re f`) in PDF space.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Checks header, trailer, and that every xref entry points at its object.
pub fn check_structure(pdf: &[u8]) -> Result<Structure, String> {
    if !pdf.starts_with(b"%PDF-1.4\n") {
        return Err("missing %PDF-1.4 header".into());
    }
    let tail = &pdf[pdf.len().saturating_sub(64)..];
    let eof = find(tail, b"%%EOF").ok_or("missing %%EOF trailer marker")?;
    if tail[eof + 5..].iter().any(|b| !b.is_ascii_whitespace()) {
        return Err("bytes after %%EOF".into());
    }
    let sx = rfind(pdf, b"startxref\n").ok_or("missing startxref")?;
    let xref_offset: usize = ascii_line(pdf, sx + "startxref\n".len())?
        .parse()
        .map_err(|_| "startxref is not a number")?;
    if !pdf
        .get(xref_offset..)
        .is_some_and(|s| s.starts_with(b"xref\n"))
    {
        return Err(format!("startxref {xref_offset} does not point at 'xref'"));
    }
    let mut pos = xref_offset + "xref\n".len();
    let header = ascii_line(pdf, pos)?;
    pos += header.len() + 1;
    let (first, count) = header
        .split_once(' ')
        .and_then(|(a, b)| Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?)))
        .ok_or("malformed xref subsection header")?;
    if first != 0 {
        return Err("xref must start at object 0".into());
    }
    let mut stream_objects = Vec::new();
    for n in 0..count {
        let entry = pdf
            .get(pos..pos + 20)
            .ok_or_else(|| format!("xref entry {n} truncated"))?;
        pos += 20;
        let entry = std::str::from_utf8(entry).map_err(|_| format!("xref entry {n} not ASCII"))?;
        if entry.len() != 20 || !entry.ends_with(" \n") {
            return Err(format!("xref entry {n} is not 20 bytes: {entry:?}"));
        }
        let offset: usize = entry[0..10]
            .parse()
            .map_err(|_| format!("xref entry {n} offset"))?;
        let kind = &entry[17..18];
        match (n, kind) {
            (0, "f") => continue,
            (0, _) => return Err("object 0 must be free".into()),
            (_, "n") => {}
            (_, k) => return Err(format!("xref entry {n} has unexpected type {k:?}")),
        }
        let expect = format!("{n} 0 obj\n");
        if !pdf
            .get(offset..)
            .is_some_and(|s| s.starts_with(expect.as_bytes()))
        {
            return Err(format!(
                "xref entry {n} offset {offset} does not point at {expect:?}"
            ));
        }
        let body_start = offset + expect.len();
        let body = pdf
            .get(body_start..)
            .ok_or(format!("object {n} has no endobj"))?;
        let end = find(body, b"endobj").ok_or(format!("object {n} has no endobj"))?;
        if find(&body[..end], b"stream\n").is_some() {
            stream_objects.push(n);
        }
    }
    if !pdf.get(pos..).is_some_and(|s| s.starts_with(b"trailer\n")) {
        return Err("xref table not followed by trailer".into());
    }
    let trailer = ascii_line(pdf, pos + "trailer\n".len())?;
    if !trailer.contains(&format!("/Size {count}")) {
        return Err(format!(
            "trailer /Size disagrees with xref count {count}: {trailer}"
        ));
    }
    if !trailer.contains("/Root 1 0 R") {
        return Err(format!("trailer has no /Root 1 0 R: {trailer}"));
    }
    Ok(Structure {
        object_count: count - 1,
        xref_offset,
        stream_objects,
    })
}

/// Returns the raw bytes of the given stream object's data.
pub fn stream_data(pdf: &[u8], object: usize) -> Result<Vec<u8>, String> {
    let head = format!("\n{object} 0 obj\n");
    let at = find(pdf, head.as_bytes()).ok_or(format!("object {object} not found"))? + 1;
    let dict_start = at + head.len() - 1;
    let dict = ascii_line(pdf, dict_start)?;
    let len_at = dict
        .find("/Length ")
        .ok_or(format!("object {object} has no /Length"))?;
    let length: usize = dict[len_at + 8..]
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|d| d.parse().ok())
        .ok_or(format!("object {object} has non-numeric /Length"))?;
    let data_start = dict_start + dict.len() + 1 + "stream\n".len();
    let data = pdf
        .get(data_start..data_start + length)
        .ok_or(format!("object {object} stream truncated"))?;
    if !pdf[data_start + length..].starts_with(b"\nendstream") {
        return Err(format!(
            "object {object} /Length {length} does not end at endstream"
        ));
    }
    Ok(data.to_vec())
}

/// Recovers each `Tj` from a content stream written by [`crate::writer`].
pub fn placements(content: &[u8]) -> Result<Vec<Placement>, String> {
    let mut out = Vec::new();
    let mut font: Option<(String, f64)> = None;
    let mut td = None;
    let mut continues = false;
    let mut pos = 0;
    while pos < content.len() {
        let end = content[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(content.len(), |e| pos + e);
        let line = &content[pos..end];
        pos = end + 1;
        if line.ends_with(b" Tf") {
            let s = std::str::from_utf8(line).map_err(|_| "Tf line not ASCII")?;
            let mut parts = s.split(' ');
            let name = parts.next().unwrap_or("");
            let name = name
                .strip_prefix('/')
                .filter(|n| {
                    n.len() > 1 && n.starts_with('F') && n[1..].bytes().all(|b| b.is_ascii_digit())
                })
                .ok_or_else(|| format!("unexpected font resource {name}"))?;
            let size = parts
                .next()
                .unwrap_or("")
                .parse::<f64>()
                .map_err(|_| "Tf size")?;
            font = Some((name.to_string(), size));
        } else if line == b"BT" {
            font = None;
            continues = false;
        } else if line.ends_with(b" Td") {
            let s = std::str::from_utf8(line).map_err(|_| "Td line not ASCII")?;
            let mut parts = s.split(' ');
            let x = parts
                .next()
                .unwrap_or("")
                .parse::<f64>()
                .map_err(|_| "Td x")?;
            let y = parts
                .next()
                .unwrap_or("")
                .parse::<f64>()
                .map_err(|_| "Td y")?;
            td = Some((x, y));
        } else if line.ends_with(b") Tj") || line.ends_with(b"> Tj") {
            let (x, y) = td.ok_or("Tj without preceding Td")?;
            let (name, font_size) = font.clone().ok_or("Tj without Tf")?;
            let body = &line[1..line.len() - 4];
            let bytes = match line[0] {
                b'(' => unescape(body),
                b'<' => unhex(body)?,
                _ => {
                    return Err(format!(
                        "unexpected Tj operand {:?}",
                        String::from_utf8_lossy(line)
                    ));
                }
            };
            out.push(Placement {
                font: name,
                font_size,
                x,
                y,
                continues,
                bytes,
            });
            continues = true;
        } else if line == b"ET" {
            td = None;
        }
    }
    Ok(out)
}

/// Recovers each `re f` rectangle from a content stream written by
/// [`crate::writer`].
pub fn rules(content: &[u8]) -> Result<Vec<Rule>, String> {
    let mut out = Vec::new();
    for line in content.split(|&b| b == b'\n') {
        if !line.ends_with(b" re f") {
            continue;
        }
        let s = std::str::from_utf8(line).map_err(|_| "re line not ASCII")?;
        let nums: Vec<f64> = s
            .split(' ')
            .take(4)
            .map(|n| n.parse::<f64>())
            .collect::<Result<_, _>>()
            .map_err(|_| format!("malformed rectangle line {s:?}"))?;
        if nums.len() != 4 {
            return Err(format!("malformed rectangle line {s:?}"));
        }
        out.push(Rule {
            x: nums[0],
            y: nums[1],
            width: nums[2],
            height: nums[3],
        });
    }
    Ok(out)
}

fn unhex(s: &[u8]) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(s).map_err(|_| "hex string not ASCII")?;
    if text.len() % 2 != 0 {
        return Err(format!("odd-length hex string {text:?}"));
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| format!("bad hex {text:?}")))
        .collect()
}

fn unescape(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'\\' && i + 1 < s.len() {
            out.push(match s[i + 1] {
                b'n' => b'\n',
                b'r' => b'\r',
                other => other,
            });
            i += 2;
        } else {
            out.push(s[i]);
            i += 1;
        }
    }
    out
}

fn ascii_line(pdf: &[u8], start: usize) -> Result<String, String> {
    let rest = pdf.get(start..).ok_or("offset past end of file")?;
    let end = rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
    String::from_utf8(rest[..end].to_vec()).map_err(|_| format!("non-ASCII line at {start}"))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn rfind(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).rposition(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::check_structure;

    #[test]
    fn oversized_startxref_returns_error() {
        let pdf = b"%PDF-1.4\nstartxref\n999999999\n%%EOF\n";
        assert_eq!(
            check_structure(pdf).unwrap_err(),
            "startxref 999999999 does not point at 'xref'"
        );
    }

    #[test]
    fn oversized_xref_entry_offset_returns_error() {
        let pdf = b"%PDF-1.4\nxref\n0 2\n0000000000 65535 f \n9999999999 00000 n \ntrailer\n<< /Size 2 /Root 1 0 R >>\nstartxref\n9\n%%EOF\n";
        assert_eq!(
            check_structure(pdf).unwrap_err(),
            "xref entry 1 offset 9999999999 does not point at \"1 0 obj\\n\""
        );
    }
}
