//! Real player/profile regressions for Havoc's absent native damage producers.
use super::*;
use shared::protocol::{CombatLogEvent, CombatLogKind};

fn damage(run: &Run, spell: Option<u32>) -> Vec<CombatLogEvent> {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id == spell
                && event.amount > 0
        })
        .cloned()
        .collect()
}

fn advance(run: &mut Run, frames: usize) {
    for _ in 0..frames {
        run.app.update();
    }
}

fn cast(run: &mut Run, spell: u32, self_target: bool) {
    let actor = run.actor;
    let primary = run.targets[0];
    let target = if self_target { actor } else { primary };
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(spell, target, Some(primary)),
    )
    .expect("ordinary native Havoc cast");
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_first_blood_splits_native_primary_and_secondary_strikes() {
    let _seed = crate::random::seed_scope(70_448);
    for (parent, primary_strikes, secondary_strikes) in [
        (188499, [391374, 391378], [199552, 200685]),
        (210152, [393055, 393054], [210153, 210155]),
    ] {
        let mut run = prepare_literal(577, 3).unwrap();
        havoc_tests::earn_havoc_fury(&mut run, 120.0);
        if parent == 210152 {
            cast(&mut run, 191427, false);
            preparation::finish_pending_setup(&mut run).unwrap();
        }
        cast(&mut run, parent, true);
        advance(&mut run, 50);
        let primary = run.targets[0].to_bits();
        for (spell, count) in [(primary_strikes[0], 3), (primary_strikes[1], 1)] {
            let hits = damage(&run, Some(spell));
            assert_eq!(hits.len(), count, "First Blood child {spell}: {hits:?}");
            assert!(hits.iter().all(|hit| hit.target == Some(primary)));
        }
        for (spell, per_target) in [(secondary_strikes[0], 3), (secondary_strikes[1], 1)] {
            let hits = damage(&run, Some(spell));
            assert!(hits.iter().all(|hit| hit.target != Some(primary)));
            for target in &run.targets[1..] {
                assert_eq!(
                    hits.iter()
                        .filter(|hit| hit.target == Some(target.to_bits()))
                        .count(),
                    per_target
                );
            }
        }
    }
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_eye_beam_ticks_damage_every_native_frontal_enemy() {
    let _seed = crate::random::seed_scope(70_448);
    for count in [1, 3] {
        let mut run = prepare_literal(577, count).unwrap();
        havoc_tests::earn_havoc_fury(&mut run, 30.0);
        cast(&mut run, 198013, false);
        preparation::finish_pending_setup(&mut run).unwrap();
        let ticks = damage(&run, Some(198030));
        for target in &run.targets {
            assert!(
                ticks
                    .iter()
                    .any(|event| event.target == Some(target.to_bits())),
                "native Eye Beam must damage frontal enemy {target:?}; got {ticks:?}"
            );
        }
    }
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_burning_blades_real_weapon_damage_produces_native_periodic_damage() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).unwrap();
    havoc_tests::start_havoc_swings(&mut run);
    for _ in 0..300 {
        run.app.update();
        if !damage(&run, Some(453177)).is_empty() {
            break;
        }
    }
    assert!(
        !damage(&run, None).is_empty(),
        "ordinary native weapon damage must precede the producer"
    );
    let ticks = damage(&run, Some(453177));
    assert!(
        ticks
            .iter()
            .any(|event| event.periodic && event.target == Some(run.targets[0].to_bits())),
        "selected native Burning Blades must deal actual periodic damage: {ticks:?}"
    );
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_demonsurge_native_meta_arms_one_explosion_per_ability() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 3).unwrap();
    havoc_tests::earn_havoc_fury(&mut run, 120.0);
    cast(&mut run, 191427, false);
    preparation::finish_pending_setup(&mut run).unwrap();
    cast(&mut run, 210152, true);
    advance(&mut run, 40);
    let first = damage(&run, Some(452416));
    for target in &run.targets {
        assert!(
            first
                .iter()
                .any(|event| event.target == Some(target.to_bits())),
            "native Death Sweep must produce Demonsurge on {target:?}: {first:?}"
        );
    }
    cast(&mut run, 201427, false);
    advance(&mut run, 40);
    let second = damage(&run, Some(452416)).len();
    assert_eq!(
        second - first.len(),
        3,
        "Annihilation has its own native three-target use"
    );
    cast(&mut run, 201427, false);
    advance(&mut run, 40);
    assert_eq!(
        damage(&run, Some(452416)).len(),
        second,
        "the same Annihilation use cannot explode twice in one Meta entry"
    );
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_initiative_first_native_hit_grants_buff_without_repeated_refresh() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 1).unwrap();
    havoc_tests::start_havoc_swings(&mut run);
    for _ in 0..100 {
        run.app.update();
        if !damage(&run, None).is_empty() {
            break;
        }
    }
    assert!(
        !damage(&run, None).is_empty(),
        "actual native weapon hit required"
    );
    run.app.update();
    let actor = run.actor;
    let expires = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 391215 && aura.caster == actor)
        .and_then(|aura| aura.expires_at)
        .expect("first native enemy hit must grant the real Initiative buff");
    for _ in 0..400 {
        if crate::auras::aura_now(run.app.world()) > expires + 0.1 {
            break;
        }
        run.app.update();
    }
    assert!(
        !crate::auras::has_aura(run.app.world(), actor, 391215),
        "repeated hits on the tracked enemy must not refresh Initiative"
    );
}
