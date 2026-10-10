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

#[test]
fn dummy2_evoker_pyre_explodes_at_target_and_hits_nearby_enemies_only() {
    let mut h = harness(Vec::new(), &[357211, 393568, 357212], 13).expect("real world.db required");
    let nearby = spawn_npc(&mut h.app, 5.0);
    let distant = spawn_npc(&mut h.app, 30.0);
    let ally = spawn_player(&mut h.app, 1, 4.0);
    h.set_power(PowerType::Essence, 6.0);
    h.cast_and_deliver(357211, Some(h.npc));
    h.advance(25);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(440.0).contains(&(5000.0 - h.health(h.npc))));
    assert!(varied(440.0).contains(&(5000.0 - h.health(nearby))));
    assert_eq!(h.health(distant), 5000.0);
    assert_eq!(h.health(ally), 1000.0);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(
                |event| event.kind == shared::protocol::CombatLogKind::Damage
                    && event.spell_id == Some(357212)
            )
            .count(),
        2
    );
}

#[test]
fn dummy2_warlock_drain_soul_doubles_ticks_below_twenty_percent() {
    for (health, damage) in [(5000.0, 111.375), (1000.0, 111.375), (800.0, 222.75)] {
        let mut h = harness(Vec::new(), &[198590, 205292], 9).expect("real world.db required");
        h.cast_and_deliver(198590, Some(h.npc));
        // Change health after application: execute damage must not snapshot.
        h.app
            .world_mut()
            .get_mut::<shared::components::Health>(h.npc)
            .unwrap()
            .current = health;
        h.advance(22);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let ticks: Vec<_> = h
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(198590)
                    && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .collect();
        assert_eq!(ticks.len(), 1);
        assert!(
            varied(damage).contains(&(health - h.health(h.npc))),
            "ticks: {ticks:?}"
        );
    }
}

#[test]
fn dummy2_warlock_drain_soul_target_death_grants_one_whole_shard() {
    let mut h = harness(Vec::new(), &[198590, 205292], 9).expect("real world.db required");
    h.set_power(PowerType::SoulShards, 0.0);
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(h.npc)
        .unwrap()
        .current = 100.0;
    h.cast_and_deliver(198590, Some(h.npc));
    h.advance(25);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(h.health(h.npc), 0.0);
    assert_eq!(h.power(PowerType::SoulShards), 10.0);
}

#[test]
fn dummy2_warlock_drain_soul_expiry_on_living_target_grants_no_shard() {
    let mut h = harness(Vec::new(), &[198590, 205292], 9).expect("real world.db required");
    h.set_power(PowerType::SoulShards, 0.0);
    h.cast_and_deliver(198590, Some(h.npc));
    h.advance(110);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.health(h.npc) > 0.0);
    assert_eq!(h.power(PowerType::SoulShards), 0.0);
}

#[test]
fn dummy2_warlock_stopping_drain_soul_grants_no_shard_or_later_ticks() {
    use lightyear::prelude::*;
    use shared::protocol::{CombatChannel, StopSpellCast};
    let mut h = harness(Vec::new(), &[198590, 205292], 9).expect("real world.db required");
    h.set_power(PowerType::SoulShards, 0.0);
    h.cast_and_deliver(198590, Some(h.npc));
    h.app
        .world_mut()
        .get_mut::<MessageSender<StopSpellCast>>(h.connection)
        .unwrap()
        .send::<CombatChannel>(StopSpellCast);
    h.advance(110);
    assert_eq!(h.health(h.npc), 5000.0);
    assert_eq!(h.power(PowerType::SoulShards), 0.0);
}
