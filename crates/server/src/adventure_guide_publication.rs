//! One final Guide presentation per frame avoids reordering full/clock type buffers.
use super::*;
use shared::protocol::AdventureGuideClock;

#[derive(Component, Default)]
pub(crate) struct GuidePublication {
    published: Option<AdventureGuideSnapshot>,
    pending: Option<AdventureGuideSnapshot>,
    force_full: bool,
}

pub(super) fn queue_update(
    world: &mut World,
    connection: Entity,
    snapshot: AdventureGuideSnapshot,
    force_full: bool,
) {
    let mut connection = world.entity_mut(connection);
    let mut publication = connection.entry::<GuidePublication>().or_default();
    publication.pending = Some(snapshot);
    publication.force_full |= force_full;
}

pub(super) fn flush_updates(world: &mut World) {
    let updates: Vec<_> = world
        .query_filtered::<(Entity, &mut GuidePublication), (With<Authenticated>, With<InWorld>)>()
        .iter_mut(world)
        .filter_map(|(connection, mut publication)| {
            let snapshot = publication.pending.take()?;
            let force_full = std::mem::take(&mut publication.force_full);
            Some((connection, snapshot, force_full))
        })
        .collect();
    for (connection, snapshot, force_full) in updates {
        publish_snapshot(world, connection, snapshot, force_full);
    }
}

fn publish_snapshot(
    world: &mut World,
    connection: Entity,
    mut snapshot: AdventureGuideSnapshot,
    force_full: bool,
) {
    let previous = world
        .get::<GuidePublication>(connection)
        .and_then(|publication| publication.published.as_ref());
    let clock_only = previous.is_some_and(|previous| same_content(previous, &mut snapshot));
    if clock_only && !force_full {
        publish_clock(world, connection, snapshot.month.now_unix);
        return;
    }
    publish_full(world, connection, snapshot);
}

fn same_content(previous: &AdventureGuideSnapshot, current: &mut AdventureGuideSnapshot) -> bool {
    let now_unix = current.month.now_unix;
    current.month.now_unix = previous.month.now_unix;
    let equal = *current == *previous;
    current.month.now_unix = now_unix;
    equal
}

fn publish_clock(world: &mut World, connection: Entity, now_unix: i64) {
    let previous_time = world
        .get::<GuidePublication>(connection)
        .and_then(|publication| publication.published.as_ref())
        .map(|snapshot| snapshot.month.now_unix);
    if previous_time == Some(now_unix) {
        return;
    }
    world
        .get_mut::<MessageSender<AdventureGuideClock>>(connection)
        .expect("protocol registers the Guide clock sender")
        .send::<InstanceChannel>(AdventureGuideClock { now_unix });
    let mut publication = world.get_mut::<GuidePublication>(connection).unwrap();
    let baseline = publication
        .published
        .as_mut()
        .expect("Guide clock baseline");
    baseline.month.now_unix = now_unix;
}

fn publish_full(world: &mut World, connection: Entity, snapshot: AdventureGuideSnapshot) {
    world
        .get_mut::<MessageSender<AdventureGuideSnapshot>>(connection)
        .expect("protocol registers the full Guide sender")
        .send::<InstanceChannel>(snapshot.clone());
    world
        .get_mut::<GuidePublication>(connection)
        .expect("queued Guide publication")
        .published = Some(snapshot);
}
