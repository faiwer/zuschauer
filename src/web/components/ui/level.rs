/// How good something looks, worst last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Unknown,
    Ok,
    Warning,
    Problem,
}

impl Level {
    /// The `.tint-*` class.
    pub fn class(self) -> &'static str {
        match self {
            Self::Unknown => "tint-unknown",
            Self::Ok => "tint-ok",
            Self::Warning => "tint-warning",
            Self::Problem => "tint-problem",
        }
    }

    /// Worth coloring text for.
    pub fn is_bad(self) -> bool {
        self >= Self::Warning
    }

    /// Classes coloring text by the level when it's bad, else none. One
    /// string: maud glues a toggled class to a dynamic one without a space.
    pub fn text_class(self) -> &'static str {
        match self {
            Self::Warning => "tint-text tint-warning",
            Self::Problem => "tint-text tint-problem",
            Self::Unknown | Self::Ok => "",
        }
    }
}
