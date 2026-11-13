// Pitex-authored preview publication dedup. Copyright (c) 2026 Pitex contributors.
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Decide whether a preview publication would rewrite bytes already on disk.
//!
//! The helper publishes by creating `pN/` and renaming it into place. Readers
//! keep that directory until they `release` it. A later pass whose PDF, log,
//! and SyncTeX inputs match that directory must not write those bytes again.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    pub pdf: u64,
    pub log: u64,
    pub sync: u64,
    pub pages: i32,
    pub has_sync: bool,
}

#[derive(Clone, Debug)]
pub struct Kept {
    pub seq: i32,
    pub complete: bool,
    pub stamp: Stamp,
    pub pdf: PathBuf,
    pub log: PathBuf,
    pub sync: PathBuf,
}

pub enum Plan<'a> {
    /// The kept directory already holds this output. `idle_seq` is set when
    /// the caller should tell the client that this complete generation is the
    /// kept publication. An intermediate duplicate emits nothing.
    Unchanged { idle_seq: Option<i32> },
    Store {
        pdf_reuse: Option<&'a Path>,
        log_reuse: Option<&'a Path>,
        sync_reuse: Option<&'a Path>,
    },
}

pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn stamp(pdf: &[u8], log: &[u8], sync: Option<(&[u8], i32)>) -> Stamp {
    match sync {
        Some((bytes, pages)) => Stamp {
            pdf: fnv1a64(pdf),
            log: fnv1a64(log),
            sync: fnv1a64(bytes),
            pages,
            has_sync: true,
        },
        None => Stamp {
            pdf: fnv1a64(pdf),
            log: fnv1a64(log),
            sync: 0,
            pages: 0,
            has_sync: false,
        },
    }
}

pub fn kept() -> &'static Mutex<Option<Kept>> {
    static KEPT: Mutex<Option<Kept>> = Mutex::new(None);
    &KEPT
}

pub fn plan<'a>(stamp: Stamp, complete: bool, alive: bool, kept: Option<&'a Kept>) -> Plan<'a> {
    let Some(kept) = kept.filter(|_| alive) else {
        return Plan::Store {
            pdf_reuse: None,
            log_reuse: None,
            sync_reuse: None,
        };
    };
    let pdf_same = stamp.pdf == kept.stamp.pdf;
    let log_same = stamp.log == kept.stamp.log;
    let sync_same = stamp.has_sync == kept.stamp.has_sync
        && (!stamp.has_sync || (stamp.sync == kept.stamp.sync && stamp.pages == kept.stamp.pages));
    if pdf_same && log_same && sync_same && (!complete || kept.complete) {
        return Plan::Unchanged {
            idle_seq: if complete { Some(kept.seq) } else { None },
        };
    }
    Plan::Store {
        pdf_reuse: pdf_same.then_some(kept.pdf.as_path()),
        log_reuse: log_same.then_some(kept.log.as_path()),
        sync_reuse: (sync_same && stamp.has_sync).then_some(kept.sync.as_path()),
    }
}

/// Write `bytes` into `directory/name`, or hard-link `reuse` when it is the
/// same content. The returned count is bytes passed to the write syscall.
/// A hard link returns 0. New files are mode 0600, matching `write_file`.
pub fn place_artifact(
    directory: &Path,
    name: &str,
    bytes: &[u8],
    reuse: Option<&Path>,
) -> std::io::Result<u64> {
    let dest = directory.join(name);
    if let Some(source) = reuse {
        if std::fs::hard_link(source, &dest).is_ok() {
            return Ok(0);
        }
    }
    let mut file = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&dest)?
        }
        #[cfg(not(unix))]
        {
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&dest)?
        }
    };
    file.write_all(bytes)?;
    Ok(bytes.len() as u64)
}

pub fn link_or_false(source: &Path, dest: &Path) -> bool {
    std::fs::hard_link(source, dest).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(complete: bool) -> Kept {
        Kept {
            seq: 3,
            complete,
            stamp: stamp(b"%PDF-1", b"log", Some((b"sync", 1))),
            pdf: PathBuf::from("/out/p3/main.pdf"),
            log: PathBuf::from("/out/p3/main.log"),
            sync: PathBuf::from("/out/p3/main.synctex"),
        }
    }

    #[test]
    fn identical_intermediate_is_silent() {
        let kept = sample(false);
        let again = stamp(b"%PDF-1", b"log", Some((b"sync", 1)));
        match plan(again, false, true, Some(&kept)) {
            Plan::Unchanged { idle_seq: None } => {}
            Plan::Unchanged { idle_seq: Some(_) } => panic!("an intermediate duplicate must not emit idle"),
            Plan::Store { .. } => panic!("identical intermediate planned a write"),
        }
    }

    #[test]
    fn complete_pass_over_incomplete_reuses_the_files() {
        let kept = sample(false);
        let again = stamp(b"%PDF-1", b"log", Some((b"sync", 1)));
        match plan(again, true, true, Some(&kept)) {
            Plan::Store {
                pdf_reuse,
                log_reuse,
                sync_reuse,
            } => {
                assert!(pdf_reuse.is_some() && log_reuse.is_some() && sync_reuse.is_some());
            }
            Plan::Unchanged { .. } => panic!("a complete pass still needs a published event"),
        }
    }

    #[test]
    fn identical_complete_pass_does_not_store() {
        let kept = sample(true);
        let again = stamp(b"%PDF-1", b"log", Some((b"sync", 1)));
        match plan(again, true, true, Some(&kept)) {
            Plan::Unchanged { idle_seq } => assert_eq!(idle_seq, Some(3)),
            Plan::Store { .. } => panic!("identical output planned a write"),
        }
    }

    #[test]
    fn released_publication_is_written_again() {
        let kept = sample(true);
        let again = stamp(b"%PDF-1", b"log", Some((b"sync", 1)));
        match plan(again, true, false, Some(&kept)) {
            Plan::Store {
                pdf_reuse,
                log_reuse,
                sync_reuse,
            } => {
                assert!(pdf_reuse.is_none() && log_reuse.is_none() && sync_reuse.is_none());
            }
            Plan::Unchanged { .. } => panic!("a released directory cannot be reused"),
        }
    }

    #[test]
    fn changed_pdf_is_stored_and_unchanged_log_is_linked() {
        let kept = sample(true);
        let edited = stamp(b"%PDF-2", b"log", Some((b"sync", 1)));
        match plan(edited, true, true, Some(&kept)) {
            Plan::Store {
                pdf_reuse,
                log_reuse,
                sync_reuse,
            } => {
                assert!(pdf_reuse.is_none());
                assert_eq!(log_reuse.map(Path::to_path_buf), Some(kept.log.clone()));
                assert_eq!(sync_reuse.map(Path::to_path_buf), Some(kept.sync.clone()));
            }
            Plan::Unchanged { .. } => panic!("a new pdf was treated as unchanged"),
        }
    }

    #[test]
    fn place_artifact_reports_zero_when_the_inode_is_linked() {
        let root = std::env::temp_dir().join(format!("pitex-place-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("p1")).unwrap();
        std::fs::create_dir_all(root.join("p2")).unwrap();
        let source = root.join("p1/main.pdf");
        std::fs::write(&source, b"%PDF-same").unwrap();
        let wrote = place_artifact(root.join("p2").as_path(), "main.pdf", b"%PDF-same", Some(&source)).unwrap();
        assert_eq!(wrote, 0);
        assert_eq!(std::fs::read(root.join("p2/main.pdf")).unwrap(), b"%PDF-same");
        let fresh = place_artifact(root.join("p2").as_path(), "main.log", b"log-bytes", None).unwrap();
        assert_eq!(fresh, b"log-bytes".len() as u64);
        let _ = std::fs::remove_dir_all(&root);
    }
}
