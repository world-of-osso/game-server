//! SimC76f8118 sc_monk.cpp853–877/2581–2607: strikes reduce Brew cooldowns.
use super::combat;
use bevy::prelude::*;
use combat::auras::procs::{AuraEvent, PROC_HIT_DODGE, PROC_HIT_MISS};
use combat::auras::{self, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    effects::EffectCtx,
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, reduce_cooldown, script_effect_value},
};
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

const BLACKOUT_COMBO: u32 = 228563;
const FACE_PALM: u32 = 389942;
const MEDITATIVE_FOCUS: u32 = 1271105;
const INFUSION: u32 = 1241059;
const SALSALABIMS_STRENGTH: u32 = 383697;
const BREATH_OF_FIRE: u32 = 115181;
const ANVIL_AND_STAVE: u32 = 386937;
const AVOIDED: u32 = PROC_HIT_DODGE | PROC_HIT_MISS;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterResourceValues(cast_hook),
    SpellScriptHook::AfterPolicyDirectDamage(tiger_palm_hit),
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AvoidanceProcHitMask(avoidance_mask),
];

// DB2 386937 has only dummy effects: "Each time you dodge or an enemy misses you, the
// remaining cooldown on your Brews is reduced by ${$s1/10}.1 sec." SimC76f8118
// sc_monk.cpp7245–7252 applies the same rank-scaled value on dodge only.
fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(ANVIL_AND_STAVE).or_default();
    script.check_proc = Some(attack_avoided);
    script.on_proc = Some(anvil_and_stave);
}

fn avoidance_mask(spell: u32) -> Option<u32> {
    (spell == ANVIL_AND_STAVE).then_some(AVOIDED)
}

fn attack_avoided(_: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    event.target == Some(aura.caster) && event.hit_mask & AVOIDED != 0
}

fn anvil_and_stave(world: &mut World, unit: Entity, _: &AuraInstance, _: Option<Entity>) {
    let seconds = f64::from(script_effect_value(world, unit, ANVIL_AND_STAVE, 0)) / 10.0;
    brew_reduction(world, unit, seconds);
}

fn selected(world: &World, caster: Entity, spell: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == spell)
    })
}

fn effect(world: &World, spell: u32, index: u8) -> f64 {
    f64::from(
        world
            .resource::<SpellStore>()
            .get(spell)
            .expect("native Brew source")
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("native Brew effect")
            .base_points,
    )
}

pub(super) fn brew_reduction(world: &mut World, caster: Entity, seconds: f64) {
    let celestial = if selected(world, caster, INFUSION) {
        INFUSION
    } else {
        322507
    };
    for brew in [119582, celestial, 115203, 115399] {
        reduce_cooldown(world, caster, brew, seconds);
    }
}

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    match spell {
        121253 => Some(keg_smash),
        100780 => Some(consume_combo),
        _ => None,
    }
}

fn is_brewmaster(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 268)
}

fn consume_combo(world: &mut World, caster: Entity, _: &SpellInfo) {
    if is_brewmaster(world, caster) {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == BLACKOUT_COMBO && aura.caster == caster
        });
    }
}

fn keg_smash(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !is_brewmaster(world, caster) {
        return;
    }
    let mut reduction = effect(world, spell.id, 3);
    if auras::has_aura(world, caster, BLACKOUT_COMBO) {
        reduction += effect(world, BLACKOUT_COMBO, 2);
        if selected(world, caster, MEDITATIVE_FOCUS) {
            reduction -= effect(world, MEDITATIVE_FOCUS, 2) / 1000.0;
        }
    }
    brew_reduction(world, caster, reduction);
    if selected(world, caster, SALSALABIMS_STRENGTH) {
        // SimC76f8118 sc_monk.cpp2589–2592 resets Breath of Fire on Keg Smash.
        let breath = world
            .resource::<SpellStore>()
            .get(BREATH_OF_FIRE)
            .expect("Breath of Fire spell");
        let now = combat::spell_cast::history_now(world);
        let remaining = world
            .get::<combat::spell_cast::SpellHistory>(caster)
            .map_or(0, |history| history.remaining_cooldown_ms(breath, now));
        reduce_cooldown(
            world,
            caster,
            BREATH_OF_FIRE,
            (remaining as f64 + 1.0) / 1000.0,
        );
    }
    consume_combo(world, caster, spell);
}

fn tiger_palm_hit(world: &mut World, ctx: &EffectCtx, _target: Entity, _critical: bool) {
    if ctx.spell.id != 100780 || !is_brewmaster(world, ctx.caster) {
        return;
    }
    let mut reduction = effect(world, 100780, 2);
    if selected(world, ctx.caster, FACE_PALM) {
        reduction += effect(world, FACE_PALM, 2) / 1000.0;
    }
    brew_reduction(world, ctx.caster, reduction);
}
