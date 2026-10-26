//! Affliction Nightfall: actual owned Corruption/Wither threshold ticks and native consumers.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::{ChargedSpellModifier, SpellInfo};
use shared::components::ActiveSpec;

const TALENT: u32 = 108558;
const BUFF: u32 = 264571;
const SATAIEL: u32 = 449637;
// SimC76f8118 sc_warlock.hpp1065, not a spell-damage coefficient.
const INCREMENT_MAX: f64 = 0.130;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PeriodicDamageFinished(landed_tick),
    SpellScriptHook::AfterForegroundSpecial(consume_earned_use),
    SpellScriptHook::LaunchDamageModifier(snapshot_bolt_bonus),
    SpellScriptHook::CastTimeProcs(&[BUFF]),
    SpellScriptHook::PreserveUnadmittedProcs(&[BUFF]),
    SpellScriptHook::OwnsPolicyProc(owns_nightfall_uses),
];

#[derive(Component)]
struct NightfallProgress {
    passive: u32,
    progress: f64,
}

fn landed_tick(world: &mut World, ctx: &EffectCtx<'_>) {
    if !matches!(ctx.spell.id, 146739 | 445474) {
        return;
    }
    if world
        .get::<ActiveSpec>(ctx.caster)
        .is_none_or(|spec| spec.0 != 265)
    {
        return;
    }
    let Some(passive) = read_passive(world, ctx.caster) else {
        return;
    };
    let count = active_dots(world, ctx.caster, ctx.spell.id);
    assert!(count > 0, "real Nightfall source retains its owned dot");
    // SimC76f8118 init1190–1225: independent threshold progress, uniform initial
    // phase and rollover; native timing/hero normalization and active-dot count.
    let maximum = INCREMENT_MAX
        * super::affliction_agony::creeping_normalization(world, ctx.caster)
        * sataiel_multiplier(world, ctx.caster)
        * (count as f64).powf(-2.0 / 3.0);
    let initial = world
        .get::<NightfallProgress>(ctx.caster)
        .filter(|state| state.passive == passive)
        .map_or_else(combat::random::random::<f64>, |state| state.progress);
    let (progress, proc) = super::affliction_agony::advance_progress(
        initial,
        combat::random::random::<f64>() * maximum,
    );
    world
        .entity_mut(ctx.caster)
        .insert(NightfallProgress { passive, progress });
    if proc {
        combat::auras::apply_aura(
            world,
            &combat::auras::AuraApplication {
                caster: ctx.caster,
                target: ctx.caster,
                spell_id: BUFF,
                effect_mask: u32::MAX,
                source: combat::auras::AuraSource::Cast,
            },
        );
    }
}

fn read_passive(world: &World, caster: Entity) -> Option<u32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == TALENT)
        .map(|aura| aura.instance_id)
}

fn active_dots(world: &mut World, caster: Entity, spell: u32) -> usize {
    world
        .query::<&ActiveAuras>()
        .iter(world)
        .filter(|auras| {
            auras
                .iter()
                .any(|aura| aura.caster == caster && aura.spell_id == spell)
        })
        .count()
}

fn sataiel_multiplier(world: &World, caster: Entity) -> f64 {
    if !combat::auras::has_aura(world, caster, SATAIEL) {
        return 1.0;
    }
    let percent = combat::spell_cast::scripts::script_effect_value(world, caster, SATAIEL, 0);
    1.0 + f64::from(percent) / 100.0
}

// SimC76f8118 actions877–878/2182/2338: cast execution owns these uses.
// The native broad proc mask cannot also spend a charge on queued cast events.
fn owns_nightfall_uses(spell: u32) -> bool {
    spell == BUFF
}

// SimC76f8118 actions902–908: the instant Bolt's one native bonus survives
// spending its admitted use before impact. Mid-hardcast gains do not qualify.
fn snapshot_bolt_bonus(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Option<ChargedSpellModifier> {
    if spell.id != 686 || spell.cast_was_hardcast {
        return None;
    }
    let aura = find_admitted_use(world, caster, spell)?;
    let amount = aura.effects.iter().find(|effect| effect.index == 1)?.amount;
    Some(ChargedSpellModifier {
        aura_spell_id: BUFF,
        op: combat::auras::SpellModOp::HealingAndDamage as i32,
        percent: true,
        amount,
    })
}

fn find_admitted_use<'a>(
    world: &'a World,
    caster: Entity,
    spell: &SpellInfo,
) -> Option<&'a combat::auras::AuraInstance> {
    let auras = world.get::<ActiveAuras>(caster)?;
    spell
        .cast_proc_mods
        .iter()
        .filter_map(|id| auras.get(*id))
        .find(|aura| aura.caster == caster && aura.spell_id == BUFF)
}

fn consume_earned_use(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(spell.id, 198590 | 1261153) {
        return;
    }
    // SimC76f8118 actions877–878/2182/2338: exactly one execution-owned use.
    // Native Grasp shares Bolt's cast-time mask; generic charge consumption
    // already spends its admitted use. Drain Soul's unadmitted use is manual.
    if find_admitted_use(world, caster, spell).is_none() {
        super::cast_relationships::decrement(world, caster, BUFF);
    }
}
