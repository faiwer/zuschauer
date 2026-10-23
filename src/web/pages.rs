//! Page handlers.

use axum::extract::State;
use maud::{DOCTYPE, Markup, html};

use crate::config::Config;
use crate::model::{EntityId, MetricKind, Sample, now};
use crate::store::store;

pub async fn index(State(config): State<&'static Config>) -> Markup {
    let rows = latest_rows(config);

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
                    @for (label, sample) in rows {
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

fn latest_rows(config: &Config) -> Vec<(String, Option<Sample>)> {
    let mut wanted = vec![
        ("CPU".to_owned(), EntityId::Host, MetricKind::Cpu),
        ("Memory".to_owned(), EntityId::Host, MetricKind::Memory),
    ];
    for mount in &config.host.storage {
        wanted.push((
            format!("Storage {}", mount.path.display()),
            EntityId::Mount(mount.path.clone()),
            MetricKind::Storage,
        ));
    }

    let store = store();
    wanted
        .into_iter()
        .map(|(label, entity, kind)| {
            let sample = store
                .series(&entity, kind)
                .and_then(|series| series.latest());
            (label, sample)
        })
        .collect()
}

fn age(sample: Sample) -> String {
    let seconds = now().saturating_sub(sample.at);
    format!("{seconds}s ago")
}
