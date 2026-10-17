//! Actual lethal-damage/persistence boundary; pinned TC Player.cpp6695–6913.
use super::*;
use crate::maps::InMap;
use crate::persistence::ClientId;
use crate::spell_cast::effects::{EffectCtx, deal_damage};
use shared::components::{Health, Player};

#[test]
fn achievements_criteria_world_honor_kill_keeps_lifetime_xp_without_inventing_level_or_wallet_award()
 {
    let (mut world, players, file) = native_flag_match(&shared::battleground::warsong_gulch());
    world.init_resource::<Time>();
    world.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .store
            .clone(),
    );
    world.init_resource::<Messages<crate::death::PlayerDeathTransition>>();
    world.init_resource::<crate::death::DeathServerState>();
    world.init_resource::<crate::death::DeathTransitionCounters>();
    world.remove_resource::<PvpServerState>();
    let (character, killer) = players[0];
    let victim = players[1].1;
    for player in [killer, victim] {
        world.entity_mut(player).insert((
            InMap::continent(0),
            PlayerPvpState::default(),
            crate::currency::PlayerCurrencies::default(),
        ));
    }
    world.get_mut::<Player>(victim).unwrap().race = 2;
    world
        .entity_mut(killer)
        .insert(shared::components::CurrentPvpFaction(2));
    world
        .entity_mut(victim)
        .insert(shared::components::CurrentPvpFaction(1));
    hit(&mut world, killer, victim, 1.0);
    assert_eq!(serialized_progress(&world, killer)["lifetime_honor_xp"], 0);
    // A lethal same-team kill cannot generate honor.
    let friendly = players[2].1;
    world.entity_mut(friendly).insert((
        InMap::continent(0),
        shared::components::CurrentPvpFaction(2),
    ));
    hit(&mut world, killer, friendly, 1000.0);
    assert_eq!(serialized_progress(&world, killer)["lifetime_honor_xp"], 0);
    // Level30 gray cutoff is20; equality is ineligible.
    let gray = players[3].1;
    world.entity_mut(gray).insert((
        InMap::continent(0),
        Level(20),
        shared::components::CurrentPvpFaction(1),
    ));
    hit(&mut world, killer, gray, 1000.0);
    assert_eq!(serialized_progress(&world, killer)["lifetime_honor_xp"], 0);
    hit(&mut world, killer, victim, 1000.0);
    assert_eq!(world.get::<Health>(victim).unwrap().current, 0.0);
    // TC ceil(30 * 1.55) =47, with no group/aura multiplier.
    let progress = serialized_progress(&world, killer);
    assert_eq!(progress["lifetime_honor_xp"], 47);
    assert_eq!(
        world
            .get::<crate::achievement::PlayerAchievementState>(killer)
            .unwrap()
            .criteria_progress
            .get(&63413),
        Some(&1),
        "one real eligible world kill must credit unconditional authored type35"
    );
    assert!(
        progress["honor_level"].is_null(),
        "HONOR_LEVEL_XP_CURVE gap"
    );
    assert_eq!(progress["honor"], 0, "XP is not spendable1792");
    assert_eq!(serialized_progress(&world, victim)["lifetime_honor_xp"], 0);
    hit(&mut world, killer, victim, 1000.0);
    assert_eq!(
        serialized_progress(&world, killer)["lifetime_honor_xp"],
        47,
        "dead-target replay"
    );
    let snapshot = serde_json::to_value(pvp_support::build_pvp_snapshot(
        world.get::<PlayerPvpState>(killer).unwrap(),
    ))
    .unwrap();
    assert_eq!(snapshot["lifetime_honor_xp"], 47);
    assert!(snapshot["honor_level"].is_null());
    assert_eq!(world.get::<ClientId>(killer).unwrap().0, character);
    drop(world);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = crate::persistence::load_pvp(&db, character);
    assert_eq!(
        serde_json::to_value(saved).unwrap()["lifetime_honor_xp"],
        47
    );
}

fn serialized_progress(world: &World, player: Entity) -> serde_json::Value {
    serde_json::to_value(&world.get::<PlayerPvpState>(player).unwrap().progress).unwrap()
}

fn hit(world: &mut World, killer: Entity, victim: Entity, amount: f32) {
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    let ctx = EffectCtx {
        caster: killer,
        spell: data.store.get(133).unwrap(),
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    deal_damage(world, &ctx, victim, amount, false, false);
}
