//! `/sys/class/hwmon/hwmon*/`: one directory per driver instance, with its
//! `name` and a `tempN_input` file per sensor, optionally with a
//! `tempN_label`. The `hwmonN` numbers can change between boots, names and
//! labels don't.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(super) const ROOT: &str = "/sys/class/hwmon";

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Found {
    /// `driver/label`, or `driver/tempN` when the sensor has no label.
    pub id: String,
    /// The `tempN_input` file.
    pub path: PathBuf,
}

pub(super) fn discover() -> io::Result<Vec<Found>> {
    discover_in(Path::new(ROOT))
}

/// Sorted by ID. A missing `root` means no sensors.
fn discover_in(root: &Path) -> io::Result<Vec<Found>> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut found = Vec::new();
    for entry in entries {
        let dir = entry?.path();
        let (Ok(driver), Ok(files)) = (fs::read_to_string(dir.join("name")), fs::read_dir(&dir))
        else {
            continue;
        };
        let driver = driver.trim();
        for file in files {
            let file = file?.file_name();
            let Some(input) = file
                .to_str()
                .and_then(|name| name.strip_suffix("_input"))
                .filter(|input| is_temperature(input))
            else {
                continue;
            };
            let label = fs::read_to_string(dir.join(format!("{input}_label")));
            let label = label
                .as_deref()
                .map(str::trim)
                .ok()
                .filter(|label| !label.is_empty())
                .unwrap_or(input);
            found.push(Found {
                id: format!("{driver}/{label}"),
                path: dir.join(&file),
            });
        }
    }
    found.sort_by(|a, b| a.id.cmp(&b.id).then_with(|| a.path.cmp(&b.path)));
    Ok(found)
}

/// `temp1`, not `fan1` or `in0`.
fn is_temperature(input: &str) -> bool {
    input
        .strip_prefix("temp")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Degrees Celsius. The file holds millidegrees.
pub(super) fn read(path: &Path) -> io::Result<f32> {
    let text = fs::read_to_string(path)?;
    let millidegrees: i32 = text.trim().parse().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("unexpected reading {:?}", text.trim()),
        )
    })?;
    Ok(millidegrees as f32 / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(test: &str, files: &[(&str, &str)]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("zuschauer-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (path, content) in files {
            let path = root.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        root
    }

    #[test]
    fn finds_temperature_inputs() {
        let root = tree(
            "hwmon-finds",
            &[
                ("hwmon0/name", "chip\n"),
                ("hwmon0/temp1_input", "28850\n"),
                ("hwmon0/temp1_label", "Sensor A/B\n"),
                ("hwmon0/temp2_input", "30000\n"),
                ("hwmon0/temp2_label", "\n"),
                ("hwmon0/temp1_max", "80000\n"),
                ("hwmon0/fan1_input", "1200\n"),
                ("hwmon1/name", "board\n"),
                ("hwmon1/in0_input", "5000\n"),
                ("hwmon2/temp1_input", "1000\n"),
            ],
        );
        let ids: Vec<_> = discover_in(&root)
            .unwrap()
            .into_iter()
            .map(|f| f.id)
            .collect();
        assert_eq!(ids, ["chip/Sensor A/B", "chip/temp2"]);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn missing_root_means_no_sensors() {
        let root = std::env::temp_dir().join("zuschauer-hwmon-does-not-exist");
        assert_eq!(discover_in(&root).unwrap(), []);
    }

    #[test]
    fn reads_millidegrees() {
        let root = tree(
            "hwmon-reads",
            &[("ok", "28850\n"), ("negative", "-5500\n"), ("bad", "n/a\n")],
        );
        assert_eq!(read(&root.join("ok")).unwrap(), 28.85);
        assert_eq!(read(&root.join("negative")).unwrap(), -5.5);
        assert!(read(&root.join("bad")).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
