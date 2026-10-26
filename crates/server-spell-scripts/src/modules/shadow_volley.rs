//! SimC76f8118 sc_priest_shadow.cpp960–1079/1380–1401/1829–1846/1960–2000.
//! Earned native action overrides own permission; no learned-spell grants.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason};
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ActionAvailable(available),
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::ConsumedEffectValues(&[(VOIDFORM, 2), (FOUR_PIECE, 0)]),
];
const VOIDFORM: u32 = 228260;
const VOIDFORM_BUFF: u32 = 194249;
const VOLLEY: u32 = 1242173;
const USES: u32 = 1242171;
const CRUSHING_VOID: u32 = 1279437;
const SLAM: u32 = 1227280;
const FOUR_PIECE: u32 = 1296580;

#[derive(Component, Default)]
struct TierUses(u8);

fn stacks(world: &World, caster: Entity, spell: u32) -> u8 {
    world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras.iter().find(|aura| aura.caster == caster && aura.spell_id == spell)
    }).map_or(0, |aura| aura.stacks)
}

// Readiness is tested on the requested action before332 can alias Voidform
// into Volley. Triggered initial Volley bypasses foreground permission/consumption.
fn available(world: &World, caster: Entity, spell: u32) -> bool {
    let uses = stacks(world, caster, USES) + stacks(world, caster, CRUSHING_VOID);
    match spell {
        VOIDFORM => uses == 0 && !auras::has_aura(world, caster, VOIDFORM_BUFF),
        VOLLEY => uses > 0,
        _ => true,
    }
}

fn after_cast(spell: u32) -> Option<spell_cast::scripts::AfterCastHook> {
    match spell {
        VOIDFORM => Some(enter_voidform),
        VOLLEY => Some(consume_volley),
        SLAM => Some(earn_tier_use),
        _ => None,
    }
}

fn grant(world: &mut World, caster: Entity, spell: u32, count: u8) {
    for _ in 0..count {
        auras::apply_aura(world, &AuraApplication {
            caster, target: caster, spell_id: spell, effect_mask: 3, source: AuraSource::Cast,
        });
    }
}

fn enter_voidform(world: &mut World, caster: Entity, _: &SpellInfo) {
    // SimC1384–1393: Voidform aura precedes its E2 ordinary uses.
    auras::apply_aura(world, &AuraApplication {
        caster, target: caster, spell_id: VOIDFORM_BUFF, effect_mask: 3, source: AuraSource::Cast,
    });
    let uses = spell_cast::scripts::script_effect_value(world, caster, VOIDFORM, 2) as u8;
    grant(world, caster, USES, uses);
}

fn spend(world: &mut World, caster: Entity, spell: u32) {
    let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras.iter().find(|aura| aura.caster == caster && aura.spell_id == spell)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        auras::spend_stack(world, caster, instance);
    }
}

fn consume_volley(world: &mut World, caster: Entity, _: &SpellInfo) {
    // SimC1057–1065 decrements both pools, only after an ordinary cast.
    spend(world, caster, CRUSHING_VOID);
    spend(world, caster, USES);
    if let Some(mut uses) = world.get_mut::<TierUses>(caster) {
        uses.0 = uses.0.saturating_sub(1);
    }
}

fn earn_tier_use(world: &mut World, caster: Entity, _: &SpellInfo) {
    if !auras::has_aura(world, caster, FOUR_PIECE) {
        return;
    }
    let chance = spell_cast::scripts::script_effect_value(world, caster, FOUR_PIECE, 0);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    let pool = if auras::has_aura(world, caster, VOIDFORM_BUFF) { USES } else { CRUSHING_VOID };
    let before = stacks(world, caster, pool);
    grant(world, caster, pool, 1);
    if stacks(world, caster, pool) > before {
        let mut entity = world.entity_mut(caster);
        let mut uses = entity.entry::<TierUses>().or_default().into_mut();
        uses.0 += 1;
    }
}

fn register(scripts: &mut AuraScripts) {
    // SimC1057–1058 consumes on Volley, never an unrelated successful spell.
    scripts.0.entry(CRUSHING_VOID).or_default().check_proc = Some(|_, _, _| false);
    scripts.0.insert(VOIDFORM_BUFF, AuraScript {
        on_remove: Some(leave_voidform), ..Default::default()
    });
}

fn crushing_selected(world: &World, caster: Entity) -> bool {
    // sc_priest_shadow.cpp2451 selects Crushing Void; native70448 talent1279354.
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits.active_talent_spells().any(|spell| spell.spell_id == 1279354)
    })
}

fn leave_voidform(world: &mut World, caster: Entity, _: &AuraInstance, _: RemoveReason) {
    let regular = stacks(world, caster, USES);
    let tier = world.get::<TierUses>(caster).map_or(0, |uses| uses.0);
    let transfer = regular.min(tier);
    if crushing_selected(world, caster) {
        spell_cast::scripts::reduce_cooldown(world, caster, VOLLEY, f64::MAX);
        grant(world, caster, CRUSHING_VOID, 1);
    }
    if transfer > 0 {
        auras::remove_auras(world, caster, RemoveReason::Cancelled,
            |aura| aura.spell_id == USES && aura.caster == caster);
        grant(world, caster, CRUSHING_VOID, transfer);
    }
}
