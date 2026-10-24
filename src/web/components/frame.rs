use maud::{DOCTYPE, Markup, html};

/// The whole HTML document around a page's `content`.
pub fn render_frame(title: &str, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { (title) }
            }
            body { (content) }
        }
    }
}
