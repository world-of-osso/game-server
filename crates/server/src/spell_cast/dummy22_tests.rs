//! Native Mainline15407 channel: six authored ticks, damage/slow and Insanity.
use super::test_harness::*;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::persistence::ClientId;
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::components::{Health, PowerType};
use shared::protocol::CombatLogKind;
use shared::protocol::{CombatChannel, StopSpellCast};

fn rewind(grouped: bool, raid: bool) -> Harness {
    let mut spell = world_db().unwrap().store.get(363534).unwrap().clone();
    for effect in &mut spell.effects {
        effect.variance = 0.0;
    }
    let mut h = harness(vec![spell], &[363534], 13).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(ClientId(1));
    let mut groups = GroupRegistry::default();
    if grouped {
        let ally = spawn_player(&mut h.app, 1, 3.0);
        h.app.world_mut().entity_mut(ally).insert(ClientId(2));
        let online: OnlineView = [(1, h.caster, "Evoker"), (2, ally, "Ally")]
            .into_iter()
            .map(|(id, entity, name)| {
                (
                    id,
                    OnlineMember {
                        entity: entity.to_bits(),
                        name: name.into(),
                        class: 13,
                        level: 10,
                        portrait: default(),
                        dungeon_difficulty: 1,
                        raid_difficulty: Default::default(),
                    },
                )
            })
            .collect();
        groups.sync_presence(&online);
        groups.invite(1, "Ally", &online);
        groups.respond_invite(2, true);
        if raid {
            groups.convert_to_raid(1);
        }
    }
    h.app.world_mut().insert_resource(groups);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 100.0;
    h
}

fn rewind_heals(h: &mut Harness) -> Vec<(i32, i32)> {
    let caster = h.caster;
    h.cast_and_deliver(363534, Some(caster));
    h.advance(100);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 363534));
    h.inbox
        .log
        .iter()
        .filter(|e| {
            e.spell_id == Some(363534)
                && e.kind == CombatLogKind::Heal
                && e.target == Some(caster.to_bits())
        })
        .map(|e| (e.amount, e.overflow))
        .collect()
}

#[test]
fn dummy22_rewind_native_minimum_is_increased_200_percent_when_solo() {
    let mut h = rewind(false, false);
    assert_eq!(rewind_heals(&mut h), vec![(120, 0); 5]);
    assert_eq!(
        h.app.world().get::<Health>(h.caster).unwrap().current,
        700.0
    );
}

#[test]
fn dummy22_rewind_party_is_not_a_raid_for_native_s4_bonus() {
    let mut h = rewind(true, false);
    assert_eq!(rewind_heals(&mut h), vec![(120, 0); 5]);
}

#[test]
fn dummy22_rewind_raid_retains_native_unboosted_minimum() {
    let mut h = rewind(true, true);
    assert_eq!(rewind_heals(&mut h), vec![(40, 0); 5]);
    assert_eq!(
        h.app.world().get::<Health>(h.caster).unwrap().current,
        300.0
    );
}

#[test]
fn dummy22_rewind_bonus_uses_native_heal_cap_and_overheal() {
    let mut h = rewind(false, false);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 950.0;
    assert_eq!(
        rewind_heals(&mut h),
        vec![(120, 70), (120, 120), (120, 120), (120, 120), (120, 120)]
    );
    assert_eq!(
        h.app.world().get::<Health>(h.caster).unwrap().current,
        1000.0
    );
}

#[test]
fn dummy22_rewind_named_bonus_does_not_triple_flash_heal() {
    let mut spell = world_db().unwrap().store.get(2061).unwrap().clone();
    for effect in &mut spell.effects {
        effect.variance = 0.0;
    }
    let mut h = harness(vec![spell], &[2061], 5).unwrap();
    let caster = h.caster;
    h.app.world_mut().get_mut::<Health>(caster).unwrap().current = 100.0;
    h.cast_and_deliver(2061, Some(caster));
    h.advance(100);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let heals: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(2061) && e.kind == CombatLogKind::Heal)
        .map(|e| e.amount)
        .collect();
    assert_eq!(heals, vec![682]);
}

fn mind_flay() -> Harness {
    let mut spell = world_db().unwrap().store.get(15407).unwrap().clone();
    for effect in &mut spell.effects {
        effect.variance = 0.0;
    }
    let mut h = harness(vec![spell], &[15407], 5).unwrap();
    h.set_power(PowerType::Insanity, 0.0);
    h
}

#[test]
fn dummy22_mind_flay_native_channel_pays_six_three_insanity_ticks() {
    let mut h = mind_flay();
    let target = h.npc;
    h.cast_and_deliver(15407, Some(target));
    h.advance(100);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 15407));
    let grants: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(15407) && e.kind == CombatLogKind::Energize)
        .collect();
    assert_eq!(grants.len(), 6, "15407s4 is the authored tick count");
    assert!(grants.iter().all(|e| e.amount == 300 && e.periodic));
    assert_eq!(h.power(PowerType::Insanity), 1800.0);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|e| e.spell_id == Some(15407) && e.kind == CombatLogKind::Damage && e.periodic)
            .count(),
        6
    );
}

#[test]
fn dummy22_mind_flay_has_no_initial_grant_and_first_tick_is_native_750ms() {
    let mut h = mind_flay();
    let target = h.npc;
    h.cast_and_deliver(15407, Some(target));
    h.advance(10);
    assert_eq!(h.power(PowerType::Insanity), 0.0);
    h.advance(6);
    assert_eq!(h.power(PowerType::Insanity), 300.0);
}

#[test]
fn dummy22_mind_flay_cancel_stops_both_target_damage_and_self_resource() {
    let mut h = mind_flay();
    let target = h.npc;
    h.cast_and_deliver(15407, Some(target));
    h.advance(17);
    assert_eq!(h.power(PowerType::Insanity), 300.0);
    h.app
        .world_mut()
        .get_mut::<MessageSender<StopSpellCast>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(StopSpellCast);
    h.advance(3);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 15407));
    assert!(!crate::auras::has_aura(h.app.world(), target, 15407));
    h.inbox.log.clear();
    h.advance(100);
    assert_eq!(h.power(PowerType::Insanity), 300.0);
    assert!(h.inbox.log.iter().all(|e| e.spell_id != Some(15407)));
}

#[test]
fn dummy22_mind_flay_raw_insanity_cap_reports_each_tick_overflow() {
    let mut h = mind_flay();
    h.set_power(PowerType::Insanity, 9900.0);
    let target = h.npc;
    h.cast_and_deliver(15407, Some(target));
    h.advance(100);
    let grants: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|e| e.spell_id == Some(15407) && e.kind == CombatLogKind::Energize)
        .map(|e| (e.amount, e.overflow))
        .collect();
    assert_eq!(
        grants,
        vec![
            (300, 200),
            (300, 300),
            (300, 300),
            (300, 300),
            (300, 300),
            (300, 300)
        ]
    );
    assert_eq!(h.power(PowerType::Insanity), 10000.0);
}

#[test]
fn dummy22_mind_flay_friendly_rejection_pays_no_resource() {
    let mut h = mind_flay();
    let ally = spawn_player(&mut h.app, 5, 3.0);
    h.cast_and_deliver(15407, Some(ally));
    h.advance(100);
    assert!(!h.inbox.failed.is_empty());
    assert!(h.inbox.go.is_empty());
    assert_eq!(h.power(PowerType::Insanity), 0.0);
}
