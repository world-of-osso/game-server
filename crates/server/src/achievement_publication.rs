//! Per-connection presentation stream; gameplay evaluation and persistence stay synchronous.
use super::*;
use shared::protocol::AchievementDelta;

#[derive(Component, Default)]
pub(super) struct AchievementPublication {
    published: Option<AchievementSnapshot>,
    pending: Option<AchievementSnapshot>,
    toasts: Vec<AchievementToastSnapshot>,
}

pub(super) fn queue_update(
    world: &mut World,
    connection: Entity,
    snapshot: AchievementSnapshot,
    toasts: Vec<AchievementToastSnapshot>,
) {
    let mut connection = world.entity_mut(connection);
    let mut publication = connection
        .entry::<AchievementPublication>()
        .or_default()
        .into_mut();
    publication.pending = Some(snapshot);
    publication.toasts.extend(toasts);
}

pub(crate) fn flush_updates(world: &mut World) {
    let updates: Vec<_> = world
        .query_filtered::<(Entity, &mut AchievementPublication), (With<InWorld>, With<Authenticated>)>()
        .iter_mut(world)
        .filter_map(|(connection, mut publication)| {
            let snapshot = publication.pending.take()?;
            Some((connection, snapshot, std::mem::take(&mut publication.toasts)))
        })
        .collect();
    for (connection, snapshot, toasts) in updates {
        publish_snapshot(world, connection, snapshot);
        publish_toasts(world, connection, toasts);
    }
}

fn publish_snapshot(world: &mut World, connection: Entity, snapshot: AchievementSnapshot) {
    let delta = world
        .get::<AchievementPublication>(connection)
        .and_then(|publication| publication.published.as_ref())
        .map(|previous| diff_snapshots(previous, &snapshot));
    match delta {
        Some(delta) if delta == empty_delta() => return,
        Some(delta) => world
            .get_mut::<MessageSender<AchievementDelta>>(connection)
            .expect("protocol registers the achievement delta sender")
            .send::<AchievementChannel>(delta),
        None => world
            .get_mut::<MessageSender<AchievementStateUpdate>>(connection)
            .expect("protocol registers the achievement baseline sender")
            .send::<AchievementChannel>(AchievementStateUpdate {
                snapshot: Some(snapshot.clone()),
                completed: None,
                message: Some("achievement progress updated".into()),
                error: None,
            }),
    }
    world
        .get_mut::<AchievementPublication>(connection)
        .expect("queued achievement publication")
        .published = Some(snapshot);
}

fn publish_toasts(world: &mut World, connection: Entity, toasts: Vec<AchievementToastSnapshot>) {
    let mut sender = world
        .get_mut::<MessageSender<AchievementStateUpdate>>(connection)
        .expect("protocol registers the achievement toast sender");
    for toast in toasts {
        sender.send::<AchievementChannel>(AchievementStateUpdate {
            snapshot: None,
            completed: Some(toast),
            message: Some("achievement completed".into()),
            error: None,
        });
    }
}

fn empty_delta() -> AchievementDelta {
    AchievementDelta {
        earned_ids: None,
        progress_upserts: Vec::new(),
        removed_progress_ids: Vec::new(),
    }
}

fn diff_snapshots(
    previous: &AchievementSnapshot,
    current: &AchievementSnapshot,
) -> AchievementDelta {
    let previous_rows: BTreeMap<_, _> = previous
        .progress
        .iter()
        .map(|row| (row.achievement_id, row))
        .collect();
    let current_ids: BTreeSet<_> = current
        .progress
        .iter()
        .map(|row| row.achievement_id)
        .collect();
    AchievementDelta {
        earned_ids: (previous.earned_ids != current.earned_ids).then(|| current.earned_ids.clone()),
        progress_upserts: current
            .progress
            .iter()
            .filter(|row| previous_rows.get(&row.achievement_id).copied() != Some(*row))
            .cloned()
            .collect(),
        removed_progress_ids: previous_rows
            .keys()
            .filter(|id| !current_ids.contains(id))
            .copied()
            .collect(),
    }
}
