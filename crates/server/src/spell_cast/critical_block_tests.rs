//! Native Protection mastery76857 chance253; actual blocked player swings.
use super::test_harness::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::melee::PlayerDefense;
use crate::stats::CharacterStats;
use bevy::prelude::*;
use shared::components::Rotation;
use shared::protocol::CombatLogKind;

const CRITICAL_BLOCK: u32 = 76857;

fn shield_duel(mastery_points: f32) -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[CRITICAL_BLOCK], 1).unwrap();
    let defender = spawn_player(&mut h.app, 1, 3.0);
    h.app.world_mut().entity_mut(h.caster).insert((
        crate::duel::DuelOpponent(defender),
        crate::class_progression::KnownSpells::fixture(Vec::new()),
    ));
    h.app.world_mut().entity_mut(defender).insert((
        crate::duel::DuelOpponent(h.caster),
        Rotation {
            x: 0.0,
            y: std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    load_native_armor_constants(&mut h);
    h.advance(2);
    equip_shield_and_weapon(&mut h, defender, mastery_points);
    (h, defender)
}

fn load_native_armor_constants(h: &mut Harness) {
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let constants = crate::world_data::load_expected_stats(&conn).armor_constants();
    assert!(
        constants.at(10).unwrap() > 1.0,
        "native level10 armor constant required"
    );
    h.app.insert_resource(constants);
}

fn equip_shield_and_weapon(h: &mut Harness, defender: Entity, mastery_points: f32) {
    let armor_constant = h
        .app
        .world()
        .resource::<crate::level_scaling::ArmorConstants>()
        .at(10)
        .unwrap();
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<CharacterStats>(defender)
        .unwrap();
    stats.mastery_points = mastery_points;
    stats.defense = PlayerDefense {
        block: 100.0,
        shield_block: armor_constant,
        ..Default::default()
    };
    drop(stats);
    let mut stats = h
        .app
        .world_mut()
        .get_mut::<CharacterStats>(h.caster)
        .unwrap();
    stats.attack_power = 0.0;
    stats.crit_pct = 0.0;
    stats.main_hand.min = 333.0;
    stats.main_hand.max = 333.0;
}

fn apply_mastery(h: &mut Harness, unit: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: CRITICAL_BLOCK,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

fn blocked_swing(h: &mut Harness, defender: Entity) -> (i32, i32, bool) {
    h.inbox = Inbox::default();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::combat::AutoAttack::new(defender));
    for _ in 0..120 {
        h.advance(1);
        let hit = h.inbox.log.iter().find(|event| {
            event.source == Some(h.caster.to_bits())
                && event.target == Some(defender.to_bits())
                && event.spell_id.is_none()
                && event.kind == CombatLogKind::Damage
        });
        if let Some(hit) = hit {
            let result = (hit.amount, hit.blocked, hit.crit);
            h.app
                .world_mut()
                .entity_mut(h.caster)
                .remove::<crate::combat::AutoAttack>();
            return result;
        }
    }
    panic!("no real blocked swing in six seconds: {:?}", h.inbox.log);
}

#[test]
fn class_completeness_native_critical_block_doubles_already_truncated_autoattack_block() {
    let (mut h, defender) = shield_duel(100.0);
    apply_mastery(&mut h, defender);
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(defender)
            .unwrap()
            .modifier(253),
        150.0
    );
    // 333 ×50%=166.5 truncates to166 BEFORE doubling, TC Unit.cpp1469–1473.
    assert_eq!(blocked_swing(&mut h, defender), (1, 332, false));
}

#[test]
fn class_completeness_native_zero_critical_block_keeps_ordinary_block() {
    let (mut h, defender) = shield_duel(0.0);
    apply_mastery(&mut h, defender);
    assert_eq!(blocked_swing(&mut h, defender), (167, 166, false));
}
