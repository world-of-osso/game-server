//! Actual Takedown's owner and main-pet children retain their native payloads.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource};
use combat::spell_cast::{self, script_registry::SpellScriptHook, scripts::script_effect_value};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::BeforeProcLaunch(grant_twin_fangs),
    SpellScriptHook::SuccessfulCastRelationships(after_cast),
];

const TAKEDOWN: u32 = 1250646;
const PLAYER_DAMAGE: u32 = 1253859;
const PET_DAMAGE: u32 = 1253862;
const OWNER_FOCUS: u32 = 1258571;
const TWIN_FANGS: u32 = 1272139;
const TIP: u32 = 260286;

fn grant_twin_fangs(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_hunter.cpp6390–6393: the selected native count
    // is granted before Takedown's buff and actual owner damage impact.
    if spell.id != TAKEDOWN || !super::cast_relationships::has_talent(world, caster, TWIN_FANGS) {
        return;
    }
    let count = script_effect_value(world, caster, TWIN_FANGS, 0) as u8;
    for _ in 0..count {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: TIP,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    }
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_hunter.cpp6396–6405: the native Takedown buff and
    // shared forced mark already exist before these two distinct actor impacts.
    if spell.id != TAKEDOWN {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    // sc_hunter.cpp6368–6379: native1258571 supplies the owner's Focus;
    // the pet child must not grant a second resource payload.
    spell_cast::cast_triggered(world, caster, OWNER_FOCUS, Some(caster), 1);
    spell_cast::cast_triggered(world, caster, PLAYER_DAMAGE, Some(target), 1);
    cast_pet_damage(world, caster, target);
}

fn cast_pet_damage(world: &mut World, owner: Entity, target: Entity) {
    let Some(pet) = combat::hunter_pet::summoned_pet(world, owner) else {
        return;
    };
    if !spell_cast::targets::is_alive(world, pet) {
        return;
    }
    let Some(mut spell) = spell_cast::load_triggered_spell(world, PET_DAMAGE, 1) else {
        return;
    };
    // sc_hunter.cpp3096–3105: this pet payload's native energize effect is
    // not reflected in-game; only its authored damage effect executes here.
    spell.effects.retain(|effect| effect.index != 1);
    spell_cast::launch_triggered_spell(world, pet, &spell, Some(target), 1);
}
