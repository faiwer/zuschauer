use maud::{Markup, html};

/// The theme switch and the source link. Both switch buttons are rendered;
/// the CSS shows the one leading away from the current theme.
pub fn render_footer() -> Markup {
    html! {
        footer.footer {
            button.footer-link.theme-to-light type="button"
                onclick="localStorage.setItem('theme', 'light'); location.reload()" {
                "Switch to light theme"
            }
            button.footer-link.theme-to-dark type="button"
                onclick="localStorage.setItem('theme', 'dark'); location.reload()" {
                "Switch to dark theme"
            }
            span.footer-separator aria-hidden="true" { "◆" }
            a.footer-link href=(env!("CARGO_PKG_REPOSITORY")) target="_blank" rel="noopener" {
                "GitHub"
            }
        }
    }
}
