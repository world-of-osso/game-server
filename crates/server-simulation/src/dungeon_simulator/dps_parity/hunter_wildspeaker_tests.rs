use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_wildspeaker_commands_existing_bear_before_consuming_next_howl() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1232739),
        "native profile selects Wildspeaker"
    );
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    for ready in [471878, 472324, 472325] {
        let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
        while !crate::auras::has_aura(run.app.world(), run.actor, ready)
            && crate::spell_cast::now(run.app.world()) < deadline
        {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), run.actor, ready));
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            34026,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary KC consumes naturally earned beast");
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    let bear = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == run.actor && summon.spell_id == 471990)
        .map(|(bear, _)| bear)
        .expect("actual naturally earned Bear");
    let attempts = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(bear.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.spell_id == Some(1232922)
                    && matches!(
                        event.kind,
                        shared::protocol::CombatLogKind::Damage
                            | shared::protocol::CombatLogKind::Miss(_)
                    )
            })
            .count()
    };
    // SimC76f8118 sc_hunter.cpp6590–6604 commands existing DireCritters
    // before consuming Howl. A newly summoned Bear cannot join the earlier KC.
    assert_eq!(
        attempts(&run),
        0,
        "new Bear must not retroactively receive its summoning KC"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("next ordinary Kill Command");
    preparation::finish_pending_setup(&mut run).unwrap();
    assert_eq!(
        attempts(&run),
        1,
        "the existing Bear must execute one native Wildspeaker command at the actual primary"
    );
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_wildspeaker_dire_beasts_inherit_owner_crit() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    let mut summons = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>();
    let mut observed_actor = None;
    let mut seen = 0;
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 459550));
    let mut critical_hits = 0;
    let mut critical_bonuses = Vec::new();
    let mut white_critical_hits = 0;
    let mut seen_combat_events = 0;
    let mut beasts = std::collections::BTreeSet::new();
    let mut chances = Vec::new();
    let mut printed = false;
    engage(&mut run);
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    drive_player(&mut run).expect("native literal combat");
    let frame = FIGHT_DURATION / TICKS;
    let mut retired = Vec::new();
    for tick in 1..=TICKS {
        advance_fixture_frame(
            &mut run,
            started_at + frame * tick,
            started_at,
            &mut retired,
            &mut |_| {},
        )
        .expect("ordinary native combat frame");
        for (actor, summon) in summons.iter(run.app.world()) {
            if summon.owner == run.actor && summon.spell_id == 219199 {
                beasts.insert(actor.to_bits());
            }
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[seen_combat_events..] {
            if event.source.is_some_and(|source| beasts.contains(&source))
                && event.target == Some(run.targets[0].to_bits())
                && event.spell_id.is_none()
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.crit
            {
                white_critical_hits += 1;
            }
        }
        seen_combat_events = events.len();
        if let Some(actor) = observed_actor {
            let world = run.app.world();
            let capture = world
                .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
                .export();
            let events = capture["events"].as_array().expect("native factor events");
            for event in &events[seen..] {
                if event["event"] != "result" || event["spell_id"] != 1232922 {
                    continue;
                }
                if !printed {
                    eprintln!("WILDSPEAKER_RESULT {event}");
                    printed = true;
                }
                let owner_crit = world
                    .get::<crate::stats::CharacterStats>(run.actor)
                    .unwrap()
                    .crit_pct;
                let chance = event["factors"]["crit"]
                    .as_f64()
                    .expect("observed crit chance");
                chances.push((chance, owner_crit));
                if event["factors"]["result"] == "crit" {
                    critical_hits += 1;
                    let bonus = event["factors"]["crit_bonus"]
                        .as_f64()
                        .expect("observed crit bonus");
                    let effect = crate::spell_cast::scripts::script_effect_value(
                        world, run.actor, 459550, 1,
                    );
                    critical_bonuses.push((bonus, owner_crit, effect));
                }
            }
            seen = events.len();
            if world
                .get::<crate::spell_cast::summon::SummonedCreature>(actor)
                .is_none()
            {
                observed_actor = None;
            }
        }
        if observed_actor.is_none() {
            let actor = summons
                .iter(run.app.world())
                .find(|(_, summon)| summon.owner == run.actor && summon.spell_id == 219199)
                .map(|(actor, _)| actor);
            if let Some(actor) = actor {
                let world = run.app.world();
                let spell = world
                    .resource::<crate::spell_info::SpellStore>()
                    .get(1232922)
                    .unwrap();
                let owner = crate::auras::spell_mods::modify(
                    world,
                    run.actor,
                    spell,
                    crate::auras::SpellModOp::HealingAndDamage,
                    100.0,
                );
                let pet = crate::auras::spell_mods::modify(
                    world,
                    actor,
                    spell,
                    crate::auras::SpellModOp::HealingAndDamage,
                    100.0,
                );
                eprintln!(
                    "WILDSPEAKER_OWNER_MOD owner={owner} pet={pet} hunterpet={} savagery={}",
                    world.get::<crate::hunter_pet::HunterPet>(actor).is_some(),
                    crate::auras::has_aura(world, run.actor, 424557)
                );
                run.app.insert_resource(
                    crate::spell_cast::effects::factor_capture::FactorCapture::new(
                        actor,
                        run.targets[0],
                        1232922,
                        "wildspeaker",
                        100,
                    ),
                );
                observed_actor = Some(actor);
                seen = 0;
            }
        }
    }
    assert!(
        !chances.is_empty(),
        "actual Dire Beast Wildspeaker must land a hit"
    );
    // SimC76f8118 pet.cpp217–220: DireCritters inherit their owner's live crit.
    assert!(
        chances
            .iter()
            .all(|(chance, owner)| *chance + 0.001 >= f64::from(*owner) / 100.0),
        "actual Dire Beast Wildspeaker must inherit owner crit: {chances:?}"
    );
    assert!(
        critical_hits > 0,
        "actual Dire Beast Wildspeaker must land a natural crit"
    );
    assert!(
        white_critical_hits > 0,
        "actual Dire Beast white swings must land a natural crit"
    );
    // SimC76f8118 sc_hunter.cpp3192–3199 scales the actual Wildspeaker
    // critical bonus by native Go for the Throat and live owner crit.
    assert!(
        critical_bonuses
            .iter()
            .all(|(bonus, crit, effect)| *bonus + 0.001
                >= 1.0 + f64::from(*crit * *effect) / 10_000.0),
        "actual Wildspeaker crits omit Go for the Throat: {critical_bonuses:?}"
    );
}
