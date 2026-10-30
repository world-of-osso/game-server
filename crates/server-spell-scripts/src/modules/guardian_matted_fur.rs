//! SimC76f8118 sc_druid.cpp3453–3471, 7087–7092, 8799–8805.
use bevy::prelude::*;

use super::combat;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;
use combat::stats::{CharacterStats, StatModifiers, WeaponAttackType};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeOnCast(on_cast),
    SpellScriptHook::OwnsPolicyProc(owns_proc),
];

const TALENT: u32 = 385786;
const SHIELD: u32 = 385787;

fn owns_proc(spell: u32) -> bool {
    spell == TALENT
}

fn on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if !matches!(spell, 22812 | 61336)
        || !world.get::<TraitLoadout>(caster).is_some_and(|loadout| {
            loadout
                .active_talent_spells()
                .any(|talent| talent.spell_id == TALENT)
        })
    {
        return;
    }
    let stats = world
        .get::<CharacterStats>(caster)
        .expect("Matted Fur caster stats");
    // SimC76f8118 sc_druid.cpp2998–3001 uses total attack power, including the weapon.
    let attack_power = stats.total_attack_power(
        WeaponAttackType::MainHand,
        world.get::<StatModifiers>(caster),
    );
    let versatility = 1.0 + stats.versatility_pct / 100.0;
    let store = world.resource::<SpellStore>();
    let coefficient = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0) / 100.0;
    let mut shield = store.get(SHIELD).expect("Matted Fur absorb").clone();
    shield
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Matted Fur absorb effect")
        .base_points = attack_power * coefficient * versatility;
    spell_cast::launch_triggered_spell(world, caster, &shield, Some(caster), depth + 1);
}
