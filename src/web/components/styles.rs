/// Every component's CSS, in the order it's inlined: later files override
/// earlier ones.
pub const STYLES: &[&str] = &[
    include_str!("ui/frame.css"),
    include_str!("ui/panel.css"),
    include_str!("ui/empty.css"),
    include_str!("ui/gauge.css"),
    include_str!("ui/graph/graph.css"),
    include_str!("ui/tooltip.css"),
    include_str!("ui/modal.css"),
    include_str!("ui/footer.css"),
    include_str!("domain/header.css"),
    include_str!("domain/alerts.css"),
    include_str!("domain/containers.css"),
    include_str!("domain/host.css"),
    include_str!("domain/apps/apps.css"),
];
