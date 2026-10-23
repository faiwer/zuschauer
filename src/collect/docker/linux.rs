//! Finds running containers by walking `/sys/fs/cgroup` and reads their CPU
//! and memory from cgroup files. No API calls: Quadlet containers are named
//! after their systemd unit, others are resolved separately, once per new
//! container ID.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

use super::cgroup::{container_id, field, quadlet_unit};
use super::reading::{Reading, Snapshot};

const ROOT: &str = "/sys/fs/cgroup";
/// Rootless Podman nests containers about 6 levels deep.
const MAX_DEPTH: usize = 8;

pub fn read() -> io::Result<Snapshot> {
    let mut containers = Vec::new();
    scan(Path::new(ROOT), 0, &mut containers);
    Ok(Snapshot {
        cores: cores(),
        containers,
    })
}

fn scan(dir: &Path, depth: usize, out: &mut Vec<Reading>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let parent = dir.file_name().and_then(OsStr::to_str).unwrap_or("");
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        let path = entry.path();
        if let Some(id) = container_id(name, parent) {
            out.extend(read_container(&path, id, quadlet_unit(name, parent)));
        } else if depth < MAX_DEPTH {
            scan(&path, depth + 1, out);
        }
    }
}

fn read_container(dir: &Path, id: &str, name: Option<&str>) -> Option<Reading> {
    let at = Instant::now();
    let cpu_usage_usec = field(
        &fs::read_to_string(dir.join("cpu.stat")).ok()?,
        "usage_usec",
    )?;
    // Missing when the memory controller isn't delegated to this cgroup.
    let memory = read_u64(&dir.join("memory.current")).map(|current| {
        let inactive_file = fs::read_to_string(dir.join("memory.stat"))
            .ok()
            .and_then(|text| field(&text, "inactive_file"))
            .unwrap_or(0);
        current.saturating_sub(inactive_file)
    });
    let paused = fs::read_to_string(dir.join("cgroup.freeze")).is_ok_and(|s| s.trim() == "1");
    Some(Reading {
        id: id.to_owned(),
        name: name.map(str::to_owned),
        at,
        cpu_usage_usec,
        memory,
        paused,
    })
}

fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Online cores of the host, not of this process's CPU affinity.
fn cores() -> u32 {
    // SAFETY: sysconf has no preconditions.
    let n = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
    u32::try_from(n).unwrap_or(1).max(1)
}
