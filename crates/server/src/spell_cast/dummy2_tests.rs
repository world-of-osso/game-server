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

#[test]
fn dummy2_monk_blackout_kick_reduces_rising_sun_kick_recharge_with_rank_two() {
    let mut h =
        harness(Vec::new(), &[100784, 261917, 107428, 185099], 10).expect("real world.db required");
    h.set_power(PowerType::Chi, 6.0);
    h.cast_and_deliver(107428, Some(h.npc));
    h.advance(35);
    let before = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&1518]
        .next_ready_at;
    h.inbox.charges.clear();
    h.set_power(PowerType::Chi, 6.0);
    h.cast_and_deliver(100784, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .charges
            .iter()
            .any(|update| update.spell_id == 107428)
    );
    let after = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&1518]
        .next_ready_at;
    assert_eq!(after, before - 1.0);
}

#[test]
fn dummy2_monk_blackout_kick_reduces_fists_of_fury_cooldown_with_rank_two() {
    let mut h = harness(Vec::new(), &[100784, 261917, 113656, 117418, 120086], 10)
        .expect("real world.db required");
    h.set_power(PowerType::Chi, 6.0);
    h.cast_and_deliver(113656, Some(h.npc));
    h.advance(90); // Fists channel finished; its 24-second cooldown is running.
    let before = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .spell_ready_at[&113656];
    h.inbox.cooldowns.clear();
    h.set_power(PowerType::Chi, 6.0);
    h.cast_and_deliver(100784, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|update| update.spell_id == 113656)
    );
    let after = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .spell_ready_at[&113656];
    assert_eq!(after, before - 1.0);
}

#[test]
fn dummy2_monk_blackout_kick_without_rank_two_does_not_reduce_recharge() {
    let mut h = harness(Vec::new(), &[100784, 107428, 185099], 10).expect("real world.db required");
    h.set_power(PowerType::Chi, 6.0);
    h.cast_and_deliver(107428, Some(h.npc));
    h.advance(35);
    let before = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&1518]
        .next_ready_at;
    h.set_power(PowerType::Chi, 6.0);
    h.cast_and_deliver(100784, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let after = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&1518]
        .next_ready_at;
    assert_eq!(after, before);
}

#[test]
fn dummy2_rogue_backstab_deals_twenty_percent_more_from_behind() {
    let mut h = harness(Vec::new(), &[53], 4).expect("real world.db required");
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(53, Some(h.npc)); // NPC faces +X; caster is at X=0.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(107.31 * 1.2).contains(&(5000.0 - h.health(h.npc))));
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
}

#[test]
fn dummy2_rogue_backstab_from_front_keeps_base_damage() {
    let mut h = harness(Vec::new(), &[53], 4).expect("real world.db required");
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.caster)
        .unwrap()
        .x = 5.0;
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(53, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(107.31).contains(&(5000.0 - h.health(h.npc))));
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
}
