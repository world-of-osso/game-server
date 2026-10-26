//! SimC76f8118 sc_rogue.cpp6618–6631,11568–11605: off-mark damage
//! contributes a native fraction of its resolved amount to the owned live mark.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::SpellScriptHook,
    targets,
};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedDamage(after_spell),
    SpellScriptHook::LandedDamagePolicy(after_white),
];
const TALENT: u32 = 457055;
const MARK: u32 = 457129;
const DAMAGE: u32 = 457236;

fn after_spell(world: &mut World, ctx: &EffectCtx, target: Entity, _crit: bool, amount: f32) {
    copy_to_mark(world, ctx.caster, target, amount, ctx.depth + 1);
}

fn after_white(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: Option<u32>,
    amount: f32,
    _periodic: bool,
) {
    // ResolvedDamage owns spells and excludes copies. This hook supplies only
    // the actual mitigated weapon outcome, not MeleeResidual's pre-mitigation amount.
    if spell.is_none() {
        copy_to_mark(world, caster, target, amount, 1);
    }
}

fn read_fraction(world: &World, caster: Entity) -> Option<f32> {
    let talent = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == TALENT && aura.caster == caster)?;
    let amount = talent
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Singular Focus retains its copied fraction")
        .amount;
    Some(amount / 100.0)
}

fn find_owned_mark(world: &mut World, caster: Entity, source: Entity) -> Option<Entity> {
    let now = spell_cast::now(world);
    world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .find_map(|(unit, auras)| {
            let marked = auras.iter().any(|aura| {
                aura.spell_id == MARK
                    && aura.caster == caster
                    && aura.expires_at.is_none_or(|expires| expires > now)
            });
            (unit != source && marked && targets::is_alive(world, unit)).then_some(unit)
        })
}

fn copy_to_mark(world: &mut World, caster: Entity, source: Entity, amount: f32, depth: u8) {
    if amount <= 0.0 {
        return;
    }
    let Some(fraction) = read_fraction(world, caster) else {
        return;
    };
    let Some(target) = find_owned_mark(world, caster, source) else {
        return;
    };
    // sc_rogue.cpp1971–1995/2340–2350: the residual's own label3936
    // still receives native Mastery and target-health Zoldyck. Native payload
    // CANNOT_CRIT/NO_DONE_PCT flags prevent a second crit/general damage bonus;
    // background depth and the marked-recipient exclusion prevent proc recursion.
    spell_cast::cast_triggered_with_effect0_amount(
        world, caster, DAMAGE, Some(target), depth, amount * fraction,
    );
}
