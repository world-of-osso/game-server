use super::*;
use crate::auras::{self, ActiveAuras, AuraApplication, AuraSource};
use crate::spell_info::SpellStore;

#[test]
#[ignore = "requires selected MID2 Protection profile and native combat catalogs"]
fn dps_parity_protection_demolish_stacks_wrecked_on_actual_impacts() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(73, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(auras::has_aura(run.app.world(), actor, 429636));
    engage(&mut run);
    for _ in 0..5 {
        auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 440989,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    let might = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 440989 && aura.caster == actor)
        .unwrap()
        .stacks;
    assert_eq!(might, 5);
    let shield = run
        .app
        .world()
        .resource::<SpellStore>()
        .get(23922)
        .unwrap()
        .clone();
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 436358, Some(target), false)
        .unwrap();
    for _ in 0..60 {
        run.app.update();
    }
    let stacks = run
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 447513 && aura.caster == actor)
        .expect("Demolish impacts apply Wrecked")
        .stacks;
    assert_eq!(
        stacks, 10,
        "each impact adds five stacks up to the native cap"
    );
    let after = auras::damage_taken_multiplier(run.app.world(), target, actor, &shield);
    auras::remove_auras(
        run.app.world_mut(),
        target,
        auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 447513 && aura.caster == actor,
    );
    let before = auras::damage_taken_multiplier(run.app.world(), target, actor, &shield);
    assert!(
        (after / before - 1.20).abs() < 0.001,
        "ten Wrecked stacks: {before} -> {after}"
    );
}

fn devastating_crit_multiplier(run: &mut Run) -> f64 {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let actor = run.actor;
    let target = run.targets[0];
    run.app.world_mut().get_mut::<crate::stats::CharacterStats>(actor).unwrap().crit_pct = 100.0;
    run.app.insert_resource(FactorCapture::new(actor, target, 236282, "devastator", 1));
    assert!(crate::spell_cast::try_cast_triggered(run.app.world_mut(), actor, 236282, Some(target), 1));
    let capture = run.app.world().resource::<FactorCapture>().export();
    let hit = capture["events"].as_array().unwrap().iter().find(|event| event["event"] == "direct_amount" && event["spell_id"] == 236282).expect("Devastator hit");
    hit["applied_multipliers"].as_array().unwrap().iter().find(|factor| factor["factor"] == "critical_damage").unwrap()["value"].as_f64().unwrap()
}

#[test]
#[ignore = "requires selected MID2 Protection profile and native combat catalogs"]
fn dps_parity_protection_wrecked_does_not_increase_devastator_crit_multiplier() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(73, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let target = run.targets[0];
    let before = devastating_crit_multiplier(&mut run);
    assert!(crate::spell_cast::cast_triggered_with_overrides(run.app.world_mut(), actor, 447513, Some(target), 1, crate::spell_cast::TriggeredSpellOverrides { aura_stacks: Some(10), ..Default::default() }));
    let after = devastating_crit_multiplier(&mut run);
    assert!((before - after).abs() < 0.001, "Wrecked must not modify Devastator critical damage: {before} -> {after}");
}
