//! SimC76f8118 sc_druid.cpp5974–5982 and 11376–11381.
use bevy::prelude::*;

use super::combat;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

const SET: u32 = 1296608;
const FORMS: &[u32] = &[102558, 50334];

#[derive(Component)]
struct Extension {
    aura_id: u32,
    milliseconds: u32,
}

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterForegroundSpecial(after_cast)];

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 77758 {
        return;
    }
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return;
    };
    let Some(set) = auras
        .iter()
        .find(|aura| aura.spell_id == SET && aura.caster == caster)
    else {
        return;
    };
    let value = |index| {
        set.effects
            .iter()
            .find(|effect| effect.index == index)
            .expect("Guardian 4pc extension effect")
            .amount
            .max(0.0) as u32
    };
    let (add, cap) = (value(0), value(1));
    let Some(form) = auras
        .iter()
        .find(|aura| FORMS.contains(&aura.spell_id) && aura.caster == caster)
    else {
        return;
    };
    let (id, form_spell) = (form.instance_id, form.spell_id);
    let previous = world
        .get::<Extension>(caster)
        .filter(|state| state.aura_id == id)
        .map_or(0, |state| state.milliseconds);
    if previous >= cap {
        return;
    }
    auras::extend_owned_duration(world, caster, caster, &[form_spell], add);
    world.entity_mut(caster).insert(Extension {
        aura_id: id,
        milliseconds: previous + add,
    });
}
