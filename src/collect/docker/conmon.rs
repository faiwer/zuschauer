//! Podman container names from `conmon`, the monitor process Podman starts
//! per container. Its command line has the container's ID and name, and
//! `/proc/<pid>/cmdline` is readable by any user, so this names every user's
//! containers without root or their owner's API socket.

#[cfg(target_os = "linux")]
use std::collections::HashMap;

/// Container ID → name, for every running `conmon`. Empty without Podman.
#[cfg(target_os = "linux")]
pub fn names() -> HashMap<String, String> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return HashMap::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .as_encoded_bytes()
                .iter()
                .all(u8::is_ascii_digit)
        })
        .filter(|entry| {
            std::fs::read_to_string(entry.path().join("comm"))
                .is_ok_and(|comm| comm.trim() == "conmon")
        })
        .filter_map(|entry| parse(&std::fs::read(entry.path().join("cmdline")).ok()?))
        .collect()
}

/// `(id, name)` from `conmon`'s NUL-separated arguments: `-c <id>` and
/// `-n <name>`, or their long forms.
pub fn parse(cmdline: &[u8]) -> Option<(String, String)> {
    let mut args = cmdline.split(|&b| b == 0).map(String::from_utf8_lossy);
    let (mut id, mut name) = (None, None);
    while let Some(arg) = args.next() {
        match arg.as_ref() {
            "-c" | "--cid" => id = args.next(),
            "-n" | "--name" => name = args.next(),
            _ => {}
        }
    }
    Some((id?.into_owned(), name?.into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmdline(args: &[&str]) -> Vec<u8> {
        args.join("\0").into_bytes()
    }

    #[test]
    fn reads_id_and_name() {
        let line = cmdline(&[
            "/usr/bin/conmon",
            "--api-version",
            "1",
            "-c",
            "b97f",
            "-u",
            "b97f",
            "-r",
            "/usr/bin/crun",
            "-n",
            "web",
            "--exit-dir",
            "/run/libpod/exits",
        ]);
        assert_eq!(parse(&line), Some(("b97f".to_owned(), "web".to_owned())));
    }

    #[test]
    fn reads_long_forms() {
        let line = cmdline(&["conmon", "--cid", "b97f", "--name", "web"]);
        assert_eq!(parse(&line), Some(("b97f".to_owned(), "web".to_owned())));
    }

    #[test]
    fn needs_both() {
        assert_eq!(parse(&cmdline(&["conmon", "-c", "b97f"])), None);
        assert_eq!(parse(&cmdline(&["conmon", "-n", "web"])), None);
        assert_eq!(parse(&cmdline(&["conmon", "-c"])), None);
    }
}
