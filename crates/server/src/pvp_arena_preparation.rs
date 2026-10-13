//! TC a352b1fa Battleground.cpp:1047-1050,416: source preparation aura,
//! removed when the arena opens. No synthetic duration replaces the match clock.
use super::{PlayerPvpState, PvpServerState};
use crate::maps::InMap;
use bevy::prelude::*;
const PREPARATION_SPELL: u32 = 32727;
const RESET_INTERVAL_SECS: f64 = 5.0;

#[derive(Component)]
struct PreparationResetAt(f64);

pub(super) fn sync_preparation(world: &mut World) {
    let mut query = world.query::<(Entity, &PlayerPvpState, &InMap)>();
    let actors: Vec<_> = query
        .iter(world)
        .filter_map(|(entity, state, map)| {
            let bg = state
                .battleground
                .as_ref()
                .filter(|bg| bg.arena.is_some() && bg.accepted)?;
            let matched = world
                .resource::<PvpServerState>()
                .battleground_matches
                .get(&bg.match_id)?;
            (matched.map == Some(*map)).then_some((
                entity,
                matched.instance.phase == shared::battleground::BgPhase::Waiting,
            ))
        })
        .collect();
    for (entity, preparing) in actors {
        let applied = world
            .get::<crate::auras::ActiveAuras>(entity)
            .is_some_and(|auras| auras.iter().any(|aura| aura.spell_id == PREPARATION_SPELL));
        match (preparing, applied) {
            (true, false) => {
                reset_entry(world, entity);
                crate::auras::apply_aura(
                    world,
                    &crate::auras::AuraApplication {
                        caster: entity,
                        target: entity,
                        spell_id: PREPARATION_SPELL,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            (false, true) => clear_preparation(world, entity),
            _ => {}
        }
        if preparing {
            reset_preparing_resources(world, entity);
        }
    }
}

pub(super) fn reset_entry(world: &mut World, entity: Entity) {
    reset_resources(world, entity);
    crate::spell_cast::reset_arena_cooldowns(world, entity);
    world.entity_mut(entity).remove::<PreparationResetAt>();
}

fn reset_preparing_resources(world: &mut World, entity: Entity) {
    let now = world.resource::<Time>().elapsed_secs_f64();
    let due = world
        .get::<PreparationResetAt>(entity)
        .is_none_or(|last| now - last.0 > RESET_INTERVAL_SECS);
    if due {
        reset_resources(world, entity);
        world.entity_mut(entity).insert(PreparationResetAt(now));
    }
}

/// TC Player.cpp:1804: only the primary pool, not every class resource.
fn reset_resources(world: &mut World, entity: Entity) {
    use crate::spell_cast::powers::{PowerPools, change_power, power_value};
    use shared::components::{Health, PowerType};
    if let Some(mut health) = world.get_mut::<Health>(entity) {
        health.current = health.max;
    }
    let primary = world
        .get::<PowerPools>(entity)
        .and_then(|pools| pools.order.first())
        .copied();
    let Some(primary) = primary else { return };
    let Some((current, max)) = power_value(world, entity, primary) else {
        return;
    };
    let next = match primary {
        PowerType::Mana | PowerType::Energy => max,
        PowerType::Rage | PowerType::RunicPower | PowerType::LunarPower => 0.0,
        _ => return,
    };
    change_power(world, entity, primary, next - current);
}

pub(super) fn clear_preparation(world: &mut World, entity: Entity) {
    reset_resources(world, entity);
    world.entity_mut(entity).remove::<PreparationResetAt>();
    crate::auras::remove_auras(world, entity, crate::auras::RemoveReason::Expired, |aura| {
        aura.spell_id == PREPARATION_SPELL
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spell_cast::powers::{PowerPool, PowerPools, power_value};
    use shared::components::{Health, Mana, PowerType};

    #[test]
    fn arena_preparation_resets_only_primary_power_at_entry_after_five_seconds_and_exit() {
        for (primary, expected) in [
            (PowerType::Mana, 100.0),
            (PowerType::Energy, 100.0),
            (PowerType::Rage, 0.0),
            (PowerType::RunicPower, 0.0),
            (PowerType::LunarPower, 0.0),
            (PowerType::Focus, 50.0),
        ] {
            let mut world = World::new();
            world.insert_resource(Time::<()>::default());
            let actor = world
                .spawn((
                    Health {
                        current: 25.0,
                        max: 200.0,
                    },
                    Mana {
                        current: 50.0,
                        max: 100.0,
                    },
                    PowerPools {
                        order: vec![primary, PowerType::HolyPower],
                        pools: vec![
                            PowerPool {
                                power: primary,
                                current: 50.0,
                                max: 100.0,
                                regen_per_sec: 0.0,
                            },
                            PowerPool {
                                power: PowerType::HolyPower,
                                current: 2.0,
                                max: 5.0,
                                regen_per_sec: 0.0,
                            },
                        ],
                        ..Default::default()
                    },
                ))
                .id();
            reset_preparing_resources(&mut world, actor);
            assert_eq!(world.get::<Health>(actor).unwrap().current, 200.0);
            assert_eq!(power_value(&world, actor, primary).unwrap().0, expected);
            assert_eq!(
                power_value(&world, actor, PowerType::HolyPower).unwrap().0,
                2.0
            );
            world.get_mut::<Health>(actor).unwrap().current = 25.0;
            world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(5));
            reset_preparing_resources(&mut world, actor);
            assert_eq!(
                world.get::<Health>(actor).unwrap().current,
                25.0,
                "TC strictly greater than5000ms"
            );
            world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(1));
            reset_preparing_resources(&mut world, actor);
            assert_eq!(world.get::<Health>(actor).unwrap().current, 200.0);
            world.get_mut::<Health>(actor).unwrap().current = 25.0;
            clear_preparation(&mut world, actor);
            assert_eq!(
                world.get::<Health>(actor).unwrap().current,
                200.0,
                "source gate-open reset"
            );
        }
    }
}
