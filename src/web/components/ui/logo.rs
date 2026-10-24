use maud::{Markup, html};

/// In its own colors, so it stays out of the `currentColor` sprite. Served
/// as `/favicon.svg`.
pub const LOGO_SVG: &str = include_str!("../../icons/logo.svg");

/// Sized by `.icon`.
pub fn render_logo() -> Markup {
    html! { img.icon src="/favicon.svg" alt=""; }
}
