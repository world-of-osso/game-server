//! SimC76f8118 sc_priest_shadow.cpp1491–1514/sc_priest.cpp1606–1683/3543–3562.
//! Native Torrent starts a hasted Rift driver at its actual hostile primary.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason};
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook, TargetPlanHook};
use combat::spell_cast::{self, PlannedEffect, TargetPlan, targets};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHit(effect_hit),
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::TargetPlan(target_plan),
];
const TORRENT: u32 = 263165;
const DRIVER: u32 = 459314;
const OBJECT: u32 = 447445;
const DAMAGE: u32 = 447448;

#[derive(Component)]
struct Rift {
    primary: Entity,
    ticks: u32,
}

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    (spell == TORRENT && index == 0).then_some(start_rift)
}

fn start_rift(world: &mut World, caster: Entity, primary: Entity, depth: u8) {
    let shadow = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 258);
    if !shadow {
        return;
    }
    // The actual enemy effect supplies its recipient even when no controller selected
    // a separate cast_primary_target. Caster-side Torrent auras are not producers.
    // SimC1677–1681 expires the old window before starting the new driver clock.
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == DRIVER && aura.caster == caster
    });
    world.entity_mut(caster).insert(Rift { primary, ticks: 0 });
    spell_cast::cast_triggered(world, caster, DRIVER, Some(caster), depth + 1);
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        DRIVER,
        AuraScript {
            on_periodic: Some(pulse),
            on_remove: Some(clear_rift),
            ..Default::default()
        },
    );
    scripts.0.entry(TORRENT).or_default().on_remove = Some(finish_channel);
}

fn pulse(world: &mut World, unit: Entity, _: &AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    let Some(mut rift) = world.get_mut::<Rift>(unit) else {
        return;
    };
    rift.ticks += 1;
    // SimC3555–3562: the first travelling tick cannot reliably reach enemies.
    if rift.ticks < 2 {
        return;
    }
    let primary = rift.primary;
    spell_cast::cast_triggered(world, unit, DAMAGE, Some(primary), 1);
}

fn clear_rift(world: &mut World, unit: Entity, _: &AuraInstance, _: RemoveReason) {
    world.entity_mut(unit).remove::<Rift>();
}

fn finish_channel(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    // The root has both enemy and caster auras; perform last_tick once at the caster.
    if unit != aura.caster {
        return;
    }
    let duration = world.get::<ActiveAuras>(unit).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == unit && aura.spell_id == DRIVER)
            .map(|aura| aura.duration_ms)
    });
    if let Some(duration) = duration {
        // SimC1491–1496: channel end restores full remaining lifetime, not tick phase.
        auras::set_owned_duration(world, unit, unit, DRIVER, duration);
    }
}

fn target_plan(spell: u32) -> Option<(targets::Relation, TargetPlanHook)> {
    (spell == DAMAGE).then_some((targets::Relation::Enemy, plan_pulse))
}

fn plan_pulse(
    world: &mut World,
    caster: Entity,
    _: &SpellInfo,
    primary: Option<Entity>,
) -> TargetPlan {
    let primary = primary.expect("native Rift damage retains source primary");
    let Some(center) = targets::position(world, primary) else {
        return Vec::new();
    };
    let radius = read_base_radius(world);
    let units = targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    vec![PlannedEffect {
        index: 0,
        units,
        dest: None,
        chain_multipliers: Vec::new(),
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

fn read_base_radius(world: &World) -> f32 {
    // SimC1611: the native object radius, halved, not the explicit damage row's range.
    world
        .resource::<SpellStore>()
        .get(OBJECT)
        .expect("native Rift object")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Rift radius effect")
        .radius[0]
        .expect("native Rift radius")
        .radius_max
        / 2.0
}
