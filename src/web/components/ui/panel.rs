use maud::{Markup, html};

use super::icon::{Icon, icon};

/// A page section: a title with an icon over its `body`. `class` names the
/// content box, for its styles.
pub fn render_panel(title_icon: Icon, title: &str, class: &str, body: Markup) -> Markup {
    html! {
        section.panel {
            h2.panel-title {
                (icon(title_icon))
                (title)
            }
            div.panel-content.(class) { (body) }
        }
    }
}
