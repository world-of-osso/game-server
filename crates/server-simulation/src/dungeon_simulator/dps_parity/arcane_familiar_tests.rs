//! Actual literal Arcane Intellect precombat must establish the selected Familiar.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::class_progression::TraitLoadout;
use shared::components::Mana;

#[test]
#[ignore = "requires private native70448 Arcane player profile"]
fn arcane_familiar_native_precombat_intellect_grants_selected_max_mana_aura() {
    // SimC76f8118 sc_mage.cpp3080–3092: actual Arcane Intellect executes
    // and triggers the talent-gated210126 Familiar;6360–6380 and the native
    // 70448 effect0/178 add10% maximum Mana without changing native stats.
    let mut run = prepare_measurement(62, 1).unwrap();
    let world = run.app.world();
    assert!(
        world
            .get::<TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|talent| talent.spell_id == 205022),
        "native profile must actually select Arcane Familiar"
    );
    let actions = report::combat(world, &run.damage_targets, 1.0);
    assert!(
        actions["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["spell_id"] == 1459 && action["count"].as_f64().unwrap() > 0.0),
        "native precombat Arcane Intellect must execute: {actions}"
    );
    let pre_mana = world.get::<Mana>(run.actor).unwrap().max;
    let present = world
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .any(|aura| aura.spell_id == 210126 && aura.caster == run.actor);
    // Direct application is the same native aura/values, used only as an oracle;
    // it does not replace the actual precombat producer under test.
    let actor = run.actor;
    if !present {
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 210126,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    }
    let corrected = run.app.world().get::<Mana>(actor).unwrap().max;
    assert!(
        present,
        "selected Familiar must already be active after real Intellect: observed max Mana{pre_mana}, native210126 yields{corrected}"
    );
}
