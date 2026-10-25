//! DEVASTATION_GIANTKILLER_SIMC_POLICY: SimC76f8118 sc_evoker.cpp4613–4629.
//! Native362980 owns the mastery amount; TC a352b1fa has no consequence script.
use bevy::prelude::*;
use shared::components::Health;

use super::combat;
use combat::auras::{ActiveAuras, has_aura};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];

const EVOKER_FAMILY: u32 = 224;
const GIANTKILLER: u32 = 362980;
const TYRANNY: u32 = 376888;
const DRAGONRAGE: u32 = 375087;
const MINIMUM_HEALTH_FRACTION: f32 = 0.3;
// sc_evoker.cpp5721/6165/7449: Melt Armor, Deep Breath's DoT and Unbound Flame.
const FULL_MASTERY_CHILDREN: &[u32] = &[441172, 353759, 1292322];

fn read_mastery_amount(world: &World, caster: Entity) -> Option<f32> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == GIANTKILLER)?;
    Some(
        aura.effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native Giantkiller retains mastery effect0")
            .amount,
    )
}

fn receives_full_mastery(world: &World, caster: Entity, spell: u32) -> bool {
    let full_damage = has_aura(world, caster, DRAGONRAGE) || FULL_MASTERY_CHILDREN.contains(&spell);
    has_aura(world, caster, TYRANNY) && full_damage
}

fn multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    if spell
        .class_options
        .as_ref()
        .is_none_or(|options| options.spell_class_set != EVOKER_FAMILY)
    {
        return 1.0;
    }
    let Some(mastery) = read_mastery_amount(world, caster) else {
        return 1.0;
    };
    let fraction = if receives_full_mastery(world, caster, spell.id) {
        1.0
    } else {
        let health = world
            .get::<Health>(target)
            .expect("native damage target retains Health");
        (health.current / health.max).max(MINIMUM_HEALTH_FRACTION)
    };
    1.0 + mastery * fraction / 100.0
}
