use super::*;

fn wait_for_beast(run: &mut Run, spell: u32) {
    let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
    while !crate::auras::has_aura(run.app.world(), run.actor, spell)
        && crate::spell_cast::now(run.app.world()) < deadline
    {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, spell),
        "naturally earned beast{spell}"
    );
}

fn consume_beasts(run: &mut Run) {
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary KC consumes naturally earned beasts");
    preparation::finish_pending_setup(run).unwrap();
}

fn prepare_bear_ready() -> Run {
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1232739));
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    for ready in [471878, 472324] {
        wait_for_beast(&mut run, ready);
        consume_beasts(&mut run);
    }
    wait_for_beast(&mut run, 472325);
    run
}

fn find_bear(run: &mut Run) -> Entity {
    run.app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == run.actor && summon.spell_id == 471990)
        .map(|(unit, _)| unit)
        .expect("real naturally earned Bear")
}

fn owned_aura(run: &Run, unit: Entity, spell: u32) -> crate::auras::AuraInstance {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(unit)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == unit)
        .unwrap_or_else(|| panic!("native aura{spell} must be owned by{unit:?}"))
        .clone()
}

fn cast_wrath(run: &mut Run) {
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        19574,
        Some(run.actor),
        false,
    )
    .expect("actual native owner Bestial Wrath");
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_wildspeaker_wrath_reaches_an_existing_actual_bear() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_bear_ready();
    consume_beasts(&mut run);
    let bear = find_bear(&mut run);
    assert!(!crate::auras::has_aura(run.app.world(), bear, 1235388));
    cast_wrath(&mut run);
    // SimC76f8118 hunter6729–6736 applies native Wildspeaker Wrath to
    // existing DireCritters; the ordinary stable-pet buff is not a substitute.
    let wrath = owned_aura(&run, bear, 1235388);
    assert!(wrath.duration_ms > 0 && wrath.expires_at.is_some());
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_wildspeaker_new_bear_inherits_only_remaining_owner_wrath() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_bear_ready();
    cast_wrath(&mut run);
    preparation::finish_pending_setup(&mut run).unwrap();
    let owner_deadline = owned_aura(&run, run.actor, 19574).expires_at.unwrap();
    consume_beasts(&mut run);
    let bear = find_bear(&mut run);
    let wrath = owned_aura(&run, bear, 1235388);
    // SimC76f8118 hunter1866–1867 inherits remaining time, never a fresh
    // full duration when a DireCritter joins an already running owner BW.
    assert!(
        (wrath.expires_at.unwrap() - owner_deadline).abs() < 0.002,
        "new Bear must inherit actual owner BW deadline: pet={:?}, owner={owner_deadline}",
        wrath.expires_at
    );
}
