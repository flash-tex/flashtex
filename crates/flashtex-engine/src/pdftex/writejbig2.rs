//! `writejbig2.c`, ported: JBIG2 images (`/JBIG2Decode`, PDF 1.4). The
//! segments of the selected page are copied, with page associations set to
//! 1; the file's page-0 segments that they refer to become one
//! `/JBIG2Globals` stream per file, written at the end of the run
//! ([`Globals::flush_jbig2_page0_objects`]).
//!
//! The C code's lists and AVL trees become vectors and ordered maps; a
//! tree's `avl_probe` keeps the first entry of a key and so does
//! `entry().or_insert` here.

use super::cfile::{CFile, Whence};
use super::images::{ImageData, ImageEntry};
use crate::generated::Globals;
use std::collections::BTreeMap;

// 7.3 Segment types
const M_SYMBOL_DICTIONARY: u32 = 0;
const M_INTERMEDIATE_TEXT_REGION: u32 = 4;
const M_IMMEDIATE_TEXT_REGION: u32 = 6;
const M_IMMEDIATE_LOSSLESS_TEXT_REGION: u32 = 7;
const M_PATTERN_DICTIONARY: u32 = 16;
const M_INTERMEDIATE_HALFTONE_REGION: u32 = 20;
const M_IMMEDIATE_HALFTONE_REGION: u32 = 22;
const M_IMMEDIATE_LOSSLESS_HALFTONE_REGION: u32 = 23;
const M_INTERMEDIATE_GENERIC_REGION: u32 = 36;
const M_IMMEDIATE_GENERIC_REGION: u32 = 38;
const M_IMMEDIATE_LOSSLESS_GENERIC_REGION: u32 = 39;
const M_INTERMEDIATE_GENERIC_REFINEMENT_REGION: u32 = 40;
const M_IMMEDIATE_GENERIC_REFINEMENT_REGION: u32 = 42;
const M_IMMEDIATE_LOSSLESS_GENERIC_REFINEMENT_REGION: u32 = 43;
const M_PAGE_INFORMATION: u32 = 48;
const M_END_OF_PAGE: u32 = 49;
const M_END_OF_STRIPE: u32 = 50;
const M_END_OF_FILE: u32 = 51;
const M_PROFILES: u32 = 52;
const M_TABLES: u32 = 53;
const M_EXTENSION: u32 = 62;

/// `JBIG2_IMAGE_INFO`.
pub struct Jbig2Image {
    pub selected_page: i32,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Initial,
    HaveInfo,
}

/// `SEGINFO`.
#[derive(Clone, Default)]
struct SegInfo {
    segnum: u64,
    isrefered: bool,
    refers: bool,
    seghdrflags: u32,
    pageassocsizeflag: bool,
    reftosegcount: u32,
    countofrefered: u32,
    fieldlen: u32,
    segnumwidth: u32,
    segpage: i64,
    segdatalen: u64,
    hdrstart: i64,
    hdrend: i64,
    datastart: i64,
    dataend: i64,
    endofstripeflag: bool,
    endofpageflag: bool,
    pageinfoflag: bool,
    endoffileflag: bool,
}

/// `PAGEINFO`.
#[derive(Default)]
struct PageInfo {
    /// `segments`, in file order.
    segments: Vec<SegInfo>,
    /// the tree of `segments` by number (`segments_maketree`).
    seg_tree: BTreeMap<u64, usize>,
    pagenum: u64,
    width: u32,
    height: u32,
    xres: u32,
    yres: u32,
    pagesegmentflags: u32,
    stripinginfo: u32,
    stripedheight: u32,
}

/// `FILEINFO`.
#[derive(Default)]
struct FileInfo {
    filename: Vec<u8>,
    filesize: i64,
    /// `pages` (not including page 0), in file order.
    pages: Vec<PageInfo>,
    /// the tree of `pages` by page number (`pages_maketree`).
    page_tree: BTreeMap<u64, usize>,
    /// `page0`: at most one entry.
    page0: Option<PageInfo>,
    filehdrflags: u32,
    sequentialaccess: bool,
    numofpages: u64,
    streamstart: i64,
    pdfpage0objnum: u64,
    phase: Phase,
    /// The depth of `markpage0seg`'s recursion.
    depth: u32,
}

#[derive(Default)]
pub struct State {
    /// `file_tree`: by file name (`strcmp` order).
    files: BTreeMap<Vec<u8>, FileInfo>,
}

impl Globals {
    /// `ygetc`: the next byte; the end of the file stops the run.
    fn ygetc(&mut self, f: &mut CFile) -> u32 {
        let c = f.getc();
        if c < 0 {
            self.pdftex_fail("getc() failed; premature end of JBIG2 image file");
        }
        c as u32
    }

    fn jb_read2bytes(&mut self, f: &mut CFile) -> u32 {
        let c = self.ygetc(f);
        (c << 8) + self.ygetc(f)
    }

    fn jb_read4bytes(&mut self, f: &mut CFile) -> u64 {
        let l = self.jb_read2bytes(f);
        // `(l << 16) + read2bytes(f)` in unsigned int, then unsigned long
        ((l << 16).wrapping_add(self.jb_read2bytes(f))) as u64
    }

    /// `readfilehdr`: Annex D.4.
    fn readfilehdr(&mut self, fip: &mut FileInfo, f: &mut CFile) {
        // Annex D.4.1 ID string
        const JBIG2_ID: [u8; 8] = [0x97, b'J', b'B', b'2', 0x0d, 0x0a, 0x1a, 0x0a];
        self.xfseek(f, 0, Whence::Set, &fip.filename.clone());
        for b in JBIG2_ID {
            if self.ygetc(f) != b as u32 {
                self.pdftex_fail(
                    "readfilehdr(): reading JBIG2 image file failed: ID string missing",
                );
            }
        }
        // Annex D.4.2 File header flags
        fip.filehdrflags = self.ygetc(f);
        fip.sequentialaccess = fip.filehdrflags & 0x01 != 0;
        if fip.sequentialaccess {
            // Annex D.1 vs. Annex D.2
            let name = fip.filename.clone();
            self.xfseek(f, 0, Whence::End, &name);
            fip.filesize = f.tell();
            self.xfseek(f, 9, Whence::Set, &name);
        }
        // Annex D.4.3 Number of pages: C tests `!(flags >> 1) & 0x01`,
        // which is true when no bit above bit 0 is set
        if ((fip.filehdrflags >> 1) == 0) as u32 & 0x01 != 0 {
            // known number of pages
            fip.numofpages = self.jb_read4bytes(f);
        }
    }

    /// `checkseghdrflags`.
    fn checkseghdrflags(&mut self, sip: &mut SegInfo) {
        sip.endofstripeflag = false;
        sip.endofpageflag = false;
        sip.pageinfoflag = false;
        sip.endoffileflag = false;
        // 7.3 Segment types
        match sip.seghdrflags & 0x3f {
            M_SYMBOL_DICTIONARY
            | M_INTERMEDIATE_TEXT_REGION
            | M_IMMEDIATE_TEXT_REGION
            | M_IMMEDIATE_LOSSLESS_TEXT_REGION
            | M_PATTERN_DICTIONARY
            | M_INTERMEDIATE_HALFTONE_REGION
            | M_IMMEDIATE_HALFTONE_REGION
            | M_IMMEDIATE_LOSSLESS_HALFTONE_REGION
            | M_INTERMEDIATE_GENERIC_REGION
            | M_IMMEDIATE_GENERIC_REGION
            | M_IMMEDIATE_LOSSLESS_GENERIC_REGION
            | M_INTERMEDIATE_GENERIC_REFINEMENT_REGION
            | M_IMMEDIATE_GENERIC_REFINEMENT_REGION
            | M_IMMEDIATE_LOSSLESS_GENERIC_REFINEMENT_REGION => {}
            M_PAGE_INFORMATION => sip.pageinfoflag = true,
            M_END_OF_PAGE => sip.endofpageflag = true,
            M_END_OF_STRIPE => sip.endofstripeflag = true,
            M_END_OF_FILE => sip.endoffileflag = true,
            M_PROFILES | M_TABLES | M_EXTENSION => {}
            _ => self.pdftex_fail("checkseghdrflags(): unknown segment type in JBIG2 image file"),
        }
    }

    /// `readseghdr`: a segment header, on the first reading of the file;
    /// false at the end of a sequential file.
    fn readseghdr(&mut self, fip: &FileInfo, f: &mut CFile, sip: &mut SegInfo) -> bool {
        sip.hdrstart = f.tell();
        if fip.sequentialaccess && sip.hdrstart == fip.filesize {
            return false; // no endoffileflag is ok for sequentialaccess
        }
        // 7.2.2 Segment number
        sip.segnum = self.jb_read4bytes(f);
        // 7.2.3 Segment header flags
        sip.seghdrflags = self.ygetc(f);
        self.checkseghdrflags(sip);
        if fip.sequentialaccess && sip.endoffileflag {
            // accept shorter segment, makes it compliant with Example 3.4
            // of PDFRef. 5th ed.
            return true;
        }
        sip.pageassocsizeflag = (sip.seghdrflags >> 6) & 0x01 != 0;
        // 7.2.4 Referred-to segment count and retention flags
        sip.reftosegcount = self.ygetc(f);
        sip.countofrefered = sip.reftosegcount >> 5;
        if sip.countofrefered < 5 {
            sip.fieldlen = 1;
        } else {
            sip.fieldlen = 5 + sip.countofrefered / 8;
            self.xfseek(f, sip.fieldlen as i64 - 1, Whence::Cur, &fip.filename);
        }
        // 7.2.5 Referred-to segment numbers
        sip.segnumwidth = if sip.segnum <= 256 {
            1
        } else if sip.segnum <= 65536 {
            2
        } else {
            4
        };
        for _ in 0..sip.countofrefered {
            match sip.segnumwidth {
                1 => {
                    self.ygetc(f);
                }
                2 => {
                    self.jb_read2bytes(f);
                }
                _ => {
                    self.jb_read4bytes(f);
                }
            }
        }
        // 7.2.6 Segment page association
        sip.segpage = if sip.pageassocsizeflag {
            self.jb_read4bytes(f) as i64
        } else {
            self.ygetc(f) as i64
        };
        // 7.2.7 Segment data length
        sip.segdatalen = self.jb_read4bytes(f);
        sip.hdrend = f.tell();
        // ---- at end of segment header ----
        true
    }

    /// `writeseghdr`: a segment header into the PDF, marking the page-0
    /// segments it refers to and setting a page association > 0 to 1.
    fn writeseghdr(&mut self, fip: &mut FileInfo, f: &mut CFile, sip: &mut SegInfo) {
        // 7.2.2 Segment number, 7.2.3 Segment header flags, 7.2.4
        // Referred-to segment count and retention flags
        for _ in 0..5 + sip.fieldlen {
            let c = self.ygetc(f);
            self.c_pdf_out(c as u8);
        }
        // 7.2.5 Referred-to segment numbers
        for _ in 0..sip.countofrefered {
            let referedseg: u64 = match sip.segnumwidth {
                1 => {
                    let r = self.ygetc(f) as u64;
                    self.c_pdf_out(r as u8);
                    r
                }
                2 => {
                    let r = self.jb_read2bytes(f) as u64;
                    self.c_pdf_out(((r >> 8) & 0xff) as u8);
                    self.c_pdf_out((r & 0xff) as u8);
                    r
                }
                _ => {
                    let r = self.jb_read4bytes(f);
                    self.c_pdf_out(((r >> 24) & 0xff) as u8);
                    self.c_pdf_out(((r >> 16) & 0xff) as u8);
                    self.c_pdf_out(((r >> 8) & 0xff) as u8);
                    self.c_pdf_out((r & 0xff) as u8);
                    r
                }
            };
            if fip.page0.is_some() && !sip.refers {
                self.markpage0seg(fip, f, referedseg);
            }
        }
        if sip.countofrefered > 0 {
            sip.refers = true;
        }
        // 7.2.6 Segment page association
        if sip.pageassocsizeflag {
            for _ in 0..3 {
                self.ygetc(f);
                self.c_pdf_out(0);
            }
        }
        self.ygetc(f);
        self.c_pdf_out(if sip.segpage > 0 { 1 } else { 0 });
        // 7.2.7 Segment data length
        for _ in 0..4 {
            let c = self.ygetc(f);
            self.c_pdf_out(c as u8);
        }
    }

    /// `checkseghdr`: for recursive marking of referred page-0 segments.
    fn checkseghdr(&mut self, fip: &mut FileInfo, f: &mut CFile, sip: &mut SegInfo) {
        let name = fip.filename.clone();
        // 7.2.2 Segment number, 7.2.3 Segment header flags, 7.2.4
        // Referred-to segment count and retention flags
        self.xfseek(f, sip.fieldlen as i64 + 5, Whence::Cur, &name);
        // 7.2.5 Referred-to segment numbers
        for _ in 0..sip.countofrefered {
            let referedseg = match sip.segnumwidth {
                1 => self.ygetc(f) as u64,
                2 => self.jb_read2bytes(f) as u64,
                _ => self.jb_read4bytes(f),
            };
            if !sip.refers {
                self.markpage0seg(fip, f, referedseg);
            }
        }
        if sip.countofrefered > 0 {
            sip.refers = true;
        }
        // 7.2.6 Segment page association, 7.2.7 Segment data length
        let skip = if sip.pageassocsizeflag { 8 } else { 5 };
        self.xfseek(f, skip, Whence::Cur, &name);
    }

    /// `markpage0seg`: mark page-0 segment `referedseg` as referred to,
    /// and (once) the segments it refers to. Like C, this reads the
    /// references from the file's current position. A reference cycle,
    /// which recurses without end in C, stops the run.
    fn markpage0seg(&mut self, fip: &mut FileInfo, f: &mut CFile, referedseg: u64) {
        let Some(i) = fip
            .page0
            .as_ref()
            .and_then(|p| p.seg_tree.get(&referedseg).copied())
        else {
            return;
        };
        let mut sip = fip.page0.as_ref().unwrap().segments[i].clone();
        if !sip.refers && sip.countofrefered > 0 {
            fip.depth += 1;
            if fip.depth > 1_000 {
                self.pdftex_fail("markpage0seg(): segment references loop in JBIG2 image file");
            }
            self.checkseghdr(fip, f, &mut sip);
            fip.depth -= 1;
        }
        // C works on the segment itself: merge what happened to the copy
        // (flags only ever go from false to true)
        let cur = &mut fip.page0.as_mut().unwrap().segments[i];
        cur.refers |= sip.refers;
        cur.isrefered = true;
    }

    /// `findstreamstart`: the start of the data of a random-access file
    /// (Annex D.2), after the end-of-file segment header.
    fn findstreamstart(&mut self, fip: &mut FileInfo, f: &mut CFile) -> i64 {
        let mut tmp = SegInfo::default();
        loop {
            // find random-access stream start
            self.readseghdr(fip, f, &mut tmp);
            if tmp.endoffileflag {
                break;
            }
        }
        fip.streamstart = tmp.hdrend;
        self.readfilehdr(fip, f);
        fip.streamstart
    }

    /// `rd_jbig2_info`: the pages and segments of the file.
    fn rd_jbig2_info(&mut self, fip: &mut FileInfo) {
        let name = fip.filename.clone();
        let Some(mut f) = CFile::open(&name) else {
            self.fatal_perror(&name);
        };
        let mut streampos: i64 = 0; // for random-access only
        let mut currentpage: u64 = 0;
        self.readfilehdr(fip, &mut f);
        if !fip.sequentialaccess {
            // D.2 Random-access organisation
            streampos = self.findstreamstart(fip, &mut f);
        }
        loop {
            // loop over segments
            let mut sip = SegInfo::default();
            if !self.readseghdr(fip, &mut f, &mut sip) || sip.endoffileflag {
                break;
            }
            let on_page = sip.segpage > 0;
            if on_page {
                if sip.segpage as u64 > currentpage {
                    fip.pages.push(PageInfo::default());
                    currentpage = sip.segpage as u64;
                }
                if fip.pages.is_empty() {
                    // C reads `fip->pages.last->d` of an empty list here
                    self.pdftex_fail(
                        "read_jbig2_info(): page association out of order in JBIG2 image file",
                    );
                }
            } else if fip.page0.is_none() {
                fip.page0 = Some(PageInfo::default());
            }
            if !fip.sequentialaccess {
                sip.datastart = streampos;
            } else {
                sip.datastart = sip.hdrend;
            }
            sip.dataend = sip.datastart.wrapping_add(sip.segdatalen as i64);
            if !fip.sequentialaccess && (sip.pageinfoflag || sip.endofstripeflag) {
                self.xfseek(&mut f, sip.datastart, Whence::Set, &name);
            }
            let mut seekdist = sip.segdatalen as i64;
            let (pageinfo, endofstripe) = (sip.pageinfoflag, sip.endofstripeflag);
            let (mut info, mut striped) = (None, None);
            // 7.4.8 Page information segment syntax
            if pageinfo {
                let w = self.jb_read4bytes(&mut f) as u32;
                let h = self.jb_read4bytes(&mut f) as u32;
                let xr = self.jb_read4bytes(&mut f) as u32;
                let yr = self.jb_read4bytes(&mut f) as u32;
                let fl = self.ygetc(&mut f);
                // 7.4.8.6 Page striping information
                let si = self.jb_read2bytes(&mut f);
                info = Some((w, h, xr, yr, fl, si));
                seekdist -= 19;
            }
            if endofstripe {
                striped = Some(self.jb_read4bytes(&mut f) as u32);
                seekdist -= 4;
            }
            if !fip.sequentialaccess && (pageinfo || endofstripe) {
                self.xfseek(&mut f, sip.hdrend, Whence::Set, &name);
            }
            if !fip.sequentialaccess {
                streampos = streampos.wrapping_add(sip.segdatalen as i64);
            }
            if fip.sequentialaccess {
                self.xfseek(&mut f, seekdist, Whence::Cur, &name);
            }
            let segpage = sip.segpage as u64;
            let endofpage = sip.endofpageflag;
            let pip = if on_page {
                fip.pages.last_mut().unwrap()
            } else {
                fip.page0.as_mut().unwrap()
            };
            if let Some((w, h, xr, yr, fl, si)) = info {
                pip.pagenum = segpage;
                pip.width = w;
                pip.height = h;
                pip.xres = xr;
                pip.yres = yr;
                pip.pagesegmentflags = fl;
                pip.stripinginfo = si;
            }
            if let Some(sh) = striped {
                pip.stripedheight = sh;
            }
            if endofpage && currentpage != 0 && (pip.stripinginfo >> 15) != 0 {
                pip.height = pip.stripedheight;
            }
            if !endofpage {
                pip.segments.push(sip);
            }
        }
        fip.phase = Phase::HaveInfo;
    }

    /// Copy one segment (header, then data) into the PDF.
    fn wr_jbig2_segment(&mut self, fip: &mut FileInfo, f: &mut CFile, sip: &mut SegInfo) {
        let name = fip.filename.clone();
        self.xfseek(f, sip.hdrstart, Whence::Set, &name);
        // mark refered-to page 0 segments, change segpages > 1 to 1
        self.writeseghdr(fip, f, sip);
        self.xfseek(f, sip.datastart, Whence::Set, &name);
        let mut i = sip.datastart;
        while i < sip.dataend {
            let c = self.ygetc(f);
            self.c_pdf_out(c as u8);
            i += 1;
        }
    }

    /// `wr_jbig2`: page `page`'s XObject (after the dictionary pdftex.web
    /// has begun), or for page 0 the file's `/JBIG2Globals` object.
    fn wr_jbig2(&mut self, fip: &mut FileInfo, page: u64) {
        let name = fip.filename.clone();
        let slot = if page > 0 {
            let Some(&i) = fip.page_tree.get(&page) else {
                self.pdftex_fail(&format!(
                    "read_jbig2_info(): page {page} not found in JBIG2 image file"
                ));
            };
            let pi = &fip.pages[i];
            let (w, h) = (pi.width as i32, pi.height as i32);
            let len = getstreamlen(&pi.segments, true);
            self.pdf_puts(b"/Type /XObject\n");
            self.pdf_puts(b"/Subtype /Image\n");
            self.pdf_printf(format!("/Width {w}\n").as_bytes());
            self.pdf_printf(format!("/Height {h}\n").as_bytes());
            self.pdf_puts(b"/ColorSpace /DeviceGray\n");
            self.pdf_puts(b"/BitsPerComponent 1\n");
            self.pdf_printf(format!("/Length {len}\n").as_bytes());
            self.pdf_puts(b"/Filter [/JBIG2Decode]\n");
            if fip.page0.is_some() {
                if fip.pdfpage0objnum == 0 {
                    self.pdf_create_obj(0, 0);
                    fip.pdfpage0objnum = self.obj_ptr as u64;
                }
                self.pdf_printf(
                    format!(
                        "/DecodeParms [<< /JBIG2Globals {} 0 R >>]\n",
                        fip.pdfpage0objnum
                    )
                    .as_bytes(),
                );
            }
            Some(i)
        } else {
            let len = getstreamlen(&fip.page0.as_ref().unwrap().segments, false);
            self.pdf_begin_dict(fip.pdfpage0objnum as i32, 0);
            self.pdf_printf(format!("/Length {len}\n").as_bytes());
            None
        };
        self.pdf_puts(b">>\n");
        self.pdf_puts(b"stream\n");
        let Some(mut f) = CFile::open(&name) else {
            self.fatal_perror(&name);
        };
        match slot {
            Some(i) => {
                // loop over page segments (page 0's are marked meanwhile)
                let mut segs = std::mem::take(&mut fip.pages[i].segments);
                for sip in segs.iter_mut() {
                    self.wr_jbig2_segment(fip, &mut f, sip);
                }
                fip.pages[i].segments = segs;
            }
            None => {
                // page 0: the referred segments, as marked when reached
                let n = fip.page0.as_ref().unwrap().segments.len();
                for k in 0..n {
                    let mut sip = fip.page0.as_ref().unwrap().segments[k].clone();
                    if sip.isrefered {
                        self.wr_jbig2_segment(fip, &mut f, &mut sip);
                        let cur = &mut fip.page0.as_mut().unwrap().segments[k];
                        cur.refers |= sip.refers;
                    }
                }
            }
        }
        self.pdf_end_stream();
    }

    /// `read_jbig2_info`.
    pub(crate) fn read_jbig2_info(&mut self, st: &mut State, e: &mut ImageEntry) {
        let selected_page = match &e.data {
            ImageData::Jbig2(j) => j.selected_page,
            _ => 0,
        };
        if selected_page < 1 {
            self.pdftex_fail(&format!(
                "read_jbig2_info(): page {selected_page} not in JBIG2 image file; page must be > 0"
            ));
        }
        let name = e.name.clone().unwrap_or_default();
        let mut fip = st.files.remove(&name).unwrap_or_else(|| FileInfo {
            filename: name.clone(),
            ..Default::default()
        });
        if fip.phase == Phase::Initial {
            self.rd_jbig2_info(&mut fip);
            // pages_maketree: the first page of each number
            for (i, p) in fip.pages.iter().enumerate() {
                fip.page_tree.entry(p.pagenum).or_insert(i);
            }
            if let Some(p0) = fip.page0.as_mut() {
                for (i, s) in p0.segments.iter().enumerate() {
                    p0.seg_tree.entry(s.segnum).or_insert(i);
                }
            }
        }
        let Some(&i) = fip.page_tree.get(&(selected_page as u64)) else {
            st.files.insert(name, fip);
            self.pdftex_fail(&format!(
                "read_jbig2_info(): page {selected_page} not found in JBIG2 image file"
            ));
        };
        let pip = &fip.pages[i];
        e.num_pages = fip.numofpages as i32;
        e.width = pip.width as i32;
        e.height = pip.height as i32;
        e.x_res = (pip.xres as f64 * 0.0254 + 0.5) as i32;
        e.y_res = (pip.yres as f64 * 0.0254 + 0.5) as i32;
        st.files.insert(name, fip);
    }

    /// `write_jbig2`.
    pub(crate) fn write_jbig2(&mut self, st: &mut State, e: &mut ImageEntry) {
        let selected_page = match &e.data {
            ImageData::Jbig2(j) => j.selected_page,
            _ => self.pdftex_fail("unknown type of image"),
        };
        let name = e.name.clone().unwrap_or_default();
        let Some(mut fip) = st.files.remove(&name) else {
            self.pdftex_fail("unknown type of image");
        };
        let pagenum = fip
            .page_tree
            .get(&(selected_page as u64))
            .map(|&i| fip.pages[i].pagenum);
        match pagenum {
            Some(p) => self.wr_jbig2(&mut fip, p),
            None => self.pdftex_fail(&format!(
                "read_jbig2_info(): page {selected_page} not found in JBIG2 image file"
            )),
        }
        st.files.insert(name, fip);
    }

    /// `flushjbig2page0objects`: the `/JBIG2Globals` of every file that
    /// has page-0 segments, in file-name order.
    pub fn flush_jbig2_page0_objects(&mut self) {
        self.with_images(|g, st| {
            let names: Vec<Vec<u8>> = st.jbig2.files.keys().cloned().collect();
            for n in names {
                let mut fip = st.jbig2.files.remove(&n).unwrap();
                if fip.page0.is_some() {
                    g.wr_jbig2(&mut fip, 0);
                }
                st.jbig2.files.insert(n, fip);
            }
        });
    }
}

/// `getstreamlen`: the bytes of the segments written (all of a page's, or
/// the referred ones of page 0).
fn getstreamlen(segs: &[SegInfo], refer: bool) -> i64 {
    let mut len: i64 = 0;
    for sip in segs {
        if refer || sip.isrefered {
            len += (sip.hdrend - sip.hdrstart) + (sip.dataend - sip.datastart);
        }
    }
    len
}
