//! SimC76f8118 sc_monk.cpp4070–4089, 5407–5433 and 7300–7305.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
};
use combat::spell_info::SpellStore;
use combat::stats::CharacterStats;

const FORTUNE: u32 = 216519;
const HEAL: u32 = 216521;
const LEECH: u32 = 143924;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::HealReceived(receive_heal)];

fn receive_heal(
    world: &mut World,
    target: Entity,
    spell: u32,
    raw: f32,
    effective: f32,
    percent_based: bool,
) {
    if raw <= 0.0
        || percent_based
        || matches!(spell, HEAL | LEECH)
        || !world
            .get::<ActiveSpec>(target)
            .is_some_and(|spec| spec.0 == 268)
    {
        return;
    }
    let stats = world
        .get::<CharacterStats>(target)
        .expect("Brewmaster stats");
    let chance = stats.crit_pct.clamp(0.0, 100.0);
    let versatility = 1.0 + stats.versatility_pct / 100.0;
    use combat::random::Rng;
    if combat::random::thread_rng().gen_range(0.0..100.0) >= chance {
        return;
    }
    let store = world.resource::<SpellStore>();
    let percent = store
        .get(FORTUNE)
        .expect("native Celestial Fortune")
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Celestial Fortune percentage")
        .base_points
        / 100.0;
    let heal = store
        .get(HEAL)
        .expect("native Celestial Fortune heal")
        .clone();
    let ctx = EffectCtx {
        caster: target,
        spell: &heal,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::apply_fixed_heal(world, &ctx, target, effective * percent * versatility);
}
