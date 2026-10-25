//! Second batch: real catalog casts, not direct script-hook calls.
use super::test_harness::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

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
        .next_ready_at();
    h.inbox.charges.clear();
    h.cast_and_deliver(193455, Some(h.npc));
    h.advance(5);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(390.4 * 220.0 / 340.0).contains(&(5000.0 - h.health(h.npc))));
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
        .next_ready_at();
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
            .on_cooldown(kill_command, super::history_now(h.app.world()))
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
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[100784, 261917, 107428, 185099],
        10,
    )
    .expect("real world.db required");
    h.set_power(PowerType::Chi, 6.0);
    h.cast_and_deliver(107428, Some(h.npc));
    h.advance(35);
    let before = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&1518]
        .next_ready_at();
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
        .next_ready_at();
    assert_eq!(after, before - 1.0);
}

#[test]
fn dummy2_monk_blackout_kick_reduces_fists_of_fury_cooldown_with_rank_two() {
    let mut h = harness_with_native_enemy_dodge_reduction(
        Vec::new(),
        &[100784, 261917, 113656, 117418, 120086],
        10,
    )
    .expect("real world.db required");
    // This is a cooldown-sequence fixture, not a target-death test. Real cone damage
    // now reaches the target; keep it alive for the subsequent Blackout Kick.
    *h.app
        .world_mut()
        .get_mut::<shared::components::Health>(h.npc)
        .unwrap() = shared::components::Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    h.set_power(PowerType::Chi, 6.0);
    h.cast_and_deliver(113656, Some(h.npc));
    h.advance(90); // Fists channel finished; its 24-second cooldown is running.
    assert!(
        h.health(h.npc) > 0.0,
        "cooldown follow-up needs a living target"
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(117418)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
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
    let update = h
        .inbox
        .cooldowns
        .iter()
        .find(|update| update.spell_id == 113656)
        .expect("Fists of Fury cooldown update");
    assert_eq!(
        update.duration_ms, 24000,
        "keep full timer duration for the client sweep"
    );
    assert!(update.remaining_ms < update.duration_ms);
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
    let mut h =
        harness_with_native_enemy_dodge_reduction(Vec::new(), &[100784, 107428, 185099], 10)
            .expect("real world.db required");
    h.set_power(PowerType::Chi, 6.0);
    h.cast_and_deliver(107428, Some(h.npc));
    h.advance(35);
    let before = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&1518]
        .next_ready_at();
    h.set_power(PowerType::Chi, 6.0);
    h.cast_and_deliver(100784, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let after = h
        .app
        .world()
        .get::<super::SpellHistory>(h.caster)
        .unwrap()
        .charges[&1518]
        .next_ready_at();
    assert_eq!(after, before);
}

#[test]
fn dummy2_rogue_backstab_deals_twenty_percent_more_from_behind() {
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[53], 4)
        .expect("real world.db required");
    h.set_power(PowerType::Energy, 100.0);
    h.cast_and_deliver(53, Some(h.npc)); // NPC faces +X; caster is at X=0.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(107.31 * 260.0 / 370.0 * 1.2).contains(&(5000.0 - h.health(h.npc))));
    assert_eq!(h.power(PowerType::ComboPoints), 1.0);
}

fn frontal_backstab_harness(guaranteed_block: bool) -> Harness {
    let mut h = harness_with_native_enemy_dodge_reduction(Vec::new(), &[53, 10021], 4)
        .expect("real world.db required");
    h.app.insert_resource(load_native_creature_world(&[6]));
    if guaranteed_block {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.npc,
                target: h.npc,
                spell_id: 10021,
                effect_mask: 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(h.caster)
        .unwrap()
        .x = 5.0;
    h.set_power(PowerType::Energy, 100.0);
    h
}

fn backstab_damage_before_block(h: &Harness, guaranteed_block: bool) -> f32 {
    let mut hits = h.inbox.log.iter().filter(|event| {
        event.spell_id == Some(53)
            && event.kind == CombatLogKind::Damage
            && event.target == Some(h.npc.to_bits())
    });
    let hit = hits.next().expect("Backstab hit the native NPC");
    assert!(hits.next().is_none(), "one Backstab damage receipt");
    assert_eq!(5000.0 - h.health(h.npc), hit.amount as f32);
    if guaranteed_block {
        assert!(hit.blocked > 0, "native10021 must exercise actual block");
    }
    // Backstab's positional coefficient precedes the target's native block.
    (hit.amount + hit.blocked) as f32
}

#[test]
fn dummy2_rogue_backstab_from_front_keeps_base_damage() {
    for guaranteed_block in [false, true] {
        let mut h = frontal_backstab_harness(guaranteed_block);
        h.cast_and_deliver(53, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let damage = backstab_damage_before_block(&h, guaranteed_block);
        assert!(varied(107.31 * 260.0 / 370.0).contains(&damage));
        assert_eq!(h.power(PowerType::ComboPoints), 1.0);
    }
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
    // Active SpellEffect 891602: 4.84 SP, fixture SP100, native variance5%.
    assert!(varied(484.0).contains(&(5000.0 - h.health(h.npc))));
    assert!(varied(484.0).contains(&(5000.0 - h.health(nearby))));
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

#[test]
fn dummy2_mage_fire_blast_guarantees_a_critical_hit_with_zero_crit_stats() {
    let mut h = harness(Vec::new(), &[108853], 8).expect("real world.db required");
    h.cast_and_deliver(108853, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let damage = h
        .inbox
        .log
        .iter()
        .find(|event| {
            event.spell_id == Some(108853) && event.kind == shared::protocol::CombatLogKind::Damage
        })
        .unwrap();
    assert!(damage.crit);
    assert!(varied(98.625 * 2.0).contains(&(5000.0 - h.health(h.npc))));
}

#[test]
fn dummy2_dk_death_coil_damages_an_enemy_through_47632() {
    let mut h =
        harness(Vec::new(), &[47541, 47632, 47633, 77575], 6).expect("real world.db required");
    h.set_power(PowerType::RunicPower, 1000.0);
    h.cast_and_deliver(47541, Some(h.npc));
    h.advance(12);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(211.812).contains(&(5000.0 - h.health(h.npc))));
    assert!(h.power(PowerType::RunicPower) < 1000.0);
    assert!(h.inbox.log.iter().any(|event| event.spell_id == Some(47632)
        && event.kind == shared::protocol::CombatLogKind::Damage));
}

#[test]
fn dummy2_dk_death_coil_heals_self_while_lichborne() {
    let mut h = harness(Vec::new(), &[47541, 47632, 47633, 49039, 77575], 6)
        .expect("real world.db required");
    h.cast_and_deliver(49039, Some(h.caster));
    h.advance(35);
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(h.caster)
        .unwrap()
        .current = 100.0;
    h.set_power(PowerType::RunicPower, 1000.0);
    h.cast_and_deliver(47541, Some(h.caster));
    h.advance(12);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(varied(463.75).contains(&(h.health(h.caster) - 100.0)));
    assert!(h.inbox.log.iter().any(|event| event.spell_id == Some(47633)
        && event.kind == shared::protocol::CombatLogKind::Heal));
}

#[test]
fn dummy2_dk_death_coil_rejects_living_ally_without_spending_runic_power() {
    let mut h =
        harness(Vec::new(), &[47541, 47632, 47633, 77575], 6).expect("real world.db required");
    let ally = spawn_player(&mut h.app, 1, 2.0);
    h.set_power(PowerType::RunicPower, 1000.0);
    h.cast_and_deliver(47541, Some(ally));
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.power(PowerType::RunicPower), 1000.0);
    assert_eq!(h.health(ally), 1000.0);
}

#[test]
fn dummy2_dk_death_coil_extends_both_outbreak_plagues_by_one_second() {
    let mut h = harness(
        Vec::new(),
        &[47541, 47632, 47633, 77575, 1240996, 191587],
        6,
    )
    .expect("real world.db required");
    h.cast_and_deliver(77575, Some(h.npc));
    h.advance(35);
    let expirations: Vec<_> = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .filter(|aura| [1240996, 191587].contains(&aura.spell_id))
        .map(|aura| (aura.spell_id, aura.expires_at.unwrap()))
        .collect();
    assert_eq!(expirations.len(), 2);
    h.set_power(PowerType::RunicPower, 1000.0);
    h.cast_and_deliver(47541, Some(h.npc));
    h.advance(12);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    for (spell, before) in expirations {
        let after = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(h.npc)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == spell)
            .unwrap()
            .expires_at
            .unwrap();
        assert_eq!(after, before + 1.0);
    }
}

#[test]
fn dummy2_shaman_chain_heal_chooses_injured_allies_and_falls_off_each_jump() {
    let mut h = harness(Vec::new(), &[1064], 7).expect("real world.db required");
    let primary = spawn_player(&mut h.app, 1, 2.0);
    let near = spawn_player(&mut h.app, 1, 4.0);
    let first = spawn_player(&mut h.app, 1, 17.0);
    let second = spawn_player(&mut h.app, 1, 35.0);
    let third = spawn_player(&mut h.app, 1, 53.0);
    for (unit, current, max) in [
        (primary, 1000.0, 5000.0),
        (near, 800.0, 1000.0),
        (first, 1000.0, 4000.0),
        (second, 1000.0, 6000.0),
        (third, 1000.0, 7000.0),
    ] {
        *h.app
            .world_mut()
            .get_mut::<shared::components::Health>(unit)
            .unwrap() = shared::components::Health { current, max };
    }
    h.cast_and_deliver(1064, Some(primary));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    for (unit, base) in [
        (primary, 231.0),
        (first, 231.0 * 0.7),
        (second, 231.0 * 0.7 * 0.7),
        (third, 231.0 * 0.7 * 0.7 * 0.7),
    ] {
        assert!(
            varied(base).contains(&(h.health(unit) - 1000.0)),
            "unit {unit:?}: {}",
            h.health(unit)
        );
    }
    assert_eq!(
        h.health(near),
        800.0,
        "most injured takes priority over nearest"
    );
    assert_eq!(h.health(h.npc), 5000.0);
    let targets: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(1064) && event.kind == shared::protocol::CombatLogKind::Heal
        })
        .map(|event| event.target)
        .collect();
    assert_eq!(
        targets,
        [primary, first, second, third].map(|unit| Some(unit.to_bits()))
    );
}

#[test]
fn dummy2_shaman_chain_heal_uses_catalog_twenty_yard_jump_radius() {
    let mut h = harness(Vec::new(), &[1064], 7).expect("real world.db required");
    let primary = spawn_player(&mut h.app, 1, 2.0);
    let edge = spawn_player(&mut h.app, 1, 22.0);
    let beyond = spawn_player(&mut h.app, 1, 45.0);
    for unit in [primary, edge, beyond] {
        h.app
            .world_mut()
            .get_mut::<shared::components::Health>(unit)
            .unwrap()
            .current = 100.0;
    }
    h.cast_and_deliver(1064, Some(primary));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.health(edge) > 100.0);
    assert_eq!(h.health(beyond), 100.0);
}

#[test]
fn dummy2_dh_demon_spikes_grants_agility_armor_and_expires() {
    let mut h =
        harness_with(Vec::new(), &[203720, 203819], 12, true).expect("real world.db required");
    let before = *h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    h.cast_and_deliver(203720, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let after = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    assert_eq!(
        after.armor,
        before.armor + (before.primary.agility * 0.75).floor()
    );
    assert!(after.armor > before.armor);
    assert_eq!(after.defense.parry, before.defense.parry);
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::StatModifiers>(h.caster)
            .unwrap()
            .parry_pct,
        0.0
    );
    h.advance(245); // Catalog203819 lasts twelve seconds.
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 203819));
    assert_eq!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .armor,
        before.armor
    );
}

#[test]
fn dummy2_dh_demon_spikes_rank_two_adds_eight_parry_points() {
    let mut h = harness_with(Vec::new(), &[203720, 203819, 321028], 12, true)
        .expect("real world.db required");
    // The unarmed harness cannot parry; assert the cast's nondiminishing
    // modifier (effective chance also requires a parry skill and weapon).
    let before = h
        .app
        .world()
        .get::<crate::stats::StatModifiers>(h.caster)
        .map_or(0.0, |mods| mods.parry_pct);
    h.cast_and_deliver(203720, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let after = h
        .app
        .world()
        .get::<crate::stats::StatModifiers>(h.caster)
        .unwrap()
        .parry_pct;
    assert_eq!(after, before + 8.0);
}

#[test]
fn dummy2_dh_demon_spikes_armor_tracks_agility_changes_while_active() {
    // Authentic passive23794 (Agility +15) is removed through normal learned-
    // passive reconciliation while the cast's Demon Spikes aura stays active.
    let mut h = harness_with(Vec::new(), &[203720, 203819, 23794], 12, true)
        .expect("real world.db required");
    h.cast_and_deliver(203720, Some(h.caster));
    let before = *h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            203720, 203819,
        ]));
    h.advance(4);
    let after = h
        .app
        .world()
        .get::<crate::stats::CharacterStats>(h.caster)
        .unwrap();
    assert_eq!(after.primary.agility, before.primary.agility - 15.0);
    assert_eq!(after.armor, (after.primary.agility * 0.75).floor());
    assert!(after.armor < before.armor);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 203819));
}

#[test]
fn dummy2_hunter_cobra_shot_level_curve_reaches_unity_at_twenty() {
    for (level, multiplier) in [(19, 328.0 / 340.0), (20, 1.0), (80, 1.0)] {
        let mut h = harness(Vec::new(), &[193455, 34026], 3).expect("real world.db required");
        h.app
            .world_mut()
            .get_mut::<shared::components::Level>(h.caster)
            .unwrap()
            .0 = level;
        h.set_power(PowerType::Focus, 100.0);
        h.cast_and_deliver(193455, Some(h.npc));
        h.advance(5);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(varied(390.4 * multiplier).contains(&(5000.0 - h.health(h.npc))));
    }
}

#[test]
fn dummy2_rogue_backstab_level_curve_reaches_unity_at_twenty() {
    for (level, multiplier, guaranteed_block) in [
        (19, 359.0 / 370.0, false),
        (20, 1.0, false),
        (80, 1.0, false),
        (20, 1.0, true),
    ] {
        let mut h = frontal_backstab_harness(guaranteed_block);
        h.app
            .world_mut()
            .get_mut::<shared::components::Level>(h.caster)
            .unwrap()
            .0 = level;
        h.cast_and_deliver(53, Some(h.npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        let damage = backstab_damage_before_block(&h, guaranteed_block);
        assert!(varied(107.31 * multiplier).contains(&damage));
    }
}
