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
fn havoc_font_of_venomous_rage_channels_native_primary_and_secondary_damage() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(577, 3).unwrap();
    let mut bindings = server::simc_apl_native::Bindings::default();
    let equipment =
        literal_rotation::equipment::Equipment::load(run.app.world(), run.actor, &mut bindings)
            .unwrap();
    let item = equipment
        .uses()
        .map(|(_, item)| item)
        .find(|item| item.on_use.spell_id == 1297908)
        .expect("equipped native Font on-use");
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Item(item, run.targets[0]),
    )
    .unwrap();
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.actor)
            .is_some_and(|cast| cast.cast_type == shared::casting::CastType::Channel)
    );
    preparation::finish_pending_setup(&mut run).unwrap();
    let primary = run.targets[0].to_bits();
    let ticks = damage(&run, Some(1297908));
    assert!(
        !ticks.is_empty(),
        "native Font equip values must produce channel damage"
    );
    assert!(
        ticks
            .iter()
            .all(|tick| tick.periodic && tick.target == Some(primary))
    );
    let splatter = damage(&run, Some(1307222));
    assert!(
        splatter.iter().all(|hit| hit.target != Some(primary)),
        "Venom Splatter must exclude the selected primary"
    );
    for target in &run.targets[1..] {
        assert_eq!(
            splatter
                .iter()
                .filter(|hit| hit.target == Some(target.to_bits()))
                .count(),
            ticks.len(),
            "one native splatter per channel tick and secondary"
        );
    }
}

#[test]
#[ignore = "requires prepared native spec577 player profile"]
fn havoc_eye_beam_ticks_damage_every_native_frontal_enemy() {
    let _seed = crate::random::seed_scope(70_448);
    for count in [1, 3] {
        let mut run = prepare_literal(577, count).unwrap();
        havoc_tests::earn_havoc_fury(&mut run, 30.0);
        run.app
            .world_mut()
            .init_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>();
        cast(&mut run, 198013, false);
        eprintln!(
            "Eye Beam initial: {}",
            report::engagement(run.app.world(), run.actor, &run.targets)
        );
        preparation::finish_pending_setup(&mut run).unwrap();
        let ticks = damage(&run, Some(198030));
        eprintln!(
            "Eye Beam debug: empowered={:?}, casts={:?}, state={}",
            damage(&run, Some(1287949)),
            run.app
                .world()
                .get_resource::<crate::spell_cast::test_harness::ObservedTriggeredCasts>()
                .map(|casts| &casts.0),
            report::engagement(run.app.world(), run.actor, &run.targets)
        );
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
