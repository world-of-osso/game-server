//! Sudden Doom commands the actual Army or raises a lesser ghoul.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::AfterCastHook, summon::SummonedCreature};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AuraRollAfterCast(after_cast)];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 47541 | 1242174 | 207317 | 383269).then_some(command_ghouls)
}

fn command_ghouls(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, 455386) || !combat::auras::has_aura(world, caster, 81340) { return; }
    // SimC76f8118 sc_death_knight.cpp13527–13559: a Sudden Doom spender
    // commands every active Army ghoul, otherwise summons its real lesser ghoul.
    if !combat::auras::has_aura(world, caster, 42650) {
        if !spell_cast::try_cast_triggered(world, caster, 275430, Some(caster), 1) {
            warn!(?caster, "Doomed Bidding lesser ghoul summon refused");
        }
        return;
    }
    let Some(target) = world.get::<combat::networking::CombatTarget>(caster).and_then(|target| target.0) else {
        warn!(?caster, "Doomed Bidding has no selected target"); return;
    };
    let order = if matches!(spell.id, 207317 | 383269) { 1300330 } else { 1294264 };
    let now = combat::auras::aura_now(world);
    let ghouls: Vec<_> = world.query::<(Entity, &SummonedCreature)>().iter(world)
        .filter_map(|(entity, summon)| (summon.owner == caster && summon.spell_id == 1294026
            && summon.expires_at.is_some_and(|expiry| expiry > now)).then_some(entity)).collect();
    for ghoul in ghouls {
        if !spell_cast::try_cast_triggered(world, ghoul, order, Some(target), 1) {
            warn!(?ghoul, order, "Doomed Bidding Army order refused");
        }
    }
}
