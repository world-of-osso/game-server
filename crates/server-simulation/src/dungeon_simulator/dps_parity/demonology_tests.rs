//! Real profile/controller boundary, not a second synthetic rotation.
use super::*;

fn prepare_antoran_tyrant() -> (Run, Entity) {
    let mut run = prepare_literal(266, 1).unwrap();
    engage(&mut run);
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        265187,
        Some(run.actor),
        false,
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let actor = run
        .app
        .world_mut()
        .query::<(
            Entity,
            &crate::spell_cast::summon::SummonedCreature,
            &shared::components::Npc,
        )>()
        .iter(run.app.world())
        .find(|(_, summon, npc)| summon.owner == run.actor && npc.template_id == 250289)
        .map(|(entity, _, _)| entity)
        .expect("selected Antoran Armaments must summon actual250289");
    (run, actor)
}

#[test]
#[ignore = "requires private266 inputs; actual native alternative Tyrant root"]
fn demonology_antoran_tyrant_repeats_burning_cleave() {
    let _seed = crate::random::seed_scope(70448);
    let (mut run, tyrant) = prepare_antoran_tyrant();
    for _ in 0..200 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let hits = events
        .iter()
        .filter(|event| {
            event.source == Some(tyrant.to_bits())
                && event.spell_id == Some(1264093)
                && event.kind == shared::protocol::CombatLogKind::Damage
        })
        .count();
    assert!(
        hits >= 3,
        "real Antoran Tyrant must bind and repeat native Burning Cleave, got{hits}"
    );
}

#[test]
#[ignore = "requires private266 inputs; actual native alternative Tyrant SP inheritance"]
fn demonology_antoran_tyrant_inherits_same_owner_sp_as_base_tyrant() {
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    let _seed = crate::random::seed_scope(70448);
    let (mut run, tyrant) = prepare_antoran_tyrant();
    let expected = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .spell_power;
    run.app.world_mut().insert_resource(FactorCapture::new(
        tyrant,
        run.targets[0],
        1264093,
        "burning_cleave",
        1,
    ));
    for _ in 0..160 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    let formula = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| event["event"] == "direct_amount" && event["spell_id"] == 1264093)
        .expect("actual native Burning Cleave must calculate damage");
    let inherited = formula["factors"]["s_power"].as_f64().unwrap();
    assert!(
        (inherited - f64::from(expected)).abs() < 1.0,
        "alternative Tyrant keeps default owner SP1.0: actual={inherited}, owner={expected}"
    );
}

#[test]
#[ignore = "requires private266 profile and native world DB"]
fn demonology_literal_fixture_executes_profile_instead_of_native_kit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(266, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some(),
        "266 must hand control to the literal APL before native warmup"
    );
    assert!(crate::hunter_pet::summoned_pet(run.app.world(), run.actor).is_some());
    engage(&mut run);
    for _ in 0..400 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.targets, 20.0);
    let actions = combat["actions"].as_array().unwrap();
    assert!(
        actions
            .iter()
            .any(|row| row["spell_id"] == 686 && row["damage"].as_u64().unwrap() > 0),
        "literal precast/filler must actually damage the target"
    );
    assert!(
        actions
            .iter()
            .any(|row| row["spell_id"] == 1276452 && row["count"].as_u64().unwrap() > 0),
        "literal priority must cast selected Grimoire: Imp Lord"
    );
    assert!(
        !actions.iter().any(|row| row["spell_id"] == 702),
        "native follower Curse of Weakness is absent from the pinned APL"
    );
}
