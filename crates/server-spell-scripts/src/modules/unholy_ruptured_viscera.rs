//! Naturally expired lesser ghouls produce their native Ruptured Viscera.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, summon::SummonedCreature};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AdditionalReachActions(autonomous_actions),
    SpellScriptHook::SummonExpired(expire),
];

fn autonomous_actions(world: &World, actor: Entity) -> Vec<u32> {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return Vec::new();
    };
    let ghoul = matches!(summon.spell_id, 275430 | 1294026);
    let selected =
        combat::spell_cast::script_registry::reach_source_selected(world, summon.owner, 288848);
    if ghoul && selected {
        vec![1247379]
    } else {
        Vec::new()
    }
}

fn expire(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    if !matches!(summon.spell_id, 275430 | 1294026)
        || !combat::auras::has_aura(world, summon.owner, 288848)
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp3697–3704: the two pet critical
    // multipliers expire before the actual ghoul's Ruptured Viscera.
    combat::auras::remove_auras(world, actor, combat::auras::RemoveReason::Expired, |aura| {
        matches!(aura.spell_id, 1238902 | 1250728) && aura.caster == actor
    });
    if !spell_cast::try_cast_triggered(world, actor, 1247379, Some(actor), 1) {
        warn!(
            ?actor,
            "native Ruptured Viscera refused on lesser ghoul expiry"
        );
    }
}
