//! Lycara's Teachings follows real Bear and Cat Form transitions.
use super::*;
use crate::auras::ActiveAuras;

#[test]
#[ignore = "requires selected Lycara's Teachings and native world data"]
fn guardian_lycaras_bear_versatility_follows_form_transitions() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
    let talent = auras
        .iter()
        .find(|a| a.spell_id == 378988 && a.caster == actor)
        .unwrap();
    let talent_amount = talent.effects.iter().find(|e| e.index == 0).unwrap().amount;
    let bear = auras
        .iter()
        .find(|a| a.spell_id == 378991 && a.caster == actor)
        .unwrap();
    assert_eq!(
        bear.effects.iter().find(|e| e.index == 0).unwrap().amount,
        talent_amount
    );
    assert!(talent_amount > 0.0);
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 768, Some(actor), false).unwrap();
    let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
    assert!(!auras.has_spell(378991));
    assert!(auras.has_spell(378990));
    for _ in 0..40 {
        run.app.update();
    }
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    let auras = run.app.world().get::<ActiveAuras>(actor).unwrap();
    assert!(auras.has_spell(378991));
    assert!(!auras.has_spell(378990));
}
