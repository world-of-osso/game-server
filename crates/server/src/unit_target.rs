//! Replicated `UnitTarget` (target-of-target) mirrors each unit's current
//! target: players from `CombatTarget` (SetTarget), NPCs from their
//! `CreatureState::Pursuing` chase target, none while confused or feared
//! (`Unit::SetConfused`/`SetFeared` → `SetTarget(Empty)`). Written only on change.

use bevy::prelude::*;
use shared::components::UnitTarget;

use crate::auras::{Confused, Feared};
use crate::creature_ai::CreatureState;
use crate::grid_activity::GridDormant;
use crate::networking::CombatTarget;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct UnitTargetSync;

pub struct UnitTargetPlugin;

impl Plugin for UnitTargetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (sync_player_unit_target, sync_creature_unit_target).in_set(UnitTargetSync),
        );
    }
}

fn sync_player_unit_target(
    mut commands: Commands,
    query: Query<(Entity, &CombatTarget, Option<&UnitTarget>), Changed<CombatTarget>>,
) {
    for (entity, target, current) in &query {
        write_if_changed(
            &mut commands,
            entity,
            current,
            target.0.map(Entity::to_bits),
        );
    }
}

type CreatureTargets<'w, 's, F> = Query<
    'w,
    's,
    (
        Entity,
        &'static CreatureState,
        Has<Confused>,
        Has<Feared>,
        Option<&'static UnitTarget>,
    ),
    F,
>;

/// Active creatures whose chase state changed or that became confused or feared.
type Retargeted = (
    Or<(Changed<CreatureState>, Added<Confused>, Added<Feared>)>,
    Without<GridDormant>,
);

/// Active creatures whose state or confusion changed, plus creatures whose grid just
/// woke: a change made while dormant is not seen by the `Changed` scan, and no player
/// is in range of a dormant creature to read its target.
fn sync_creature_unit_target(
    mut commands: Commands,
    changed: CreatureTargets<Retargeted>,
    all: CreatureTargets<()>,
    mut woke: RemovedComponents<GridDormant>,
    mut unconfused: RemovedComponents<Confused>,
    mut unfeared: RemovedComponents<Feared>,
) {
    let resync = woke
        .read()
        .chain(unconfused.read())
        .chain(unfeared.read())
        .filter_map(|entity| all.get(entity).ok());
    for (entity, state, confused, feared, current) in changed.iter().chain(resync) {
        let target = match *state {
            CreatureState::Pursuing { target } if !confused && !feared => Some(target),
            _ => None,
        };
        write_if_changed(&mut commands, entity, current, target);
    }
}

fn write_if_changed(
    commands: &mut Commands,
    entity: Entity,
    current: Option<&UnitTarget>,
    target: Option<u64>,
) {
    if current != Some(&UnitTarget(target)) {
        commands.entity(entity).insert(UnitTarget(target));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lightyear::connection::host::HostClient;
    use lightyear::prelude::*;
    use shared::protocol::{CombatChannel, ProtocolPlugin, SetTarget};

    use super::*;
    use crate::networking::PlayerLink;

    struct Harness {
        app: App,
        player: Entity,
        connection: Entity,
    }

    fn harness() -> Harness {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.add_plugins(lightyear::prelude::server::ServerPlugins {
            tick_duration: Duration::from_millis(50),
        });
        app.add_plugins(ProtocolPlugin);
        app.add_plugins(UnitTargetPlugin);
        app.add_systems(Update, crate::messaging::process_target_messages);
        app.finish();
        app.cleanup();
        app.update();
        let player = app.world_mut().spawn(CombatTarget::default()).id();
        let connection = app
            .world_mut()
            .spawn((
                RemoteId(PeerId::Local(1)),
                Connected,
                HostClient { buffer: Vec::new() },
                PlayerLink(player),
                MessageSender::<SetTarget>::default(),
                MessageReceiver::<SetTarget>::default(),
            ))
            .id();
        app.update();
        Harness {
            app,
            player,
            connection,
        }
    }

    impl Harness {
        /// Send + HostClient loopback delivery takes 3 frames.
        fn set_target(&mut self, target: Option<Entity>) {
            self.app
                .world_mut()
                .get_mut::<MessageSender<SetTarget>>(self.connection)
                .unwrap()
                .send::<CombatChannel>(SetTarget {
                    target_entity: target.map(Entity::to_bits),
                });
            for _ in 0..3 {
                self.app.update();
            }
        }

        fn unit_target(&self) -> Option<UnitTarget> {
            self.app.world().get::<UnitTarget>(self.player).copied()
        }
    }

    #[test]
    fn set_target_mirrors_into_unit_target_and_clears() {
        let mut h = harness();
        assert_eq!(h.unit_target(), Some(UnitTarget(None)));
        let npc = h.app.world_mut().spawn_empty().id();
        h.set_target(Some(npc));
        assert_eq!(h.unit_target(), Some(UnitTarget(Some(npc.to_bits()))));
        h.set_target(None);
        assert_eq!(h.unit_target(), Some(UnitTarget(None)));
    }

    #[test]
    fn unchanged_target_does_not_rewrite_unit_target() {
        let mut h = harness();
        let npc = h.app.world_mut().spawn_empty().id();
        h.set_target(Some(npc));
        let written = h
            .app
            .world()
            .entity(h.player)
            .get_ref::<UnitTarget>()
            .unwrap()
            .last_changed();
        h.set_target(Some(npc));
        let after = h
            .app
            .world()
            .entity(h.player)
            .get_ref::<UnitTarget>()
            .unwrap()
            .last_changed();
        assert_eq!(after, written);
    }

    #[test]
    fn pursuing_creature_targets_its_chase_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UnitTargetPlugin);
        let player = app.world_mut().spawn_empty().id();
        let npc = app.world_mut().spawn(CreatureState::Idle).id();
        app.update();
        assert_eq!(app.world().get::<UnitTarget>(npc), Some(&UnitTarget(None)));
        *app.world_mut().get_mut::<CreatureState>(npc).unwrap() = CreatureState::Pursuing {
            target: player.to_bits(),
        };
        app.update();
        assert_eq!(
            app.world().get::<UnitTarget>(npc),
            Some(&UnitTarget(Some(player.to_bits())))
        );
        *app.world_mut().get_mut::<CreatureState>(npc).unwrap() = CreatureState::Returning;
        app.update();
        assert_eq!(app.world().get::<UnitTarget>(npc), Some(&UnitTarget(None)));
    }

    #[test]
    fn state_changed_while_dormant_reaches_the_target_on_wake() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UnitTargetPlugin);
        let player = app.world_mut().spawn_empty().id();
        let npc = app
            .world_mut()
            .spawn((
                CreatureState::Pursuing {
                    target: player.to_bits(),
                },
                GridDormant,
            ))
            .id();
        app.update();
        app.update();
        *app.world_mut().get_mut::<CreatureState>(npc).unwrap() = CreatureState::Pursuing {
            target: player.to_bits(),
        };
        app.update();
        assert_eq!(app.world().get::<UnitTarget>(npc), None);

        app.world_mut().entity_mut(npc).remove::<GridDormant>();
        app.update();
        assert_eq!(
            app.world().get::<UnitTarget>(npc),
            Some(&UnitTarget(Some(player.to_bits())))
        );
    }
}
