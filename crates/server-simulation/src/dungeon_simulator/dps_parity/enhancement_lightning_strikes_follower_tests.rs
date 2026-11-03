//! Retail Lightning Strikes is earned by one full Maelstrom Weapon spend.
use super::*;
use crate::auras::{ActiveAuras, RemoveReason};
use shared::protocol::CombatLogKind;

fn maelstrom_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 344179 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

fn earn_ten(run: &mut Run) {
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
    for _ in 0..6000 {
        if maelstrom_stacks(run) == 10 {
            run.app
                .world_mut()
                .entity_mut(run.actor)
                .remove::<crate::combat::AutoAttack>();
            return;
        }
        run.app.update();
    }
    panic!("ordinary native melee did not earn ten Maelstrom Weapon stacks");
}

fn lava_lash_damage(with_buff: bool) -> f64 {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    earn_ten(&mut run);
    enhancement_generation_tests::cast(&mut run, 188196, target);
    assert!(crate::auras::has_aura(run.app.world(), actor, 384451));
    if !with_buff {
        crate::auras::remove_auras(
            run.app.world_mut(),
            actor,
            RemoveReason::Cancelled,
            |aura| aura.spell_id == 384451,
        );
    }
    run.app.insert_resource(
        crate::spell_cast::effects::factor_capture::FactorCapture::new(
            actor,
            target,
            60103,
            "lightning_strikes",
            1,
        ),
    );
    let _damage_seed = crate::random::seed_scope(818181);
    enhancement_generation_tests::cast(&mut run, 60103, target);
    for _ in 0..60 {
        let capture = run
            .app
            .world()
            .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
            .export();
        if let Some(hit) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "result" && event["spell_id"] == 60103)
        {
            return hit["factors"]["actual_amount"].as_f64().unwrap();
        }
        run.app.update();
    }
    panic!("real Lava Lash did not deal damage");
}

#[test]
#[ignore = "native263 reference gear, traits and private world database"]
fn enhancement_lightning_strikes_increases_real_lava_lash_damage() {
    let plain = lava_lash_damage(false);
    let buffed = lava_lash_damage(true);
    assert!(
        (buffed - plain * 1.25).abs() <= 2.0,
        "real Lightning Strikes must increase Lava Lash damage: {buffed}/{plain}"
    );
}

#[test]
#[ignore = "native263 reference gear, traits and private world database"]
fn enhancement_lightning_strikes_uses_one_ten_stack_spend_and_next_strike() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 384450));
    // Eliminate independent refunds so each observed spend has an unambiguous stack count.
    crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == 1262635,
    );
    engage(&mut run);
    for _ in 0..2 {
        earn_ten(&mut run);
        for id in [344179, 187881] {
            crate::auras::set_self_cast_stacks(run.app.world_mut(), actor, id, 5);
        }
        enhancement_generation_tests::cast(&mut run, 188196, target);
        assert!(
            !crate::auras::has_aura(run.app.world(), actor, 384451),
            "two separate five-stack spends must not accumulate toward the retail proc"
        );
    }
    earn_ten(&mut run);
    enhancement_generation_tests::cast(&mut run, 188196, target);
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 384451),
        "the actual ten-stack Bolt must earn Lightning Strikes"
    );
    // Give the actor real MW auras without a new melee strike consuming the first use.
    for _ in 0..10 {
        for id in [187881, 344179] {
            crate::spell_cast::cast_triggered(run.app.world_mut(), actor, id, Some(actor), 1);
        }
    }
    assert_eq!(maelstrom_stacks(&run), 10);
    enhancement_generation_tests::cast(&mut run, 188196, target);
    let lightning_stacks = |run: &Run| {
        run.app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 384451 && aura.caster == actor)
            .map_or(0, |aura| aura.stacks)
    };
    assert_eq!(
        lightning_stacks(&run),
        2,
        "two real ten-stack spends grant two uses"
    );
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    enhancement_generation_tests::cast(&mut run, 60103, target);
    for _ in 0..60 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(60103)
                && event.amount > 0),
        "the real Lava Lash must deal damage"
    );
    assert_eq!(
        lightning_stacks(&run),
        1,
        "the next eligible strike must consume one use, not both"
    );
    assert!(
        maelstrom_stacks(&run) > 0,
        "the strike must earn a Maelstrom Weapon stack"
    );
}
