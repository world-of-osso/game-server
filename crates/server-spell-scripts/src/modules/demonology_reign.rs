//! Reign of Tyranny extends actual Dreadstalkers at the Tyrant's summon.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::{self, targets};
use shared::components::Npc;

const TYRANT: u32 = 265187;
const REIGN: u32 = 1276748;
const DEMONIC_POWER: u32 = 1276788;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::SummonInitialized(on_tyrant)];

fn on_tyrant(world: &mut World, tyrant: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(tyrant) else {
        return;
    };
    if summon.spell_id != TYRANT
        || world
            .get::<Npc>(tyrant)
            .is_none_or(|npc| !matches!(npc.template_id, 135002 | 250289))
    {
        return;
    }
    let owner = summon.owner;
    let owned: Vec<_> = world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .filter(|(actor, summon)| summon.owner == owner && targets::is_alive(world, *actor))
        .map(|(actor, summon)| (actor, summon.spell_id))
        .collect();
    if combat::auras::has_aura(world, owner, REIGN) {
        // SimC76f8118 actions.cpp3586–3600: Reign extends living dogs by 15s.
        for &(dog, root) in &owned {
            if matches!(root, 193331 | 193332) {
                if let Some(mut summon) = world.get_mut::<SummonedCreature>(dog) {
                    if let Some(expiry) = summon.expires_at.as_mut() {
                        *expiry += 15.0;
                    }
                }
            }
        }
    }
    // actions.cpp3608–3620: in the pinned bug model each Vilefiend also counts
    // once for the newly summoned Tyrant's native Demonic Power.
    for (_, root) in owned {
        if matches!(root, 1251781 | 455465 | 455476)
            && !spell_cast::try_cast_triggered(world, tyrant, DEMONIC_POWER, Some(tyrant), 1)
        {
            warn!(?tyrant, "native Demonic Power stack refused");
        }
    }
}
