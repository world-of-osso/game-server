//! Native70448 aura345; TC a352b1fa Unit.cpp1697–1703.
use super::*;
use crate::auras::{AuraApplication, AuraSource, RemoveReason, apply_aura, remove_auras};
use crate::spell_cast::test_harness::{Harness, harness, world_db};
use shared::components::UnitLevel;
use shared::protocol::CombatLogKind;

const FIND_WEAKNESS: u32 = 91_021;
const COLOSSUS_SMASH: u32 = 108_126;
const SHATTERING_SMASH: u32 = 165_944;
const FIND_WEAKNESS_RANK: u32 = 316_220;

fn fixture() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[
            FIND_WEAKNESS,
            COLOSSUS_SMASH,
            SHATTERING_SMASH,
            FIND_WEAKNESS_RANK,
            1160,
            450990,
        ],
        1,
    )
    .unwrap();
    let caster = h.caster;
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    h.advance(3);
    remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |_| true);
    h.advance(3);
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::world_data::load_expected_stats(&conn).armor_constants());
    h.app.world_mut().entity_mut(caster).insert(UnitLevel(80));
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert((UnitLevel(10), CreatureArmor(2000.0)));
    h
}

fn apply(h: &mut Harness, spell_id: u32, caster: Entity, target: Entity) {
    let index = if spell_id == SHATTERING_SMASH { 1 } else { 0 };
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(h.app.world(), target, spell_id));
}

fn remove_bypass(h: &mut Harness) {
    let target = h.npc;
    remove_auras(h.app.world_mut(), target, RemoveReason::Cancelled, |aura| {
        aura.effects.iter().any(|effect| effect.aura_type == 345)
    });
}

fn physical_hit(h: &Harness) -> f32 {
    let spell = world_db().unwrap().store.get(12294).unwrap();
    armor_reduced(h.app.world(), h.caster, h.npc, 1_000_000.0, spell)
}

fn assert_effective_armor(h: &mut Harness, expected: f32) {
    let actual = physical_hit(h);
    remove_bypass(h);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(CreatureArmor(expected));
    assert_eq!(
        actual,
        physical_hit(h),
        "source-equivalent armor {expected}"
    );
}

#[test]
#[ignore = "census-tail345 RED before native caster armor bypass consumer"]
fn census_tail_345_native_find_weakness_bypasses_full_armor_fraction_and_removes() {
    let mut h = fixture();
    let before = physical_hit(&h);
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, FIND_WEAKNESS, caster, target);
    assert!(physical_hit(&h) > before);
    assert_effective_armor(&mut h, 1600.0);
    h.app
        .world_mut()
        .entity_mut(target)
        .insert(CreatureArmor(2000.0));
    assert_eq!(physical_hit(&h), before);
}

#[test]
#[ignore = "census-tail345 RED before native caster armor bypass consumer"]
fn census_tail_345_native_sum_not_product_and_hundred_percent_cap() {
    let mut h = fixture();
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, SHATTERING_SMASH, caster, target);
    apply(&mut h, FIND_WEAKNESS_RANK, caster, target);
    assert_effective_armor(&mut h, 400.0);
    h.app
        .world_mut()
        .entity_mut(target)
        .insert(CreatureArmor(2000.0));
    apply(&mut h, COLOSSUS_SMASH, caster, target);
    apply(&mut h, FIND_WEAKNESS, caster, target);
    assert_eq!(physical_hit(&h), 1_000_000.0);
}

#[test]
#[ignore = "census-tail345 RED before native caster armor bypass consumer"]
fn census_tail_345_native_foreign_caster_and_attacker_side_aura_do_not_bypass() {
    let mut h = fixture();
    let before = physical_hit(&h);
    let caster = h.caster;
    let target = h.npc;
    apply(&mut h, FIND_WEAKNESS, target, target);
    apply(&mut h, COLOSSUS_SMASH, caster, caster);
    assert_eq!(physical_hit(&h), before);
    apply(&mut h, FIND_WEAKNESS_RANK, caster, target);
    assert!(physical_hit(&h) > before);
}

#[test]
#[ignore = "census-tail345 RED before native caster armor bypass consumer"]
fn census_tail_345_native_bypass_precedes_ignore_floor_and_capped_penetration() {
    let mut h = fixture();
    let caster = h.caster;
    let target = h.npc;
    h.app
        .world_mut()
        .entity_mut(target)
        .insert(CreatureArmor(2001.0));
    apply(&mut h, 1160, caster, caster);
    apply(&mut h, 450990, caster, caster);
    apply(&mut h, FIND_WEAKNESS, caster, target);
    assert_effective_armor(&mut h, 2001.0 * 0.8);
}

fn next_swing(h: &mut Harness) -> i32 {
    let before = h.inbox.log.len();
    for _ in 0..300 {
        h.advance(1);
        if let Some(event) = h.inbox.log[before..].iter().find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(h.npc.to_bits())
                && event.amount > 0
        }) {
            assert!(!event.crit);
            assert_eq!(event.absorbed, 0);
            return event.amount + event.blocked;
        }
    }
    panic!("ordinary player autoattack produced no positive hit in15s");
}

#[test]
#[ignore = "census-tail345 RED before native caster armor bypass consumer"]
fn census_tail_345_native_autoswing_obeys_recipient_bypass_and_removal() {
    let mut h = fixture();
    let caster = h.caster;
    let target = h.npc;
    h.app.world_mut().entity_mut(target).insert(UnitLevel(80));
    let mut stats = h.app.world_mut().get_mut::<CharacterStats>(caster).unwrap();
    stats.attack_power = 0.0;
    stats.crit_pct = 0.0;
    stats.main_hand.min = 100.0;
    stats.main_hand.max = 100.0;
    drop(stats);
    h.attack(target);
    let before = next_swing(&mut h);
    assert!(
        before > 15 && before < 100,
        "unsaturated native armor: {before}"
    );
    apply(&mut h, COLOSSUS_SMASH, caster, target);
    assert_eq!(next_swing(&mut h), 100);
    remove_bypass(&mut h);
    assert_eq!(next_swing(&mut h), before);
    h.stop_attack();
    h.advance(3);
}
