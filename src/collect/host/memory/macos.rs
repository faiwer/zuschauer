//! Reads Mach `host_statistics64(HOST_VM_INFO64)` page counts and computes
//! what Activity Monitor calls "Memory Used": app memory + wired + compressed.
//! File-backed cache and purgeable memory are left out.

use std::io;

use super::super::mach::{KERN_SUCCESS, MachPort, host, sysctl_u64};
use super::Usage;

const HOST_VM_INFO64: i32 = 4;

/// `struct vm_statistics64` from `<mach/vm_statistics.h>`.
#[repr(C)]
#[derive(Default)]
struct VmStatistics64 {
    free_count: u32,
    active_count: u32,
    inactive_count: u32,
    wire_count: u32,
    zero_fill_count: u64,
    reactivations: u64,
    pageins: u64,
    pageouts: u64,
    faults: u64,
    cow_faults: u64,
    lookups: u64,
    hits: u64,
    purges: u64,
    purgeable_count: u32,
    speculative_count: u32,
    decompressions: u64,
    compressions: u64,
    swapins: u64,
    swapouts: u64,
    compressor_page_count: u32,
    throttled_count: u32,
    external_page_count: u32,
    internal_page_count: u32,
    total_uncompressed_pages_in_compressor: u64,
}

unsafe extern "C" {
    fn host_statistics64(host: MachPort, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
    static vm_kernel_page_size: usize;
}

pub fn read() -> io::Result<Usage> {
    let mut stats = VmStatistics64::default();
    let mut count = (size_of::<VmStatistics64>() / size_of::<i32>()) as u32;
    // SAFETY: `stats` matches `vm_statistics64` and `count` holds its size in
    // 32-bit words, as the call expects.
    let rc =
        unsafe { host_statistics64(host(), HOST_VM_INFO64, (&raw mut stats).cast(), &mut count) };
    if rc != KERN_SUCCESS {
        return Err(io::Error::other(format!(
            "host_statistics64 failed: kern_return_t {rc}"
        )));
    }
    // SAFETY: set by the kernel before the process starts, never written after.
    let page_size = unsafe { vm_kernel_page_size } as u64;
    let app = stats
        .internal_page_count
        .saturating_sub(stats.purgeable_count);
    let pages =
        u64::from(app) + u64::from(stats.wire_count) + u64::from(stats.compressor_page_count);
    Ok(Usage {
        used: pages * page_size,
        total: sysctl_u64(c"hw.memsize")?,
    })
}
