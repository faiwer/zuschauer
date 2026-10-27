use maud::{Markup, html};

use super::icon::{Icon, icon};

/// A page section: a title with an icon over its `body`. `class` names the
/// content box, for its styles.
pub fn render_panel(title_icon: Icon, title: &str, class: &str, body: Markup) -> Markup {
    render_panel_with(title_icon, title, None, class, body)
}

/// Like `render_panel`, with `action`, like a button, at the title's end.
pub fn render_panel_with(
    title_icon: Icon,
    title: &str,
    action: Option<Markup>,
    class: &str,
    body: Markup,
) -> Markup {
    html! {
        section.panel {
            h2.panel-title {
                (icon(title_icon))
                (title)
                @if let Some(action) = action {
                    span.panel-action { (action) }
                }
            }
            div.panel-content.(class) { (body) }
        }
    }
}
