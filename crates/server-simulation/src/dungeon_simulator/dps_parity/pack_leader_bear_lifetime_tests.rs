use super::*;

fn consume_natural_beast(run: &mut Run, ready: u32) -> f64 {
    let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
    while !crate::auras::has_aura(run.app.world(), run.actor, ready)
        && crate::spell_cast::now(run.app.world()) < deadline
    {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, ready),
        "actual native clock must earn readiness{ready}"
    );
    let started = crate::spell_cast::now(run.app.world());
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary KC consumes actual earned beast");
    preparation::finish_pending_setup(run).unwrap();
    started
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_pack_leader_bear_summon_retires_at_its_authored_lifetime() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    for ready in [471878, 472324] {
        consume_natural_beast(&mut run, ready);
    }
    let born = consume_natural_beast(&mut run, 472325);
    let bear = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == run.actor && summon.spell_id == 471990)
        .map(|(bear, _)| bear)
        .expect("actual owned native Bear");
    let lifetime_ms = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(471993)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .as_ref()
        .unwrap()
        .duration_ms;
    assert!(
        lifetime_ms > 0,
        "native linked Bear driver supplies a finite lifetime"
    );
    let deadline = born + f64::from(lifetime_ms) / 1000.0;
    let margin = run.app.world().resource::<Time>().delta_secs_f64() * 2.0;
    assert!(margin > 0.0);
    // SimC76f8118 hunter3797/7835: real Bear uses471993's authored
    // duration, not471990's raw infinite summon duration.
    while crate::spell_cast::now(run.app.world()) < deadline - margin {
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(bear).is_ok(),
        "native Bear cannot retire before its duration"
    );
    while crate::spell_cast::now(run.app.world()) <= deadline + margin {
        run.app.update();
    }
    assert!(
        run.app.world().get_entity(bear).is_err(),
        "actual Bear must retire at native471993 lifetime, not remain a permanent Wildspeaker body: born={born}, deadline={deadline}"
    );
}
