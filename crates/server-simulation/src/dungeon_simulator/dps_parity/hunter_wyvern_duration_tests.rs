use super::*;

fn cry_deadline(run: &Run) -> f64 {
    run.app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 471881 && aura.caster == run.actor)
        .expect("actual earned native Wyvern Cry")
        .expires_at
        .expect("finite native Cry")
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_hunter_fury_of_wyvern_command_extends_existing_cry_without_refresh() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 472550));
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
    while !crate::auras::has_aura(run.app.world(), run.actor, 471878)
        && crate::spell_cast::now(run.app.world()) < deadline
    {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 471878));
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary KC consumes actual Wyvern and earns Cry");
    preparation::finish_pending_setup(&mut run).unwrap();
    let before = cry_deadline(&run);
    let extension = f64::from(crate::spell_cast::scripts::script_effect_value(
        run.app.world(),
        run.actor,
        472550,
        1,
    )) / 1000.0;
    assert!(
        extension > 0.0,
        "selected native Fury must author a duration extension"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        34026,
        Some(run.targets[0]),
        false,
    )
    .expect("second legal native KC extends already-active Cry");
    let after = cry_deadline(&run);
    // SimC76f8118 hunter6626–6630: extend the remaining Cry, not a
    // fresh full-duration application; native472550 effect2 owns the time.
    assert!(
        (after - before - extension).abs() < 0.001,
        "actual BM KC must extend existing Cry by native effect2: before={before}, after={after}, extension={extension}"
    );
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_hunter_wyvern_extension_caps_and_resets_after_actual_expiry() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, Some(run.targets[0]));
    let base = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(471881)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .as_ref()
        .unwrap()
        .duration_ms as u32;
    let cap =
        (crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 472550, 3)
            * 1000.0) as u32;
    let extension =
        crate::spell_cast::scripts::script_effect_value(run.app.world(), run.actor, 472550, 1)
            as u32;
    let owned_cry = |run: &Run| {
        run.app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.spell_id == 471881 && aura.caster == run.actor)
                    .cloned()
            })
    };
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started_at);
    let deadline = crate::spell_cast::now(run.app.world()) + 299.0;
    let mut at_cap = false;
    let mut cap_guard_seen = false;
    let mut capped_cry_expired = false;
    let mut reset_seen = false;
    let mut maximum_extra = 0;
    let mut refusals = std::collections::BTreeMap::<String, usize>::new();
    while crate::spell_cast::now(run.app.world()) < deadline {
        if at_cap && owned_cry(&run).is_none() {
            capped_cry_expired = true;
            at_cap = false;
        }
        // Ordinary native BW/KC/Barbed casts and their real cooldowns, GCD,
        // Focus regeneration and War Orders recovery; no state injection.
        for (spell, target) in [
            (19574, run.actor),
            (34026, run.targets[0]),
            (217200, run.targets[0]),
        ] {
            let before = owned_cry(&run);
            let ready = crate::auras::has_aura(run.app.world(), run.actor, 471878);
            if let Err(error) = crate::spell_cast::creature_cast(
                run.app.world_mut(),
                run.actor,
                spell,
                Some(target),
                false,
            ) {
                *refusals.entry(format!("{spell}:{error:?}")).or_default() += 1;
                continue;
            }
            if spell == 34026
                && let Some(after) = owned_cry(&run)
            {
                let extra = after.duration_ms.saturating_sub(base);
                maximum_extra = maximum_extra.max(extra);
                assert!(
                    extra <= cap,
                    "actual Cry extension must respect native cap: extra={extra}, cap={cap}"
                );
                if at_cap
                    && !ready
                    && let Some(before) = before.as_ref()
                {
                    assert_eq!(
                        after.expires_at, before.expires_at,
                        "KC cannot extend a capped Cry"
                    );
                    cap_guard_seen = true;
                }
                if capped_cry_expired && before.is_none() {
                    assert_eq!(
                        extra, extension,
                        "new Cry must recover its native extension budget after actual expiry"
                    );
                    reset_seen = true;
                }
                at_cap |= extra == cap;
            }
            break;
        }
        // Use the same fixed-time target clock as the actual300s driver.
        let boundary = run.app.world().resource::<Time>().elapsed() + Duration::from_millis(100);
        let tick = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            advance_fixture_step(&mut run, boundary).unwrap();
        }));
        if let Err(panic) = tick {
            let pets: Vec<_> = run
                .app
                .world_mut()
                .query::<(Entity, &crate::hunter_pet::HunterPet)>()
                .iter(run.app.world())
                .map(|(pet, data)| (pet, data.owner))
                .collect();
            for (pet, owner) in pets {
                eprintln!(
                    "FURY_REGEN at={boundary:?} pet={pet:?} owner={owner:?} actor={:?} owner_exists={} stats={} powers={:?}",
                    run.actor,
                    run.app.world().get_entity(owner).is_ok(),
                    run.app
                        .world()
                        .get::<crate::stats::CharacterStats>(owner)
                        .is_some(),
                    run.app
                        .world()
                        .get::<crate::spell_cast::powers::PowerPools>(owner)
                );
            }
            std::panic::resume_unwind(panic);
        }
    }
    assert!(
        cap_guard_seen && reset_seen,
        "native workload must reach cap, attempt another KC, expire and earn a new Cry: maximum_extra={maximum_extra}, cap={cap}, cap_guard={cap_guard_seen}, reset={reset_seen}, refusals={refusals:?}"
    );
}
