use maud::{DOCTYPE, Markup, PreEscaped, html};

use super::super::styles::STYLES;
use super::icon::render_icon_sprite;

/// The whole HTML document around a page's `content`, with all the CSS
/// inlined.
pub fn render_frame(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover";
                meta name="color-scheme" content="dark";
                // Keeps iOS from turning dates and numbers into phone links.
                meta name="format-detection" content="telephone=no, date=no";
                meta name="mobile-web-app-capable" content="yes";
                meta name="apple-mobile-web-app-title" content="Zuschauer";
                link rel="icon" type="image/svg+xml" href="/favicon.svg";
                title { (title) }
                style {
                    @for css in STYLES { (PreEscaped(css)) }
                }
            }
            body {
                (render_icon_sprite())
                (content)
            }
        }
    }
}
