use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_festering_strike_earns_ghouls_and_next_native_scythe() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    let cast_strike = |run: &mut Run| {
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                85948,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(run).unwrap();
    };
    cast_strike(&mut run);
    let ready = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1254252 && aura.caster == run.actor)
        .expect("Festering Strike must earn the native ready-ghoul counter")
        .stacks;
    assert!(
        (2..=3).contains(&ready),
        "native effects2/3 supply two or three ghouls"
    );
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 458123));
    cast_strike(&mut run);
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1241077),
        "the real replacement Scythe must open its native tracking window"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(458128)
                && event.kind == CombatLogKind::CastSuccess)
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_putrefy_summons_native_lesser_ghoul() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(1277098)
                && event.kind == CombatLogKind::Summon
                && event.target.is_some()),
        "Putrefy must summon its native lesser ghoul"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_putrefy_ghoul_impacts_the_enemy() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..300 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(1277016)
            && event.target == Some(run.targets[0].to_bits())
            && event.kind == CombatLogKind::Damage
            && event.amount > 0),
        "the summoned Putrefy ghoul must deliver native single-target impact damage"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_putrefy_impact_raises_native_magus() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(252, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1256813));
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    for _ in 0..100 {
        run.app.update();
    }
    let world = run.app.world_mut();
    assert!(
        world
            .query::<&shared::components::Npc>()
            .iter(world)
            .any(|npc| npc.template_id == 163366),
        "selected Lord of the Dead must raise a native Magus after Putrefy impact"
    );
}

#[test]
#[ignore = "requires private native252 profile/traits and player actor"]
fn dk252_raise_dead_produces_a_permanent_controlled_melee_pet() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor)
        .expect("ordinary precombat Raise Dead must populate the native current-pet relation");
    assert_eq!(
        run.app
            .world()
            .get::<shared::components::Npc>(pet)
            .unwrap()
            .template_id,
        26125
    );
    assert_eq!(
        run.app
            .world()
            .get::<crate::hunter_pet::HunterPet>(pet)
            .unwrap()
            .owner,
        run.actor
    );
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    for _ in 0..1300 {
        drive_player(&mut run).unwrap();
        run.app.update();
    }
    assert!(
        crate::spell_cast::targets::is_alive(run.app.world(), pet),
        "the specialization ghoul is permanent, not the one-minute Frost/Blood guardian"
    );
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(pet.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "the real pet must reach and attack the selected hostile primary"
    );
}
