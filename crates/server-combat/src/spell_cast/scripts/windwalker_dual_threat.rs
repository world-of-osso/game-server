//! SimC76f8118 sc_monk.cpp2038–2108: selected Dual Threat replaces eligible
//! white impacts, shares one alternating gate across hands and credits Flurry
//! through its actual native451839 child rather than the replaced white impact.
use bevy::prelude::*;
use shared::components::WeaponType;

use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::combat::WhiteImpactReplacement;
use crate::stats::CharacterStats;

const TALENT: u32 = 451823;
pub(super) const DAMAGE: u32 = 451839;
/// SimC76f8118 sc_monk.cpp2051: authored two-hand normalization, not an AP coefficient.
const TWO_HAND_ADJUSTMENT: f32 = 3.6 / 2.6 * 2.0;

pub(crate) fn read_replacement(
    auras: Option<&ActiveAuras>,
    traits: Option<&TraitLoadout>,
    caster: Entity,
) -> Option<WhiteImpactReplacement> {
    if !traits?
        .active_talent_spells()
        .any(|spell| spell.spell_id == TALENT)
    {
        return None;
    }
    let passive = auras?
        .iter()
        .find(|aura| aura.spell_id == TALENT && aura.caster == caster)?;
    let chance_pct = passive
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Dual Threat retains its replacement chance")
        .amount;
    Some(WhiteImpactReplacement {
        spell_id: DAMAGE,
        chance_pct,
    })
}

pub(crate) fn roll_replacement(
    replacement: Option<WhiteImpactReplacement>,
    allowed: &mut bool,
    landed: bool,
) -> Option<u32> {
    let replacement = replacement?;
    let replace =
        *allowed && landed && crate::random::random::<f32>() * 100.0 < replacement.chance_pct;
    *allowed = !replace;
    replace.then_some(replacement.spell_id)
}

pub(super) fn damage(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    let stats = world
        .get::<CharacterStats>(caster)
        .expect("native Dual Threat caster requires equipped weapon stats");
    if stats.main_hand.kind == WeaponType::TwoHand {
        amount * TWO_HAND_ADJUSTMENT
    } else {
        amount
    }
}
