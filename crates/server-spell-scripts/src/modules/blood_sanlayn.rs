//! Blood San'layn's cooldown-driven Gift of the San'layn.
use bevy::prelude::*;
use shared::components::ActiveSpec;
use super::combat;
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook, TriggeredSpellOverrides};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterForegroundSpecial(grant_gift),
];

// SimC76f8118 sc_death_knight.cpp7279–7300/9531–9536: Vampiric Blood
// and Dancing Rune Weapon grant Gift for their native duration.
fn grant_gift(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !matches!(spell.id, 49028 | 55233)
        || !world.get::<ActiveSpec>(caster).is_some_and(|spec| spec.0 == 250)
        || !world.get::<KnownSpells>(caster).is_some_and(|known| known.contains(434152))
    {
        return;
    }
    let duration = spell.misc.as_ref().and_then(|misc| misc.duration).expect("Blood cooldown duration").duration_ms;
    spell_cast::cast_triggered_with_overrides(world, caster, 434153, Some(caster), 1,
        TriggeredSpellOverrides { duration_ms: Some(duration), ..Default::default() });
}
