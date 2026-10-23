//! Minimal alternate healer decisions must use real source actors and native casts.
use super::*;
use crate::spell_cast::test_harness::Harness;
use shared::components::Health;
use shared::protocol::CombatLogKind;

fn spawn_native_alternate(spec: u32, entry: u32) -> (Harness, Entity) {
    let (mut h, unit) = kit_tests::at_level(entry, 32, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    let profile = simulator::healer_profiles::configure(h.app.world(), spec).unwrap();
    assert_eq!(profile.entry, entry);
    assert_ne!(
        profile.heal_spell, 0,
        "spec{spec} requires its policy-owned native primary heal/GCD reference"
    );
    let owner = h.caster;
    spawning::initialize(h.app.world_mut(), owner, unit, profile.clone()).unwrap();
    assert!(
        h.app
            .world()
            .get::<crate::class_progression::KnownSpells>(unit)
            .is_some_and(|known| known.contains(profile.heal_spell)),
        "primary heal must belong to the published native book at level32"
    );
    let (_, gcd) = crate::spell_cast::healer_idle_gcd(h.app.world(), unit).unwrap();
    assert!(gcd > 0.0, "a child with no native GCD is not a reference");
    (h, unit)
}

fn wound_native_healer(h: &mut Harness, unit: Entity) {
    let maximum = h.app.world().get::<Health>(unit).unwrap().max;
    let source = crate::level_scaling::UnitScaling::of(h.app.world(), h.npc);
    let target = crate::level_scaling::UnitScaling::of(h.app.world(), unit);
    let health_per_damage = crate::level_scaling::health_loss(&source, &target, 1.0);
    assert!(health_per_damage > 0.0);
    let enemy = h.npc;
    crate::combat::deal_direct_damage(
        h.app.world_mut(),
        enemy,
        unit,
        maximum * 0.4 / health_per_damage,
    );
    h.advance(60);
    assert!(h.health(unit) > 0.0 && h.health(unit) < maximum);
}

fn assert_native_healer_recovers(spec: u32, entry: u32) {
    let _rng = crate::random::seed_scope(u64::from(spec));
    let (mut h, unit) = spawn_native_alternate(spec, entry);
    wound_native_healer(&mut h, unit);
    let follower = h.app.world().get::<Follower>(unit).unwrap().clone();
    let owner = h.caster;
    let primary = follower.profile.heal_spell;
    let first_go = h.inbox.go.len();
    let first_log = h.inbox.log.len();
    for _ in 0..160 {
        kit_ai::update(h.app.world_mut(), unit, &follower, &[unit, owner], None);
        h.advance(1);
        let primary_cast = h.inbox.go[first_go..]
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == primary);
        let useful_heal = h.inbox.log[first_log..].iter().any(|event| {
            event.source == Some(unit.to_bits())
                && event.target == Some(unit.to_bits())
                && event.kind == CombatLogKind::Heal
                && event.amount > event.overflow
        });
        if primary_cast && useful_heal {
            return;
        }
    }
    panic!("spec{spec} must launch its native primary and actually heal its injured patient");
}

#[test]
fn native_healer_kit_65_holy_paladin_recovers_patient() {
    assert_native_healer_recovers(65, 209057);
}

#[test]
fn native_healer_kit_256_discipline_recovers_patient() {
    assert_native_healer_recovers(256, 215011);
}

#[test]
fn native_healer_kit_264_restoration_shaman_recovers_patient() {
    assert_native_healer_recovers(264, 209053);
}

#[test]
fn native_healer_kit_270_mistweaver_recovers_patient() {
    assert_native_healer_recovers(270, 227734);
}

#[test]
fn native_healer_kit_1468_preservation_recovers_patient() {
    assert_native_healer_recovers(1468, 215240);
}
