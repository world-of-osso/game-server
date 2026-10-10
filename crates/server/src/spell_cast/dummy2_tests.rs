//! Second batch: real catalog casts, not direct script-hook calls.
use super::test_harness::*;
use shared::components::PowerType;

#[test]
fn dummy2_hunter_cobra_shot_damages_and_reduces_kill_command_cooldown() {
    let mut h = harness(Vec::new(), &[193455, 34026], 3).expect("real world.db required");
    h.set_power(PowerType::Focus, 100.0);
    h.cast_and_deliver(34026, Some(h.npc));
    h.advance(35); // Kill Command's GCD has elapsed; recharge is still running.
    let before = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&2067]
        .next_ready_at;
    h.inbox.charges.clear();
    h.cast_and_deliver(193455, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(390.4).contains(&(5000.0 - h.health(h.npc))));
    assert!(
        h.inbox
            .charges
            .iter()
            .any(|update| update.spell_id == 34026),
        "Kill Command recharge update"
    );
    let ready = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&2067]
        .next_ready_at;
    assert_eq!(ready, before - 1.0);
}

#[test]
fn dummy2_hunter_cobra_shot_does_not_create_a_kill_command_cooldown() {
    let mut h = harness(Vec::new(), &[193455, 34026], 3).expect("real world.db required");
    h.set_power(PowerType::Focus, 100.0);
    h.cast_and_deliver(193455, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let kill_command = world_db().unwrap().store.get(34026).unwrap();
    assert!(
        !h.app
            .world()
            .get::<super::SpellHistory>(h.caster)
            .unwrap()
            .on_cooldown(kill_command, super::now(h.app.world()))
    );
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|update| update.spell_id == 34026)
    );
}
