use std::ffi::OsStr;
use std::io;
use std::path::Path;

pub fn c_arg_strings() -> Vec<Vec<u8>> {
    std::env::args_os().map(|arg| c_string_bytes(&arg)).collect()
}

pub fn c_string_bytes(arg: &OsStr) -> Vec<u8> {
    let mut bytes = os_bytes(arg);
    bytes.push(0);
    bytes
}

fn os_bytes(arg: &OsStr) -> Vec<u8> {
    arg.as_encoded_bytes().to_vec()
}

/// True when `snprintf` wrote a full string into a buffer of `cap` bytes.
pub fn c_printf_fits(written: i32, cap: usize) -> bool {
    written >= 0 && (written as usize) < cap
}

/// The first xdv, dvi, pdf, synctex, or log open owns that slot.
/// A later open with the same extension is an ordinary auxiliary file.
pub fn claims_primary_slot(extension: &[u8], slot_empty: bool) -> bool {
    slot_empty && matches!(extension, b"xdv" | b"dvi" | b"pdf" | b"synctex" | b"log")
}

pub fn predictor_dimensions(colors: i32, bpc: i32, columns: i32) -> Option<(usize, usize)> {
    if colors < 1 || bpc < 1 || columns < 1 {
        return None;
    }
    let colors = colors as i64;
    let bpc = bpc as i64;
    let columns = columns as i64;
    let bpp = (colors * bpc + 7) / 8;
    let row = (colors * bpc * columns + 7) / 8;
    if bpp <= 0 || row <= 0 {
        return None;
    }
    let row = usize::try_from(row).ok()?;
    if row.wrapping_add(1) == 0 {
        return None;
    }
    Some((usize::try_from(bpp).ok()?, row))
}

/// Deletes a directory's immediate children, or unlinks `path` when it is a symlink.
/// A directory symlink is not followed, so the target's files stay.
pub fn remove_c_directory(path: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        return remove_directory_without_following(Path::new(OsStr::from_bytes(path)));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "directory removal is unix-only",
        ))
    }
}

pub fn remove_directory_without_following(path: &Path) -> io::Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return std::fs::remove_file(path);
    }
    #[cfg(unix)]
    {
        return unlink_children_nofollow(path);
    }
    #[cfg(not(unix))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "directory removal is unix-only",
        ))
    }
}

#[cfg(unix)]
fn unlink_children_nofollow(path: &Path) -> io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "path contains a NUL")
    })?;
    unsafe {
        let fd = libc::open(
            c_path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW,
        );
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let dir = libc::fdopendir(fd);
        if dir.is_null() {
            libc::close(fd);
            return Err(io::Error::last_os_error());
        }
        loop {
            *errno_ptr() = 0;
            let entry = libc::readdir(dir);
            if entry.is_null() {
                if *errno_ptr() != 0 {
                    libc::closedir(dir);
                    return Err(io::Error::last_os_error());
                }
                break;
            }
            let name = std::ffi::CStr::from_ptr((*entry).d_name.as_ptr());
            let bytes = name.to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            libc::unlinkat(libc::dirfd(dir), name.as_ptr(), 0);
        }
        libc::closedir(dir);
    }
    std::fs::remove_dir(path)
}

#[cfg(unix)]
fn errno_ptr() -> *mut i32 {
    #[cfg(target_os = "linux")]
    unsafe {
        libc::__errno_location()
    }
    #[cfg(not(target_os = "linux"))]
    unsafe {
        libc::__error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;

    #[test]
    fn non_utf8_argument_keeps_its_bytes() {
        #[cfg(unix)]
        {
            let arg = OsStr::from_bytes(&[0xff, 0xfe]);
            assert_eq!(c_string_bytes(arg), vec![0xff, 0xfe, 0]);
        }
    }

    #[test]
    fn printf_fit_rejects_a_truncated_write() {
        assert!(c_printf_fits(4095, 4096));
        assert!(!c_printf_fits(4096, 4096));
        assert!(!c_printf_fits(-1, 4096));
    }

    #[test]
    fn second_output_open_does_not_claim_the_slot() {
        assert!(claims_primary_slot(b"log", true));
        assert!(claims_primary_slot(b"pdf", true));
        assert!(claims_primary_slot(b"xdv", true));
        assert!(claims_primary_slot(b"synctex", true));
        assert!(!claims_primary_slot(b"log", false));
        assert!(!claims_primary_slot(b"pdf", false));
        assert!(!claims_primary_slot(b"dvi", false));
        assert!(!claims_primary_slot(b"synctex", false));
        assert!(!claims_primary_slot(b"aux", true));
    }

    #[test]
    fn predictor_rejects_a_negative_column_count() {
        assert_eq!(predictor_dimensions(1, 1, -15), None);
        assert_eq!(predictor_dimensions(0, 8, 1), None);
        assert_eq!(predictor_dimensions(1, 0, 1), None);
        assert_eq!(predictor_dimensions(1, 8, 1), Some((1, 1)));
    }

    #[cfg(unix)]
    #[test]
    fn remove_dir_does_not_follow_a_directory_symlink() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("pitex-remove-dir-{stamp}"));
        fs::create_dir(&root).unwrap();
        let secret = root.join("secret");
        fs::create_dir(&secret).unwrap();
        fs::write(secret.join("notes.txt"), b"keep").unwrap();
        let link = root.join("p1");
        std::os::unix::fs::symlink(&secret, &link).unwrap();

        remove_directory_without_following(&link).unwrap();

        assert!(secret.join("notes.txt").is_file());
        assert_eq!(fs::read(secret.join("notes.txt")).unwrap(), b"keep");
        assert!(!link.exists());
        assert!(secret.is_dir());

        let real = root.join("p2");
        fs::create_dir(&real).unwrap();
        fs::write(real.join("gone.txt"), b"x").unwrap();
        remove_directory_without_following(&real).unwrap();
        assert!(!real.exists());

        let _ = fs::remove_dir_all(&root);
    }
}
