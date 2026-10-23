use std::path::Path;
use std::process::ExitCode;

use super::hwmon::{ROOT, discover, read};

/// Prints every sensor with its current reading, quoted the way the config's
/// `host.temperature` keys are written.
pub fn list() -> ExitCode {
    let found = match discover(Path::new(ROOT)) {
        Ok(found) => found,
        Err(e) => {
            eprintln!("{ROOT}: {e}");
            return ExitCode::FAILURE;
        }
    };
    if found.is_empty() {
        println!("no temperature sensors in {ROOT}");
        return ExitCode::SUCCESS;
    }
    let width = found.iter().map(|f| f.id.len() + 2).max().unwrap_or(0);
    for sensor in &found {
        let reading = read(&sensor.path).map_or_else(|e| e.to_string(), |c| format!("{c:.1}C"));
        println!("{:width$}  {reading}", format!("\"{}\"", sensor.id));
    }
    ExitCode::SUCCESS
}
