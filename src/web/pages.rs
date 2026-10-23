//! Page handlers.

use maud::{DOCTYPE, Markup, html};

use crate::model::{EntityId, MetricKind, Sample, now};
use crate::store::store;

pub async fn index() -> Markup {
    let cpu = store()
        .series(&EntityId::Host, MetricKind::Cpu)
        .and_then(|series| series.latest());

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "Zuschauer" }
            }
            body {
                h1 { "Host" }
                p {
                    "CPU: "
                    @match cpu {
                        Some(sample) => {
                            strong { (format!("{:.1}%", sample.value)) }
                            " (" (age(sample)) ")"
                        }
                        None => "no data yet",
                    }
                }
            }
        }
    }
}

fn age(sample: Sample) -> String {
    let seconds = now().saturating_sub(sample.at);
    format!("{seconds}s ago")
}
