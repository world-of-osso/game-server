//! SimC76f8118 sc_rogue.cpp10924–10932/10982–11010: expiring Shadow Blades
//! grants Lingering Darkness with only its server-side Subtlety effects.
use super::{combat, has_talent};
use bevy::prelude::*;
use combat::auras::scripts::AuraScripts;
use combat::auras::{self, AuraApplication, AuraInstance, AuraSource, RemoveReason};
use shared::components::ActiveSpec;

const BLADES: u32 = 121471;
const TALENT: u32 = 457056;
const DARKNESS: u32 = 457273;
// Zero-based effects 1/3 (Shadow direct/periodic), 4/5 (Dark Brew) and 6 (label).
// Effects 0/2 are Assassination's poison mods (sc_rogue.cpp10987–10991).
const SUBTLETY_EFFECTS: u32 = 0b111_1010;

pub(super) fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(BLADES).or_default();
    assert!(
        script.on_remove.is_none(),
        "Shadow Blades removal hook taken"
    );
    script.on_remove = Some(grant_darkness);
}

fn grant_darkness(world: &mut World, unit: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    let subtlety = world
        .get::<ActiveSpec>(unit)
        .is_some_and(|spec| spec.0 == 261);
    if unit != aura.caster || !subtlety || !has_talent(world, unit, TALENT) {
        return;
    }
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: DARKNESS,
            effect_mask: SUBTLETY_EFFECTS,
            source: AuraSource::Cast,
        },
    );
}
