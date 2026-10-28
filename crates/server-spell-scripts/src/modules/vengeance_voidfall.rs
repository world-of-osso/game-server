//! Vengeance Voidfall builder and spending events.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};

const VOIDFALL: u32 = 1253304;
const MASS_ACCELERATION: u32 = 1256295;
const METEORIC_FALL: u32 = 1253391;
const WORLD_KILLER: u32 = 1256353;
const BUILDING: u32 = 1256301;
const SPENDING: u32 = 1256302;
const FINAL_HOUR: u32 = 1256322;
const METEOR: u32 = 1256303;
const WORLD_KILLER_MISSILE: u32 = 1256616;

#[derive(Component, Default)]
struct VoidfallAttempts(u32);
struct PendingMeteor {
    at: f64,
    target: Entity,
    spell: u32,
}
#[derive(Component, Default)]
struct PendingMeteors(Vec<PendingMeteor>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeLaunch(launch),
    SpellScriptHook::AfterNativeCastUpdate(update),
];

fn selected(world: &World, caster: Entity, spell: u32) -> bool {
    world
        .get::<combat::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == spell)
        })
}
fn effect(world: &World, spell: u32, index: u8) -> f32 {
    world
        .resource::<SpellStore>()
        .get(spell)
        .and_then(|s| s.effects.iter().find(|e| e.index == index))
        .expect("native Voidfall effect")
        .base_points
}
fn own_stack(world: &World, caster: Entity, spell: u32) -> Option<(u8, u8)> {
    world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|a| a.spell_id == spell && a.caster == caster)
        })
        .map(|a| (a.stacks, a.max_stacks))
}
fn apply(world: &mut World, caster: Entity, spell: u32) {
    combat::auras::apply_aura(
        world,
        &combat::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: combat::auras::AuraSource::Cast,
        },
    );
}
fn add_building(world: &mut World, caster: Entity, count: u8) {
    if count == 0 {
        return;
    }
    apply(world, caster, BUILDING);
    let (stacks, max) = own_stack(world, caster, BUILDING).expect("Voidfall building aura");
    let previous = stacks.saturating_sub(1);
    let next = previous.saturating_add(count).min(max);
    assert!(combat::auras::set_self_cast_stacks(
        world, caster, BUILDING, next
    ));
    if next == max && max > 0 {
        combat::auras::remove_auras(world, caster, combat::auras::RemoveReason::Consumed, |a| {
            a.spell_id == BUILDING && a.caster == caster
        });
        apply(world, caster, SPENDING);
        assert!(combat::auras::set_self_cast_stacks(
            world, caster, SPENDING, max
        ));
    }
}
fn roll_builder(world: &mut World, caster: Entity) {
    if own_stack(world, caster, SPENDING).is_some() {
        return;
    }
    let chance = f64::from(effect(world, VOIDFALL, 2)) / 100.0;
    let coefficient = combat::random::accumulated_proc_constant(chance);
    let mut attempts = world
        .entity_mut(caster)
        .entry::<VoidfallAttempts>()
        .or_default()
        .into_mut();
    attempts.0 += 1;
    let success = combat::random::random::<f64>() < coefficient * f64::from(attempts.0);
    if success {
        attempts.0 = 0;
    }
    drop(attempts);
    if success {
        add_building(world, caster, effect(world, VOIDFALL, 0) as u8);
    }
}
fn launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !selected(world, caster, VOIDFALL) {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp2977–3003,3141–3170,9336–9354.
    match spell.id {
        263642 => roll_builder(world, caster),
        187827 if selected(world, caster, MASS_ACCELERATION) => {
            add_building(world, caster, effect(world, MASS_ACCELERATION, 0) as u8)
        }
        228478 | 247455 => spend(world, caster, spell),
        _ => {}
    }
}
fn spend(world: &mut World, caster: Entity, spell: &SpellInfo) {
    let Some((stacks, maximum)) = own_stack(world, caster, SPENDING) else {
        return;
    };
    let Some(target) = spell.cast_primary_target else {
        error!(?caster, "Voidfall spender has no target");
        return;
    };
    let meteor_fall = selected(world, caster, METEORIC_FALL) && stacks == maximum;
    if spell.id == 247455 && !meteor_fall {
        return;
    }
    let world_killer = selected(world, caster, WORLD_KILLER);
    let count = if meteor_fall { stacks } else { 1 };
    let delay = world
        .resource::<SpellStore>()
        .get(METEOR)
        .and_then(|s| s.misc.as_ref())
        .expect("native meteor timing")
        .speed as f64;
    let now = spell_cast::now(world);
    for i in 0..count {
        let id = if world_killer && i == count - 1 && (meteor_fall || stacks == 1) {
            WORLD_KILLER_MISSILE
        } else {
            METEOR
        };
        world
            .entity_mut(caster)
            .entry::<PendingMeteors>()
            .or_default()
            .into_mut()
            .0
            .push(PendingMeteor {
                at: now + f64::from(i) * delay,
                target,
                spell: id,
            });
    }
}
fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let actors: Vec<_> = world
        .query_filtered::<Entity, With<PendingMeteors>>()
        .iter(world)
        .collect();
    for caster in actors {
        let due = {
            let mut pending = world.get_mut::<PendingMeteors>(caster).unwrap();
            let (due, later) = std::mem::take(&mut pending.0)
                .into_iter()
                .partition(|m| m.at <= now);
            pending.0 = later;
            due
        };
        for meteor in due {
            launch_meteor(world, caster, meteor);
        }
    }
}
fn launch_meteor(world: &mut World, caster: Entity, meteor: PendingMeteor) {
    let Some((stacks, _)) = own_stack(world, caster, SPENDING) else {
        return;
    };
    let Some(position) = spell_cast::targets::position(world, meteor.target) else {
        error!(?caster, "Voidfall target position missing");
        return;
    };
    // SimC76f8118 sc_demon_hunter.cpp6584–6596 consumes one stack at missile execution.
    spell_cast::cast_triggered_at_destination(world, caster, meteor.spell, position, 1);
    if stacks == 1 {
        combat::auras::remove_auras(world, caster, combat::auras::RemoveReason::Consumed, |a| {
            a.spell_id == SPENDING && a.caster == caster
        });
    } else {
        assert!(combat::auras::set_self_cast_stacks(
            world,
            caster,
            SPENDING,
            stacks - 1
        ));
    }
    if selected(world, caster, 1253805) {
        apply(world, caster, FINAL_HOUR);
    }
}
