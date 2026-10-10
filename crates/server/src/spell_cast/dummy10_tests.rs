//! Batch10: unchanged local Mainline rows, native cast/impact boundaries.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Position, PowerType};
use shared::protocol::CombatLogKind;

fn landed(h: &Harness) {
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}
fn hits(h: &Harness, spell: u32) -> Vec<Entity> {
    h.inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(spell) && e.kind == CombatLogKind::Damage)
        .filter_map(|e| e.target.and_then(Entity::try_from_bits))
        .collect()
}
fn apply(h: &mut Harness, id: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
}
#[test]
fn dummy10_blade_flurry_initial_strike_caps_five_or_eight_with_rank() {
    for rank in [false, true] {
        let mut h = harness(Vec::new(), &[13877, 331850, 272026], 4).unwrap();
        if rank {
            apply(&mut h, 272026);
        }
        let near: Vec<_> = (0..8)
            .map(|i| spawn_npc(&mut h.app, 1.0 + i as f32 * 0.5))
            .collect();
        let far = spawn_npc(&mut h.app, 8.1);
        let ally = spawn_player(&mut h.app, 4, 1.0);
        h.cast_and_deliver(13877, None);
        h.advance(8);
        landed(&h);
        let targets = hits(&h, 331850);
        assert_eq!(targets.len(), if rank { 8 } else { 5 });
        assert!(!targets.contains(&far));
        assert!(!targets.contains(&ally));
        assert!(targets.iter().all(|e| near.contains(e) || *e == h.npc));
    }
}
#[test]
fn dummy10_blade_flurry_generates_combo_points_only_with_deft_maneuvers() {
    for talent in [false, true] {
        let mut h = harness(Vec::new(), &[13877, 331850, 381878], 4).unwrap();
        h.set_power(PowerType::ComboPoints, 0.0);
        spawn_npc(&mut h.app, 2.0);
        spawn_npc(&mut h.app, 3.0);
        if talent {
            apply(&mut h, 381878);
        }
        h.cast_and_deliver(13877, None);
        h.advance(8);
        landed(&h);
        assert_eq!(hits(&h, 331850).len(), 3);
        assert_eq!(
            h.power(PowerType::ComboPoints),
            if talent { 3.0 } else { 0.0 }
        );
    }
}
#[test]
fn dummy10_consecration_stays_at_cast_position_pulses_and_expires() {
    let mut h = harness(Vec::new(), &[26573, 81297], 2).unwrap();
    let far = spawn_npc(&mut h.app, 8.1);
    let ally = spawn_player(&mut h.app, 2, 1.0);
    h.cast_and_deliver(26573, None);
    h.advance(5);
    landed(&h);
    assert!(hits(&h, 81297).is_empty());
    h.app.world_mut().entity_mut(h.caster).insert(Position {
        x: 40.0,
        y: 0.0,
        z: 0.0,
    });
    h.advance(22);
    assert_eq!(hits(&h, 81297), vec![h.npc]);
    assert!(!hits(&h, 81297).contains(&far));
    assert!(!hits(&h, 81297).contains(&ally));
    h.advance(300);
    let count = hits(&h, 81297).len();
    h.advance(40);
    assert_eq!(hits(&h, 81297).len(), count);
}
#[test]
fn dummy10_feral_lunge_launches_named_bite_only_at_hit_enemy() {
    let mut h = harness(Vec::new(), &[196884, 215802], 7).unwrap();
    let other = spawn_npc(&mut h.app, 6.0);
    h.cast_and_deliver(196884, Some(h.npc));
    h.advance(12);
    landed(&h);
    assert_eq!(hits(&h, 215802), vec![h.npc]);
    assert!(!hits(&h, 215802).contains(&other));
}
#[test]
fn dummy10_divine_storm_soft_cap_reduces_damage_only_beyond_five() {
    fn damage(count: usize) -> f32 {
        let mut h = harness(Vec::new(), &[53385, 403460, 407467], 2).unwrap();
        h.set_power(PowerType::HolyPower, 5.0);
        for i in 1..count {
            spawn_npc(&mut h.app, 1.0 + i as f32 * 0.1);
        }
        h.cast_and_deliver(53385, None);
        h.advance(12);
        landed(&h);
        assert_eq!(hits(&h, 53385).len(), count);
        h.inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(53385) && e.kind == CombatLogKind::Damage)
            .map(|e| e.amount as f32)
            .sum::<f32>()
            / count as f32
    }
    let five = damage(5);
    let twenty = damage(20);
    assert!(
        twenty / five > 0.45 && twenty / five < 0.55,
        "{five} {twenty}"
    );
}
#[test]
fn dummy10_voidform_applies_named_damage_buff_for_native_duration() {
    let mut h = harness(Vec::new(), &[228260, 194249], 5).unwrap();
    h.cast_and_deliver(228260, Some(h.npc));
    h.advance(45);
    landed(&h);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 194249));
    let mods = h
        .app
        .world()
        .get::<crate::auras::AuraStatMods>(h.caster)
        .unwrap();
    assert!(mods.damage_done(32) > 1.0);
    h.advance(700);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 194249));
}
#[test]
fn dummy10_arcane_surge_applies_named_buff_after_cast_not_before() {
    let mut h = harness(Vec::new(), &[365350, 365362], 8).unwrap();
    h.cast_and_deliver(365350, Some(h.npc));
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 365362));
    h.advance(70);
    landed(&h);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 365362));
    assert!(
        h.app
            .world()
            .get::<crate::auras::AuraStatMods>(h.caster)
            .unwrap()
            .damage_done(64)
            > 1.0
    );
    h.advance(500);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 365362));
}
#[test]
fn dummy10_protection_judgment_generates_holy_power_only_with_rank() {
    for rank in [false, true] {
        let mut h = harness(Vec::new(), &[275779, 315867, 220637], 2).unwrap();
        h.set_power(PowerType::HolyPower, 0.0);
        if rank {
            apply(&mut h, 315867);
        }
        h.cast_and_deliver(275779, Some(h.npc));
        h.advance(25);
        landed(&h);
        assert_eq!(h.power(PowerType::HolyPower), if rank { 1.0 } else { 0.0 });
    }
}
#[test]
fn dummy10_hand_of_guldan_uses_native_three_shard_meteor_and_target_area() {
    let mut h = harness(Vec::new(), &[105174, 86040], 9).unwrap();
    h.set_power(PowerType::SoulShards, 30.0);
    let near = spawn_npc(&mut h.app, 6.0);
    let far = spawn_npc(&mut h.app, 14.0);
    let ally = spawn_player(&mut h.app, 9, 6.0);
    h.cast_and_deliver(105174, Some(h.npc));
    h.advance(100);
    landed(&h);
    assert_eq!(h.power(PowerType::SoulShards), 0.0);
    let targets = hits(&h, 86040);
    assert!(targets.contains(&h.npc));
    assert!(targets.contains(&near));
    assert!(!targets.contains(&far));
    assert!(!targets.contains(&ally));
    let child = world_db().unwrap().store.get(86040).unwrap();
    let base = super::effects::scaled_amount(h.app.world(), h.caster, child, &child.effects[0]);
    for event in h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(86040) && e.kind == CombatLogKind::Damage)
    {
        assert!(
            varied(base * 3.0).contains(&(event.amount as f32)),
            "{} {base}",
            event.amount
        );
    }
}
#[test]
fn dummy10_lightning_bolt_grants_rank_maelstrom_once_per_successful_cast() {
    for rank in [false, true] {
        let mut h = harness(Vec::new(), &[188196, 343725], 7).unwrap();
        h.set_power(PowerType::Maelstrom, 0.0);
        if rank {
            apply(&mut h, 343725);
        }
        h.cast_and_deliver(188196, Some(h.npc));
        h.advance(70);
        landed(&h);
        assert_eq!(h.power(PowerType::Maelstrom), if rank { 6.0 } else { 0.0 });
    }
}
