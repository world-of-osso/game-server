//! Native AllowDeadTarget admission is independent of aura persistence while dead.
use shared::components::Mana;
use shared::protocol::CombatLogKind;

use super::test_harness::*;
use crate::auras::has_aura;

const DEATH_TOUCH: u32 = 5;
const FREEZE: u32 = 9454;
const FLAME_SHOCK: u32 = 188389;
const NATURAL_FIXTURE_MANA: f32 = 10_000.0;

fn native_fixture() -> Harness {
    let data = world_db().unwrap();
    let spells = [DEATH_TOUCH, FREEZE, FLAME_SHOCK]
        .into_iter()
        .map(|id| data.store.get(id).expect("native recipient spells").clone())
        .collect();
    harness(spells, &[], 7).unwrap()
}

fn kill_target(h: &mut Harness) {
    super::creature::creature_cast(h.app.world_mut(), h.caster, DEATH_TOUCH, Some(h.npc), false)
        .unwrap();
    assert_eq!(h.health(h.npc), 0.0);
    assert!(h.app.world().get_entity(h.npc).is_ok());
}

#[test]
fn native_normal_creature_freeze_accepts_present_dead_recipient() {
    let mut h = native_fixture();
    kill_target(&mut h);
    super::creature::creature_cast(h.app.world_mut(), h.caster, FREEZE, Some(h.npc), false)
        .unwrap();
    assert!(!has_aura(h.app.world(), h.npc, FREEZE));
    assert_eq!(h.health(h.npc), 0.0);
}

#[test]
fn native_triggered_creature_freeze_accepts_present_dead_recipient() {
    let mut h = native_fixture();
    kill_target(&mut h);
    super::creature::creature_cast(h.app.world_mut(), h.caster, FREEZE, Some(h.npc), true).unwrap();
    assert!(!has_aura(h.app.world(), h.npc, FREEZE));
    assert_eq!(h.health(h.npc), 0.0);
}

#[test]
fn native_player_request_freeze_accepts_present_dead_recipient() {
    let mut h = native_fixture();
    kill_target(&mut h);
    h.cast(FREEZE, Some(h.npc));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::CastSuccess
            && event.spell_id == Some(FREEZE)
            && event.target == Some(h.npc.to_bits())
    }));
    assert!(!has_aura(h.app.world(), h.npc, FREEZE));
    assert_eq!(h.health(h.npc), 0.0);
}

#[test]
fn native_normal_creature_freeze_still_stuns_living_recipient() {
    let mut h = native_fixture();
    super::creature::creature_cast(h.app.world_mut(), h.caster, FREEZE, Some(h.npc), false)
        .unwrap();
    assert!(has_aura(h.app.world(), h.npc, FREEZE));
    assert!(h.health(h.npc) > 0.0);
}

#[test]
fn native_normal_creature_flame_shock_still_refuses_corpse_before_payment() {
    let mut h = native_fixture();
    kill_target(&mut h);
    assert!(
        super::creature::creature_cast(
            h.app.world_mut(),
            h.caster,
            FLAME_SHOCK,
            Some(h.npc),
            false,
        )
        .is_err()
    );
    assert!(!has_aura(h.app.world(), h.npc, FLAME_SHOCK));
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        NATURAL_FIXTURE_MANA,
    );
}
