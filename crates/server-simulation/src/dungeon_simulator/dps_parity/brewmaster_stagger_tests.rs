//! Native Brewmaster incoming damage and Purifying Brew.
use super::*;
use server::test_support::healer_intake::{Hit, HitKind};
use shared::components::Health;

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn stagger_defers_a_real_hit_and_purifying_brew_clears_the_live_pool() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let attacker = run.targets[0];
    let hit = Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 1_000_000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    let before = run.app.world().get::<Health>(actor).unwrap().current;
    crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
    let immediate = before - run.app.world().get::<Health>(actor).unwrap().current;
    let pool = crate::auras::stagger::pool_size(run.app.world(), actor);
    assert!(
        immediate > 0.0 && pool > 0.0,
        "damage must split between immediate and deferred: {immediate}/{pool}"
    );
    assert!(crate::auras::has_aura(run.app.world(), actor, 124255));
    for _ in 0..5 {
        run.app.update();
    }
    let before_purify = crate::auras::stagger::pool_size(run.app.world(), actor);
    let max_health = run.app.world().get::<Health>(actor).unwrap().max;
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 119582, Some(actor), false)
        .unwrap();
    let after = crate::auras::stagger::pool_size(run.app.world(), actor);
    assert!(
        before_purify * 0.6 > max_health * 0.08,
        "fixture must exercise the selected Mantra percentage, not the minimum"
    );
    let cleared = (before_purify * 0.6)
        .max(max_health * 0.08)
        .min(before_purify);
    assert!(
        (before_purify - after - cleared).abs() < 2.0,
        "real Purifying Brew: {before_purify} -> {after}, expected clear {cleared}"
    );
    let health_after = run.app.world().get::<Health>(actor).unwrap().current;
    for _ in 0..6 {
        run.app.update();
    }
    assert!(
        run.app.world().get::<Health>(actor).unwrap().current < health_after,
        "remaining Stagger ticks must pay damage"
    );
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn expected_swing_damage_enters_stagger_before_purify() {
    let _seed = crate::random::seed_scope(70448);
    let mut expected = prepare_literal(268, 1).unwrap();
    let mut full = prepare_literal(268, 1).unwrap();
    engage(&mut expected);
    engage(&mut full);
    let hit = Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 100_000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Dodge".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    let measure = |run: &mut Run, weighted: bool| {
        let actor = run.actor;
        let attacker = run.targets[0];
        let before = run.app.world().get::<Health>(actor).unwrap().current;
        if weighted {
            crate::combat::replay_expected_healer_hit(
                run.app.world_mut(),
                attacker,
                actor,
                &hit,
                0.5,
            )
            .unwrap();
        } else {
            let mut landed = hit.clone();
            if let HitKind::Melee { outcome, .. } = &mut landed.kind {
                *outcome = "Hit".into();
            }
            crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &landed)
                .unwrap();
        }
        let immediate = before - run.app.world().get::<Health>(actor).unwrap().current;
        let pool = crate::auras::stagger::pool_size(run.app.world(), actor);
        assert!(immediate > 0.0 && pool > 0.0);
        (immediate, pool)
    };
    let (weighted_direct, weighted_pool) = measure(&mut expected, true);
    let (full_direct, full_pool) = measure(&mut full, false);
    assert!((weighted_direct - full_direct * 0.5).abs() < 3.0);
    assert!((weighted_pool - full_pool * 0.5).abs() < 3.0);
    for (run, before) in [(&mut expected, weighted_pool), (&mut full, full_pool)] {
        crate::auras::stagger::purify_fraction(run.app.world_mut(), run.actor, 0.6);
        let after = crate::auras::stagger::pool_size(run.app.world(), run.actor);
        assert!((after - before * 0.4).abs() < 3.0);
    }
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn stagger_absorb_records_self_damage_scale() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let attacker = run.targets[0];
    let hit = Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 100_000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
    assert!(crate::auras::stagger::pool_size(run.app.world(), actor) > 0.0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 1241059, Some(actor), false)
        .unwrap();
    run.app
        .insert_resource(server::test_support::healer_intake::Capture {
            members: vec![actor],
            ..Default::default()
        });
    for _ in 0..20 {
        run.app.update();
    }
    let capture = run
        .app
        .world()
        .resource::<server::test_support::healer_intake::Capture>();
    let absorbed = capture
        .consumed_absorbs_by_spell
        .get(&(actor, 1241059))
        .copied()
        .unwrap_or(0.0);
    let usage = capture
        .consumed_absorbs
        .get(&actor)
        .expect("consumed self shield");
    assert!(
        absorbed > 0.0,
        "Stagger ticks must consume Celestial Infusion"
    );
    assert_eq!(usage.unknown_scale, 0.0);
    assert_eq!(usage.non_unit, 0.0);
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn stagger_defers_a_live_creature_auto_attack() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let attacker = run.targets[0];
    run.app
        .world_mut()
        .entity_mut(attacker)
        .insert(crate::combat::AutoAttack::new(actor));
    for _ in 0..100 {
        run.app.update();
        if crate::auras::stagger::pool_size(run.app.world(), actor) > 0.0 {
            break;
        }
    }
    assert!(
        crate::auras::stagger::pool_size(run.app.world(), actor) > 0.0,
        "a real live auto-attack must populate Stagger"
    );
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn stagger_mid_tick_refresh_preserves_deadline_and_small_pool_clears() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let attacker = run.targets[0];
    let hit = Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 1,
        raw: 50_000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: 0,
            critical_multiplier: 1.0,
            weapon_speed: 2.0,
        },
    };
    crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
    let initial = crate::auras::stagger::pool_size(run.app.world(), actor);
    assert!(initial > 0.0);
    for _ in 0..5 {
        run.app.update();
    }
    crate::combat::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit).unwrap();
    let refreshed = crate::auras::stagger::pool_size(run.app.world(), actor);
    assert!(refreshed > initial, "a second hit adds unpaid damage");
    let before_tick = run.app.world().get::<Health>(actor).unwrap().current;
    for _ in 0..6 {
        run.app.update();
    }
    assert!(
        run.app.world().get::<Health>(actor).unwrap().current < before_tick,
        "the first tick keeps its original deadline"
    );
    let pool = crate::auras::stagger::pool_size(run.app.world(), actor);
    let max_health = run.app.world().get::<Health>(actor).unwrap().max;
    assert!(
        pool < max_health * 0.08,
        "fixture reaches the minimum clear"
    );
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 119582, Some(actor), false)
        .unwrap();
    assert_eq!(
        crate::auras::stagger::pool_size(run.app.world(), actor),
        0.0
    );
    assert!(!crate::auras::has_aura(run.app.world(), actor, 124255));
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn stagger_defers_incoming_magic_spell_damage() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    let attacker = run.targets[0];
    let hit = Hit {
        tick: 0,
        source_guid: 1,
        pack: 0,
        target_slot: 0,
        school: 4,
        raw: 100_000.0,
        level_scaling_factor: 1.0,
        kind: HitKind::Spell {
            spell_id: 133,
            depth: 0,
            bonus_source_guid: None,
            effect_index: Some(0),
            periodic: false,
            crit: false,
            resolved_source: true,
            blocked: false,
        },
    };
    crate::spell_cast::effects::replay_healer_hit(run.app.world_mut(), attacker, actor, &hit)
        .unwrap();
    assert!(
        crate::auras::stagger::pool_size(run.app.world(), actor) > 0.0,
        "actual incoming magic must be deferred"
    );
}
