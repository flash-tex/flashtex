//! The oracle side: a minimal `World` written independently of the host's
//! (no confinement, no caching beyond one compile), compiling in the test
//! process with the same pinned `typst` crates. What it produces is what
//! the host must reproduce (DESIGN.md §15: Typst's own output is the
//! oracle).
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_kit::fonts::FontStore;
use typst_layout::PagedDocument;

pub struct OracleWorld {
    root: PathBuf,
    main: FileId,
    library: LazyHash<Library>,
    fonts: FontStore,
    cache: Mutex<HashMap<FileId, Bytes>>,
}

impl OracleWorld {
    pub fn new(root: &Path, main: &str, font_dir: &Path) -> OracleWorld {
        let mut fonts = FontStore::new();
        fonts.extend(typst_kit::fonts::scan(font_dir));
        OracleWorld {
            root: root.to_path_buf(),
            main: RootedPath::new(VirtualRoot::Project, VirtualPath::new(main).unwrap()).intern(),
            library: LazyHash::new(Library::builder().build()),
            fonts,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn compile(&self) -> PagedDocument {
        let r = typst::compile::<PagedDocument>(self);
        match r.output {
            Ok(d) => d,
            Err(e) => panic!(
                "oracle compile failed: {:?}",
                e.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
            ),
        }
    }
}

impl World for OracleWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        self.fonts.book()
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        let b = self.file(id)?;
        let t = std::str::from_utf8(&b).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, t.to_string()))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let mut c = self.cache.lock().unwrap();
        if let Some(b) = c.get(&id) {
            return Ok(b.clone());
        }
        let p = self.root.join(id.vpath().get_without_slash());
        let b = Bytes::new(std::fs::read(&p).map_err(|e| FileError::from_io(e, &p))?);
        c.insert(id, b.clone());
        Ok(b)
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.font(index)
    }
    fn today(&self, _: Option<Duration>) -> Option<Datetime> {
        None
    }
}
