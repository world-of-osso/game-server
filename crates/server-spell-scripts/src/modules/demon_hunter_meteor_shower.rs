//! Annihilator Dark Matter earns a ground-targeted Meteor Shower.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

const DARK_MATTER: u32 = 1256307;
const DARK_MATTER_BUFF: u32 = 1256308;
struct Shower {
    position: Vec3,
    damage: u32,
    next: f64,
    remaining: u32,
}
#[derive(Component, Default)]
struct Showers(Vec<Shower>);
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeLaunch(launch),
    SpellScriptHook::AfterNativeCastUpdate(update),
];
fn selected(world: &World, caster: Entity) -> bool {
    world
        .get::<combat::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == DARK_MATTER)
        })
}
fn launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !selected(world, caster) {
        return;
    }
    let spec = world.get::<ActiveSpec>(caster).map(|spec| spec.0);
    match (spec, spell.id) {
        (Some(581), 187827) | (Some(1480), 1217605) => {
            // SimC76f8118 sc_demon_hunter.cpp4960,5027: actual Meta earns Dark Matter.
            combat::auras::apply_aura(
                world,
                &combat::auras::AuraApplication {
                    caster,
                    target: caster,
                    spell_id: DARK_MATTER_BUFF,
                    effect_mask: u32::MAX,
                    source: combat::auras::AuraSource::Cast,
                },
            );
        }
        (Some(581), 247455) => begin_shower(world, caster, spell, 1264130),
        (Some(1480), 1221162) => begin_shower(world, caster, spell, 1264129),
        _ => {}
    }
}
fn begin_shower(world: &mut World, caster: Entity, spell: &SpellInfo, damage: u32) {
    if !combat::auras::has_aura(world, caster, DARK_MATTER_BUFF) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        error!(?caster, "Meteor Shower target missing");
        return;
    };
    let Some(position) = spell_cast::targets::position(world, target) else {
        error!(?caster, "Meteor Shower target position missing");
        return;
    };
    let count = world
        .resource::<SpellStore>()
        .get(DARK_MATTER)
        .and_then(|spell| spell.effects.iter().find(|effect| effect.index == 0))
        .expect("native Dark Matter pulse count")
        .base_points;
    assert!(
        count > 0.0 && count.fract() == 0.0,
        "invalid native Meteor Shower count"
    );
    combat::auras::remove_auras(
        world,
        caster,
        combat::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == DARK_MATTER_BUFF && aura.caster == caster,
    );
    // SimC76f8118 sc_demon_hunter.cpp3177–3198,6652–6694: the first
    // ground pulse is 0.5s after the spender and there are effect1 pulses.
    let next = spell_cast::now(world) + 0.5;
    world
        .entity_mut(caster)
        .entry::<Showers>()
        .or_default()
        .into_mut()
        .0
        .push(Shower {
            position,
            damage,
            next,
            remaining: count as u32,
        });
}
fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let actors: Vec<_> = world
        .query_filtered::<Entity, With<Showers>>()
        .iter(world)
        .collect();
    for actor in actors {
        let due = {
            let mut showers = world.get_mut::<Showers>(actor).unwrap();
            let mut due = Vec::new();
            for shower in &mut showers.0 {
                while shower.remaining > 0 && shower.next <= now {
                    due.push((shower.position, shower.damage));
                    shower.remaining -= 1;
                    shower.next += 0.5;
                }
            }
            showers.0.retain(|shower| shower.remaining > 0);
            due
        };
        for (position, spell) in due {
            spell_cast::cast_triggered_at_destination(world, actor, spell, position, 1);
        }
    }
}
