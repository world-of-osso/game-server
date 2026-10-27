//! SimC76f8118 actions2980–3170/pets919–985: actual Imp selection and retirement.
use super::{self as guardian, super as cast};
use crate::random::Rng;
use bevy::prelude::*;
use cast::summon::SummonedCreature;
use shared::components::{Npc, PowerType};

const AVAILABILITY: u32 = 296553;
const IMPLOSION: u32 = 196277;
const EXPLOSION: u32 = 196278;
const IMP_ROOTS: &[(u32, u32)] = &[(104317, 55659), (279910, 143622)];
// pets.cpp1846/2583/2707/2802 excludes Dominion and Diabolist demons.
const EXCLUDED: &[u32] = &[260076, 260077, 258242, 255147, 228574, 228575, 228576];

#[derive(Component)]
struct SpawnOrder(u64);
#[derive(Resource, Default)]
struct NextSpawnOrder(u64);

#[derive(Component)]
struct Imploding {
    owner: Entity,
    primary: Entity,
    due: f64,
}

pub(super) fn is_imp(world: &World, unit: Entity) -> bool {
    let Some(summon) = world.get::<SummonedCreature>(unit) else {
        return false;
    };
    world
        .get::<Npc>(unit)
        .is_some_and(|npc| IMP_ROOTS.contains(&(summon.spell_id, npc.template_id)))
}

pub(super) fn on_spawn(world: &mut World, unit: Entity) {
    if !is_imp(world, unit) {
        return;
    }
    let owner = world.get::<SummonedCreature>(unit).unwrap().owner;
    let order = {
        let mut next = world.get_resource_or_init::<NextSpawnOrder>();
        next.0 += 1;
        next.0
    };
    world.entity_mut(unit).insert(SpawnOrder(order));
    cast::cast_triggered(world, owner, AVAILABILITY, Some(owner), 0);
}

pub(in crate::spell_cast) fn on_remove(world: &mut World, unit: Entity) {
    if !is_imp(world, unit) {
        return;
    }
    let owner = world.get::<SummonedCreature>(unit).unwrap().owner;
    let others = owned_imps(world, owner).into_iter().any(|imp| imp != unit);
    if !others {
        crate::auras::remove_auras(
            world,
            owner,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == AVAILABILITY && aura.caster == owner,
        );
    }
}

fn owned_imps(world: &mut World, owner: Entity) -> Vec<Entity> {
    let units: Vec<_> = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .filter(|(unit, summon)| summon.owner == owner && cast::targets::is_alive(world, *unit))
        .map(|(unit, _)| unit)
        .collect();
    units
        .into_iter()
        .filter(|unit| is_imp(world, *unit))
        .collect()
}

pub(in crate::spell_cast) fn after_cast(
    world: &mut World,
    owner: Entity,
    spell: &crate::spell_info::SpellInfo,
) {
    if spell.id != IMPLOSION {
        return;
    }
    let Some(primary) = spell.cast_primary_target else {
        return;
    };
    let mut imps = owned_imps(world, owner);
    let candidates = count_candidates(world, owner);
    let cap = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points as usize;
    let selected = candidates.min(cap);
    let count = sample_imps(imps.len(), candidates, selected);
    imps.sort_by(|a, b| {
        energy(world, *a).total_cmp(&energy(world, *b)).then(
            world
                .get::<SpawnOrder>(*a)
                .unwrap()
                .0
                .cmp(&world.get::<SpawnOrder>(*b).unwrap().0),
        )
    });
    for (index, imp) in imps.into_iter().take(count).enumerate() {
        // Source actions3110–3147:65yd/s travel and50ms per selected Imp.
        let from = cast::targets::position(world, imp).unwrap();
        let to = cast::targets::position(world, primary).unwrap();
        let due = cast::now(world) + f64::from(from.distance(to)) / 65.0 + index as f64 * 0.05;
        cast::cancel_cast(world, imp);
        world.entity_mut(imp).insert(Imploding {
            owner,
            primary,
            due,
        });
    }
    summon_replacements(world, owner, selected);
}

fn energy(world: &World, imp: Entity) -> f32 {
    cast::powers::power_value(world, imp, PowerType::Energy)
        .expect("native Imp Energy")
        .0
}

fn count_candidates(world: &mut World, owner: Entity) -> usize {
    world
        .query::<(Entity, &crate::hunter_pet::SummonedDemon, &Npc)>()
        .iter(world)
        .filter(|(unit, _, npc)| {
            let controller = world
                .get::<SummonedCreature>(*unit)
                .map(|summon| summon.owner)
                .or_else(|| {
                    world
                        .get::<crate::hunter_pet::HunterPet>(*unit)
                        .map(|pet| pet.owner)
                });
            controller == Some(owner)
                && cast::targets::is_alive(world, *unit)
                && !EXCLUDED.contains(&npc.template_id)
        })
        .count()
}

fn sample_imps(mut imps: usize, mut candidates: usize, selected: usize) -> usize {
    let mut count = 0;
    for _ in 0..selected {
        if imps == 0 {
            break;
        }
        if crate::random::thread_rng().gen_bool(imps as f64 / candidates as f64) {
            count += 1;
            imps -= 1;
        }
        candidates -= 1;
    }
    count
}

fn summon_replacements(world: &mut World, owner: Entity, selected: usize) {
    const TO_HELL: u32 = 1281511;
    let knows = world
        .get::<crate::class_progression::KnownSpells>(owner)
        .is_some_and(|known| known.contains(TO_HELL));
    if !knows {
        return;
    }
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(TO_HELL)
        .unwrap();
    let value = |index| {
        spell
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .unwrap()
            .base_points as usize
    };
    let count = selected / value(1) * value(0);
    for _ in 0..count {
        let before = owned_imps(world, owner);
        cast::cast_triggered(world, owner, 279910, Some(owner), 0);
        for imp in owned_imps(world, owner)
            .into_iter()
            .filter(|imp| !before.contains(imp))
        {
            // actions3164–3169: actual replacement actors receive both buffs.
            cast::cast_triggered(world, imp, 1250772, Some(imp), 0);
            cast::cast_triggered(world, imp, 1281512, Some(imp), 0);
        }
    }
}

/// Called before normal guardian decisions; an airborne Imp cannot cast again.
pub(super) fn advance(world: &mut World, imp: Entity) -> bool {
    let Some(state) = world.get::<Imploding>(imp) else {
        return false;
    };
    let (owner, primary, due) = (state.owner, state.primary, state.due);
    if cast::now(world) < due {
        return true;
    }
    if cast::targets::is_alive(world, primary) {
        explode(world, imp, owner, primary);
    }
    guardian::core::on_implosion(world, owner);
    cast::summon::despawn_summon(world, imp);
    true
}

fn explode(world: &mut World, imp: Entity, owner: Entity, primary: Entity) {
    let mut spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(EXPLOSION)
        .unwrap()
        .clone();
    spell.cast_destination =
        cast::targets::position(world, primary).map(|position| position.to_array());
    let mut plan = cast::plan_targets(world, owner, &spell, Some(primary), false)
        .expect("native Implosion explosion targets");
    let primary_multiplier = energy(world, imp) / 100.0;
    for effect in &mut plan {
        effect.chain_multipliers = effect
            .units
            .iter()
            .map(|unit| {
                (
                    *unit,
                    if *unit == primary {
                        primary_multiplier
                    } else {
                        1.0
                    },
                )
            })
            .collect();
    }
    cast::missiles::launch(world, owner, &spell, plan, 1);
}
