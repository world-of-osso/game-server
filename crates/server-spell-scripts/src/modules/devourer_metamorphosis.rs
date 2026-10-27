//! Source-qualified Devourer Metamorphosis entry.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{self, ActiveAuras, AuraInstance, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    self, powers, script_registry::SpellScriptHook, scripts::script_effect_value,
};
use shared::casting::{CastState, CastType};
use shared::components::{ActiveSpec, CombatStatus, PowerType};

const META: u32 = 1217607;
const VOID_BUILDUP: u32 = 473671;
const SOUL_GLUTTON: u32 = 1247534;
const VOIDRUSH: u32 = 1272422;
const VOID_RAY: u32 = 473728;
const COLLAPSING_STAR: u32 = 1221150;

#[derive(Component)]
struct DrainDeadline(f64);

#[derive(Component)]
struct VoidrushUntil(f64);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::OnCast(trigger_voidrush),
    SpellScriptHook::AfterNativeCastUpdate(update_drain),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        META,
        AuraScript {
            on_apply: Some(grant_entry_fury),
            ..Default::default()
        },
    );
}

fn trigger_voidrush(world: &mut World, caster: Entity, spell: u32, _: u8) {
    if !matches!(spell, 1245412 | 1245483 | 1246167 | 1259431)
        || !world.get::<KnownSpells>(caster).is_some_and(|known| known.contains(VOIDRUSH))
    {
        return;
    }
    // SimC76f8118 cpp3223–3239,9940–9943: these actions trigger a 0.5s Voidrush window.
    let until = spell_cast::now(world) + 0.5;
    world.entity_mut(caster).insert(VoidrushUntil(until));
}

fn grant_entry_fury(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if unit != aura.caster || world.get::<ActiveSpec>(unit).map(|spec| spec.0) != Some(1480) {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp9192–9195 consumes the soul stack on entry.
    // The native admission aura represents that same readiness.
    auras::remove_auras(world, unit, RemoveReason::Consumed, |aura| {
        aura.caster == unit && matches!(aura.spell_id, 1225789 | 1213809)
    });
    // SimC76f8118 sc_demon_hunter.cpp12445–12452 grants 200 Fury on Meta entry.
    // Native PowerPools owns the cap and the actual applied amount.
    if powers::change_power(world, unit, PowerType::Fury, 200.0).is_none() {
        error!(?unit, "Devourer Meta entry has no Fury pool");
    }
    let first = spell_cast::now(world) + drain_interval(world, unit, 0);
    world.entity_mut(unit).insert(DrainDeadline(first));
}

/// SimC76f8118 sc_demon_hunter.cpp10339–10343: base drain before situational reductions.
pub fn devourer_base_drain_ps(world: &World, unit: Entity) -> f64 {
    let stacks = auras::paid_periodic_ticks(world, unit, unit, META);
    base_drain_ps(world, unit, stacks)
}

fn base_drain_ps(world: &World, unit: Entity, stacks: u64) -> f64 {
    let mut rate = 15.0 + 1.40 * (0.0775 * stacks as f64).exp();
    if world
        .get::<KnownSpells>(unit)
        .is_some_and(|known| known.contains(SOUL_GLUTTON))
    {
        rate /= 1.0 + f64::from(script_effect_value(world, unit, SOUL_GLUTTON, 1)) / 100.0;
    }
    rate
}

// SimC76f8118 sc_demon_hunter.cpp1424–1478,12462–12500: the fitted
// rate schedules native473671 Fury events; a pending event is never retimed.
fn drain_interval(world: &World, unit: Entity, stacks: u64) -> f64 {
    let amount = -f64::from(script_effect_value(world, unit, VOID_BUILDUP, 0));
    assert!(amount > 0.0, "Void Buildup must spend native Fury");
    let mut rate = base_drain_ps(world, unit, stacks);
    let stunned = world
        .get::<ActiveAuras>(unit)
        .is_some_and(ActiveAuras::is_stunned);
    let out_of_combat = !world
        .get::<CombatStatus>(unit)
        .is_some_and(|status| status.0);
    if out_of_combat
        || stunned
        || world.get::<VoidrushUntil>(unit).is_some_and(|window| window.0 > spell_cast::now(world))
        || is_protected_cast(world, unit)
    {
        rate *= 0.1;
    }
    if stacks < 1 {
        rate = 10.0;
    }
    (amount / rate).clamp(0.0, 1.0)
}

fn is_protected_cast(world: &World, unit: Entity) -> bool {
    world.get::<CastState>(unit).is_some_and(|cast| {
        (cast.cast_type == CastType::Channel && cast.spell_id == VOID_RAY)
            || (cast.cast_type == CastType::Normal && cast.spell_id == COLLAPSING_STAR)
    })
}

fn update_drain(world: &mut World) {
    let now = spell_cast::now(world);
    let units: Vec<_> = world
        .query_filtered::<Entity, With<DrainDeadline>>()
        .iter(world)
        .collect();
    for unit in units {
        while let Some(deadline) = world.get::<DrainDeadline>(unit).map(|pending| pending.0) {
            if !auras::has_aura(world, unit, META) {
                world.entity_mut(unit).remove::<DrainDeadline>();
                break;
            }
            if deadline > now {
                break;
            }
            // SimC sc_demon_hunter.cpp12521–12525 consumes the native
            // Void Buildup resource amount rather than casting its energize effect.
            let amount = script_effect_value(world, unit, VOID_BUILDUP, 0);
            powers::change_power(world, unit, PowerType::Fury, amount)
                .expect("Devourer Meta requires Fury pool");
            let fury = powers::power_value(world, unit, PowerType::Fury)
                .expect("Devourer Meta requires Fury pool")
                .0;
            if fury <= 0.0 && !is_protected_cast(world, unit) {
                auras::remove_auras(world, unit, RemoveReason::Expired, |aura| {
                    aura.spell_id == META && aura.caster == unit
                });
                world.entity_mut(unit).remove::<DrainDeadline>();
                break;
            }
            let stacks = auras::paid_periodic_ticks(world, unit, unit, META);
            let next = deadline + drain_interval(world, unit, stacks);
            world
                .get_mut::<DrainDeadline>(unit)
                .expect("active drain")
                .0 = next;
        }
    }
}
