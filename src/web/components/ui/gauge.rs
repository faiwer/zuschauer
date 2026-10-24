use maud::{Markup, html};

/// A ring filled to `percent`, with `value` in the middle and `label` under
/// it. `None` leaves the ring empty, and `error` goes under the label.
pub fn render_gauge(label: &str, percent: Option<f32>, value: &str, error: Option<&str>) -> Markup {
    html! {
        div.gauge {
            div.gauge-ring {
                // SVG elements need closing tags: a bare `<circle>` would
                // swallow its siblings.
                svg viewBox="0 0 36 36" aria-hidden="true" {
                    circle.gauge-track cx="18" cy="18" r="16" pathLength="100" {}
                    @if let Some(percent) = percent {
                        circle.gauge-fill.(level(percent)) cx="18" cy="18" r="16" pathLength="100"
                            stroke-dasharray=(format!("{:.1} 100", percent.clamp(0.0, 100.0)))
                            transform="rotate(-90 18 18)" {}
                    }
                }
                span.gauge-value { (value) }
            }
            span.gauge-label { (label) }
            @if let Some(error) = error {
                span.gauge-error { (error) }
            }
        }
    }
}

/// The fill's color class.
fn level(percent: f32) -> &'static str {
    match percent {
        p if p < 40.0 => "low",
        p if p < 80.0 => "normal",
        p if p < 90.0 => "high",
        _ => "critical",
    }
}
