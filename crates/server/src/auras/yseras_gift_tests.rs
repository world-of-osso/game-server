//! TC a352b1fa spell_druid.cpp:3072-3097:145108 ticks3% of owner max
//! health every5s, self145109 when injured, else group145110. Both native
//! children carry SPELL_ATTR2_CANT_CRIT; their effect0 is overridden per tick.
use shared::components::Health;
use shared::protocol::CombatLogKind;

use super::*;
use crate::class_progression::KnownSpells;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::persistence::ClientId;
use crate::spell_cast::test_harness::{Harness, harness, spawn_player};
use crate::stats::CharacterStats;

const YSERAS_GIFT: u32 = 145_108;
const HEAL_SELF: u32 = 145_109;
const HEAL_GROUP: u32 = 145_110;
const DRUID: u8 = 11;
const TICK_FRAMES: usize = 5000 / 50 + 2;
const OWNER_MAX_HEALTH: f32 = 2000.0;
const NATIVE_PERCENT_HEAL: f32 = 60.0;

fn fixture() -> Harness {
    let mut h = harness(Vec::new(), &[YSERAS_GIFT, HEAL_SELF, HEAL_GROUP], DRUID).unwrap();
    let unit = h.caster;
    *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
        current: 500.0,
        max: OWNER_MAX_HEALTH,
    };
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: YSERAS_GIFT,
            effect_mask: u32::MAX,
            source: AuraSource::KnownSpell,
        },
    );
    h
}

fn add_group_member(h: &mut Harness) -> Entity {
    let unit = h.caster;
    h.app.world_mut().entity_mut(unit).insert(ClientId(1));
    let member = spawn_player(&mut h.app, DRUID, 3.0);
    h.app.world_mut().entity_mut(member).insert(ClientId(2));
    h.app.world_mut().get_mut::<Health>(member).unwrap().current = 400.0;
    let online: OnlineView = [(1, unit), (2, member)]
        .into_iter()
        .map(|(id, entity)| {
            (
                id,
                OnlineMember {
                    entity: entity.to_bits(),
                    name: format!("Gift{id}"),
                    class: DRUID,
                    level: 80,
                    portrait: default(),
                    dungeon_difficulty: 1,
                    raid_difficulty: default(),
                },
            )
        })
        .collect();
    let mut groups = GroupRegistry::default();
    groups.invite(1, "Gift2", &online);
    groups.respond_invite(2, true);
    assert!(groups.is_in_same_group(1, 2));
    h.app.insert_resource(groups);
    member
}

#[test]
fn class_completeness_native_yseras_gift_self_tick_scales_owner_health_and_stops_after_unlearning()
{
    let mut h = fixture();
    h.advance(TICK_FRAMES - 10);
    assert_eq!(h.health(h.caster), 500.0, "no extra-initial period");
    h.advance(10);
    assert_eq!(h.health(h.caster), 500.0 + NATIVE_PERCENT_HEAL);
    let unit = h.caster;
    h.app.world_mut().get_mut::<Health>(unit).unwrap().max = OWNER_MAX_HEALTH * 2.0;
    h.advance(TICK_FRAMES);
    assert_eq!(
        h.health(unit),
        500.0 + NATIVE_PERCENT_HEAL * 3.0,
        "reads live owner max"
    );
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(KnownSpells::fixture(vec![HEAL_SELF, HEAL_GROUP]));
    h.advance(TICK_FRAMES);
    assert!(!has_aura(h.app.world(), unit, YSERAS_GIFT));
    assert_eq!(h.health(unit), 500.0 + NATIVE_PERCENT_HEAL * 3.0);
}

#[test]
fn class_completeness_native_yseras_gift_switches_to_group_heal_at_full_owner_health() {
    let mut h = fixture();
    let member = add_group_member(&mut h);
    let unit = h.caster;
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(unit)
        .unwrap()
        .crit_pct = 100.0;
    h.advance(TICK_FRAMES);
    assert_eq!(h.health(h.caster), 500.0 + NATIVE_PERCENT_HEAL);
    assert_eq!(h.health(member), 400.0, "injured owner takes priority");
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current = OWNER_MAX_HEALTH;
    h.advance(TICK_FRAMES);
    assert_eq!(h.health(unit), OWNER_MAX_HEALTH);
    assert_eq!(
        h.health(member),
        400.0 + NATIVE_PERCENT_HEAL,
        "owner max, not member max"
    );
}

#[test]
fn class_completeness_native_yseras_gift_self_heal_logs_overflow_without_exceeding_max() {
    let mut h = fixture();
    let unit = h.caster;
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current = OWNER_MAX_HEALTH - 10.0;
    h.advance(TICK_FRAMES);
    assert_eq!(h.health(unit), OWNER_MAX_HEALTH);
    let heals: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| event.spell_id == Some(HEAL_SELF) && event.kind == CombatLogKind::Heal)
        .collect();
    assert_eq!(heals.len(), 1);
    assert_eq!(heals[0].amount, NATIVE_PERCENT_HEAL as i32);
    assert_eq!(heals[0].overflow, 50);
}

#[test]
fn class_completeness_native_yseras_heal_respects_authored_cant_crit_with_100_percent_crit() {
    let mut h = harness(Vec::new(), &[HEAL_SELF], DRUID).unwrap();
    let unit = h.caster;
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 500.0;
    h.app
        .world_mut()
        .get_mut::<CharacterStats>(unit)
        .unwrap()
        .crit_pct = 100.0;
    h.cast_and_deliver(HEAL_SELF, Some(unit));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.health(unit), 515.0);
    let heal = h
        .inbox
        .log
        .iter()
        .find(|event| event.spell_id == Some(HEAL_SELF) && event.kind == CombatLogKind::Heal)
        .unwrap();
    assert!(!heal.crit);
}

#[test]
fn class_completeness_native_yseras_gift_full_ungrouped_owner_does_not_heal_outsider() {
    let mut h = fixture();
    let unit = h.caster;
    h.app.world_mut().entity_mut(unit).insert(ClientId(1));
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current = OWNER_MAX_HEALTH;
    let outsider = spawn_player(&mut h.app, DRUID, 3.0);
    h.app.world_mut().entity_mut(outsider).insert(ClientId(2));
    h.app
        .world_mut()
        .get_mut::<Health>(outsider)
        .unwrap()
        .current = 400.0;
    h.advance(TICK_FRAMES);
    assert_eq!(h.health(unit), OWNER_MAX_HEALTH);
    assert_eq!(h.health(outsider), 400.0);
    assert!(!h.inbox.log.iter().any(|event| {
        matches!(event.spell_id, Some(HEAL_SELF | HEAL_GROUP)) && event.kind == CombatLogKind::Heal
    }));
}

#[test]
fn class_completeness_native_yseras_gift_override_does_not_change_later_direct_child_casts() {
    let mut h = fixture();
    h.advance(TICK_FRAMES);
    assert_eq!(h.health(h.caster), 500.0 + NATIVE_PERCENT_HEAL);
    let unit = h.caster;
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 500.0;
    h.cast_and_deliver(HEAL_SELF, Some(unit));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.health(unit),
        515.0,
        "per-cast percent override never mutates native15 child definition"
    );
}
