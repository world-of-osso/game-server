//! Explicit native controller target context; no count-to-identity inference.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness, spawn_npc, spawn_player};
use shared::components::Health;

const SMASH: u32 = 167105;
const REND: u32 = 388539;

fn enemies() -> (Harness, Vec<Entity>, Bindings) {
    let mut h = harness(Vec::new(), &[SMASH, REND], 1).unwrap();
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
        name: "colossus_smash".into(),
        target: target.map(Entity::to_bits),
        options: Default::default(),
    }
}

#[test]
fn native_target_context_current_and_explicit_row_are_distinct() {
    let (h, targets, b) = enemies();
    let facts = NativeFacts::new(
        h.app.world(),
        h.caster,
        &b,
        encounter(h.app.world()),
        targets.len(),
    )
    .with_targets(Some(targets[0]), &targets);
    assert_eq!(facts.value("target.health.pct").unwrap(), 25.0);
    assert_eq!(
        facts
            .value_for(&action(Some(h.npc)), "target.health.pct")
            .unwrap(),
        80.0
    );
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(h.caster)
            .unwrap()
            .0,
        Some(h.npc)
    );
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
    let facts = NativeFacts::new(
        h.app.world(),
        h.caster,
        &b,
        encounter(h.app.world()),
        targets.len(),
    )
    .with_targets(Some(third), &targets);
    assert_eq!(facts.value("active_dot.rend_dot").unwrap(), 2.0);
    assert_eq!(
        facts.value("dot.rend_dot.ticking").unwrap(),
        0.0,
        "current third has only foreign Rend"
    );
    assert!(facts.value("active_dot.unknown").is_err());
}

#[test]
fn native_target_context_movement_event_is_explicitly_absent() {
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
        Encounter::Dungeon { started_at: 0.0 },
        targets.len(),
    )
    .with_targets(Some(targets[0]), &targets);
    assert!(
        dungeon
            .value("raid_event.movement.in")
            .unwrap_err()
            .starts_with("DUNGEON_TTD_UNAVAILABLE:")
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
