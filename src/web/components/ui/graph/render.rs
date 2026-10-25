use std::fmt::Write;

use maud::{Markup, html};

use crate::model::Timestamp;

use super::super::format::clock;
use super::scale::Scale;
use super::time::time_ticks;

/// The SVG's own units: the line is drawn in a 1000×1000 box, stretched to
/// the plot.
const BOX: f64 = 1000.0;
/// A pause longer than this many typical intervals breaks the line.
const GAP: Timestamp = 3;

/// A line of `points` (oldest first) over round value ticks labeled with
/// `unit`, e.g. `"%"` or `" MiB"`. Stretches to the width. A series of
/// zeros is a sentence instead: its line would hide under the axis.
pub fn render_graph(points: &[(Timestamp, f32)], unit: &str) -> Markup {
    let (Some(&(first, _)), Some(&(last, _))) = (points.first(), points.last()) else {
        return render_too_few();
    };
    if last <= first {
        return render_too_few();
    }
    let (min, max) = points
        .iter()
        .fold((f64::MAX, f64::MIN), |(min, max), &(_, v)| {
            (min.min(v.into()), max.max(v.into()))
        });
    if min == 0.0 && max == 0.0 {
        return html! { p.graph-empty { "0" (unit) " the whole time" } };
    }
    let scale = Scale::new(min, max);
    let decimals = scale.decimals();
    let path = path(points, &scale, first, last);
    let (times, time_step) = time_ticks(first, last);
    html! {
        div.graph {
            div.graph-y {
                @for tick in scale.ticks() { span { (format!("{tick:.decimals$}{unit}")) } }
            }
            div.graph-plot {
                @for tick in scale.ticks().skip(1) {
                    div.graph-grid style=(format!("bottom: {:.3}%", scale.position(tick) * 100.0)) {}
                }
                svg viewBox="0 0 1000 1000" preserveAspectRatio="none" aria-hidden="true" {
                    path d=(path) {}
                }
            }
            div.graph-x {
                @for at in &times {
                    span style=(format!("left: {:.3}%", f64::from(at - first) / f64::from(last - first) * 100.0)) {
                        (clock(*at))
                        @if time_step < 60 { (format!(":{:02}", at % 60)) }
                    }
                }
            }
        }
    }
}

fn render_too_few() -> Markup {
    html! { p.graph-empty { "Not enough data yet" } }
}

/// Moves to a point after a gap, draws a line to it otherwise.
fn path(points: &[(Timestamp, f32)], scale: &Scale, first: Timestamp, last: Timestamp) -> String {
    let gap = typical_interval(points).saturating_mul(GAP);
    let span = f64::from(last - first);
    let mut d = String::new();
    let mut prev: Option<Timestamp> = None;
    for &(at, value) in points {
        let x = f64::from(at - first) / span * BOX;
        let y = (1.0 - scale.position(value.into())) * BOX;
        let command = match prev {
            Some(prev) if at - prev <= gap => 'L',
            _ => 'M',
        };
        let _ = write!(d, "{command}{x:.1},{y:.1}");
        prev = Some(at);
    }
    d
}

/// The median time between points.
fn typical_interval(points: &[(Timestamp, f32)]) -> Timestamp {
    let mut intervals: Vec<Timestamp> = points.windows(2).map(|w| w[1].0 - w[0].0).collect();
    intervals.sort_unstable();
    intervals.get(intervals.len() / 2).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_pause_breaks_the_line() {
        let points = [(0, 1.0), (10, 2.0), (20, 1.0), (100, 2.0), (110, 1.0)];
        let d = path(&points, &Scale::new(1.0, 2.0), 0, 110);
        let moves: Vec<_> = d.match_indices('M').map(|(i, _)| i).collect();
        assert_eq!(moves.len(), 2, "{d}");
        assert_eq!(d.matches('L').count(), 3, "{d}");
    }
}
