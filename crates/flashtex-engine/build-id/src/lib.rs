//! The flashtex-engine build id: the SHA-256 of every source of the engine
//! crate that can affect what a format holds (see [`hash::DIRS`] and
//! [`hash::FILES`]: all of its Rust sources, the change files, the C shims,
//! the string pool, the capacities, its manifest and build script).
//!
//! The format cache (crates/flashtex-engine/src/formats.rs) keys formats by
//! it, so every binary built from one engine shares formats, and any change
//! to the engine's sources -- generated or hand-written -- makes new ones.
//! It is computed by this crate's build script, which reruns on every such
//! change; the engine's own build script (kpathsea, zlib and the image
//! libraries, with `cc`) does not.

pub mod hash;

/// The engine build id (hex SHA-256).
pub const ENGINE_BUILD_ID: &str = env!("FLASHTEX_ENGINE_BUILD_ID");

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn engine_dir() -> PathBuf {
        PathBuf::from(env!("FLASHTEX_ENGINE_DIR"))
    }

    #[test]
    fn the_id_is_the_hash_of_the_sources_as_they_are() {
        assert_eq!(hash::engine_sources_id(&engine_dir()), ENGINE_BUILD_ID);
        let inputs = hash::inputs(&engine_dir());
        for must in [
            "src/system.rs",
            "src/resolver.rs",
            "src/formats.rs",
            "src/generated/mod.rs",
            "src/pdftex/mod.rs",
            "changes/web2c-run.ch",
            "pdftex.pool",
            "web2rust-default.args",
            "Cargo.toml",
        ] {
            assert!(
                inputs.contains(&PathBuf::from(must)),
                "{must} is not hashed"
            );
        }
    }

    fn copy_tree(from: &Path, to: &Path) {
        for rel in hash::inputs(from) {
            let dst = to.join(&rel);
            std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
            std::fs::copy(from.join(&rel), dst).unwrap();
        }
    }

    #[test]
    fn one_byte_of_a_hand_written_engine_file_changes_the_id() {
        let t = std::env::temp_dir().join(format!("flashtex-build-id-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&t);
        copy_tree(&engine_dir(), &t);
        let before = hash::engine_sources_id(&t);
        assert_eq!(before, ENGINE_BUILD_ID, "the copy hashes as the original");
        for file in [
            "src/system.rs",
            "src/resolver.rs",
            "src/formats.rs",
            "src/pdftex/mod.rs",
        ] {
            let p = t.join(file);
            let mut data = std::fs::read(&p).unwrap();
            // A byte of code, not of a comment: the first `fn`.
            let i = data.windows(3).position(|w| w == b"fn ").unwrap();
            data[i] = b'F';
            std::fs::write(&p, &data).unwrap();
            let after = hash::engine_sources_id(&t);
            assert_ne!(after, before, "{file}");
            data[i] = b'f';
            std::fs::write(&p, &data).unwrap();
            assert_eq!(hash::engine_sources_id(&t), before);
        }
        let _ = std::fs::remove_dir_all(&t);
    }
}
