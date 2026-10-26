//! SimC76f8118 sc_priest_shadow.cpp1521–1619/2655–2669.
//! Eligible actual hits/ticks copy native Psychic Link's fraction to other own VT.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    self,
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
};
use combat::spell_info::SpellStore;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedDamage(copy_link),
    SpellScriptHook::ConsumedEffectValues(&[(TALENT, 0)]),
];
const TALENT: u32 = 199484;
const PAYLOAD: u32 = 199486;
const VAMPIRIC_TOUCH: u32 = 34914;
// sc_priest.cpp285/1053/1446 and sc_priest_shadow.cpp71/1301/1313/1465/1902/1923.
const SOURCES: &[u32] = &[
    8092, 15407, 32379, 263165, 323673, 335467, 391403, 450983, 1243105, 1243106,
];

fn native_fraction(world: &World, caster: Entity) -> Option<f32> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == TALENT)?;
    let amount = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Psychic Link fraction")
        .amount;
    Some(amount / 100.0)
}

fn read_recipients(world: &mut World, caster: Entity, primary: Entity) -> Vec<Entity> {
    let now = spell_cast::now(world);
    world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter(|(target, auras)| {
            *target != primary
                && auras.iter().any(|aura| {
                    let owned_vt = aura.caster == caster && aura.spell_id == VAMPIRIC_TOUCH;
                    let live = aura.expires_at.is_none_or(|expiry| expiry > now);
                    let ticking = aura.effects.iter().any(|effect| effect.period_ms > 0);
                    owned_vt && live && ticking
                })
        })
        .map(|(target, _)| target)
        .collect()
}

fn copy_link(world: &mut World, source: &EffectCtx<'_>, primary: Entity, _: bool, amount: f32) {
    if amount <= 0.0 || !SOURCES.contains(&source.spell.id) {
        return;
    }
    let Some(fraction) = native_fraction(world, source.caster) else {
        return;
    };
    let recipients = read_recipients(world, source.caster, primary);
    if recipients.is_empty() {
        return;
    }
    deliver_link(world, source, recipients, amount * fraction);
}

fn deliver_link(world: &mut World, source: &EffectCtx<'_>, recipients: Vec<Entity>, amount: f32) {
    let spell = world
        .resource::<SpellStore>()
        .get(PAYLOAD)
        .expect("native Psychic Link damage payload")
        .clone();
    let ctx = EffectCtx {
        caster: source.caster,
        spell: &spell,
        depth: source.depth + 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    for target in recipients {
        // SimC1525–1539: result_amount includes the source crit; no new crit roll.
        // Native payload metadata supplies the recipient's damage/absorb path.
        effects::deal_damage(world, &ctx, target, amount, false, false);
    }
}
