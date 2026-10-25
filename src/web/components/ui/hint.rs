use std::sync::atomic::{AtomicUsize, Ordering};

use maud::{Markup, html};

/// Unique for the process's life, so on every page too.
pub fn hint_id() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("hint-{}", NEXT.fetch_add(1, Ordering::Relaxed))
}

/// What `tooltip.js` shows over the `[data-hint=id]` element.
pub fn render_hint(id: &str, content: Markup) -> Markup {
    html! {
        div.hint hidden id=(id) { (content) }
    }
}
