//! Explicit native controller target context; no count-to-identity inference.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness, spawn_npc, spawn_player};
use shared::components::{Health, Npc};

const SMASH: u32 = 167105;
const REND: u32 = 388539;

fn enemies() -> (Harness, Vec<Entity>, Bindings) {
    let mut h = harness(Vec::new(), &[SMASH, REND, 133, 116, 1230298], 1).unwrap();
    let secondary = spawn_npc(&mut h.app, 2.0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(Some(h.npc)));
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 8000.0,
        max: 10_000.0,
    });
    h.app.world_mut().entity_mut(secondary).insert(Health {
        current: 2500.0,
        max: 10_000.0,
    });
    let targets = vec![secondary, h.npc];
    let bindings = Bindings {
        actions: [("colossus_smash".into(), SMASH)].into(),
        buffs: [("rend_dot".into(), REND)].into(),
        ..Default::default()
    };
    (h, targets, bindings)
}

fn encounter(world: &World) -> Encounter {
    Encounter::Fixture {
        started_at: crate::spell_cast::now(world),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    }
}

fn action(target: Option<Entity>) -> Action {
    Action {
        source_row: None,
        name: "colossus_smash".into(),
        target: target.map(Entity::to_bits),
        options: Default::default(),
    }
}

#[test]
fn native_target_context_current_and_explicit_row_are_distinct() {
    let (mut h, targets, b) = enemies();
    let read = |h: &Harness, row: Entity| {
        let facts = NativeFacts::new(
            h.app.world(),
            h.caster,
            &b,
            encounter(h.app.world()),
            targets.len(),
        )
        .with_targets(Some(targets[0]), &targets);
        (
            facts.value("target.health.pct").unwrap(),
            facts
                .value_for(&action(Some(row)), "target.health.pct")
                .unwrap(),
        )
    };
    // SimC76f8118 player.cpp10879–10884: the actor's own selection in the
    // candidates is its current target, over the controller's.
    assert_eq!(read(&h, targets[0]), (80.0, 25.0));
    // Without one, the controller's current and an explicit row stay distinct.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(None));
    assert_eq!(read(&h, h.npc), (25.0, 80.0));
}

#[test]
fn native_target_context_candidates_keep_native_order_and_range() {
    const MORTAL_STRIKE: u32 = 12294;
    let (mut h, mut targets, mut b) = enemies();
    // Native167105 is caster-area47/16 with self range1, not a direct-target
    // range oracle. Native12294 targets enemy6 with five-yard range2.
    h.app.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .store
            .clone(),
    );
    b.actions.insert("mortal_strike".into(), MORTAL_STRIKE);
    let proposed = Action {
        source_row: None,
        name: "mortal_strike".into(),
        ..action(None)
    };
    let far = spawn_npc(&mut h.app, 100.0);
    targets.insert(1, far);
    let facts = NativeFacts::new(
        h.app.world(),
        h.caster,
        &b,
        encounter(h.app.world()),
        targets.len(),
    )
    .with_targets(Some(targets[0]), &targets);
    assert_eq!(
        facts.target_ids(&proposed).unwrap(),
        vec![targets[0].to_bits(), h.npc.to_bits()]
    );
}

#[test]
fn native_target_context_expired_or_absent_current_never_redirects() {
    let (mut h, targets, b) = enemies();
    let stale = targets[0];
    assert!(h.app.world_mut().despawn(stale));
    // The actor's own selection is outside the candidates, so retarget
    // (player.cpp10879–10884) has nothing to adopt either.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(Some(stale)));
    for current in [Some(stale), None] {
        let facts = NativeFacts::new(h.app.world(), h.caster, &b, encounter(h.app.world()), 1)
            .with_targets(current, &targets[1..]);
        assert!(
            facts.value("target.health.pct").is_err(),
            "explicit context cannot borrow actor selection"
        );
    }
}

#[test]
fn native_target_context_active_dot_counts_only_real_owned_instances() {
    let (mut h, mut targets, b) = enemies();
    let third = spawn_npc(&mut h.app, 3.0);
    let foreign = spawn_player(&mut h.app, 1, 0.0);
    targets.push(third);
    for (index, &target) in targets.iter().enumerate() {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: if index == 2 { foreign } else { h.caster },
                target,
                spell_id: REND,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
        let expected_caster = if index == 2 { foreign } else { h.caster };
        assert!(
            h.app
                .world()
                .get::<crate::auras::ActiveAuras>(target)
                .unwrap()
                .iter()
                .any(|aura| aura.spell_id == REND && aura.caster == expected_caster)
        );
    }
    // SimC76f8118 player.cpp10879–10884: the actor's own selection is current.
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(Some(third)));
    let facts = NativeFacts::new(
        h.app.world(),
        h.caster,
        &b,
        encounter(h.app.world()),
        targets.len(),
    )
    .with_targets(Some(third), &targets);
    assert_eq!(facts.value("active_dot.rend_dot").unwrap(), 2.0);
    assert_eq!(facts.value("active_dots.rend_dot").unwrap(), 2.0);
    assert_eq!(
        facts.value("dot.rend_dot.ticking").unwrap(),
        0.0,
        "current third has only foreign Rend"
    );
    assert!(facts.value("active_dot.unknown").is_err());
}

#[test]
fn class_follower_target_context_movement_event_is_explicitly_absent() {
    let (h, targets, b) = enemies();
    let facts = NativeFacts::new(
        h.app.world(),
        h.caster,
        &b,
        encounter(h.app.world()),
        targets.len(),
    )
    .with_targets(Some(targets[0]), &targets);
    assert_eq!(facts.value("raid_event.movement.exists").unwrap(), 0.0);
    assert_eq!(facts.value("raid_event.movement.remains").unwrap(), 0.0);
    assert_eq!(
        facts.value("raid_event.movement.in").unwrap(),
        i64::MAX as f64 / 1000.0
    );
    assert!(facts.value("raid_event.movement.unknown").is_err());
    let dungeon = NativeFacts::new(
        h.app.world(),
        h.caster,
        &b,
        Encounter::dungeon(0.0),
        targets.len(),
    )
    .with_targets(Some(targets[0]), &targets);
    assert_eq!(
        dungeon.value("raid_event.movement.in").unwrap(),
        i64::MAX as f64 / 1000.0
    );
}

#[test]
fn native_target_context_no_actual_cast_is_not_a_casting_debuff() {
    let (h, targets, b) = enemies();
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(targets[0])
            .is_none()
    );
    let facts = NativeFacts::new(
        h.app.world(),
        h.caster,
        &b,
        encounter(h.app.world()),
        targets.len(),
    )
    .with_targets(Some(targets[0]), &targets);
    assert_eq!(facts.value("target.debuff.casting.react").unwrap(), 0.0);
}

#[test]
fn target_cast_reaction_tracks_cast_lifetime_and_seeded_delay() {
    use shared::casting::CastState;
    fn run(seed: u64, extra_cast: bool) -> (Vec<f32>, u64) {
        let _seed = crate::random::seed_scope(seed);
        let (mut h, targets, bindings) = enemies();
        let target = h.npc;
        let caster = h.caster;
        if extra_cast {
            h.app
                .world_mut()
                .entity_mut(targets[0])
                .insert(CastState::normal(1230298, caster.to_bits(), 3.0, true));
            h.app.world_mut().flush();
        }
        let mut delays = Vec::new();
        for spell in [133, 116] {
            h.app
                .world_mut()
                .entity_mut(target)
                .insert(CastState::normal(spell, caster.to_bits(), 3.0, true));
            h.app.world_mut().flush();
            let delay = h
                .app
                .world()
                .get::<reaction::CastingReactionDelay>(target)
                .unwrap()
                .0;
            assert!((0.350..=1.4).contains(&delay));
            let read = |world: &World| {
                NativeFacts::new(world, caster, &bindings, encounter(world), targets.len())
                    .with_targets(Some(target), &targets)
                    .value("target.debuff.casting.react")
                    .unwrap()
            };
            assert_eq!(read(h.app.world()), 0.0);
            h.app
                .world_mut()
                .get_mut::<CastState>(target)
                .unwrap()
                .elapsed = delay;
            assert_eq!(read(h.app.world()), 1.0);
            h.app.world_mut().entity_mut(target).remove::<CastState>();
            assert_eq!(read(h.app.world()), 0.0);
            delays.push(delay);
        }
        (delays, crate::random::random::<u64>())
    }
    let seed = 7809098381633315392;
    assert_eq!(run(seed, false), run(seed, true));
    assert_ne!(run(seed, false).0, run(seed + 1, false).0);
}

#[test]
fn missing_cast_reaction_snapshot_fails_with_fact_and_target() {
    use shared::casting::CastState;
    let (mut h, targets, bindings) = enemies();
    let target = h.npc;
    let actor = h.caster;
    h.app
        .world_mut()
        .entity_mut(target)
        .insert(CastState::normal(133, actor.to_bits(), 3.0, true));
    h.app.world_mut().flush();
    h.app
        .world_mut()
        .entity_mut(target)
        .remove::<reaction::CastingReactionDelay>();
    let program = crate::simc_apl::Program::parse(&[
        ("default", "pummel,if=target.debuff.casting.react"),
        ("default", "slam"),
    ])
    .unwrap();
    let world = h.app.world();
    let facts = NativeFacts::new(world, actor, &bindings, encounter(world), targets.len())
        .with_targets(Some(target), &targets);
    let error = program
        .select("default", &facts, &mut program.state(), |_| Ok(true))
        .unwrap_err();
    assert_eq!(
        error,
        format!("target.debuff.casting.react: target {target:?} lacks reaction snapshot")
    );
}

#[test]
fn casting_react_selects_interrupt_only_for_interruptible_native_casts() {
    use shared::casting::CastState;
    let (mut h, targets, bindings) = enemies();
    let target = h.npc;
    let actor = h.caster;
    let program = crate::simc_apl::Program::parse(&[
        ("default", "pummel,if=target.debuff.casting.react"),
        ("default", "slam"),
    ])
    .unwrap();
    for (spell, expected) in [(1230298, "slam"), (133, "pummel")] {
        h.app
            .world_mut()
            .entity_mut(target)
            .insert(CastState::normal(spell, actor.to_bits(), 3.0, true));
        h.app.world_mut().flush();
        h.app
            .world_mut()
            .get_mut::<CastState>(target)
            .unwrap()
            .elapsed = 2.0;
        let world = h.app.world();
        let facts = NativeFacts::new(world, actor, &bindings, encounter(world), targets.len())
            .with_targets(Some(target), &targets);
        let selected = program
            .select("default", &facts, &mut program.state(), |_| Ok(true))
            .unwrap()
            .unwrap();
        assert_eq!(selected.name, expected, "native cast {spell}");
        assert_eq!(
            crate::spell_cast::can_interrupt_cast(world, actor, target),
            expected == "pummel"
        );
        h.app.world_mut().entity_mut(target).remove::<CastState>();
    }
}

#[test]
fn native_dungeon_target_is_boss_reads_the_creature_boss_flag() {
    let (mut h, targets, b) = enemies();
    // Kresh (3653), a Wailing Caverns boss with the native dungeon-boss flag.
    let data = crate::test_data::world_data().clone();
    assert_ne!(data.templates[&3653].flags_extra & 0x1000_0000, 0);
    h.app.world_mut().insert_resource(data);
    let boss = spawn_npc(&mut h.app, 4.0);
    h.app.world_mut().entity_mut(boss).insert(Npc {
        template_id: 3653,
        name: "Kresh".into(),
    });
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(None));
    let dungeon = Encounter::dungeon(crate::spell_cast::now(h.app.world()));
    let read = |current: Entity| {
        NativeFacts::new(h.app.world(), h.caster, &b, dungeon, 2)
            .with_targets(Some(current), &targets)
            .value("target.is_boss")
            .unwrap()
    };
    assert_eq!(read(boss), 1.0);
    assert_eq!(read(h.npc), 0.0, "Kobold Vermin is trash");
}
