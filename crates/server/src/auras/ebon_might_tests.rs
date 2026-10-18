//! Native70448 Ebon395152/self395296, SimC76f8118 sc_evoker.cpp4731+.
//! No generic540, optional hero/set/PvP or critical-duration certificate.
use shared::components::{CombatRatings, DerivedStats, Level, Position, PowerType, UnitStats};
use shared::protocol::CombatLogKind;

use super::*;
use crate::class_progression::{ActiveSpec, KnownSpells};
use crate::spell_cast::test_harness::{
    Harness, harness_with, spawn_connection, spawn_player, varied, world_db,
};
use crate::stats::CharacterStats;

const EBON: u32 = 395_152;
const SELF_BUFF: u32 = 395_296;
const SANDS_OF_TIME: u32 = 395_153;
const ERUPTION: u32 = 395_160;
const INTELLECT: u32 = 1459;
const MASTERY: u32 = 406_380;
const FIREBALL: u32 = 133;
const IDS: &[u32] = &[
    EBON,
    SELF_BUFF,
    SANDS_OF_TIME,
    ERUPTION,
    INTELLECT,
    MASTERY,
    FIREBALL,
];

fn fixture() -> Harness {
    let mut h = harness_with(Vec::new(), IDS, 13, true).unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), ActiveSpec(1473)));
    h.advance(4);
    h
}

fn player(h: &mut Harness, class: u8, spec: u32, x: f32) -> Entity {
    let unit = spawn_player(&mut h.app, class, x);
    h.app
        .world_mut()
        .entity_mut(unit)
        .remove::<CharacterStats>()
        .insert((
            Level(80),
            ActiveSpec(spec),
            UnitStats::default(),
            CombatRatings::default(),
            DerivedStats::default(),
            KnownSpells::fixture(IDS.to_vec()),
        ));
    unit
}

fn stats(h: &Harness, unit: Entity) -> CharacterStats {
    *h.app.world().get::<CharacterStats>(unit).unwrap()
}

fn owned(h: &Harness, unit: Entity, caster: Entity, spell: u32) -> AuraInstance {
    h.app
        .world()
        .get::<ActiveAuras>(unit)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == spell)
        .unwrap()
        .clone()
}

fn cast_ebon(h: &mut Harness) {
    h.cast_and_deliver(EBON, None);
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == EBON));
}

fn apply_intellect(h: &mut Harness, unit: Entity) {
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: INTELLECT,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    h.advance(2);
}

// A bounded f32 arithmetic allowance, not spell-variance or a damage tolerance.
// Primary sums and subtracting a percent-amplified foreign grant can differ by ULPs.
fn assert_stat(actual: f32, expected: f32) {
    let precision = 4.0 * f32::EPSILON * expected.abs().max(1.0);
    assert!(
        (actual - expected).abs() <= precision,
        "actual{actual} expected{expected}"
    );
}

#[test]
fn augmentation_ebon_native_solo_cast_grants_the_separate_self_buff() {
    let mut h = fixture();
    let owner = h.caster;
    let before = stats(&h, owner).primary;
    cast_ebon(&mut h);
    assert!(has_aura(h.app.world(), owner, SELF_BUFF));
    assert!(
        !has_aura(h.app.world(), owner, EBON),
        "native ExcludeCaster"
    );
    assert_eq!(stats(&h, owner).primary, before);
}

#[test]
fn augmentation_ebon_native_three_specs_split_primary_support_and_exclude_other_roles() {
    let mut h = fixture();
    let owner = h.caster;
    let strength = player(&mut h, 1, 71, 3.0);
    let agility = player(&mut h, 3, 253, 4.0);
    // Native radius30 is500yd, not an inferred25/30yd.
    let intellect = player(&mut h, 8, 63, 450.0);
    let tank = player(&mut h, 1, 73, 2.0);
    let healer = player(&mut h, 2, 65, 2.5);
    let far = player(&mut h, 8, 63, 501.0);
    let other_map = player(&mut h, 8, 63, 5.0);
    h.app
        .world_mut()
        .entity_mut(other_map)
        .insert(crate::maps::InMap::continent(1));
    h.advance(3);
    let bases = [strength, agility, intellect].map(|unit| stats(&h, unit));
    let grant = stats(&h, owner).primary.intellect * 0.08 * (2.0 / 3.0);
    cast_ebon(&mut h);
    assert!(has_aura(h.app.world(), owner, SELF_BUFF));
    assert!(!has_aura(h.app.world(), owner, EBON));
    for unit in [tank, healer, far, other_map, h.npc] {
        assert!(!has_aura(h.app.world(), unit, EBON), "excluded{unit:?}");
    }
    assert_stat(
        stats(&h, strength).primary.strength,
        bases[0].primary.strength + grant,
    );
    assert_stat(
        stats(&h, agility).primary.agility,
        bases[1].primary.agility + grant,
    );
    assert_stat(
        stats(&h, intellect).primary.intellect,
        bases[2].primary.intellect + grant,
    );
    assert!(stats(&h, strength).attack_power > bases[0].attack_power);
    assert!(stats(&h, agility).ranged_attack_power > bases[1].ranged_attack_power);
    assert_stat(
        stats(&h, intellect).spell_power,
        bases[2].spell_power + grant,
    );
    // The500yd support test is complete; place this recipient in ordinary
    // Fireball range and observe its own authenticated damage connection.
    h.app.world_mut().entity_mut(intellect).insert(Position {
        x: 5.0,
        y: 0.0,
        z: 0.0,
    });
    let peer = spawn_connection(&mut h.app, intellect);
    let connection = std::mem::replace(&mut h.connection, peer);
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(80);
    h.connection = connection;
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let damage = h
        .inbox
        .log
        .iter()
        .find(|event| event.spell_id == Some(FIREBALL) && event.kind == CombatLogKind::Damage)
        .unwrap();
    let crit = if damage.crit { 2.0 } else { 1.0 };
    // Native70448 effect91: SP3.8812499 and unchanged0.05 variance.
    assert!(
        varied((bases[2].spell_power + grant) * 3.8812499 * crit).contains(&(damage.amount as f32))
    );
}

#[test]
fn augmentation_ebon_native_damage_role_filter_precedes_thirty_target_cap() {
    let mut h = fixture();
    let owner = h.caster;
    let tanks: Vec<_> = (1..=31).map(|n| player(&mut h, 1, 73, n as f32)).collect();
    let damage: Vec<_> = (40..=70).map(|n| player(&mut h, 8, 63, n as f32)).collect();
    h.advance(3);
    cast_ebon(&mut h);
    assert!(
        tanks
            .iter()
            .all(|&unit| !has_aura(h.app.world(), unit, EBON))
    );
    let selected: Vec<_> = damage
        .iter()
        .copied()
        .filter(|&unit| has_aura(h.app.world(), unit, EBON))
        .collect();
    assert_eq!(selected.len(), 30);
    let expected = stats(&h, owner).primary.intellect * 0.08 * (2.0 / 30.0);
    for unit in selected {
        let aura = owned(&h, unit, owner, EBON);
        let bonus = aura
            .effects
            .iter()
            .find(|effect| effect.index == 1)
            .unwrap()
            .amount;
        assert_stat(bonus, expected);
    }
}

#[test]
fn augmentation_ebon_native_updates_intellect_and_recipient_count_without_refreshing_expiry() {
    let mut h = fixture();
    let owner = h.caster;
    let allies: Vec<_> = (3..=5).map(|n| player(&mut h, 8, 63, n as f32)).collect();
    h.advance(3);
    let baseline = stats(&h, allies[0]).primary.intellect;
    cast_ebon(&mut h);
    let expiry = owned(&h, allies[0], owner, EBON).expires_at.unwrap();
    // Align observation immediately after the authored self tick, without changing it.
    let clock = owned(&h, owner, owner, SELF_BUFF)
        .effects
        .into_iter()
        .find(|effect| effect.index == 6)
        .unwrap()
        .next_tick_at;
    let frames = ((clock - aura_now(h.app.world())) / 0.05).ceil().max(0.0) as usize + 1;
    h.advance(frames);
    let old = stats(&h, allies[0]).primary.intellect;
    let before = stats(&h, owner).primary.intellect;
    apply_intellect(&mut h, owner);
    let changed = stats(&h, owner).primary.intellect;
    assert!(changed > before);
    assert_eq!(
        stats(&h, allies[0]).primary.intellect,
        old,
        "no early source update"
    );
    h.advance(20);
    assert_stat(
        stats(&h, allies[0]).primary.intellect,
        baseline + changed * 0.08 * (2.0 / 3.0),
    );
    assert_eq!(owned(&h, allies[0], owner, EBON).expires_at, Some(expiry));
    remove_auras(h.app.world_mut(), allies[2], RemoveReason::Cancelled, |a| {
        a.spell_id == EBON
    });
    h.advance(22);
    assert_stat(
        stats(&h, allies[0]).primary.intellect,
        baseline + changed * 0.08,
    );
    assert_eq!(owned(&h, allies[0], owner, EBON).expires_at, Some(expiry));
    let remaining = ((expiry - aura_now(h.app.world())) / 0.05).ceil().max(0.0) as usize + 3;
    h.advance(remaining);
    assert!(!has_aura(h.app.world(), owner, SELF_BUFF));
    for &ally in &allies {
        assert!(!has_aura(h.app.world(), ally, EBON));
    }
    assert_eq!(stats(&h, allies[0]).primary.intellect, baseline);
}

#[test]
fn augmentation_ebon_native_foreign_intellect_is_excluded_after_its_stat_multiplier() {
    let mut h = fixture();
    let first = h.caster;
    let second = player(&mut h, 13, 1473, 3.0);
    let mage = player(&mut h, 8, 63, 4.0);
    let peer = spawn_connection(&mut h.app, second);
    h.advance(3);
    apply_intellect(&mut h, second);
    let first_base = stats(&h, first).primary.intellect;
    let second_base = stats(&h, second).primary.intellect;
    let mage_base = stats(&h, mage).primary.intellect;
    cast_ebon(&mut h);
    assert!(stats(&h, second).primary.intellect > second_base);
    let connection = std::mem::replace(&mut h.connection, peer);
    cast_ebon(&mut h);
    h.connection = connection;
    h.advance(42);
    let first_bonus = owned(&h, mage, first, EBON)
        .effects
        .into_iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    let second_bonus = owned(&h, mage, second, EBON)
        .effects
        .into_iter()
        .find(|effect| effect.index == 1)
        .unwrap()
        .amount;
    assert_stat(first_bonus, first_base * 0.08);
    assert_stat(second_bonus, second_base * 0.08);
    assert_stat(
        stats(&h, mage).primary.intellect,
        mage_base + first_bonus + second_bonus,
    );
    // Native owner ticks must not repeatedly amplify each other's borrowed Intellect.
    h.advance(40);
    assert_stat(
        stats(&h, mage).primary.intellect,
        mage_base + first_bonus + second_bonus,
    );
}

#[test]
fn augmentation_ebon_native_self_damage_and_eruption_extend_naturally_created_owned_buffs() {
    let mut h = fixture();
    let owner = h.caster;
    let ally = player(&mut h, 8, 63, 3.0);
    let foreign = player(&mut h, 8, 63, 4.0);
    h.advance(3);
    let maximum = crate::spell_cast::powers::power_value(h.app.world(), owner, PowerType::Essence)
        .unwrap()
        .1;
    h.set_power(PowerType::Essence, maximum);
    cast_ebon(&mut h);
    // Foreign ownership is a control on the existing bounded flat-duration hook.
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: foreign,
            target: foreign,
            spell_id: SELF_BUFF,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    let own_expiry = owned(&h, owner, owner, SELF_BUFF).expires_at;
    let ally_expiry = owned(&h, ally, owner, EBON).expires_at;
    let foreign_expiry = owned(&h, foreign, foreign, SELF_BUFF).expires_at;
    let power = stats(&h, owner).spell_power;
    h.cast_and_deliver(ERUPTION, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        owned(&h, owner, owner, SELF_BUFF).expires_at,
        own_expiry.map(|time| time + 1.0)
    );
    assert_eq!(
        owned(&h, ally, owner, EBON).expires_at,
        ally_expiry.map(|time| time + 1.0)
    );
    assert_eq!(
        owned(&h, foreign, foreign, SELF_BUFF).expires_at,
        foreign_expiry
    );
    let hit = h
        .inbox
        .log
        .iter()
        .find(|event| event.spell_id == Some(ERUPTION) && event.kind == CombatLogKind::Damage)
        .unwrap();
    let crit = if hit.crit { 2.0 } else { 1.0 };
    assert_eq!(hit.amount as f32, (power * 3.22 * 1.20 * crit).round());
}
