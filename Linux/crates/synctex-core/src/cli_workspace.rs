//! Prepare a Unicode-safe SyncTeX CLI query without executing a process.
//!
//! The Windows CLI uses narrow filename APIs. Native filesystem operations
//! stage the bound artifacts under ASCII basenames, and only generated
//! ASCII Input names reach its argv. Results restore original identities
//! before the application's revision and containment checks consume them.

use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "anchors.rs"]
mod anchors;
pub use anchors::recount_anchors;

pub const PDF_FILE: &str = "document.pdf";
static NEXT_QUERY: AtomicU64 = AtomicU64::new(0);

pub struct CliWorkspace {
    directory: PathBuf,
    original_pdf: String,
    source_root: PathBuf,
    inputs: HashMap<String, String>, // generated ASCII alias -> original recorded Input
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl CliWorkspace {
    /// Read the original companion metadata and stage a private query copy.
    /// A hardlink preserves the PDF bytes without copying large documents;
    /// filesystems which cannot link fall back to a native filesystem copy.
    pub fn prepare(pdf: &Path, source_root: &Path) -> io::Result<Self> {
        Self::prepare_in(pdf, source_root, &std::env::temp_dir())
    }

    /// A caller-owned parent permits native validation under a Unicode user
    /// profile without mutating global TEMP or sharing another query's files.
    pub fn prepare_in(pdf: &Path, source_root: &Path, query_parent: &Path) -> io::Result<Self> {
        let compressed = pdf.with_extension("synctex.gz");
        let bytes = if compressed.is_file() {
            let mut bytes = Vec::new();
            flate2::read::MultiGzDecoder::new(std::fs::File::open(compressed)?)
                .read_to_end(&mut bytes)?;
            bytes
        } else {
            std::fs::read(pdf.with_extension("synctex"))?
        };
        // Lossy decoding could collapse distinct source identities into the
        // same replacement characters. Published previews are UTF-8; refuse
        // unknown encodings rather than guessing another source's location.
        let source =
            String::from_utf8(bytes).map_err(|_| invalid("SyncTeX metadata is not UTF-8"))?;
        let mut staged = String::with_capacity(source.len());
        let mut inputs = HashMap::new();
        let mut tags = HashSet::new();
        for line in source.lines() {
            if let Some(record) = line.strip_prefix("Input:") {
                let (tag, path) = record
                    .split_once(':')
                    .ok_or_else(|| invalid("Malformed SyncTeX input record"))?;
                let number = tag
                    .parse::<u64>()
                    .ok()
                    .filter(|n| *n > 0)
                    .ok_or_else(|| invalid("Malformed SyncTeX input tag"))?;
                if !tags.insert(number) {
                    return Err(invalid("Duplicate SyncTeX input tag"));
                }
                if path.is_empty() || path.contains('\0') {
                    return Err(invalid("Malformed SyncTeX source path"));
                }
                let alias = format!("input-{number}.tex");
                staged.push_str(&format!("Input:{tag}:{alias}"));
                inputs.insert(alias, path.to_owned());
            } else {
                staged.push_str(line);
            }
            staged.push('\n');
        }
        if inputs.is_empty() {
            return Err(invalid("SyncTeX has no input records"));
        }
        let directory = private_directory_in(query_parent)?;
        let workspace = Self {
            directory,
            original_pdf: pdf.to_string_lossy().into_owned(),
            source_root: source_root.to_path_buf(),
            inputs,
        };
        let target = workspace.directory.join(PDF_FILE);
        // A read-only hardlink shares attributes with the original and
        // prevents Windows cleanup. Only an independent copy may clear them.
        if std::fs::metadata(pdf)?.permissions().readonly()
            || std::fs::hard_link(pdf, &target).is_err()
        {
            std::fs::copy(pdf, &target)?;
            #[cfg(windows)]
            {
                let mut permissions = std::fs::metadata(&target)?.permissions();
                permissions.set_readonly(false);
                std::fs::set_permissions(&target, permissions)?;
            }
        }
        std::fs::write(
            workspace.directory.join("document.synctex"),
            recount_anchors(&staged),
        )?;
        Ok(workspace)
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The full recorded identity is compared, never its basename. A source
    /// with several tags is ambiguous and must not select the first alias.
    pub fn input_alias(&self, input: &str) -> io::Result<String> {
        let requested = path_key(input);
        let rooted = rooted_key(input, &self.source_root);
        let found = self
            .inputs
            .iter()
            .filter(|(_, original)| {
                path_key(original) == requested || rooted_key(original, &self.source_root) == rooted
            })
            .map(|(alias, _)| alias)
            .collect::<Vec<_>>();
        match found.as_slice() {
            [alias] => Ok((*alias).clone()),
            [] => Err(invalid("SyncTeX has no recorded location for this source")),
            _ => Err(invalid("SyncTeX has several input tags for this source")),
        }
    }

    /// Restore only generated names inside native result records. Unexpected
    /// Input/Output identities fail instead of escaping through a fallback.
    pub fn restore(&self, stdout: &str) -> io::Result<String> {
        let mut restored = String::with_capacity(stdout.len());
        let mut in_result = false;
        for line in stdout.lines() {
            if line == "SyncTeX result begin" {
                in_result = true;
            }
            if in_result {
                if let Some(alias) = line.strip_prefix("Input:") {
                    let source = self
                        .inputs
                        .get(alias.trim_matches('"'))
                        .ok_or_else(|| invalid("Unrecognized SyncTeX query source"))?;
                    restored.push_str("Input:");
                    restored.push_str(source);
                } else if let Some(output) = line.strip_prefix("Output:") {
                    let output = output.trim_matches('"');
                    if path_key(output) != path_key(PDF_FILE)
                        && path_key(output)
                            != path_key(&self.directory.join(PDF_FILE).to_string_lossy())
                    {
                        return Err(invalid("Unrecognized SyncTeX query PDF"));
                    }
                    restored.push_str("Output:");
                    restored.push_str(&self.original_pdf);
                } else {
                    restored.push_str(line);
                }
            } else {
                restored.push_str(line);
            }
            restored.push('\n');
            if line == "SyncTeX result end" {
                in_result = false;
            }
        }
        Ok(restored)
    }
}

impl Drop for CliWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[cfg(test)]
fn private_directory() -> io::Result<PathBuf> {
    private_directory_in(&std::env::temp_dir())
}

fn private_directory_in(parent: &Path) -> io::Result<PathBuf> {
    for _ in 0..16 {
        let counter = NEXT_QUERY.fetch_add(1, Ordering::Relaxed);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let directory = parent.join(format!(
            "pitex-synctex-{}-{nonce}-{counter}",
            std::process::id()
        ));
        #[allow(unused_mut)]
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "Unable to allocate a private SyncTeX query directory",
    ))
}

fn rooted_key(path: &str, root: &Path) -> String {
    let key = path_key(path);
    let bytes = key.as_bytes();
    if key.starts_with('/') || (bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'/') {
        key
    } else {
        path_key(&format!("{}/{key}", root.to_string_lossy()))
    }
}

fn path_key(path: &str) -> String {
    let mut path = path.trim_matches('"').replace('\\', "/");
    if let Some(rest) = path.strip_prefix("//?/UNC/") {
        path = format!("//{rest}");
    } else if let Some(rest) = path.strip_prefix("//?/") {
        path = rest.to_owned();
    }
    // The strict application's parser prefixes Windows drive paths with '/'.
    let bytes = path.as_bytes();
    if bytes.len() >= 4
        && bytes[0] == b'/'
        && bytes[1].is_ascii_alphabetic()
        && bytes[2] == b':'
        && bytes[3] == b'/'
    {
        path.remove(0);
    }
    let absolute = path.starts_with('/');
    let mut components = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                components.push("..");
            } // Preserve traversal for the caller's strict checks.
            _ => components.push(component),
        }
    }
    let key = format!(
        "{}{}",
        if absolute { "/" } else { "" },
        components.join("/")
    );
    #[cfg(windows)]
    return key.to_ascii_lowercase();
    #[cfg(not(windows))]
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (PathBuf, PathBuf) {
        let root = private_directory().unwrap();
        let pdf = root.join("문서 sample.pdf");
        std::fs::write(&pdf, b"%PDF-1.4\n%%EOF").unwrap();
        (root, pdf)
    }

    #[test]
    fn aliases_keep_same_basename_sources_distinct_and_restore_originals() {
        let (root, pdf) = fixture();
        let first = root.join("첫째/chapter.tex").to_string_lossy().into_owned();
        let second = root.join("둘째/chapter.tex").to_string_lossy().into_owned();
        let original =
            format!("SyncTeX Version:1\nInput:1:{first}\nInput:2:{second}\nContent:\n!0\n");
        std::fs::write(pdf.with_extension("synctex"), &original).unwrap();
        let workspace = CliWorkspace::prepare(&pdf, &root).unwrap();
        assert_ne!(
            workspace.input_alias(&first).unwrap(),
            workspace.input_alias(&second).unwrap()
        );
        assert!(
            workspace.input_alias("chapter.tex").is_err(),
            "basename-only lookup must not guess"
        );
        let alias = workspace.input_alias(&second).unwrap();
        let restored = workspace.restore(&format!("SyncTeX result begin\nInput:{alias}\nOutput:{PDF_FILE}\nLine:1\nSyncTeX result end\n")).unwrap();
        assert!(restored.contains(&format!("Input:{second}\n")));
        assert!(restored.contains(&format!("Output:{}\n", pdf.display())));
        assert_eq!(
            std::fs::read(pdf.with_extension("synctex")).unwrap(),
            original.as_bytes()
        );
        assert!(workspace
            .restore("SyncTeX result begin\nInput:../evil.tex\nSyncTeX result end\n")
            .is_err());
        let staging = workspace.directory().to_path_buf();
        drop(workspace);
        assert!(!staging.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn separate_queries_and_ambiguous_recorded_sources() {
        let (root, pdf) = fixture();
        std::fs::write(
            pdf.with_extension("synctex"),
            "Input:1:chapter.tex\nInput:2:chapter.tex\nContent:\n",
        )
        .unwrap();
        let first = CliWorkspace::prepare(&pdf, &root).unwrap();
        let second = CliWorkspace::prepare(&pdf, &root).unwrap();
        assert_ne!(first.directory(), second.directory());
        assert!(first
            .input_alias(&root.join("chapter.tex").to_string_lossy())
            .is_err());
        drop(first);
        assert!(second.directory().is_dir());
        drop(second);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_encoding_cannot_collapse_into_a_real_replacement_character_name() {
        let (root, pdf) = fixture();
        std::fs::write(
            pdf.with_extension("synctex"),
            b"Input:1:file\xff.tex\nInput:2:file\xef\xbf\xbd.tex\n",
        )
        .unwrap();
        assert!(CliWorkspace::prepare(&pdf, &root).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn readonly_original_remains_readonly_and_query_copy_is_removed() {
        let (root, pdf) = fixture();
        std::fs::write(
            pdf.with_extension("synctex"),
            "Input:1:chapter.tex\nContent:\n",
        )
        .unwrap();
        let original_permissions = std::fs::metadata(&pdf).unwrap().permissions();
        let mut readonly = original_permissions.clone();
        readonly.set_readonly(true);
        std::fs::set_permissions(&pdf, readonly).unwrap();
        let workspace = CliWorkspace::prepare_in(&pdf, &root, &root).unwrap();
        let staging = workspace.directory().to_path_buf();
        assert!(std::fs::metadata(&pdf).unwrap().permissions().readonly());
        drop(workspace);
        assert!(!staging.exists());
        assert!(std::fs::metadata(&pdf).unwrap().permissions().readonly());
        std::fs::set_permissions(&pdf, original_permissions).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
