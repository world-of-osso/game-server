//! SimC76f8118 Sentinel target marks, Lunar Storm and native residual bridges.
use super::combat;
use super::marksmanship::selected;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::{reduce_cooldown, script_effect_value};
use combat::spell_cast::{self, effects::EffectCtx, queued_triggers};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ArmorIgnoredSpells(ARMOR_IGNORED),
    SpellScriptHook::ResolvedDamage(after_damage),
];
use bevy::prelude::*;

const AIMED: u32 = 19434;
const RAPID_TICK: u32 = 257045;
const SENTINEL: u32 = 1253599;
const MARK: u32 = 1253601;
const LUNAR_STORM: u32 = 1253732;
const LUNAR_DAMAGE: u32 = 1253733;
const FEATHERED_FRENZY: u32 = 470943;
const MOONS_BLESSING: u32 = 1253825;
const LUNAR_CALLING: u32 = 1253852;
const BLOOD_FLETCHING: u32 = 1319011;
const BLOOD_MARK: u32 = 1319016;
const BLOOD_DAMAGE: u32 = 1319015;
const ARMAMENTS: u32 = 1253831;
const ARMAMENTS_DOT: u32 = 1253836;
const PRECISE_SHOTS: u32 = 260242;

/// sc_hunter.cpp5186–5187: Blood Fletching is a direct school-damage
/// spell that models a bleed, so its missing native bleed mechanic cannot add armor.
const ARMOR_IGNORED: &[u32] = &[BLOOD_DAMAGE];

pub(super) fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // sc_hunter.cpp6910–6911: Feathered Frenzy forces a real mark before
    // Trueshot's successful-cast effects, keeping the selected hostile primary.
    if spell.id == 288613
        && selected(world, caster, FEATHERED_FRENZY)
        && let Some(target) = spell.cast_primary_target
    {
        grant_mark(world, caster, target);
    }
}

fn after_damage(world: &mut World, ctx: &EffectCtx, target: Entity, critical: bool, damage: f32) {
    if damage <= 0.0 {
        return;
    }
    trigger_blood_damage(world, ctx, target);
    if ctx.spell.id == AIMED {
        consume_mark(world, ctx, target);
        schedule_blood_mark(world, ctx, target, critical);
    }
    if ctx.spell.id == RAPID_TICK && selected(world, ctx.caster, ARMAMENTS) {
        // sc_hunter.cpp5708–5711: resolved Rapid damage supplies effect1's
        // share; native1253836 owns rolling carry and periodic timing.
        let percent = script_effect_value(world, ctx.caster, ARMAMENTS, 0);
        super::marksmanship::bank_residual(world, ctx, target, ARMAMENTS_DOT, percent, damage);
    }
    roll_mark_from_precise(world, ctx, target);
}

fn owns_mark(world: &World, caster: Entity, target: Entity, spell: u32) -> bool {
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.caster == caster && aura.spell_id == spell)
    })
}

fn trigger_blood_damage(world: &mut World, ctx: &EffectCtx, target: Entity) {
    // sc_hunter.cpp8630–8657: later landed direct/periodic damage can trigger
    // the mark, but the callback's own child cannot recursively trigger itself.
    if ctx.spell.id == BLOOD_DAMAGE
        || !selected(world, ctx.caster, BLOOD_FLETCHING)
        || !owns_mark(world, ctx.caster, target, BLOOD_MARK)
    {
        return;
    }
    spell_cast::cast_triggered(world, ctx.caster, BLOOD_DAMAGE, Some(target), ctx.depth + 1);
}

fn schedule_blood_mark(world: &mut World, ctx: &EffectCtx, target: Entity, critical: bool) {
    // sc_hunter.cpp5411–5419: delay1ms places the new mark AFTER callbacks
    // from this applying critical Aimed Shot, without suppressing later hits.
    if critical && selected(world, ctx.caster, BLOOD_FLETCHING) {
        queued_triggers::schedule_full_trigger(world, ctx, target, BLOOD_MARK, 1);
    }
}

fn consume_mark(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if !selected(world, ctx.caster, SENTINEL) {
        return;
    }
    // sc_hunter.cpp5422–5428: consume only an actual owned mark on impact.
    let removed = auras::remove_auras(world, target, RemoveReason::Consumed, |aura| {
        aura.caster == ctx.caster && aura.spell_id == MARK
    });
    if removed.is_empty() || !selected(world, ctx.caster, LUNAR_STORM) {
        return;
    }
    let count = script_effect_value(world, ctx.caster, LUNAR_STORM, 0) as u32;
    // sc_hunter.cpp3769–3772: source-authored150ms intervals; no DB2 interval exists.
    for index in 1..=count {
        queued_triggers::schedule_full_trigger(world, ctx, target, LUNAR_DAMAGE, index * 150);
    }
}

fn roll_mark_from_precise(world: &mut World, ctx: &EffectCtx, target: Entity) {
    // sc_hunter.cpp4182–4184/4713–4716/5235–5238: only actual Precise
    // spenders roll; Multi-Shot only rolls once, at its real primary.
    if !matches!(ctx.spell.id, 185358 | 53351 | 257620) || !selected(world, ctx.caster, SENTINEL) {
        return;
    }
    let precise = ctx.spell.cast_charge_mods.as_ref().is_some_and(|mods| {
        mods.iter()
            .any(|modifier| modifier.aura_spell_id == PRECISE_SHOTS)
    });
    if !precise || ctx.spell.cast_primary_target != Some(target) {
        return;
    }
    let chance = read_mark_chance(world, ctx.caster);
    if combat::random::random::<f32>() * 100.0 < chance {
        grant_mark(world, ctx.caster, target);
    }
}

fn read_mark_chance(world: &World, caster: Entity) -> f32 {
    // sc_hunter.cpp3674–3691: MM's baseline Spotter chance plus selected
    // Moon's Blessing, multiplied by Feathered Frenzy inside actual Trueshot,
    // then the selected Lunar Calling bonus. All percentages are native.
    let mut chance = script_effect_value(world, caster, 1219616, 0);
    if selected(world, caster, MOONS_BLESSING) {
        chance += script_effect_value(world, caster, MOONS_BLESSING, 0);
    }
    if auras::has_aura(world, caster, 288613) {
        if selected(world, caster, FEATHERED_FRENZY) {
            chance *= 1.0 + script_effect_value(world, caster, FEATHERED_FRENZY, 0) / 100.0;
        }
        if selected(world, caster, LUNAR_CALLING) {
            chance += script_effect_value(world, caster, LUNAR_CALLING, 0);
        }
    }
    chance
}

fn grant_mark(world: &mut World, caster: Entity, target: Entity) {
    if !selected(world, caster, SENTINEL) {
        return;
    }
    if !spell_cast::try_cast_triggered(world, caster, MARK, Some(target), 1) {
        warn!(?caster, ?target, "Native Sentinel mark refused");
        return;
    }
    // sc_hunter.cpp3666/3709: a real mark immediately reduces Aimed's
    // running recovery by Moon's Blessing effect2, without restoring charges.
    if selected(world, caster, MOONS_BLESSING) {
        let seconds = f64::from(script_effect_value(world, caster, MOONS_BLESSING, 1)) / 1000.0;
        reduce_cooldown(world, caster, AIMED, seconds);
    }
}
