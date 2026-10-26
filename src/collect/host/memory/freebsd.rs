//! Wired memory minus the caches kept in it, plus every resident page of
//! anonymous objects: program memory, shared memory and tmpfs. The page
//! queues can't tell this apart: inactive mixes idle program memory with
//! file cache. The record parser is pure so it's tested on any OS.

#[cfg(target_os = "freebsd")]
pub use sys::read;

/// `struct kinfo_vmobject` from `<sys/user.h>`. The kernel cuts each record
/// to `kvo_structsize` bytes, so only fields before `kvo_path` are fixed.
const STRUCTSIZE_AT: usize = 0;
const RESIDENT_AT: usize = 40;
const PATH_AT: usize = 160;

/// Sums `kvo_resident` over a `vm.swap_objects` buffer, in pages.
fn resident_pages(mut records: &[u8]) -> Option<u64> {
    let mut pages = 0u64;
    while !records.is_empty() {
        let size = i32::from_ne_bytes(field(records, STRUCTSIZE_AT)?);
        let size = usize::try_from(size).ok()?;
        if size < PATH_AT || size > records.len() {
            return None;
        }
        pages = pages.checked_add(u64::from_ne_bytes(field(records, RESIDENT_AT)?))?;
        records = &records[size..];
    }
    Some(pages)
}

fn field<const N: usize>(record: &[u8], at: usize) -> Option<[u8; N]> {
    record.get(at..at + N)?.try_into().ok()
}

#[cfg(target_os = "freebsd")]
mod sys {
    use std::ffi::CStr;
    use std::io;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::super::usage::Usage;
    use super::resident_pages;

    /// Grows to fit and stays there, so a steady system reads in one call.
    static BUFFER_SIZE: AtomicUsize = AtomicUsize::new(256 * 1024);

    pub fn read() -> io::Result<Usage> {
        let page_size = number(c"hw.pagesize")?;
        let wired = number(c"vm.stats.vm.v_wire_count")?;
        // Wired by programs (`mlock`), so already among the anonymous pages.
        let user_wired = number(c"vm.stats.vm.v_user_wire_count")?;
        let pages = wired.saturating_sub(user_wired) + anonymous_pages()?;
        let caches = number(c"vfs.bufspace")? + shrinkable_arc()?;
        Ok(Usage {
            used: (pages * page_size).saturating_sub(caches),
            total: number(c"hw.physmem")?,
        })
    }

    /// The part of the ZFS ARC above `c_min`, which it gives back under
    /// pressure. Zero without ZFS.
    fn shrinkable_arc() -> io::Result<u64> {
        let size = match number(c"kstat.zfs.misc.arcstats.size") {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
            other => other?,
        };
        Ok(size.saturating_sub(number(c"kstat.zfs.misc.arcstats.c_min")?))
    }

    fn anonymous_pages() -> io::Result<u64> {
        let name = c"vm.swap_objects";
        loop {
            let mut buffer = vec![0u8; BUFFER_SIZE.load(Ordering::Relaxed)];
            match sysctl(name, &mut buffer) {
                Ok(size) => {
                    return resident_pages(&buffer[..size]).ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidData, "vm.swap_objects: bad record")
                    });
                }
                // The size hint the kernel offers counts every object at
                // full record size, which runs to hundreds of megabytes.
                Err(e) if e.raw_os_error() == Some(libc::ENOMEM) => {
                    BUFFER_SIZE.store(buffer.len() * 2, Ordering::Relaxed);
                }
                Err(e) => return Err(named(name, e)),
            }
        }
    }

    /// An unsigned integer sysctl of either width.
    fn number(name: &CStr) -> io::Result<u64> {
        let mut buffer = [0u8; 8];
        match sysctl(name, &mut buffer).map_err(|e| named(name, e))? {
            4 => Ok(u64::from(u32::from_ne_bytes([
                buffer[0], buffer[1], buffer[2], buffer[3],
            ]))),
            8 => Ok(u64::from_ne_bytes(buffer)),
            size => Err(io::Error::other(format!(
                "{}: unexpected size {size}",
                name.to_string_lossy()
            ))),
        }
    }

    /// Returns the number of bytes written. Errors keep their OS code, so
    /// callers can tell `ENOMEM` apart.
    fn sysctl(name: &CStr, buffer: &mut [u8]) -> io::Result<usize> {
        let mut size = buffer.len();
        // SAFETY: `name` is NUL-terminated and `buffer` is writable for
        // `size` bytes.
        let status = unsafe {
            libc::sysctlbyname(
                name.as_ptr(),
                buffer.as_mut_ptr().cast(),
                &mut size,
                std::ptr::null(),
                0,
            )
        };
        if status != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(size)
    }

    fn named(name: &CStr, e: io::Error) -> io::Error {
        io::Error::new(e.kind(), format!("{}: {e}", name.to_string_lossy()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(resident: u64, path: &str) -> Vec<u8> {
        let size = (PATH_AT + path.len() + 1).next_multiple_of(8);
        let mut record = vec![0u8; size];
        record[..4].copy_from_slice(&(size as i32).to_ne_bytes());
        record[RESIDENT_AT..RESIDENT_AT + 8].copy_from_slice(&resident.to_ne_bytes());
        record[PATH_AT..PATH_AT + path.len()].copy_from_slice(path.as_bytes());
        record
    }

    #[test]
    fn sums_variable_size_records() {
        let records = [
            record(3, ""),
            record(40, "/posix-shm-name"),
            record(500, ""),
        ]
        .concat();
        assert_eq!(resident_pages(&records), Some(543));
        assert_eq!(resident_pages(&[]), Some(0));
    }

    #[test]
    fn rejects_broken_records() {
        let mut records = record(3, "");
        records.truncate(records.len() - 1);
        assert_eq!(resident_pages(&records), None);
        let mut records = record(3, "");
        records[..4].copy_from_slice(&0i32.to_ne_bytes());
        assert_eq!(resident_pages(&records), None);
    }
}
