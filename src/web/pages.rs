//! Page handlers.

use maud::{DOCTYPE, Markup, html};

use crate::model::{EntityId, MetricKind, Sample, now};
use crate::store::store;

const HOST_METRICS: [(&str, MetricKind); 2] =
    [("CPU", MetricKind::Cpu), ("Memory", MetricKind::Memory)];

pub async fn index() -> Markup {
    let latest = {
        let store = store();
        HOST_METRICS.map(|(label, kind)| {
            let sample = store
                .series(&EntityId::Host, kind)
                .and_then(|series| series.latest());
            (label, sample)
        })
    };

    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "Zuschauer" }
            }
            body {
                h1 { "Host" }
                table {
                    @for (label, sample) in latest {
                        tr {
                            th { (label) }
                            @match sample {
                                Some(sample) => {
                                    td { strong { (format!("{:.1}%", sample.value)) } }
                                    td { (age(sample)) }
                                }
                                None => td colspan="2" { "no data yet" },
                            }
                        }
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
