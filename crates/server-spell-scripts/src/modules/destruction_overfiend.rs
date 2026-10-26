//! Actual Overfiend native coefficient borrowing, crit/mastery and owner funding.
use super::combat;
use bevy::prelude::*;
use combat::auras::AuraStatMods;
use combat::class_progression::TraitLoadout;
use combat::random as rand;
use combat::spell_cast::effects::{self, EffectCtx, EffectHandler};
use combat::spell_cast::script_registry::{DamageAmountHook, SpellScriptHook};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_info::{SpellEffect, SpellInfo, SpellStore};
use combat::stats::CharacterStats;
use shared::components::{ActiveSpec, Npc};

const SUMMON: u32 = 434587;
const OVERFIEND: u32 = 217429;
const BOLT: u32 = 434589;
const PLAYER_BOLT: u32 = 116858;
const AVATAR: u32 = 1245089;
const OWNER_BUFF: u32 = 457578;
const MASTERY: u32 = 77220;
const CHAOS_INCARNATE: u32 = 387275;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHandler(effect_handler),
    SpellScriptHook::SummonInitialized(apply_spawned_owner_buff),
    SpellScriptHook::OwnsPolicyProc(owns_avatar_proc),
    SpellScriptHook::AuraRollAfterCast(soul_fire_hook),
    SpellScriptHook::DamageAmount(damage_lookup),
    SpellScriptHook::CriticalChance(guaranteed_crit),
    SpellScriptHook::ActualDamageMultiplier(roll_mastery),
    SpellScriptHook::ExpectedDamageMultiplier(expected_mastery),
];

fn read_owner(world: &World, actor: Entity) -> Option<Entity> {
    let summon = world.get::<SummonedCreature>(actor)?;
    let npc = world.get::<Npc>(actor)?;
    let actual = (summon.spell_id, npc.template_id) == (SUMMON, OVERFIEND);
    let destruction = world
        .get::<ActiveSpec>(summon.owner)
        .is_some_and(|spec| spec.0 == 267);
    (actual && destruction).then_some(summon.owner)
}

fn owns_avatar_proc(spell: u32) -> bool {
    spell == AVATAR
}

fn soul_fire_hook(spell: u32) -> Option<combat::spell_cast::scripts::AfterCastHook> {
    (spell == 6353).then_some(summon_from_soul_fire)
}

fn summon_from_soul_fire(world: &mut World, caster: Entity, _: &SpellInfo) {
    let selected = world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == AVATAR)
    });
    let destruction = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 267);
    // SimC76f8118 actions5124–5136: actual Soul Fire execution summons one
    // Overfiend. Native Avatar's broad proc mask also admits unrelated casts.
    if selected
        && destruction
        && !combat::spell_cast::try_cast_triggered(world, caster, SUMMON, Some(caster), 1)
    {
        warn!(?caster, "native Soul Fire Overfiend summon refused");
    }
}

fn effect_handler(spell: u32, index: u8) -> Option<EffectHandler> {
    (spell == BOLT && index == 0).then_some(cast_native_borrowed_bolt)
}

fn apply_spawned_owner_buff(world: &mut World, actor: Entity) {
    // Native summon cast_handler bypasses per-unit effect wrappers. Observe
    // actual creation through the same actor callback as Infernal, not a query.
    if let Some(owner) = read_owner(world, actor) {
        apply_native_owner_buff(world, owner);
    }
}

fn cast_native_borrowed_bolt(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    effect: &SpellEffect,
    target: Entity,
) {
    let mut borrowed = effect.clone();
    if read_owner(world, ctx.caster).is_some() {
        // SimC76f8118 pets.cpp2370–2373: borrow native player CB SP coefficient
        // times native Avatar percent. Keep this per-resolution copy local.
        borrowed.bonus_coefficient = read_borrowed_coefficient(world);
    }
    // Preserve native variance, crit, effect context and downstream resolution.
    // Replacing the amount after critical resolution would discard the crit.
    let native = effects::handler(effect.effect).expect("native Overfiend damage effect handler");
    native(world, ctx, &borrowed, target);
}

fn read_borrowed_coefficient(world: &World) -> f32 {
    let store = world.resource::<SpellStore>();
    let primary = |spell| {
        store
            .get(spell)
            .expect("native Overfiend relationship requires its source spell")
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native Overfiend relationship requires source effect0")
    };
    primary(PLAYER_BOLT).bonus_coefficient * primary(AVATAR).base_points / 100.0
}

fn damage_lookup(spell: u32) -> Option<DamageAmountHook> {
    (spell == BOLT).then_some(scale_live_crit)
}

fn scale_live_crit(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    let Some(owner) = read_owner(world, caster) else {
        return amount;
    };
    // pets.cpp2380–2387: guaranteed outcome is separate from live pet spell crit.
    let base = world.get::<CharacterStats>(owner).unwrap().crit_pct;
    let bonus = world
        .get::<AuraStatMods>(owner)
        .map_or(0.0, |mods| mods.spell_crit_pct);
    amount * (1.0 + (base + bonus) / 100.0)
}

fn guaranteed_crit(world: &World, caster: Entity, _target: Entity, spell: u32) -> Option<f32> {
    (spell == BOLT && read_owner(world, caster).is_some()).then_some(100.0)
}

fn read_minimum_fraction(world: &World, owner: Entity) -> f32 {
    let incarnate = world.get::<TraitLoadout>(owner).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == CHAOS_INCARNATE)
    });
    if incarnate {
        script_effect_value(world, owner, CHAOS_INCARNATE, 0) / 100.0
    } else {
        0.5
    }
}

fn read_mastery_bounds(world: &World, caster: Entity, spell: &SpellInfo) -> Option<(f32, f32)> {
    if spell.id != BOLT {
        return None;
    }
    let owner = read_owner(world, caster)?;
    if !combat::auras::has_aura(world, owner, MASTERY) {
        return None;
    }
    // pets.cpp2392–2404: Overfiend independently samples owner mastery.
    let mastery = world.get::<CharacterStats>(owner).unwrap().mastery_pct / 100.0;
    Some((
        1.0 + read_minimum_fraction(world, owner) * mastery,
        1.0 + mastery,
    ))
}

fn roll_mastery(world: &mut World, ctx: &EffectCtx<'_>, _target: Entity) -> f32 {
    let Some((minimum, maximum)) = read_mastery_bounds(world, ctx.caster, ctx.spell) else {
        return 1.0;
    };
    if minimum == maximum {
        return minimum;
    }
    rand::Rng::gen_range(&mut rand::thread_rng(), minimum..=maximum)
}

fn expected_mastery(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    read_mastery_bounds(world, caster, spell)
        .map_or(1.0, |(minimum, maximum)| (minimum + maximum) / 2.0)
}

fn apply_native_owner_buff(world: &mut World, owner: Entity) {
    let destruction = world
        .get::<ActiveSpec>(owner)
        .is_some_and(|spec| spec.0 == 267);
    if !destruction {
        return;
    }
    // actions.cpp4190–4205/init851–855: real summon also triggers native457578;
    // its unmodified aura period and raw energize operate on the existing owner.
    if !combat::spell_cast::try_cast_triggered(world, owner, OWNER_BUFF, Some(owner), 0) {
        warn!(?owner, "Native Overfiend owner buff457578 refused");
    }
}
