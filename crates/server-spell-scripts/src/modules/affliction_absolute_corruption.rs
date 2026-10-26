//! Absolute Corruption owns permanent Corruption/Wither, not catalog duration.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::script_registry::SpellScriptHook;
use shared::components::ActiveSpec;

const ABSOLUTE_CORRUPTION: u32 = 196103;
const CORRUPTION: u32 = 146739;
const WITHER: u32 = 445474;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterAuraApplication(
    preserve_absolute_lifetime,
)];

fn has_absolute_corruption(world: &World, caster: Entity) -> bool {
    let affliction = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 265);
    let selected = world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == ABSOLUTE_CORRUPTION)
    });
    affliction && selected
}

fn preserve_absolute_lifetime(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !matches!(aura.spell_id, CORRUPTION | WITHER) || !has_absolute_corruption(world, aura.caster)
    {
        return;
    }
    // SimC76f8118 actions759–762/1072–1077 models permanent DoTs as twice
    // the expected fight length. Native runtime permanence needs no fixture
    // clock; keep this actual instance's native stacks and periodic deadline.
    // Corruption's native max_stacks=0 means nonstacking, not a counter cap.
    let applied = auras::make_owned_aura_permanent(world, aura.caster, target, aura.spell_id);
    if !applied {
        warn!(
            ?target,
            spell = aura.spell_id,
            "stored Absolute Corruption aura missing"
        );
    }
}
