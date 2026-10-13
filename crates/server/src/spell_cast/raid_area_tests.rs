//! TC a352b1fa SpellInfo.cpp:304 target56 caster-area raid; spell_druid.cpp:
//! 3100-3110 Ysera's Gift145110 selects one injured raid target, preferring players.
use bevy::prelude::*;
use shared::components::Health;
use shared::protocol::CombatLogKind;

use super::test_harness::{Harness, harness, spawn_player};
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::persistence::ClientId;

const YSERAS_GIFT_GROUP_HEAL: u32 = 145_110;
const DRUID: u8 = 11;
const INITIAL_HEALTH: f32 = 500.0;
const NATIVE_HEAL: f32 = 15.0;

fn spawn_ally(h: &mut Harness, id: u64, x: f32) -> Entity {
    let unit = spawn_player(&mut h.app, DRUID, x);
    h.app.world_mut().entity_mut(unit).insert(ClientId(id));
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current = INITIAL_HEALTH;
    unit
}

fn group(h: &mut Harness, members: &[(u64, Entity)]) {
    let online: OnlineView = members
        .iter()
        .map(|&(id, unit)| {
            (
                id,
                OnlineMember {
                    entity: unit.to_bits(),
                    name: format!("Raid{id}"),
                    class: DRUID,
                    level: 80,
                    portrait: default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: default(),
                },
            )
        })
        .collect();
    let mut registry = GroupRegistry::default();
    for &(id, _) in &members[1..] {
        registry.invite(members[0].0, &format!("Raid{id}"), &online);
        registry.respond_invite(id, true);
        assert!(registry.is_in_same_group(members[0].0, id));
    }
    h.app.insert_resource(registry);
}

fn raid_fixture() -> (Harness, Entity, Entity, Entity) {
    let mut h = harness(Vec::new(), &[YSERAS_GIFT_GROUP_HEAL], DRUID).unwrap();
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert(ClientId(1));
    h.app.world_mut().get_mut::<Health>(caster).unwrap().current = INITIAL_HEALTH;
    let first = spawn_ally(&mut h, 2, 2.0);
    let second = spawn_ally(&mut h, 3, 3.0);
    let outsider = spawn_ally(&mut h, 4, 4.0);
    group(&mut h, &[(1, caster), (2, first), (3, second)]);
    (h, first, second, outsider)
}

#[test]
fn class_completeness_native_raid_heal_selects_one_group_member_not_caster_or_outsider() {
    let (mut h, first, second, outsider) = raid_fixture();
    h.cast_and_deliver(YSERAS_GIFT_GROUP_HEAL, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.health(h.caster), INITIAL_HEALTH);
    assert_eq!(h.health(outsider), INITIAL_HEALTH);
    assert_eq!(
        h.health(first) + h.health(second),
        INITIAL_HEALTH * 2.0 + NATIVE_HEAL
    );
    let heals: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(YSERAS_GIFT_GROUP_HEAL) && event.kind == CombatLogKind::Heal
        })
        .collect();
    assert_eq!(heals.len(), 1);
    assert!([Some(first.to_bits()), Some(second.to_bits())].contains(&heals[0].target));
}

#[test]
fn class_completeness_native_raid_heal_prefers_injured_member_to_full_member() {
    let (mut h, first, second, outsider) = raid_fixture();
    h.app.world_mut().get_mut::<Health>(first).unwrap().current = 1000.0;
    h.cast_and_deliver(YSERAS_GIFT_GROUP_HEAL, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.health(first), 1000.0);
    assert_eq!(h.health(second), INITIAL_HEALTH + NATIVE_HEAL);
    assert_eq!(h.health(outsider), INITIAL_HEALTH);
}
