//! `statfs(2)`. macOS's `statvfs` has 32-bit block counters, `statfs` has
//! 64-bit ones. Block counts are in units of `f_bsize`.
//!
//! On APFS the counters describe the whole container shared by all its
//! volumes, so every volume reports the same usage. `df` shows a lower
//! per-volume "Used" (from `getattrlist`), but the container is what runs out.

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
        block_size: u64::from(stat.f_bsize),
        total: stat.f_blocks,
        free: stat.f_bfree,
        available: stat.f_bavail,
    })
}
