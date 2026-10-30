//! SimC76f8118 sc_druid.cpp7871–7947,12694–12749.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, ActiveAuras, AuraInstance, RemoveReason, procs::AuraEvent, scripts::AuraScripts,
};
use combat::random::seq::SliceRandom;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook, targets};
use combat::spell_info::SpellStore;

const TALENT: u32 = 203964;
const BUFF: u32 = 213708;
const MOON_GUARDIAN: u32 = 429520;
const MOONFIRE: u32 = 8921;
const MOONFIRE_DAMAGE: u32 = 164812;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::ContextHit(moonfire_hit),
];

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &combat::spell_info::SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    let selected = world
        .get::<combat::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == 372567)
        });
    if spell.id != MOONFIRE || !selected {
        return Default::default();
    }
    let range = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.range)
        .expect("native Moonfire range")
        .max_hostile;
    combat::spell_cast::script_registry::ScriptReach::caster(range)
}

fn moonfire_hit(spell: u32) -> Option<fn(&mut World, &EffectCtx<'_>, Entity)> {
    (spell == MOONFIRE).then_some(copy_twin_moonfire)
}

// SimC76f8118 sc_druid.cpp7897–7944: Twin Moonfire copies to another enemy.
fn copy_twin_moonfire(world: &mut World, ctx: &EffectCtx<'_>, primary: Entity) {
    let caster = ctx.caster;
    if !world
        .get::<combat::class_progression::TraitLoadout>(caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == 372567)
        })
    {
        return;
    }
    let Some(origin) = targets::position(world, caster) else {
        return;
    };
    let range = ctx
        .spell
        .misc
        .as_ref()
        .and_then(|misc| misc.range)
        .expect("Moonfire native range")
        .max_hostile;
    let mut enemies =
        targets::area_units(world, caster, origin, range, targets::Relation::Enemy, 0);
    if !enemies.contains(&primary) {
        return;
    }
    enemies.retain(|&target| target != primary);
    enemies.shuffle(&mut combat::random::thread_rng());
    enemies.sort_by(|&a, &b| {
        let expiry = |target| {
            world
                .get::<ActiveAuras>(target)
                .and_then(|auras| {
                    auras
                        .iter()
                        .find(|a| a.spell_id == MOONFIRE_DAMAGE && a.caster == caster)
                })
                .and_then(|aura| aura.expires_at)
                .unwrap_or(0.0)
        };
        if ctx.depth == 0 {
            expiry(a).total_cmp(&expiry(b))
        } else {
            (expiry(a) > 0.0).cmp(&(expiry(b) > 0.0))
        }
    });
    if let Some(&target) = enemies.first() {
        spell_cast::cast_triggered(world, caster, MOONFIRE_DAMAGE, Some(target), ctx.depth + 1);
    }
}

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(TALENT).or_default();
    script.check_proc = Some(roll_galactic);
    script.on_damage_proc = Some(trigger_galactic);
    scripts.0.entry(BUFF).or_default().check_proc = Some(accept_moonfire_charge);
}

// SimC76f8118 sc_druid.cpp7947: only foreground Moonfire consumes this charge.
fn accept_moonfire_charge(_: &World, _: &AuraInstance, event: &AuraEvent) -> bool {
    event.spell_id == Some(MOONFIRE) && event.can_proc
}

fn roll_galactic(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    if world
        .get::<combat::class_progression::TraitLoadout>(aura.caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == 1252871)
        })
    {
        return false;
    }
    if event.actor != aura.caster
        || (event.target == Some(aura.caster) && event.damage <= 0.0)
        || !event.can_proc
        || event.target.is_none()
        || matches!(event.spell_id, Some(MOONFIRE | MOONFIRE_DAMAGE))
    {
        return false;
    }
    let mask = world
        .resource::<SpellStore>()
        .get(TALENT)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Galactic Guardian native proc mask")
        .proc_type_mask[0];
    if event.actor_mask[0] & mask == 0 {
        return false;
    }
    let chance = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Galactic Guardian native chance")
        .amount;
    combat::random::random::<f32>() < chance / 100.0
}

fn trigger_galactic(world: &mut World, caster: Entity, _: &AuraInstance, event: &AuraEvent) {
    let Some(target) = event.target else {
        return;
    };
    let empowered = auras::has_aura(world, caster, BUFF);
    let can_energize = empowered && auras::has_aura(world, caster, MOON_GUARDIAN);
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        MOONFIRE,
        Some(target),
        1,
        spell_cast::TriggeredSpellOverrides {
            effect_mask: Some(if can_energize { 3 } else { 1 }),
            ..Default::default()
        },
    );
    if empowered {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == BUFF && aura.caster == caster
        });
    }
    spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
}
