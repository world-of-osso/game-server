//! SimC76f8118 sc_priest_shadow.cpp1758–1871: native Slam children and ordering.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const SLAM: u32 = 1227280;
const DAMAGE: u32 = 1227621;
const VT: u32 = 34914;
const MADNESS: u32 = 335467;

fn owned_amount(run: &Run, target: Entity, spell: u32, effect: u8) -> f32 {
    run.app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == spell)
        .unwrap_or_else(|| panic!("missing native spell{spell} on {target:?}"))
        .effects
        .iter()
        .find(|state| state.index == effect)
        .unwrap()
        .amount
}

fn cast_slam(run: &mut Run) {
    engage_fixture_opponents(run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(SLAM, run.targets[0], None),
    )
    .expect("ordinary native Tentacle Slam");
}

fn damage_targets(run: &Run) -> Vec<Entity> {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .map(|event| Entity::from_bits(event.target.unwrap()))
        .collect()
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_slam_delays_native_damage_and_vt_then_hits_each_nearby_enemy_once() {
    let mut run = prepare_literal(258, 3).unwrap();
    cast_slam(&mut run);
    for _ in 0..6 {
        run.app.update();
    }
    assert!(
        damage_targets(&run).is_empty(),
        "Slam must travel before impact"
    );
    for &target in &run.targets {
        assert!(!crate::auras::has_aura(run.app.world(), target, VT));
    }
    for _ in 0..14 {
        run.app.update();
    }
    let hits = damage_targets(&run);
    assert_eq!(
        hits.len(),
        3,
        "native1227621 must hit all three enemies once"
    );
    for &target in &run.targets {
        assert_eq!(hits.iter().filter(|&&hit| hit == target).count(), 1);
        assert!(owned_amount(&run, target, VT, 0) > 0.0);
    }
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, VT));
    assert!(!damage_targets(&run).contains(&run.actor));
}

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_slam_maddening_tentacles_scales_periodic_leech_without_spending_insanity() {
    let mut run = prepare_literal(258, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    // Native ordinary payload as a control; do not grant resources or alter spell data.
    crate::auras::apply_aura(
        run.app.world_mut(),
        &AuraApplication {
            caster: actor,
            target,
            spell_id: MADNESS,
            effect_mask: 1 << 2,
            source: AuraSource::Cast,
        },
    );
    let full = owned_amount(&run, target, MADNESS, 2);
    assert!(full > 0.0);
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == MADNESS && aura.caster == actor,
    );
    let fraction = owned_amount(&run, actor, 1279353, 0) / 100.0;
    assert!(fraction > 0.0 && fraction < 1.0);
    cast_slam(&mut run);
    for _ in 0..20 {
        run.app.update();
    }
    let actual = owned_amount(&run, target, MADNESS, 2);
    assert!(
        (actual - full * fraction).abs() < full * 0.001,
        "Maddening Tentacles native percent {fraction}: full={full}, actual={actual}"
    );
    let hits = damage_targets(&run);
    assert_eq!(hits, vec![target]);
    assert!(!crate::auras::has_aura(run.app.world(), actor, MADNESS));
}
