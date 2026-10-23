//! Pure helpers for cgroup v2 directory names and files, so they're tested on
//! any OS.

/// The container ID if `dir` is a container's cgroup directory:
/// - `docker-<id>.scope`, `libpod-<id>.scope`: Docker and Podman with systemd
/// - `libpod-<id>`: Podman with the cgroupfs manager
/// - `<id>` inside `docker/`: Docker with the cgroupfs driver
///
/// `libpod-conmon-<id>.scope` (Podman's per-container monitor) doesn't match.
pub fn container_id<'a>(dir: &'a str, parent: &str) -> Option<&'a str> {
    let id = match dir
        .strip_prefix("docker-")
        .or_else(|| dir.strip_prefix("libpod-"))
    {
        Some(rest) => rest.strip_suffix(".scope").unwrap_or(rest),
        None if parent == "docker" => dir,
        None => return None,
    };
    is_id(id).then_some(id)
}

fn is_id(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A `key value` line from flat-keyed files like `cpu.stat` and `memory.stat`.
pub fn field(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        let (k, v) = line.split_once(' ')?;
        (k == key).then(|| v.trim().parse().ok())?
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "3f9ac1d2e7b84a6f9c0d1e2f3a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0f1a2b";

    #[test]
    fn matches_container_dirs() {
        assert_eq!(
            container_id(&format!("docker-{ID}.scope"), "system.slice"),
            Some(ID)
        );
        assert_eq!(
            container_id(&format!("libpod-{ID}.scope"), "machine.slice"),
            Some(ID)
        );
        assert_eq!(
            container_id(&format!("libpod-{ID}"), "libpod_parent"),
            Some(ID)
        );
        assert_eq!(container_id(ID, "docker"), Some(ID));
    }

    #[test]
    fn skips_other_dirs() {
        assert_eq!(
            container_id(&format!("libpod-conmon-{ID}.scope"), "machine.slice"),
            None
        );
        assert_eq!(container_id(ID, "system.slice"), None);
        assert_eq!(container_id("docker.service", "system.slice"), None);
        assert_eq!(container_id("docker-abc.scope", "system.slice"), None);
    }

    #[test]
    fn reads_fields() {
        let cpu_stat = "usage_usec 18442901\nuser_usec 12001000\nsystem_usec 6441901\n";
        assert_eq!(field(cpu_stat, "usage_usec"), Some(18442901));
        assert_eq!(field(cpu_stat, "system_usec"), Some(6441901));
        assert_eq!(field(cpu_stat, "nr_throttled"), None);
        assert_eq!(field("inactive_file x\n", "inactive_file"), None);
    }
}
