//! Vengeance Mass Acceleration restores Spirit Bomb when Metamorphosis is cast.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterNativeLaunch(launch)];

fn launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 187827
        || !world
            .get::<combat::class_progression::TraitLoadout>(caster)
            .is_some_and(|traits| {
                traits
                    .active_talent_spells()
                    .any(|talent| talent.spell_id == 1256295)
            })
    {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp3141–3170: Mass Acceleration
    // resets Spirit Bomb on each Vengeance Metamorphosis execution.
    let now = spell_cast::now(world);
    let remaining = {
        let spell = world
            .resource::<SpellStore>()
            .get(247454)
            .expect("native Spirit Bomb");
        world
            .get::<spell_cast::SpellHistory>(caster)
            .map_or(0, |history| history.remaining_cooldown_ms(spell, now))
    };
    if remaining > 0 {
        // remaining_cooldown_ms truncates, so include its final fractional millisecond.
        spell_cast::scripts::reduce_cooldown(
            world,
            caster,
            247454,
            (remaining + 1) as f64 / 1000.0,
        );
    }
}
