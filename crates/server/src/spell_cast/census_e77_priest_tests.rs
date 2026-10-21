//! TC a352b1fa spell_priest.cpp4483-4487: E3 is execute bonus data, not dispatch.
use super::test_harness::{harness, varied, world_db};

const DEATH: u32 = 32379;
const FLASH_HEAL: u32 = 2061;

#[test]
fn census_e77_priest_native_execute_consumes_carrier_without_hiding_unmatched_slots() {
    let mut h = harness(Vec::new(), &[DEATH, FLASH_HEAL, 46968], 5).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    let caster = h.caster;
    let victim = h.npc;
    let threshold = world_db()
        .unwrap()
        .store
        .get(DEATH)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 2)
        .unwrap()
        .base_points;
    assert_eq!(threshold, 20.0);
    let mut saw_execute = false;
    for _ in 0..60 {
        let health = *h
            .app
            .world()
            .get::<shared::components::Health>(victim)
            .unwrap();
        let below = health.current < health.max * threshold / 100.0;
        super::creature_cast(h.app.world_mut(), caster, DEATH, Some(victim), true).unwrap();
        h.advance(3);
        let loss = health.current - h.health(victim);
        assert!(
            varied(if below { 212.5 } else { 85.0 }).contains(&loss),
            "actual native health={health:?}, loss={loss}"
        );
        if below {
            saw_execute = true;
            break;
        }
        // Actual native healing pays for repeated native backlash; no Health/RNG writes.
        super::creature_cast(h.app.world_mut(), caster, FLASH_HEAL, Some(caster), true).unwrap();
        h.advance(3);
    }
    assert!(
        saw_execute,
        "actual damage must cross the native20% threshold"
    );
    assert!(
        !h.app
            .world()
            .resource::<super::effects::UnhandledEffects>()
            .0
            .contains(&(DEATH, 77)),
        "TC E3 is consumed execute data, not missing gameplay"
    );
    super::creature_cast(h.app.world_mut(), caster, 46968, Some(victim), true).unwrap();
    h.advance(3);
    assert!(
        h.app
            .world()
            .resource::<super::effects::UnhandledEffects>()
            .0
            .contains(&(46968, 77)),
        "unmatched Shockwave E1 stays a named gap"
    );
}
