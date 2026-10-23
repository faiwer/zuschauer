use std::fmt;
use std::io;

#[derive(Debug)]
pub enum LoadError {
    Read(io::Error),
    Parse {
        at: String,
        source: serde_json::Error,
    },
    Invalid(Vec<String>),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(e) => write!(f, "cannot read config: {e}"),
            Self::Parse { at, source } if at == "." => write!(f, "{source}"),
            Self::Parse { at, source } => write!(f, "{at}: {source}"),
            Self::Invalid(errors) => {
                write!(f, "invalid config:")?;
                for e in errors {
                    write!(f, "\n  - {e}")?;
                }
                Ok(())
            }
        }
    }
}
