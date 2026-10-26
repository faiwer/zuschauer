//! `kern.cp_time`: ticks spent in each CPU state, summed over all cores.

use std::io;

use super::ticks::Ticks;

/// `CPUSTATES`: user, nice, system, interrupt, idle.
const STATES: usize = 5;
const IDLE: usize = 4;

pub fn read() -> io::Result<Ticks> {
    let mut times: [libc::c_long; STATES] = [0; STATES];
    let mut size = size_of_val(&times);
    // SAFETY: the name is NUL-terminated, `times` is writable and `size`
    // holds its size in bytes.
    let status = unsafe {
        libc::sysctlbyname(
            c"kern.cp_time".as_ptr(),
            times.as_mut_ptr().cast(),
            &mut size,
            std::ptr::null(),
            0,
        )
    };
    if status != 0 {
        return Err(io::Error::last_os_error());
    }
    if size != size_of_val(&times) {
        return Err(io::Error::other(format!(
            "kern.cp_time: expected {} bytes, got {size}",
            size_of_val(&times)
        )));
    }
    let ticks = times.map(|t| t as u64);
    Ok(Ticks {
        idle: ticks[IDLE],
        total: ticks.iter().sum(),
    })
}
