//! Mach kernel API bits shared by the macOS host collectors.

use std::ffi::{CStr, c_char, c_int, c_void};
use std::io;
use std::sync::OnceLock;

pub type MachPort = u32;

pub const KERN_SUCCESS: i32 = 0;

unsafe extern "C" {
    fn mach_host_self() -> MachPort;
    fn sysctlbyname(
        name: *const c_char,
        oldp: *mut c_void,
        oldlenp: *mut usize,
        newp: *mut c_void,
        newlen: usize,
    ) -> c_int;
}

/// Every `mach_host_self()` call adds a send right to the port, so it's
/// fetched once instead of per read.
pub fn host() -> MachPort {
    static HOST: OnceLock<MachPort> = OnceLock::new();
    // SAFETY: no preconditions; returns the host port of the current task.
    *HOST.get_or_init(|| unsafe { mach_host_self() })
}

/// Reads a `u64` sysctl such as `hw.memsize`.
pub fn sysctl_u64(name: &CStr) -> io::Result<u64> {
    let mut value = 0u64;
    let mut len = size_of::<u64>();
    // SAFETY: `value` is a writable u64 and `len` holds its size; the kernel
    // writes at most `len` bytes and updates `len`.
    let rc = unsafe {
        sysctlbyname(
            name.as_ptr(),
            (&raw mut value).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if rc != 0 {
        return Err(io::Error::last_os_error());
    }
    if len != size_of::<u64>() {
        return Err(io::Error::other(format!("sysctl {name:?} is not a u64")));
    }
    Ok(value)
}
