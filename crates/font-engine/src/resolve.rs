//! Bounded font-file resolution. There is no directory scanning: a
//! [`FontSearch`] holds an explicit list of directories and a lookup checks
//! `dir/<file name>` for each, in order. Callers construct the list; the
//! macOS system directories are offered as a constant, not applied silently.

use std::path::{Path, PathBuf};

use crate::{Error, FontSource, TrueTypeFace};

/// Apple-supplied font directories on macOS. Fonts there are licensed for
/// use on the device; redistributing them (including inside a PDF) is the
/// user's decision. Nothing from these directories is committed to this
/// repository.
pub const MACOS_SYSTEM_DIRS: [&str; 3] = [
    "/System/Library/Fonts",
    "/System/Library/Fonts/Supplemental",
    "/Library/Fonts",
];

#[derive(Debug, Clone, Default)]
pub struct FontSearch {
    dirs: Vec<PathBuf>,
}

impl FontSearch {
    pub fn new() -> FontSearch {
        FontSearch::default()
    }

    pub fn with_dir(mut self, dir: impl Into<PathBuf>) -> FontSearch {
        self.dirs.push(dir.into());
        self
    }

    /// The macOS system directories, in order, appended to the list.
    pub fn with_macos_system_dirs(mut self) -> FontSearch {
        self.dirs
            .extend(MACOS_SYSTEM_DIRS.iter().map(PathBuf::from));
        self
    }

    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// First `dir/file_name` that is a regular file. At most `dirs.len()`
    /// metadata calls; never lists a directory.
    pub fn find(&self, file_name: &str) -> Option<PathBuf> {
        if Path::new(file_name).components().count() != 1 {
            return None;
        }
        self.dirs
            .iter()
            .map(|d| d.join(file_name))
            .find(|p| p.is_file())
    }

    /// Finds and parses face `face_index` of `file_name`.
    pub fn load(&self, file_name: &str, face_index: u32) -> Result<TrueTypeFace, Error> {
        let path = self.find(file_name).ok_or_else(|| {
            Error::Io(format!(
                "{file_name} not found in {} search dirs",
                self.dirs.len()
            ))
        })?;
        let bytes =
            std::fs::read(&path).map_err(|e| Error::Io(format!("{}: {e}", path.display())))?;
        TrueTypeFace::parse_with_source(bytes, FontSource::File { path, face_index })
    }
}
