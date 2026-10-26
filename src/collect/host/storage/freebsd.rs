//! `statfs(2)`. Block counts are in units of `f_bsize`.

use std::ffi::CString;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use super::blocks::Blocks;

pub fn read(path: &Path) -> io::Result<Blocks> {
    let c_path = CString::new(path.as_os_str().as_bytes())?;
    // SAFETY: all-zero is a valid `statfs` (integers and char arrays).
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c_path` is NUL-terminated and `stat` is writable.
    if unsafe { libc::statfs(c_path.as_ptr(), &mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Blocks {
        block_size: stat.f_bsize,
        total: stat.f_blocks,
        free: stat.f_bfree,
        // Negative once root dips into its reserve.
        available: u64::try_from(stat.f_bavail).unwrap_or(0),
    })
}
