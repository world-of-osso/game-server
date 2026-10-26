//! Native MID2 Unstable Empowerment follows actual owned UA stack lifetimes.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

const FOUR_PIECE: u32 = 1296569;
const UNSTABLE_AFFLICTION: u32 = 1259790;
const EMPOWERMENT: u32 = 1305774;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AfterAuraApplication(after_application),
];

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(UNSTABLE_AFFLICTION).or_default().on_remove = Some(after_removal);
}

fn after_application(world: &mut World, _target: Entity, aura: &AuraInstance) {
    // SimC76f8118 actions1567–1568: every actual UA impact updates this buff.
    if aura.spell_id == UNSTABLE_AFFLICTION {
        sync(world, aura.caster);
    }
}

fn after_removal(world: &mut World, _target: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    sync(world, aura.caster);
}

fn read_owned_stacks(world: &World, caster: Entity) -> u8 {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.caster == caster && aura.spell_id == EMPOWERMENT)
        })
        .map_or(0, |aura| aura.stacks)
}

// SimC76f8118 actions5673–5699: count active owned UA stacks globally,
// stop at the native buff cap, and do not count sleeping/dead recipients.
fn count_active_stacks(world: &mut World, caster: Entity, cap: u8) -> u8 {
    let mut query = world.query::<(Entity, &ActiveAuras)>();
    let mut total = 0_u32;
    for (target, auras) in query.iter(world) {
        if !combat::spell_cast::targets::is_alive(world, target) {
            continue;
        }
        total += auras
            .iter()
            .filter(|aura| aura.caster == caster && aura.spell_id == UNSTABLE_AFFLICTION)
            .map(|aura| u32::from(aura.stacks))
            .sum::<u32>();
        if total >= u32::from(cap) {
            return cap;
        }
    }
    total as u8
}

fn read_desired_stacks(world: &mut World, caster: Entity) -> u8 {
    if !combat::auras::has_aura(world, caster, FOUR_PIECE) {
        return 0;
    }
    let spell = world
        .resource::<SpellStore>()
        .get(EMPOWERMENT)
        .expect("native MID2 Unstable Empowerment spell is required");
    let cap = combat::auras::fresh_max_stacks(world, caster, spell);
    count_active_stacks(world, caster, cap)
}

fn apply_earned_stacks(world: &mut World, caster: Entity, current: u8, desired: u8) {
    if desired == 0 {
        combat::auras::remove_auras(world, caster, RemoveReason::Expired, |aura| {
            aura.caster == caster && aura.spell_id == EMPOWERMENT
        });
        return;
    }
    if current == 0 {
        combat::spell_cast::cast_triggered(world, caster, EMPOWERMENT, Some(caster), 1);
    }
    assert!(
        combat::auras::set_self_cast_stacks(world, caster, EMPOWERMENT, desired),
        "native Unstable Empowerment must attach before setting its earned stacks"
    );
}

pub(super) fn sync(world: &mut World, caster: Entity) {
    if world
        .get::<ActiveSpec>(caster)
        .is_none_or(|spec| spec.0 != 265)
    {
        return;
    }
    let desired = read_desired_stacks(world, caster);
    let current = read_owned_stacks(world, caster);
    if desired != current {
        apply_earned_stacks(world, caster, current, desired);
    }
}
