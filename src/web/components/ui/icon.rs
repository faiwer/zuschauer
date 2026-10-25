use std::sync::OnceLock;

use maud::{Markup, PreEscaped, html};

/// An SVG from `web/icons`. Drawn in `currentColor`, sized by `.icon`.
#[derive(Debug, Clone, Copy)]
pub enum Icon {
    Alert,
    App,
    Cert,
    Checkmark,
    Container,
    Eye,
    Host,
    Reload,
    Service,
    Storage,
    Temperature,
}

impl Icon {
    const ALL: [Self; 11] = [
        Self::Alert,
        Self::App,
        Self::Cert,
        Self::Checkmark,
        Self::Container,
        Self::Eye,
        Self::Host,
        Self::Reload,
        Self::Service,
        Self::Storage,
        Self::Temperature,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Alert => "alert",
            Self::App => "app",
            Self::Cert => "cert",
            Self::Checkmark => "checkmark",
            Self::Container => "container",
            Self::Eye => "eye",
            Self::Host => "host",
            Self::Reload => "reload",
            Self::Service => "service",
            Self::Storage => "storage",
            Self::Temperature => "temperature",
        }
    }

    fn source(self) -> &'static str {
        match self {
            Self::Alert => include_str!("../../icons/alert.svg"),
            Self::App => include_str!("../../icons/app.svg"),
            Self::Cert => include_str!("../../icons/cert.svg"),
            Self::Checkmark => include_str!("../../icons/checkmark.svg"),
            Self::Container => include_str!("../../icons/container.svg"),
            Self::Eye => include_str!("../../icons/eye.svg"),
            Self::Host => include_str!("../../icons/host.svg"),
            Self::Reload => include_str!("../../icons/reload.svg"),
            Self::Service => include_str!("../../icons/service.svg"),
            Self::Storage => include_str!("../../icons/storage.svg"),
            Self::Temperature => include_str!("../../icons/temperature.svg"),
        }
    }
}

/// A reference to the icon's symbol in the sprite.
pub fn icon(icon: Icon) -> Markup {
    html! {
        svg.icon aria-hidden="true" {
            use href=(href(icon)) {}
        }
    }
}

/// Like `icon`, colored by a `.tint-*` class. `hint`: a `render_hint` ID.
pub fn tinted_icon(icon: Icon, tint: &str, hint: Option<&str>) -> Markup {
    html! {
        svg.icon.tinted.(tint) aria-hidden="true" data-hint=[hint] {
            use href=(href(icon)) {}
        }
    }
}

fn href(icon: Icon) -> String {
    format!("#icon-{}", icon.name())
}

/// Every icon as a `<symbol>`, once per page, for `icon` to refer to.
pub fn render_icon_sprite() -> Markup {
    static SPRITE: OnceLock<String> = OnceLock::new();
    let symbols = SPRITE.get_or_init(|| Icon::ALL.map(symbol).concat());
    html! {
        svg.icon-sprite aria-hidden="true" { (PreEscaped(symbols)) }
    }
}

/// The icon's `<svg>` as a `<symbol>`: its `viewBox` and contents, without
/// the size and namespaces.
fn symbol(icon: Icon) -> String {
    let source = icon.source();
    let start = source.find("<svg").expect("icon without <svg");
    let open_end = start
        + source[start..]
            .find('>')
            .expect("icon with an unclosed <svg");
    let close = source.rfind("</svg>").expect("icon without </svg>");
    let tag = &source[start..open_end];
    let view_box = tag
        .split_once(r#"viewBox=""#)
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(view_box, _)| view_box)
        .expect("icon without a viewBox");
    let contents = &source[open_end + 1..close];
    format!(
        r#"<symbol id="icon-{}" viewBox="{view_box}">{contents}</symbol>"#,
        icon.name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_becomes_a_symbol() {
        for icon in Icon::ALL {
            let symbol = symbol(icon);
            assert!(symbol.starts_with(&format!(r#"<symbol id="icon-{}" viewBox=""#, icon.name())));
            assert!(symbol.ends_with("</symbol>"));
            assert!(!symbol.contains("<svg"));
        }
    }

    #[test]
    fn every_icon_paints_in_current_color() {
        for icon in Icon::ALL {
            assert!(icon.source().contains("currentColor"), "{icon:?}");
        }
    }
}
