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
    clear_entry_auras(world, entity);
    let pet = world
        .get::<crate::hunter_pet::PetStable>(entity)
        .and_then(|stable| stable.summoned);
    if let Some(pet) = pet {
        clear_entry_auras(world, pet);
    }
    reset_resources(world, entity);
    crate::spell_cast::reset_arena_cooldowns(world, entity);
    world.entity_mut(entity).remove::<PreparationResetAt>();
}

/// TC Unit.cpp4445: remove entering-arena buffs/debuffs by source attributes,
/// preserving passives, entering-arena stances and negative death-persistent auras.
fn clear_entry_auras(world: &mut World, entity: Entity) {
    const ALLOW_ENTERING_ARENA: u32 = 0x0020_0000;
    const ALLOW_WHILE_DEAD: u32 = 0x0010_0000;
    const REMOVE_ENTERING_ARENA: u32 = 0x4;
    let Some(auras) = world.get::<crate::auras::ActiveAuras>(entity) else {
        return;
    };
    let spells = world.resource::<crate::spell_info::SpellStore>();
    let removed: std::collections::BTreeSet<_> = auras
        .iter()
        .filter(|aura| {
            let spell = spells
                .get(aura.spell_id)
                .expect("applied aura source spell");
            let attributes = spell.misc.as_ref().map_or([0; 17], |misc| misc.attributes);
            let protected = aura.passive || attributes[4] & ALLOW_ENTERING_ARENA != 0;
            let death_persistent = aura.harmful && attributes[3] & ALLOW_WHILE_DEAD != 0;
            (!protected && !death_persistent) || attributes[5] & REMOVE_ENTERING_ARENA != 0
        })
        .map(|aura| aura.instance_id)
        .collect();
    crate::auras::remove_auras(
        world,
        entity,
        crate::auras::RemoveReason::Cancelled,
        |aura| removed.contains(&aura.instance_id),
    );
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
pub(super) fn reset_resources(world: &mut World, entity: Entity) {
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
    fn arena_entry_removes_prior_round_shadow_sight_and_delivers_native_cooldown_clear() {
        let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[498, 34709], 2).unwrap();
        h.cast_and_deliver(498, Some(h.caster));
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(498)
            .unwrap()
            .clone();
        assert!(
            h.app
                .world()
                .get::<crate::spell_cast::SpellHistory>(h.caster)
                .unwrap()
                .on_cooldown(&spell, crate::spell_cast::history_now(h.app.world()))
        );
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id: 34709,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert!(crate::auras::has_aura(h.app.world(), h.caster, 34709));
        h.inbox.cooldowns.clear();
        reset_entry(h.app.world_mut(), h.caster);
        h.advance(1);
        assert!(
            !crate::auras::has_aura(h.app.world(), h.caster, 34709),
            "prior-round Shadow Sight cannot persist into the new source preparation"
        );
        assert!(
            h.inbox
                .cooldowns
                .iter()
                .any(|update| update.spell_id == 498 && !update.is_gcd && update.remaining_ms == 0)
        );
    }

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
