//! Containers are jails, found with `jail_get(2)` and named after the jail.
//! CPU and memory are summed from the process table by jail ID, which needs
//! neither root nor `kern.racct.enable`. A reaped process's CPU time moves to
//! its parent, so it's lost only when that parent is outside the jail.

use std::collections::HashMap;
use std::ffi::CStr;
use std::io;
use std::time::Instant;

use super::api::Api;
use super::reading::{Reading, Snapshot};

/// `MAXHOSTNAMELEN`, the longest jail name.
const NAME_SIZE: usize = 256;

pub(super) async fn read(_api: Option<&Api>) -> io::Result<Snapshot> {
    let jails = jails()?;
    let usage = usage_by_jail()?;
    let at = Instant::now();
    let page_size = page_size();
    let containers = jails
        .into_iter()
        .map(|(jid, name)| {
            let usage = usage.get(&jid).copied().unwrap_or_default();
            Reading {
                id: jid.to_string(),
                name: Some(name),
                at,
                cpu_usage_usec: usage.cpu_usec,
                memory: Some(usage.resident_pages * page_size),
                paused: false,
            }
        })
        .collect();
    Ok(Snapshot {
        cores: cores(),
        containers,
    })
}

/// Jail ID and name of every live jail, nested ones included.
fn jails() -> io::Result<Vec<(i32, String)>> {
    let mut jails = Vec::new();
    let mut last_jid: i32 = 0;
    loop {
        let mut name = [0u8; NAME_SIZE];
        let mut params = [
            key(c"lastjid"),
            value(&mut last_jid),
            key(c"name"),
            libc::iovec {
                iov_base: name.as_mut_ptr().cast(),
                iov_len: name.len(),
            },
        ];
        // SAFETY: `params` holds key/value pairs; each value points to a live
        // buffer of the given length, and keys are only read.
        let jid = unsafe { libc::jail_get(params.as_mut_ptr(), params.len() as u32, 0) };
        if jid < 0 {
            let e = io::Error::last_os_error();
            if e.raw_os_error() == Some(libc::ENOENT) {
                return Ok(jails);
            }
            return Err(io::Error::new(e.kind(), format!("jail_get: {e}")));
        }
        let name = CStr::from_bytes_until_nul(&name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|_| jid.to_string());
        jails.push((jid, name));
        last_jid = jid;
    }
}

fn key(name: &'static CStr) -> libc::iovec {
    libc::iovec {
        iov_base: name.as_ptr().cast_mut().cast(),
        iov_len: name.to_bytes_with_nul().len(),
    }
}

fn value(number: &mut i32) -> libc::iovec {
    libc::iovec {
        iov_base: (number as *mut i32).cast(),
        iov_len: size_of::<i32>(),
    }
}

#[derive(Clone, Copy, Default)]
struct Usage {
    cpu_usec: u64,
    resident_pages: u64,
}

/// Over all processes of each jail. Memory shared between them, like
/// libraries, counts once per process.
fn usage_by_jail() -> io::Result<HashMap<i32, Usage>> {
    let mut usage: HashMap<i32, Usage> = HashMap::new();
    for process in processes()? {
        if process.ki_jid == 0 {
            continue;
        }
        let children = &process.ki_childtime;
        let children_usec = u64::try_from(children.tv_sec).unwrap_or(0) * 1_000_000
            + u64::try_from(children.tv_usec).unwrap_or(0);
        let jail = usage.entry(process.ki_jid).or_default();
        jail.cpu_usec += process.ki_runtime + children_usec;
        jail.resident_pages += u64::try_from(process.ki_rssize).unwrap_or(0);
    }
    Ok(usage)
}

/// `kern.proc.proc`: one record per process, threads folded in.
fn processes() -> io::Result<Vec<libc::kinfo_proc>> {
    let mib = [libc::CTL_KERN, libc::KERN_PROC, libc::KERN_PROC_PROC];
    loop {
        let mut size = 0;
        // SAFETY: a null buffer asks only for the size.
        let status = unsafe {
            libc::sysctl(
                mib.as_ptr(),
                3,
                std::ptr::null_mut(),
                &mut size,
                std::ptr::null(),
                0,
            )
        };
        if status != 0 {
            return Err(sysctl_error());
        }
        // Room for processes started between the two calls.
        let count = size / size_of::<libc::kinfo_proc>() + 16;
        // SAFETY: `kinfo_proc` is plain integers, arrays and pointers, for
        // which all-zero is valid.
        let mut processes = vec![unsafe { std::mem::zeroed::<libc::kinfo_proc>() }; count];
        let mut size = count * size_of::<libc::kinfo_proc>();
        // SAFETY: `processes` is writable for `size` bytes.
        let status = unsafe {
            libc::sysctl(
                mib.as_ptr(),
                3,
                processes.as_mut_ptr().cast(),
                &mut size,
                std::ptr::null(),
                0,
            )
        };
        if status != 0 {
            let e = sysctl_error();
            if e.raw_os_error() == Some(libc::ENOMEM) {
                continue;
            }
            return Err(e);
        }
        processes.truncate(size / size_of::<libc::kinfo_proc>());
        if let Some(process) = processes.first()
            && usize::try_from(process.ki_structsize).ok() != Some(size_of::<libc::kinfo_proc>())
        {
            return Err(io::Error::other(format!(
                "kern.proc.proc: records are {} bytes, expected {}",
                process.ki_structsize,
                size_of::<libc::kinfo_proc>()
            )));
        }
        return Ok(processes);
    }
}

fn sysctl_error() -> io::Error {
    let e = io::Error::last_os_error();
    io::Error::new(e.kind(), format!("kern.proc.proc: {e}"))
}

fn page_size() -> u64 {
    // SAFETY: sysconf has no preconditions.
    u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).unwrap_or(4096)
}

/// Online cores of the host.
fn cores() -> u32 {
    // SAFETY: sysconf has no preconditions.
    let n = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
    u32::try_from(n).unwrap_or(1).max(1)
}
