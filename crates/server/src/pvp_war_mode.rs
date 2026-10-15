//! TC a352b1fa Player.cpp:30965–31054. Territory bits from local AreaTable.
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;
use crate::persistence::GameDatabase;
use crate::pvp::{PlayerPvpState, persistence_actor, pvp_support};
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Player, Position};
use shared::protocol::{PvpStateUpdate, SetWarModeDesired, WarModeSnapshot};
const ENLISTED_INSIDE: u32 = 282559;
const ENLISTED_OUTSIDE: u32 = 269083;

#[derive(Clone, Copy)]
pub(super) struct Context {
    resting: bool,
    in_combat: bool,
    in_instance: bool,
    can_enable_area: bool,
}
fn validate_toggle(context: Context, desired: bool) -> Result<(), String> {
    if context.in_combat {
        return Err("War Mode cannot change in combat".into());
    }
    if context.in_instance {
        return Err("War Mode cannot change in an instance".into());
    }
    if !context.resting {
        return Err("War Mode changes require a rest area".into());
    }
    if desired && !context.can_enable_area {
        return Err("Enable War Mode in a friendly area that permits toggling".into());
    }
    Ok(())
}
pub(super) fn snapshot(context: Context, desired: bool) -> WarModeSnapshot {
    let enlisted_spell = if desired && !context.in_instance {
        Some(if context.can_enable_area {
            ENLISTED_INSIDE
        } else {
            ENLISTED_OUTSIDE
        })
    } else {
        None
    };
    WarModeSnapshot {
        desired,
        active: desired && !context.can_enable_area && !context.in_instance,
        can_enable: validate_toggle(context, true).is_ok(),
        can_disable: validate_toggle(context, false).is_ok(),
        enlisted_spell,
    }
}
#[derive(bevy::ecs::system::SystemParam)]
pub(super) struct Areas<'w> {
    ground: Option<ResMut<'w, crate::ground::GroundMap>>,
    areas: Option<Res<'w, crate::area_table::AreaTable>>,
    factions: Option<Res<'w, crate::faction::FactionTemplates>>,
}
impl Areas<'_> {
    pub(super) fn context(
        &mut self,
        position: &Position,
        player: &Player,
        map: &crate::maps::InMap,
        rest: &crate::rest::PlayerRestState,
        combat: &CombatStatus,
    ) -> Option<Context> {
        // Instance identity is authoritative before any ADT/AreaTable load.
        if map.instance_id != 0 {
            return Some(Context {
                resting: rest.is_resting(),
                in_combat: combat.0,
                in_instance: true,
                can_enable_area: false,
            });
        }
        let area = self
            .ground
            .as_mut()?
            .area_at(map.map_id, Vec3::new(position.x, position.y, position.z))?;
        Some(Context {
            resting: rest.is_resting(),
            in_combat: combat.0,
            in_instance: map.instance_id != 0,
            can_enable_area: self.areas.as_ref()?.permits_war_mode_enable(
                area,
                map.map_id,
                player.race,
                self.factions.as_ref()?,
            ),
        })
    }
}
type Players<'w, 's> = Query<
    'w,
    's,
    (
        &'static Position,
        &'static Player,
        &'static crate::maps::InMap,
        &'static crate::rest::PlayerRestState,
        &'static CombatStatus,
        &'static mut PlayerPvpState,
    ),
>;

pub(super) fn receive_requests(
    mut receivers: Query<
        (
            Entity,
            &PlayerLink,
            &InWorld,
            &Authenticated,
            &mut MessageReceiver<SetWarModeDesired>,
        ),
        With<Authenticated>,
    >,
    mut players: Players,
    mut areas: Areas,
    db: Res<GameDatabase>,
    mut pending: ResMut<persistence_actor::PendingPvpWrites>,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    for (connection, link, in_world, auth, mut receiver) in &mut receivers {
        for request in receiver.receive() {
            let Ok((position, player, map, rest, combat, state)) = players.get_mut(link.0) else {
                pvp_support::send_pvp_error(
                    connection,
                    &mut senders,
                    "War Mode player context unavailable",
                );
                continue;
            };
            let Some(context) = areas.context(position, player, map, rest, combat) else {
                pvp_support::send_pvp_error(
                    connection,
                    &mut senders,
                    "War Mode terrain/territory context unavailable",
                );
                continue;
            };
            if let Err(error) = validate_toggle(context, request.enabled) {
                pvp_support::send_pvp_update(connection, &state, None, Some(error), &mut senders);
                continue;
            }
            let desired = request.enabled;
            persistence_actor::enqueue(
                &db,
                &mut pending,
                persistence_actor::Owner {
                    connection,
                    player: link.0,
                    account: auth.account_id,
                    character: in_world.character_id,
                },
                persistence_actor::Change::WarMode,
                move |progress| {
                    progress.war_mode_desired = desired;
                    Ok(())
                },
            );
        }
    }
}
pub(super) fn sync_state(
    connections: Query<(Entity, &PlayerLink), With<Authenticated>>,
    mut players: Players,
    mut areas: Areas,
    mut commands: Commands,
    mut senders: Query<&mut MessageSender<PvpStateUpdate>>,
) {
    for (connection, link) in &connections {
        let Ok((position, player, map, rest, combat, mut state)) = players.get_mut(link.0) else {
            continue;
        };
        let Some(context) = areas.context(position, player, map, rest, combat) else {
            continue;
        };
        let next = snapshot(context, state.progress.war_mode_desired);
        if state.war_mode.as_ref() == Some(&next) {
            continue;
        }
        state.war_mode = Some(next.clone());
        let owner = link.0;
        commands
            .queue(move |world: &mut World| sync_enlisted_aura(world, owner, next.enlisted_spell));
        pvp_support::send_pvp_update(connection, &state, None, None, &mut senders);
    }
}
pub(super) fn sync_enlisted_aura(world: &mut World, player: Entity, spell: Option<u32>) {
    crate::auras::remove_auras(
        world,
        player,
        crate::auras::RemoveReason::Unlearned,
        |aura| {
            matches!(aura.spell_id, ENLISTED_INSIDE | ENLISTED_OUTSIDE)
                && Some(aura.spell_id) != spell
        },
    );
    if let Some(spell_id) = spell {
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: player,
                target: player,
                spell_id,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
}

#[cfg(test)]
#[path = "pvp_war_mode_fifo_tests.rs"]
mod fifo_tests;
#[cfg(test)]
pub(super) use fifo_tests::city_harness_with_spells;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn instance_transfer_removes_enlisted_even_before_terrain_is_available() {
        let mut h =
            crate::spell_cast::test_harness::harness(Vec::new(), &[282559, 269083], 2).unwrap();
        let mut state = PlayerPvpState::default();
        state.progress.war_mode_desired = true;
        state.war_mode = Some(snapshot(
            Context {
                resting: true,
                in_combat: false,
                in_instance: false,
                can_enable_area: true,
            },
            true,
        ));
        h.app.world_mut().entity_mut(h.caster).insert((
            state,
            crate::rest::PlayerRestState::fixture(90, 0),
            crate::maps::InMap::continent(0),
        ));
        sync_enlisted_aura(h.app.world_mut(), h.caster, Some(ENLISTED_INSIDE));
        assert!(crate::auras::has_aura(
            h.app.world(),
            h.caster,
            ENLISTED_INSIDE
        ));
        h.app
            .world_mut()
            .entity_mut(h.connection)
            .insert(MessageSender::<PvpStateUpdate>::default());
        h.app.add_systems(Update, sync_state);
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(crate::maps::InMap {
                map_id: 489,
                instance_id: 7,
            });
        h.app.update();
        let state = h.app.world().get::<PlayerPvpState>(h.caster).unwrap();
        assert!(
            state.progress.war_mode_desired,
            "instance entry does not erase durable preference"
        );
        assert_eq!(state.war_mode.as_ref().unwrap().enlisted_spell, None);
        assert!(!crate::auras::has_aura(
            h.app.world(),
            h.caster,
            ENLISTED_INSIDE
        ));
        assert!(!crate::auras::has_aura(
            h.app.world(),
            h.caster,
            ENLISTED_OUTSIDE
        ));
    }
    #[test]
    fn source_rest_combat_instance_and_area_rules() {
        let capital = Context {
            resting: true,
            in_combat: false,
            in_instance: false,
            can_enable_area: true,
        };
        assert!(validate_toggle(capital, true).is_ok());
        let inn = Context {
            can_enable_area: false,
            ..capital
        };
        assert!(validate_toggle(inn, true).is_err());
        assert!(validate_toggle(inn, false).is_ok());
        assert!(
            validate_toggle(
                Context {
                    resting: false,
                    ..capital
                },
                false
            )
            .is_err()
        );
        assert!(
            validate_toggle(
                Context {
                    in_combat: true,
                    ..capital
                },
                true
            )
            .is_err()
        );
        assert!(
            validate_toggle(
                Context {
                    in_instance: true,
                    ..capital
                },
                true
            )
            .is_err()
        );
        assert_eq!(snapshot(capital, true).enlisted_spell, Some(282559));
        assert!(!snapshot(capital, true).active);
        assert_eq!(snapshot(inn, true).enlisted_spell, Some(269083));
        assert!(snapshot(inn, true).active);
        assert_eq!(snapshot(inn, false).enlisted_spell, None);
    }
}
