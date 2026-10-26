use std::path::Path;

use json_comments::StripComments;

use super::error::LoadError;
use super::raw::RawConfig;
use super::resolve::resolve;
use super::resolved::Config;

/// Reads, parses and validates the config.
pub fn load(path: &Path) -> Result<Config, LoadError> {
    let text = std::fs::read_to_string(path).map_err(LoadError::Read)?;
    // Comments are replaced with whitespace, so serde_json's line:column
    // positions still match the original file.
    let mut de = serde_json::Deserializer::from_reader(StripComments::new(text.as_bytes()));
    let raw: RawConfig =
        serde_path_to_error::deserialize(&mut de).map_err(|e| LoadError::Parse {
            at: e.path().to_string(),
            source: e.into_inner(),
        })?;
    de.end().map_err(|source| LoadError::Parse {
        at: ".".to_owned(),
        source,
    })?;
    resolve(raw).map_err(LoadError::Invalid)
}
