use crate::alerts::{Observation, observe};
use crate::config::App;
use crate::model::{EntityId, Status};
use crate::store::{Entity, store};

/// Feeds each app container's status into its alert. A container that was
/// never seen counts as not running.
pub(super) fn observe_app_containers(apps: &'static [App]) {
    for container in apps.iter().flat_map(|app| &app.containers) {
        let Some(alert) = &container.alert else {
            continue;
        };
        let status = store()
            .entity(&EntityId::Container(container.name.clone()))
            .and_then(Entity::status);
        let observation = match status {
            Some(Status::Running) => Observation::Fine,
            Some(Status::Paused) => Observation::Problem("paused".to_owned()),
            _ => Observation::Problem("not running".to_owned()),
        };
        observe(alert, observation);
    }
}
