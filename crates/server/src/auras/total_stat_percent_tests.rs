//! Native137 composition; TC a352b1fa SpellAuraEffects.cpp3894-3933.
//! TDB1210.26091 + all32 pinned world updates establish no SAME_EFFECT group
//! for51271/53365. Native70448 effects43590/46012 select Strength20%/15%.
use crate::auras::{RemoveReason, has_aura, remove_auras};
use crate::spell_cast::test_harness::{Harness, harness_with, spawn_peer_connection, spawn_player};
use crate::stats::CharacterStats;
use bevy::prelude::Entity;
use shared::components::{CombatRatings, DerivedStats, UnitStats};
use shared::protocol::CombatLogKind;

const DEATH_KNIGHT: u8 = 6;
const PILLAR: u32 = 51_271;
const UNHOLY_STRENGTH: u32 = 53_365;
const FORTITUDE: u32 = 21_562;
const PILLAR_FACTOR: f32 = 1.20;
const UNHOLY_FACTOR: f32 = 1.15;
const INDEPENDENT_FACTOR: f32 = PILLAR_FACTOR * UNHOLY_FACTOR;

fn fixture() -> Harness {
    let mut h = harness_with(
        Vec::new(),
        &[PILLAR, UNHOLY_STRENGTH, FORTITUDE],
        DEATH_KNIGHT,
        true,
    )
    .expect("actual class/race/level stats and native spell catalog");
    h.advance(3);
    let baseline = stats(&h, h.caster);
    assert!(baseline.primary.strength > 0.0);
    // Native ChrClasses6: APperStrength1, APperAgility0; no fabricated AP input.
    assert_eq!(baseline.attack_power, baseline.primary.strength);
    h
}

fn stats(h: &Harness, unit: Entity) -> CharacterStats {
    *h.app.world().get::<CharacterStats>(unit).unwrap()
}

fn cast_self(h: &mut Harness, spell: u32) {
    h.cast_and_deliver(spell, None);
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox.log.iter().any(|event| {
            event.spell_id == Some(spell)
                && event.source == Some(h.caster.to_bits())
                && event.kind == CombatLogKind::CastSuccess
        }),
        "ordinary{spell}: {:?}",
        h.inbox.log
    );
    assert!(has_aura(h.app.world(), h.caster, spell));
}

fn assert_strength_factor(h: &Harness, baseline: CharacterStats, factor: f32) {
    let after = stats(h, h.caster);
    assert_eq!(after.primary.strength, baseline.primary.strength * factor);
    assert_eq!(after.attack_power, baseline.attack_power * factor);
    assert_eq!(after.primary.agility, baseline.primary.agility);
    assert_eq!(after.primary.intellect, baseline.primary.intellect);
}

#[test]
fn total_stat137_native_single_strength_effects_keep_their_authored_factors() {
    for (spell, factor) in [(PILLAR, PILLAR_FACTOR), (UNHOLY_STRENGTH, UNHOLY_FACTOR)] {
        let mut h = fixture();
        let baseline = stats(&h, h.caster);
        cast_self(&mut h, spell);
        assert_strength_factor(&h, baseline, factor);
        assert_eq!(
            stats(&h, h.caster).primary.stamina,
            baseline.primary.stamina
        );
    }
}

#[test]
fn total_stat137_native_pillar_then_unholy_strength_multiplies_instead_of_adding() {
    let mut h = fixture();
    let baseline = stats(&h, h.caster);
    cast_self(&mut h, PILLAR);
    cast_self(&mut h, UNHOLY_STRENGTH);
    assert_strength_factor(&h, baseline, INDEPENDENT_FACTOR);
}

#[test]
fn total_stat137_native_unholy_strength_then_pillar_has_the_same_product() {
    let mut h = fixture();
    let baseline = stats(&h, h.caster);
    cast_self(&mut h, UNHOLY_STRENGTH);
    cast_self(&mut h, PILLAR);
    assert_strength_factor(&h, baseline, INDEPENDENT_FACTOR);
}

#[test]
fn total_stat137_native_removal_restores_the_remaining_strength_factor() {
    let mut h = fixture();
    let baseline = stats(&h, h.caster);
    cast_self(&mut h, PILLAR);
    cast_self(&mut h, UNHOLY_STRENGTH);
    assert_strength_factor(&h, baseline, INDEPENDENT_FACTOR);
    let caster = h.caster;
    assert_eq!(
        remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
            aura.spell_id == UNHOLY_STRENGTH
        })
        .len(),
        1
    );
    h.advance(2);
    assert_strength_factor(&h, baseline, PILLAR_FACTOR);
    assert_eq!(
        remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
            aura.spell_id == PILLAR
        })
        .len(),
        1
    );
    h.advance(2);
    assert_strength_factor(&h, baseline, 1.0);
}

#[test]
fn total_stat137_native_stamina_mask_does_not_modify_strength_composition() {
    let mut h = fixture();
    let baseline = stats(&h, h.caster);
    cast_self(&mut h, PILLAR);
    cast_self(&mut h, FORTITUDE);
    assert_strength_factor(&h, baseline, PILLAR_FACTOR);
    assert_eq!(
        stats(&h, h.caster).primary.stamina,
        baseline.primary.stamina * 1.05
    );
}

#[test]
fn total_stat137_native_foreign_unholy_strength_does_not_modify_owner() {
    let mut h = fixture();
    let baseline = stats(&h, h.caster);
    cast_self(&mut h, PILLAR);
    let foreign = spawn_player(&mut h.app, DEATH_KNIGHT, 1.0);
    // Populate the normal stat-query components; StatsPlugin derives actual
    // native race/class/level values before any assertion or foreign cast.
    h.app.world_mut().entity_mut(foreign).insert((
        UnitStats::default(),
        CombatRatings::default(),
        DerivedStats::default(),
        crate::class_progression::KnownSpells::fixture(vec![UNHOLY_STRENGTH]),
    ));
    let connection = spawn_peer_connection(&mut h.app, foreign, 2);
    h.advance(3);
    assert_eq!(
        stats(&h, foreign).primary.strength,
        baseline.primary.strength
    );
    let owner = h.caster;
    let owner_connection = h.connection;
    h.caster = foreign;
    h.connection = connection;
    cast_self(&mut h, UNHOLY_STRENGTH);
    assert_strength_factor(&h, baseline, UNHOLY_FACTOR);
    h.caster = owner;
    h.connection = owner_connection;
    assert_strength_factor(&h, baseline, PILLAR_FACTOR);
    assert!(!has_aura(h.app.world(), owner, UNHOLY_STRENGTH));
}
