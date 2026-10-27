//! Source-qualified Demon Hunter Burning Blades residual damage.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook, targets};
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

const TALENT: u32 = 452408;
const DEVOURER_RESIDUAL: u32 = 1245654;
const DEVOURER_CHILDREN: &[u32] = &[1245414, 1239127];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LandedDamagePolicy(apply_chaos_brand),
    SpellScriptHook::LandedDamagePolicy(devourer_damage),
];

// SimC76f8118 sc_demon_hunter.cpp2452–2457,2594–2605: direct DH impacts
// apply the native target debuff after damage, including ordinary weapon swings.
fn apply_chaos_brand(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: Option<u32>,
    amount: f32,
    periodic: bool,
) {
    if amount <= 0.0
        || periodic
        || !matches!(
            world.get::<ActiveSpec>(caster).map(|s| s.0),
            Some(577 | 581 | 1480)
        )
    {
        return;
    }
    let dh_action = spell.is_none_or(|id| {
        world
            .resource::<SpellStore>()
            .get(id)
            .and_then(|info| info.class_options.as_ref())
            .is_some_and(|options| options.spell_class_set == 107)
    });
    if dh_action
        && targets::relation_matches(world, caster, target, targets::Relation::Enemy)
        && !spell_cast::try_cast_triggered(world, caster, 1490, Some(target), 1)
    {
        error!(?caster, ?target, "Native Chaos Brand cast refused");
    }
}

fn devourer_damage(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: Option<u32>,
    amount: f32,
    periodic: bool,
) {
    if amount <= 0.0 || periodic {
        return;
    }
    if world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(1480)
        || !world
            .get::<KnownSpells>(caster)
            .is_some_and(|spells| spells.contains(TALENT))
    {
        return;
    }
    if !spell.is_none_or(|id| DEVOURER_CHILDREN.contains(&id))
        || !targets::relation_matches(world, caster, target, targets::Relation::Enemy)
    {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp3088–3111,5830–5847,6697–6710,6875–6878.
    apply(world, caster, target, amount, DEVOURER_RESIDUAL);
}

pub(super) fn apply(world: &mut World, caster: Entity, target: Entity, damage: f32, residual: u32) {
    let Some(ticks) = native_tick_count(world, residual) else {
        error!(residual, "Native Burning Blades has no positive tick count");
        return;
    };
    let percent = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0);
    // Generic rolling-periodic refresh carries unpaid damage and keeps its tick phase.
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        caster,
        residual,
        Some(target),
        1,
        damage * percent / 100.0 / ticks as f32,
    );
}

fn native_tick_count(world: &World, residual: u32) -> Option<u32> {
    let spell = world.resource::<SpellStore>().get(residual)?;
    let duration = spell.misc.as_ref()?.duration?.duration_ms;
    let period = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)?
        .aura_period_ms;
    if duration <= 0 || period == 0 {
        return None;
    }
    let ticks = duration as u32 / period;
    (ticks > 0).then_some(ticks)
}
