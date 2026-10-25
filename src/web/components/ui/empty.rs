use maud::{Markup, html};

use super::icon::{Icon, tinted_icon};

#[derive(Debug, Clone, Copy)]
pub enum Tone {
    Positive,
    Neutral(Icon),
    Warning,
}

impl Tone {
    fn class(self) -> &'static str {
        match self {
            Self::Positive => "empty-positive",
            Self::Neutral(_) => "empty-neutral",
            Self::Warning => "empty-warning",
        }
    }

    fn icon(self) -> (Icon, &'static str) {
        match self {
            Self::Positive => (Icon::Checkmark, "tint-ok"),
            Self::Neutral(icon) => (icon, "tint-unknown"),
            Self::Warning => (Icon::Alert, "tint-warning"),
        }
    }
}

/// A toast in place of an empty list. Neutral ones show the list's icon.
pub fn render_empty(tone: Tone, title: &str, subtitle: &str) -> Markup {
    let (icon, tint) = tone.icon();
    html! {
        div.empty.(tone.class()) {
            (tinted_icon(icon, tint, None))
            div.empty-lines {
                span.empty-title { (title) }
                span.empty-subtitle { (subtitle) }
            }
        }
    }
}
