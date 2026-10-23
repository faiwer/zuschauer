//! `statvfs(2)`. Block counts are in units of `f_frsize`.

use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use super::blocks::Blocks;

pub fn read(path: &Path) -> io::Result<Blocks> {
    let c_path = CString::new(path.as_os_str().as_bytes())?;
    // SAFETY: all-zero is a valid `statvfs` (plain integers).
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c_path` is NUL-terminated and `stat` is writable.
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Blocks {
        block_size: stat.f_frsize as u64,
        total: stat.f_blocks as u64,
        free: stat.f_bfree as u64,
        available: stat.f_bavail as u64,
    })
}
