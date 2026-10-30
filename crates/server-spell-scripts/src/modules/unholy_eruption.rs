//! Native70448 Scourge Strike eruption; SimC76f8118 sc_death_knight12502–12661.
//! Spread, Scourging, ghoul readiness and the simulator bug-mode divide are excluded.
use bevy::prelude::*;

use super::combat;
use combat::auras::ActiveAuras;
use combat::spell_cast::effects::{DamageInput, EffectCtx, damage_before_mitigation};
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook};
use combat::spell_cast::scripts::AfterCastHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, targets};
use combat::spell_info::{SpellInfo, SpellStore};

const SCOURGE_STRIKE: u32 = 55090;
const VIRULENT_PLAGUE: u32 = 191587;
pub const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AdditionalReachActions(army_order_actions),
    SpellScriptHook::EffectHit(effect_hit),
    SpellScriptHook::NativeAfterCast(after_cast),
];

fn army_order_actions(world: &World, actor: Entity) -> Vec<u32> {
    if world
        .get::<spell_cast::summon::SummonedCreature>(actor)
        .is_some_and(|summon| summon.spell_id == 1294026)
    {
        vec![1294264, 1300330]
    } else {
        Vec::new()
    }
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == SCOURGE_STRIKE).then_some(spend_ready_ghoul)
}

fn spend_ready_ghoul(world: &mut World, caster: Entity, _: &SpellInfo) {
    let Some(instance) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == 1254252 && aura.caster == caster)
            .map(|aura| aura.instance_id)
    }) else {
        return;
    };
    // SimC76f8118 sc_death_knight.cpp12597–12610: an actual Scourge
    // execution spends one earned ghoul, then performs its summon or order.
    combat::auras::spend_stack(world, caster, instance);
    if combat::auras::has_aura(world, caster, 42650) {
        command_army(world, caster);
    } else if !spell_cast::try_cast_triggered(world, caster, 275430, Some(caster), 1) {
        warn!(?caster, "Scourge Strike lesser-ghoul summon refused");
    }
}

fn command_army(world: &mut World, caster: Entity) {
    let Some(target) = world
        .get::<combat::networking::CombatTarget>(caster)
        .and_then(|target| target.0)
    else {
        warn!(
            ?caster,
            "Scourge Strike Army command has no selected target"
        );
        return;
    };
    let mut enemies: Vec<_> = std::iter::once(target)
        .chain(combat::combat_refs::active_opponents(world, caster))
        .filter(|enemy| {
            targets::is_alive(world, *enemy)
                && targets::relation_matches(world, caster, *enemy, targets::Relation::Enemy)
        })
        .collect();
    enemies.sort_unstable_by_key(|enemy| enemy.to_bits());
    enemies.dedup();
    let order = if enemies.len() >= 3 { 1300330 } else { 1294264 };
    let now = combat::auras::aura_now(world);
    let ghouls: Vec<_> = world
        .query::<(Entity, &spell_cast::summon::SummonedCreature)>()
        .iter(world)
        .filter_map(|(actor, summon)| {
            (summon.owner == caster
                && summon.spell_id == 1294026
                && summon.expires_at.is_some_and(|expiry| expiry > now))
            .then_some(actor)
        })
        .collect();
    // SimC76f8118 sc_death_knight.cpp6766–6771: actual Army ghouls
    // execute Death Order or Epidemic Order, not a synthetic owner hit.
    for ghoul in ghouls {
        if !spell_cast::try_cast_triggered(world, ghoul, order, Some(target), 1) {
            warn!(?ghoul, order, "Native Army order refused");
        }
    }
}

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    ((spell, index) == (SCOURGE_STRIKE, 0)).then_some(erupt_and_apply_missing)
}
const ERUPTIONS: &[(u32, u32)] = &[(191587, 1241167), (1240996, 1241171)];

pub(super) fn erupt_and_apply_missing(
    world: &mut World,
    caster: Entity,
    target: Entity,
    depth: u8,
) {
    if !targets::is_alive(world, target) {
        return;
    }
    let fraction = script_effect_value(world, caster, SCOURGE_STRIKE, 1) / 100.0;
    for &(plague, child) in ERUPTIONS {
        let Some(amount) = owned_tick_amount(world, caster, target, plague) else {
            continue;
        };
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            caster,
            child,
            Some(target),
            depth + 1,
            amount * fraction,
        );
    }
    if targets::is_alive(world, target) {
        apply_missing_plague(world, caster, target, depth);
    }
}

///55090 applies Virulent Plague if absent. Existing marks belong to the separate
///spread/eruption contract; they are not refreshed as a substitute for spreading.
fn apply_missing_plague(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    let owned = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|a| a.spell_id == VIRULENT_PLAGUE && a.caster == caster)
    });
    if !owned {
        combat::auras::apply_aura(
            world,
            &combat::auras::AuraApplication {
                caster,
                target,
                spell_id: VIRULENT_PLAGUE,
                effect_mask: u32::MAX,
                source: combat::auras::AuraSource::Cast,
            },
        );
    }
}

/// The stored aura amount has AP/base-point snapshots, but not periodic damage
/// modifiers. Resolve those once without executing a tick or consuming its clock.
/// Native children ignore caster percent-done, while retaining their own taken stage.
pub(super) fn owned_tick_amount(
    world: &World,
    caster: Entity,
    target: Entity,
    plague: u32,
) -> Option<f32> {
    if !targets::is_alive(world, target) {
        return None;
    }
    let aura = world
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.spell_id == plague && aura.caster == caster)?;
    let state = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0 && effect.period_ms > 0)?;
    let spell = world
        .resource::<SpellStore>()
        .get(plague)
        .expect("Scourge eruption requires the active plague's native spell");
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == state.index)
        .expect("active plague effect belongs to its native spell");
    let ctx = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let damage = DamageInput {
        amount: combat::auras::periodic_live_amount(world, caster, spell, state)
            * aura.stack_multiplier(),
        crit: false,
        periodic: true,
        resolved_source: false,
        effect: Some(effect),
        bonus_caster: caster,
    };
    Some(damage_before_mitigation(world, &ctx, target, &damage))
}
