//! Earned Outlaw builder procs; native records retain values and payload damage.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::random::Rng;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const SINISTER_STRIKE: u32 = 193315;
const EXTRA_STRIKE: u32 = 197834;
const TALENT: u32 = 279876;
const OPPORTUNITY: u32 = 195627;
const FAN_THE_HAMMER: u32 = 381846;
const ROLL_THE_BONES: u32 = 1214909;
const JACKPOT: u32 = 1214937;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedDamage(after_hit),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

fn owns_proc(spell: u32) -> bool {
    spell == TALENT
}

fn after_hit(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, damage: f32) {
    if ctx.spell.id != SINISTER_STRIKE || damage <= 0.0 {
        return;
    }
    let Some(chance) = read_proc_chance(world, ctx.caster) else {
        return;
    };
    if combat::random::thread_rng().gen_range(0.0..100.0) >= chance {
        return;
    }
    // SimC76f8118 sc_rogue.cpp8255–8280: only landed roots roll;
    // a success earns1+Fan stacks and a separate native strike after300ms.
    grant_opportunity(world, ctx.caster);
    spell_cast::queued_triggers::schedule_full_trigger(world, ctx, target, EXTRA_STRIKE, 300);
}

fn read_proc_chance(world: &World, caster: Entity) -> Option<f32> {
    if !auras::has_aura(world, caster, TALENT) {
        return None;
    }
    let spells = world.resource::<SpellStore>();
    let chance = |spell| {
        spells
            .get(spell)
            .expect("native Opportunity chance spell")
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native Opportunity chance effect")
            .base_points
    };
    // SimC76f8118 sc_rogue.cpp1286–1292 omits Dragon-Bone Dice's
    // native label modifier. Apply it to RtB once, not to the already modified talent aura.
    let bonus = if auras::has_aura(world, caster, ROLL_THE_BONES) {
        script_effect_value(world, caster, JACKPOT, 0)
    } else {
        0.0
    };
    Some(chance(TALENT) + bonus)
}

fn grant_opportunity(world: &mut World, caster: Entity) {
    let extra = if auras::has_aura(world, caster, FAN_THE_HAMMER) {
        script_effect_value(world, caster, FAN_THE_HAMMER, 0) as u8
    } else {
        0
    };
    let mut spell =
        spell_cast::load_triggered_spell(world, OPPORTUNITY, 1).expect("native Opportunity buff");
    spell.cast_aura_stacks = Some(1 + extra);
    spell_cast::launch_triggered_spell(world, caster, &spell, Some(caster), 1);
}
