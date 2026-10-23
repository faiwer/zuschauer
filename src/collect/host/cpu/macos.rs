//! Reads Mach `host_statistics(HOST_CPU_LOAD_INFO)`: user, system, idle and
//! nice ticks summed over all cores. The kernel keeps them as 32-bit counters.

use std::io;

use super::super::mach::{KERN_SUCCESS, MachPort, host};
use super::ticks::Ticks;

const HOST_CPU_LOAD_INFO: i32 = 3;
const CPU_STATE_MAX: usize = 4;
const CPU_STATE_IDLE: usize = 2;

unsafe extern "C" {
    fn host_statistics(host: MachPort, flavor: i32, info: *mut i32, count: *mut u32) -> i32;
}

pub fn read() -> io::Result<Ticks> {
    let mut ticks = [0u32; CPU_STATE_MAX];
    let mut count = CPU_STATE_MAX as u32;
    // SAFETY: `ticks` is `host_cpu_load_info` (natural_t[CPU_STATE_MAX]) and
    // `count` holds its length in 32-bit words, as the call expects.
    let rc = unsafe {
        host_statistics(
            host(),
            HOST_CPU_LOAD_INFO,
            ticks.as_mut_ptr().cast(),
            &mut count,
        )
    };
    if rc != KERN_SUCCESS {
        return Err(io::Error::other(format!(
            "host_statistics failed: kern_return_t {rc}"
        )));
    }
    Ok(Ticks {
        idle: ticks[CPU_STATE_IDLE].into(),
        total: ticks.iter().map(|&t| u64::from(t)).sum(),
    })
}
