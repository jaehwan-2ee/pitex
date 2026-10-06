//! Recover source identities when a Windows TeX build writes its ANSI cwd
//! to SyncTeX. Aliases come only from known snapshot files, never wildcards.
#[path = "../../../Linux/crates/synctex-core/src/anchors.rs"]
mod anchors;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(windows)]
fn ansi_bytes(text: &str) -> Option<Vec<u8>> {
    use windows_sys::Win32::Globalization::{WideCharToMultiByte, CP_ACP};
    let wide: Vec<_> = text.encode_utf16().collect();
    let len = i32::try_from(wide.len()).ok()?;
    let size = unsafe {
        WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            len,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    if size == 0 {
        return None;
    }
    let mut bytes = vec![0; size as usize];
    let written = unsafe {
        WideCharToMultiByte(
            CP_ACP,
            0,
            wide.as_ptr(),
            len,
            bytes.as_mut_ptr(),
            size,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
    };
    (written == size).then_some(bytes)
}

#[cfg(windows)]
fn ansi_text(bytes: &[u8]) -> Option<String> {
    use windows_sys::Win32::Globalization::{MultiByteToWideChar, CP_ACP};
    if bytes.is_empty() {
        return Some(String::new());
    }
    let len = i32::try_from(bytes.len()).ok()?;
    let size =
        unsafe { MultiByteToWideChar(CP_ACP, 0, bytes.as_ptr(), len, std::ptr::null_mut(), 0) };
    if size == 0 {
        return None;
    }
    let mut wide = vec![0; size as usize];
    let written =
        unsafe { MultiByteToWideChar(CP_ACP, 0, bytes.as_ptr(), len, wide.as_mut_ptr(), size) };
    (written == size).then(|| String::from_utf16_lossy(&wide))
}

pub fn decode(bytes: Vec<u8>) -> io::Result<String> {
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(error) => {
            #[cfg(windows)]
            if let Some(text) = ansi_text(error.as_bytes()) {
                return Ok(text);
            }
            Err(io::Error::new(io::ErrorKind::InvalidData, error))
        }
    }
}

pub fn legacy_path(path: &Path) -> Option<String> {
    #[cfg(windows)]
    {
        ansi_text(&ansi_bytes(&path.to_string_lossy())?)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

fn ansi_utf8_path(path: &Path) -> Option<String> {
    #[cfg(windows)]
    {
        String::from_utf8(ansi_bytes(&path.to_string_lossy())?).ok()
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

fn utf8_ansi_path(path: &Path) -> Option<String> {
    #[cfg(windows)]
    {
        // A different ANSI Input can make the entire file require ACP
        // decoding, including records which were originally UTF-8.
        ansi_text(path.to_string_lossy().as_bytes())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

fn mixed_paths(work: &Path, relative: &Path) -> Vec<String> {
    #[cfg(windows)]
    {
        // Some engines concatenate an ANSI getcwd() with the UTF-8 input
        // filename. If another Input has invalid UTF-8, the byte decoder
        // uses ACP for the whole file; reproduce that exact mixed alias.
        let Some(mut bytes) = ansi_bytes(&work.to_string_lossy()) else {
            return Vec::new();
        };
        bytes.push(b'/');
        bytes.extend_from_slice(relative.to_string_lossy().replace('\\', "/").as_bytes());
        let mut aliases = Vec::new();
        if let Some(text) = ansi_text(&bytes) {
            aliases.push(text);
        }
        if let Ok(text) = String::from_utf8(bytes) {
            aliases.push(text);
        }
        aliases
    }
    #[cfg(not(windows))]
    {
        let _ = (work, relative);
        Vec::new()
    }
}

fn normalized(path: &str) -> Option<String> {
    let slash = path.trim_matches('"').replace('\\', "/");
    let mut components = Vec::new();
    for part in slash.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            _ => components.push(part),
        }
    }
    let prefix = if slash.starts_with("//") {
        "//"
    } else if slash.starts_with('/') {
        "/"
    } else {
        ""
    };
    let normalized = format!("{prefix}{}", components.join("/"));
    Some(if cfg!(windows) {
        normalized.to_lowercase()
    } else {
        normalized
    })
}

fn insert_alias(aliases: &mut HashMap<String, Option<PathBuf>>, key: String, relative: &Path) {
    let entry = aliases
        .entry(key)
        .or_insert_with(|| Some(relative.to_path_buf()));
    if entry.as_deref() != Some(relative) {
        *entry = None;
    }
}

pub fn rewrite<'a>(
    text: &str,
    work: &Path,
    root: &Path,
    files: impl IntoIterator<Item = &'a Path>,
    legacy: impl Fn(&Path) -> Option<String>,
) -> Option<String> {
    let mut aliases = HashMap::new();
    let legacy_work = legacy(work);
    for relative in files {
        let source = work.join(relative);
        insert_alias(
            &mut aliases,
            normalized(&source.to_string_lossy())?,
            relative,
        );
        if let Some(path) = legacy(&source) {
            insert_alias(&mut aliases, normalized(&path)?, relative);
        }
        // Native ANSI bytes can accidentally form valid UTF-8 (CP1252
        // "Â©" becomes UTF-8 "©"). Account for both interpretations before
        // choosing a source, even when the decoded spelling names a file.
        if let Some(path) = ansi_utf8_path(&source) {
            insert_alias(&mut aliases, normalized(&path)?, relative);
        }
        if let Some(path) = utf8_ansi_path(&source) {
            insert_alias(&mut aliases, normalized(&path)?, relative);
        }
        if let Some(prefix) = legacy_work.as_ref() {
            // ANSI cwd plus a still-valid UTF-8 relative source name.
            let hybrid = Path::new(prefix).join(relative);
            insert_alias(
                &mut aliases,
                normalized(&hybrid.to_string_lossy())?,
                relative,
            );
        }
        for path in mixed_paths(work, relative) {
            insert_alias(&mut aliases, normalized(&path)?, relative);
        }
    }
    let mut prefixes = vec![normalized(&work.to_string_lossy())?];
    if let Some(path) = legacy_work {
        prefixes.push(normalized(&path)?);
    }
    if let Some(path) = ansi_utf8_path(work) {
        prefixes.push(normalized(&path)?);
    }
    if let Some(path) = utf8_ansi_path(work) {
        prefixes.push(normalized(&path)?);
    }
    let mut rewritten = String::with_capacity(text.len());
    for line in text.lines() {
        if let Some((tag, path)) = line.strip_prefix("Input:").and_then(|s| s.split_once(':')) {
            let supplied = PathBuf::from(path.trim_matches('"'));
            let absolute = if supplied.is_absolute() {
                supplied
            } else {
                work.join(supplied)
            };
            let key = normalized(&absolute.to_string_lossy())?;
            match aliases.get(&key) {
                Some(Some(relative)) => rewritten.push_str(&format!(
                    "Input:{tag}:{}",
                    root.join(relative).to_string_lossy().replace('\\', "/")
                )),
                // An ANSI alias can identify several source names. Returning
                // no SyncTeX is safer than attaching coordinates to one of them.
                Some(None) => return None,
                None if prefixes
                    .iter()
                    .any(|prefix| key.starts_with(&format!("{prefix}/"))) =>
                {
                    return None
                }
                None => rewritten.push_str(line), // distribution/auxiliary input
            }
        } else {
            rewritten.push_str(line);
        }
        rewritten.push('\n');
    }
    Some(anchors::recount_anchors(&rewritten))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lossy(path: &Path) -> Option<String> {
        Some(
            path.to_string_lossy()
                .chars()
                .map(|c| if c.is_ascii() { c } else { '?' })
                .collect(),
        )
    }

    #[test]
    fn mangled_snapshot_prefix_and_dot_components_recover_original_unicode_paths() {
        let work = Path::new("/private/미리보기/sources");
        let root = Path::new("/project/논문");
        let text = "SyncTeX Version:1\nInput:1:/private/????/sources/./main.tex\nInput:2:/TeX/texmf/article.cls\nInput:3:/private/????/sources/./chapter.tex\nContent:\n";
        let result = rewrite(
            text,
            work,
            root,
            [Path::new("main.tex"), Path::new("chapter.tex")],
            lossy,
        )
        .unwrap();
        assert!(result.contains("Input:1:/project/논문/main.tex"));
        assert!(result.contains("Input:3:/project/논문/chapter.tex"));
        assert!(result.contains("Input:2:/TeX/texmf/article.cls"));
        assert!(!result.contains("/sources/"));
    }

    #[test]
    fn ambiguous_ansi_filenames_disable_synctex_while_exact_unicode_names_work() {
        let work = Path::new("/private/미리보기/sources");
        let root = Path::new("/project/논문");
        let files = [Path::new("가.tex"), Path::new("나.tex")];
        assert!(rewrite(
            "Input:1:/private/????/sources/?.tex\n",
            work,
            root,
            files,
            lossy
        )
        .is_none());
        let result = rewrite(
            "Input:1:/private/미리보기/sources/가.tex\n",
            work,
            root,
            files,
            lossy,
        )
        .unwrap();
        assert_eq!(result, "Input:1:/project/논문/가.tex\n");
        let hybrid = rewrite(
            "Input:1:/private/????/sources/가.tex\n",
            work,
            root,
            files,
            lossy,
        )
        .unwrap();
        assert_eq!(hybrid, "Input:1:/project/논문/가.tex\n");
    }

    #[test]
    fn best_fit_alias_colliding_with_literal_ascii_name_disables_synctex() {
        let work = Path::new("/private/sources");
        let root = Path::new("/project");
        let files = [Path::new("α.tex"), Path::new("a.tex")];
        let best_fit = |path: &Path| Some(path.to_string_lossy().replace('α', "a"));
        assert!(rewrite(
            "Input:1:/private/sources/a.tex\n",
            work,
            root,
            files,
            best_fit
        )
        .is_none());
    }

    #[cfg(windows)]
    #[test]
    fn native_ansi_bytes_decode_and_mixed_utf8_source_name_recover_identity() {
        let line = "Input:1:C:/café/main.tex\n";
        assert_eq!(decode(ansi_bytes(line).unwrap()).unwrap(), line);
        let work = Path::new("C:/café/sources");
        let root = Path::new("C:/논문");
        let relative = Path::new("가.tex");
        let mut raw = ansi_bytes(&work.to_string_lossy()).unwrap();
        raw.extend_from_slice(b"/./");
        raw.extend_from_slice(relative.to_string_lossy().as_bytes());
        let mut record = b"Input:1:".to_vec();
        record.extend_from_slice(&raw);
        record.push(b'\n');
        let text = decode(record).unwrap();
        let rewritten = rewrite(&text, work, root, [relative], legacy_path).unwrap();
        assert_eq!(rewritten, "Input:1:C:/논문/가.tex\n");
    }

    #[cfg(windows)]
    #[test]
    fn ansi_bytes_that_are_valid_utf8_cannot_bind_to_a_different_existing_file() {
        use windows_sys::Win32::Globalization::GetACP;
        if unsafe { GetACP() } != 1252 {
            return;
        }
        let work = Path::new("C:/private/sources");
        let root = Path::new("C:/project");
        let files = [Path::new("Â©.tex"), Path::new("©.tex")];
        let raw = ansi_bytes("Input:1:C:/private/sources/Â©.tex\n").unwrap();
        let text = decode(raw).unwrap();
        assert_eq!(text, "Input:1:C:/private/sources/©.tex\n");
        assert!(rewrite(&text, work, root, files, legacy_path).is_none());
        let work = Path::new("C:/Â©/sources");
        let unknown = "Input:1:C:/©/sources/missing.tex\n";
        assert!(rewrite(unknown, work, root, [Path::new("main.tex")], legacy_path).is_none());
        let work = Path::new("C:/미리보기/sources");
        let relative = Path::new("가.tex");
        let mut raw = format!("Input:1:{}/{}\n", work.display(), relative.display()).into_bytes();
        raw.extend_from_slice(b"Input:2:C:/external/caf\xe9.cls\n");
        let text = decode(raw).unwrap();
        let rewritten = rewrite(&text, work, root, [relative], legacy_path).unwrap();
        assert!(rewritten.contains("Input:1:C:/project/가.tex\n"));
        assert!(rewritten.contains("Input:2:C:/external/café.cls\n"));
    }

    #[test]
    fn unknown_snapshot_inputs_and_parent_traversal_are_not_bound() {
        let work = Path::new("/private/미리보기/sources");
        let root = Path::new("/project/논문");
        let files = [Path::new("main.tex")];
        assert!(rewrite(
            "Input:1:/private/????/sources/missing.tex\n",
            work,
            root,
            files,
            lossy
        )
        .is_none());
        assert!(rewrite(
            "Input:1:/private/????/sources/../outside.tex\n",
            work,
            root,
            files,
            lossy
        )
        .is_none());
        let unrelated = "Input:1:/other/private/????/sources/main.tex\n";
        assert_eq!(
            rewrite(unrelated, work, root, files, lossy).unwrap(),
            unrelated
        );
    }

    #[test]
    fn utf8_content_roundtrips_without_codepage_conversion() {
        assert_eq!(
            decode("Input:1:/논문/café.tex\n".as_bytes().to_vec()).unwrap(),
            "Input:1:/논문/café.tex\n"
        );
    }
}
