use maud::{DOCTYPE, Markup, PreEscaped, html};

use super::styles::STYLES;

/// The whole HTML document around a page's `content`, with all the CSS
/// inlined.
pub fn render_frame(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover";
                // Keeps iOS from turning dates and numbers into phone links.
                meta name="format-detection" content="telephone=no, date=no";
                title { (title) }
                style {
                    @for css in STYLES { (PreEscaped(css)) }
                }
            }
            body { (content) }
        }
    }
}
