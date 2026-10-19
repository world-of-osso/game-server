//! Player BASE_PCT80, distinct from TOTAL_PCT137: TC a352b1fa
//! SpellAuraEffects.cpp3804-3838 / Unit.cpp9882-9895.
use crate::auras::{RemoveReason, has_aura, remove_auras};
use crate::spell_cast::test_harness::{Harness, harness_with, spawn_peer_connection, spawn_player};
use crate::stats::CharacterStats;
use shared::components::{CombatRatings, DerivedStats, UnitStats};
use shared::protocol::CombatLogKind;

const MAGE: u8 = 8;
const BASE_INTELLECT: u32 = 13_326;
const FLAT_INTELLECT_PASSIVE: u32 = 9_417;
const BASE_FACTOR: f32 = 1.10;
const FLAT_INTELLECT: f32 = 10.0;

fn fixture(with_flat_passive: bool) -> Harness {
    let spells = if with_flat_passive {
        vec![BASE_INTELLECT, FLAT_INTELLECT_PASSIVE]
    } else {
        vec![BASE_INTELLECT]
    };
    let mut h = harness_with(Vec::new(), &spells, MAGE, true)
        .expect("actual native class/race/level stats and spell definitions");
    h.advance(3);
    if with_flat_passive {
        assert!(
            has_aura(h.app.world(), h.caster, FLAT_INTELLECT_PASSIVE),
            "AURA80_NATIVE_9417_KNOWN_PASSIVE_ABSENT: do not substitute a flat stat"
        );
    }
    h
}

fn primary(h: &Harness) -> UnitStats {
    h.app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .primary
}

fn native_base() -> UnitStats {
    let h = fixture(false);
    let stats = primary(&h);
    assert!(stats.intellect > 0.0);
    stats
}

fn cast_base_intellect(h: &mut Harness) {
    h.cast_and_deliver(BASE_INTELLECT, Some(h.caster));
    h.advance(2);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox.log.iter().any(|event| {
            event.spell_id == Some(BASE_INTELLECT)
                && event.source == Some(h.caster.to_bits())
                && event.kind == CombatLogKind::CastSuccess
        }),
        "ordinary13326: {:?}",
        h.inbox.log
    );
    assert!(has_aura(h.app.world(), h.caster, BASE_INTELLECT));
}

fn assert_other_stats_unchanged(actual: UnitStats, before: UnitStats) {
    assert_eq!(actual.strength, before.strength);
    assert_eq!(actual.agility, before.agility);
    assert_eq!(actual.stamina, before.stamina);
}

#[test]
fn base_stat80_native_9417_known_passive_produces_real_flat_intellect() {
    let baseline = native_base();
    let h = fixture(true);
    assert_eq!(primary(&h).intellect, baseline.intellect + FLAT_INTELLECT);
    assert_other_stats_unchanged(primary(&h), baseline);
}

#[test]
fn base_stat80_native_base_only_intellect_uses_its_authored_ten_percent() {
    let mut h = fixture(false);
    let before = primary(&h);
    cast_base_intellect(&mut h);
    assert_eq!(primary(&h).intellect, before.intellect * BASE_FACTOR);
    assert_other_stats_unchanged(primary(&h), before);
}

#[test]
fn base_stat80_native_flat_passive_is_added_after_create_stat_percentage() {
    let baseline = native_base();
    let mut h = fixture(true);
    assert_eq!(primary(&h).intellect, baseline.intellect + FLAT_INTELLECT);
    cast_base_intellect(&mut h);
    assert_eq!(
        primary(&h).intellect,
        baseline.intellect * BASE_FACTOR + FLAT_INTELLECT,
        "not (create + flat)*1.10; no invented gear or stat values"
    );
}

#[test]
fn base_stat80_native_unlearning_flat_passive_and_removing_base_buff_recompute() {
    let baseline = native_base();
    let mut h = fixture(true);
    cast_base_intellect(&mut h);
    assert_eq!(
        primary(&h).intellect,
        baseline.intellect * BASE_FACTOR + FLAT_INTELLECT
    );
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            BASE_INTELLECT,
        ]));
    h.advance(3);
    assert!(!has_aura(h.app.world(), h.caster, FLAT_INTELLECT_PASSIVE));
    assert!(has_aura(h.app.world(), h.caster, BASE_INTELLECT));
    assert_eq!(primary(&h).intellect, baseline.intellect * BASE_FACTOR);
    let caster = h.caster;
    assert_eq!(
        remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
            aura.spell_id == BASE_INTELLECT
        })
        .len(),
        1
    );
    h.advance(2);
    assert_eq!(primary(&h).intellect, baseline.intellect);
}

#[test]
fn base_stat80_native_intellect_index_does_not_modify_other_primary_stats() {
    let mut h = fixture(true);
    let before = primary(&h);
    cast_base_intellect(&mut h);
    assert_other_stats_unchanged(primary(&h), before);
}

#[test]
fn base_stat80_native_foreign_self_cast_changes_only_foreign_create_stats() {
    let baseline = native_base();
    let mut h = fixture(true);
    let foreign = spawn_player(&mut h.app, MAGE, 1.0);
    h.app.world_mut().entity_mut(foreign).insert((
        UnitStats::default(),
        CombatRatings::default(),
        DerivedStats::default(),
        crate::class_progression::KnownSpells::fixture(vec![
            BASE_INTELLECT,
            FLAT_INTELLECT_PASSIVE,
        ]),
    ));
    let connection = spawn_peer_connection(&mut h.app, foreign, 2);
    h.advance(3);
    assert!(has_aura(h.app.world(), foreign, FLAT_INTELLECT_PASSIVE));
    let owner = h.caster;
    let owner_connection = h.connection;
    h.caster = foreign;
    h.connection = connection;
    assert_eq!(primary(&h).intellect, baseline.intellect + FLAT_INTELLECT);
    cast_base_intellect(&mut h);
    assert_eq!(
        primary(&h).intellect,
        baseline.intellect * BASE_FACTOR + FLAT_INTELLECT
    );
    h.caster = owner;
    h.connection = owner_connection;
    assert_eq!(primary(&h).intellect, baseline.intellect + FLAT_INTELLECT);
    assert!(!has_aura(h.app.world(), owner, BASE_INTELLECT));
}
