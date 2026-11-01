//! SimC76f8118 Annihilator Voidfall building and meteor spending.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::random::Rng;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::{SpellInfo, SpellStore};

const VOIDFALL: u32 = 1253304;
const BUILDING: u32 = 1256301;
const SPENDING: u32 = 1256302;
const METEOR: u32 = 1256305;
const WORLD_KILLER: u32 = 1256619;

#[derive(Component, Default)]
struct VoidfallState {
    failures: u32,
    pending: Vec<Impact>,
}
struct Impact {
    at: f64,
    target: Entity,
    spell: u32,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeLaunch(after_launch),
    SpellScriptHook::AfterNativeCastUpdate(update),
];

fn selected(world: &World, actor: Entity, spell: u32) -> bool {
    world
        .get::<KnownSpells>(actor)
        .is_some_and(|known| known.contains(spell))
}

fn stacks(world: &World, actor: Entity, spell: u32) -> u8 {
    world
        .get::<ActiveAuras>(actor)
        .and_then(|auras| {
            auras
                .iter()
                .find(|a| a.caster == actor && a.spell_id == spell)
        })
        .map_or(0, |a| a.stacks)
}

fn grant_building(world: &mut World, actor: Entity, count: usize) {
    if !selected(world, actor, VOIDFALL) {
        return;
    }
    for _ in 0..count {
        assert!(
            spell_cast::try_cast_triggered(world, actor, BUILDING, Some(actor), 1),
            "native Voidfall building"
        );
        let maximum = world
            .get::<ActiveAuras>(actor)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|a| a.caster == actor && a.spell_id == BUILDING)
            })
            .expect("Voidfall building aura")
            .max_stacks;
        if stacks(world, actor, BUILDING) >= maximum {
            auras::remove_auras(world, actor, RemoveReason::Consumed, |a| {
                a.caster == actor && a.spell_id == BUILDING
            });
            for _ in 0..maximum {
                assert!(
                    spell_cast::try_cast_triggered(world, actor, SPENDING, Some(actor), 1),
                    "native Voidfall spending"
                );
            }
        }
    }
}

pub(super) fn complete_ray(world: &mut World, actor: Entity) {
    // SimC76f8118 cpp6412–6415: Meteoric Rise adds one building stack.
    if selected(world, actor, 1253377) {
        grant_building(world, actor, 1);
    }
}

fn after_launch(world: &mut World, actor: Entity, spell: &SpellInfo) {
    if !selected(world, actor, VOIDFALL) {
        return;
    }
    match spell.id {
        // SimC cpp4866,3141–3168: Meta triggers Mass Acceleration.
        1217605 if selected(world, actor, 1256295) => {
            let count = script_effect_value(world, actor, 1256295, 0) as usize;
            grant_building(world, actor, count);
            spell_cast::scripts::reduce_cooldown(world, actor, 1226019, f64::MAX);
        }
        // SimC cpp5664,2977–3005: Consume and Devour roll accumulated PRD.
        473662 | 1217610 => roll_builder(world, actor),
        // SimC cpp5981–5982,6110–6111: Reap/Cull/Eradicate spend meteors.
        1226019 | 1245453 | 1225826 => spend(
            world,
            actor,
            spell.cast_primary_target.expect("Voidfall spender target"),
        ),
        _ => {}
    }
}

fn prd_constant(probability: f64) -> f64 {
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..48 {
        let c = (low + high) / 2.0;
        let mut survival = 1.0;
        let mut expected = 0.0;
        for attempt in 1..=1000 {
            expected += survival;
            survival *= 1.0 - (c * f64::from(attempt)).min(1.0);
            if survival == 0.0 {
                break;
            }
        }
        if 1.0 / expected < probability {
            low = c;
        } else {
            high = c;
        }
    }
    (low + high) / 2.0
}

fn roll_builder(world: &mut World, actor: Entity) {
    if stacks(world, actor, SPENDING) != 0 {
        return;
    }
    let chance = f64::from(script_effect_value(world, actor, VOIDFALL, 2)) / 100.0;
    let failures = world.get::<VoidfallState>(actor).map_or(0, |s| s.failures);
    let proc = combat::random::thread_rng().gen_range(0.0..1.0)
        < (prd_constant(chance) * f64::from(failures + 1)).min(1.0);
    world
        .entity_mut(actor)
        .entry::<VoidfallState>()
        .or_default()
        .into_mut()
        .failures = if proc { 0 } else { failures + 1 };
    if proc {
        grant_building(
            world,
            actor,
            script_effect_value(world, actor, VOIDFALL, 0) as usize,
        );
    }
}

fn spend(world: &mut World, actor: Entity, target: Entity) {
    let count = stacks(world, actor, SPENDING);
    if count == 0 {
        return;
    }
    let fall = selected(world, actor, 1253391) && count == 3;
    let world_killer = selected(world, actor, 1256353);
    let launches = if fall { count } else { 1 };
    let now = spell_cast::now(world);
    let delay = f64::from(
        world
            .resource::<SpellStore>()
            .get(1256304)
            .expect("native meteor")
            .misc
            .as_ref()
            .expect("meteor misc")
            .speed,
    );
    let world_delay = f64::from(
        world
            .resource::<SpellStore>()
            .get(1256618)
            .expect("native World Killer")
            .misc
            .as_ref()
            .expect("World Killer misc")
            .speed,
    );
    for i in 0..launches {
        let final_meteor = world_killer && (if fall { i + 1 == launches } else { count == 1 });
        let impact = Impact {
            at: now + f64::from(i) * delay + if final_meteor { world_delay } else { delay },
            target,
            spell: if final_meteor { WORLD_KILLER } else { METEOR },
        };
        world
            .entity_mut(actor)
            .entry::<VoidfallState>()
            .or_default()
            .into_mut()
            .pending
            .push(impact);
    }
    let remaining = count - launches;
    if remaining == 0 {
        auras::remove_auras(world, actor, RemoveReason::Consumed, |a| {
            a.caster == actor && a.spell_id == SPENDING
        });
    } else {
        assert!(
            auras::set_owned_stacks(world, actor, actor, SPENDING, remaining),
            "native Voidfall stacks"
        );
    }
    if selected(world, actor, 1253805) {
        for _ in 0..launches {
            assert!(
                spell_cast::try_cast_triggered(world, actor, 1256322, Some(actor), 1),
                "native Final Hour"
            );
        }
    }
}

fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let actors: Vec<_> = world
        .query_filtered::<Entity, With<VoidfallState>>()
        .iter(world)
        .collect();
    for actor in actors {
        let due = {
            let mut state = world.get_mut::<VoidfallState>(actor).unwrap();
            let (due, later) = std::mem::take(&mut state.pending)
                .into_iter()
                .partition(|impact| impact.at <= now);
            state.pending = later;
            due
        };
        for impact in due {
            if !spell_cast::targets::is_alive(world, impact.target) {
                continue;
            }
            if !spell_cast::try_cast_triggered(world, actor, impact.spell, Some(impact.target), 1) {
                error!(
                    ?actor,
                    spell = impact.spell,
                    "native Voidfall impact refused"
                );
            }
            if impact.spell == WORLD_KILLER {
                let count = script_effect_value(world, actor, 1256353, 3) as usize;
                for _ in 0..count {
                    combat::soul_fragments::spawn(
                        world,
                        actor,
                        combat::soul_fragments::FragmentKind::Lesser,
                        false,
                    )
                    .expect("native World Killer fragment");
                }
            }
        }
    }
}
