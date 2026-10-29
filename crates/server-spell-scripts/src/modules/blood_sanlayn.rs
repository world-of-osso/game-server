//! Blood San'layn's cooldown-driven Gift of the San'layn.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    self, TriggeredSpellOverrides,
    effects::{self, EffectCtx, EffectHandler},
    script_registry::SpellScriptHook,
};
use combat::spell_info::SpellStore;
use combat::spell_info::{SpellEffect, SpellInfo};
use shared::components::{ActiveSpec, Health};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterForegroundSpecial(grant_gift),
    SpellScriptHook::AfterForegroundSpecial(vampiric_proc),
    SpellScriptHook::EffectHandler(vampiric_heal_handler),
    SpellScriptHook::EffectEnabled(blood_vampiric_damage_effect),
    SpellScriptHook::AfterPolicyDirectDamage(infliction_of_sorrow),
];

// SimC76f8118 sc_death_knight.cpp11325–11329: Blood's Vampiric Strike
// uses effect 5, not the separate Unholy damage effect 1.
fn blood_vampiric_damage_effect(world: &World, caster: Entity, spell: u32, index: u8) -> bool {
    !(spell == 433895
        && index == 0
        && world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250))
}

// SimC76f8118 sc_death_knight.cpp13778–13830: only an actual owned
// Blood Plague supplies remaining damage; extend it without restarting ticks.
fn infliction_of_sorrow(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if ctx.spell.id != 433895
        || !world
            .get::<ActiveSpec>(ctx.caster)
            .is_some_and(|spec| spec.0 == 250)
        || !world
            .get::<KnownSpells>(ctx.caster)
            .is_some_and(|known| known.contains(434143))
    {
        return;
    }
    let Some(plague) = world
        .get::<auras::ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 55078 && aura.caster == ctx.caster)
        })
        .cloned()
    else {
        return;
    };
    let Some(expiry) = plague.expires_at else {
        return;
    };
    let Some(state) = plague
        .effects
        .iter()
        .find(|effect| effect.index == 0 && effect.period_ms > 0)
    else {
        return;
    };
    let now = auras::aura_now(world);
    if expiry <= now {
        return;
    }
    let period = f64::from(state.period_ms) / 1000.0;
    let ticks = (1.0 + (expiry - state.next_tick_at()) / period).max(0.0) as f32;
    let store = world.resource::<SpellStore>();
    let disease = store.get(55078).expect("active Blood Plague spell");
    let effect = disease
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Blood Plague damage effect");
    let talent = store.get(434143).expect("Infliction of Sorrow talent");
    let percent = talent
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Infliction damage percent")
        .base_points
        / 100.0;
    let extension = talent
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .expect("Infliction extension")
        .base_points as u32;
    let disease_ctx = EffectCtx {
        caster: ctx.caster,
        spell: disease,
        depth: ctx.depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    let damage = effects::DamageInput {
        amount: auras::periodic_live_amount(world, ctx.caster, disease, state)
            * plague.stack_multiplier(),
        crit: false,
        periodic: true,
        resolved_source: false,
        effect: Some(effect),
        bonus_caster: ctx.caster,
    };
    let remaining = effects::damage_before_mitigation(world, &disease_ctx, target, &damage) * ticks;
    auras::extend_owned_duration(world, ctx.caster, target, &[55078], extension);
    if remaining > 0.0 {
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            ctx.caster,
            434144,
            Some(target),
            ctx.depth + 1,
            remaining * percent,
        );
    }
}

fn vampiric_heal_handler(spell: u32, index: u8) -> Option<EffectHandler> {
    (spell == 434422 && index == 0).then_some(vampiric_heal)
}

// SimC76f8118 sc_death_knight.cpp8206–8216: Blood uses the native second effect's health percentage.
fn vampiric_heal(world: &mut World, ctx: &EffectCtx<'_>, effect: &SpellEffect, target: Entity) {
    if !world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 250)
    {
        effects::handler(effects::EFFECT_HEAL_PCT).expect("native percentage heal")(
            world, ctx, effect, target,
        );
        return;
    }
    let percent = ctx
        .spell
        .effects
        .iter()
        .find(|effect| effect.index == 1)
        .expect("Blood Vampiric Strike heal percentage")
        .base_points;
    let max = world
        .get::<Health>(target)
        .expect("Vampiric Strike target health")
        .max;
    let mut heal = effect.clone();
    heal.effect = effects::EFFECT_HEAL;
    heal.base_points = max * percent / 100.0;
    effects::handler(effects::EFFECT_HEAL).expect("native critical heal")(
        world, ctx, &heal, target,
    );
}

// SimC76f8118 sc_death_knight.cpp7279–7300/9531–9536: Vampiric Blood
// and Dancing Rune Weapon grant Gift for their native duration.
fn grant_gift(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(spell.id, 49028 | 55233)
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 250)
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(434152))
    {
        return;
    }
    let base_duration = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.duration)
        .expect("Blood cooldown duration")
        .duration_ms;
    let duration = auras::spell_mods::modify(
        world,
        caster,
        spell,
        auras::SpellModOp::Duration,
        base_duration as f32,
    )
    .round() as i32;
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        434153,
        Some(caster),
        1,
        TriggeredSpellOverrides {
            duration_ms: Some(duration),
            ..Default::default()
        },
    );
    spell_cast::cast_triggered(world, caster, 433899, Some(caster), 1);
}

// SimC76f8118 sc_death_knight.cpp10131–10134/13835–13853: Death Strike
// may earn a replacement outside Gift; the replacement consumes the proc.
fn vampiric_proc(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 250)
    {
        return;
    }
    let gift = auras::has_aura(world, caster, 434153);
    if spell.id == 433895 && !gift {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == 433899 && aura.caster == caster
        });
        return;
    }
    if spell.id != 49998
        || gift
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(433901))
    {
        return;
    }
    let store = world.resource::<SpellStore>();
    let effect = |id, index| {
        store
            .get(id)
            .expect("Sanlayn spell")
            .effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("Sanlayn effect")
            .base_points
    };
    let mut chance = effect(433901, 0);
    if world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(434263))
        && spell
            .cast_primary_target
            .and_then(|target| world.get::<Health>(target))
            .is_some_and(|health| {
                health.max > 0.0 && health.current * 100.0 <= health.max * effect(434263, 0)
            })
    {
        chance += effect(434263, 1);
    }
    if combat::random::random::<f32>() * 100.0 < chance {
        spell_cast::cast_triggered(world, caster, 433899, Some(caster), 1);
    }
}
