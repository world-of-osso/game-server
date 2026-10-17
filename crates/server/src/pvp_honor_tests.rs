//! Actual lethal-damage/persistence boundary; TC amount plus our explicit mirror policy.
use super::*;
use crate::maps::InMap;
use crate::persistence::ClientId;
use crate::spell_cast::effects::{EffectCtx, deal_damage};
use shared::components::{Health, Player};

#[test]
fn honor_kill_mirrors_xp_into_wallet_under_world_of_osso_policy() {
    let (mut world, players, file) = honor_kill_world();
    let (character, killer) = players[0];
    let victim = players[1].1;
    hit(&mut world, killer, victim, 1.0);
    assert_eq!(serialized_progress(&world, killer)["lifetime_honor_xp"], 0);
    assert_eq!(honor_wallet(&world, killer), 0);
    let friendly = players[2].1;
    world.entity_mut(friendly).insert((
        InMap::continent(0),
        shared::components::CurrentPvpFaction(2),
    ));
    hit(&mut world, killer, friendly, 1000.0);
    assert_eq!(honor_wallet(&world, killer), 0);
    // Level30 gray cutoff is20; equality is ineligible.
    let gray = players[3].1;
    world.entity_mut(gray).insert((
        InMap::continent(0),
        Level(20),
        shared::components::CurrentPvpFaction(1),
    ));
    hit(&mut world, killer, gray, 1000.0);
    assert_eq!(honor_wallet(&world, killer), 0);
    hit(&mut world, killer, victim, 1000.0);
    assert_eq!(world.get::<Health>(victim).unwrap().current, 0.0);
    // TC ceil(30 * 1.55) =47; HONOR_WALLET_MIRRORS_HONOR_XP binds the same N to1792.
    let progress = serialized_progress(&world, killer);
    assert_eq!(progress["lifetime_honor_xp"], 47);
    assert!(
        progress["honor_level"].is_null(),
        "HONOR_LEVEL_XP_CURVE gap"
    );
    assert_eq!(
        honor_wallet(&world, killer),
        47,
        "explicit1:1 project policy"
    );
    assert_eq!(
        progress["honor"], 47,
        "PvP projection follows the real wallet"
    );
    assert_eq!(progress["conquest"], 0, "no Conquest projection policy");
    assert_eq!(serialized_progress(&world, victim)["lifetime_honor_xp"], 0);
    hit(&mut world, killer, victim, 1000.0);
    assert_eq!(serialized_progress(&world, killer)["lifetime_honor_xp"], 47);
    assert_eq!(
        honor_wallet(&world, killer),
        47,
        "dead-target replay cannot duplicate either award"
    );
    let snapshot = pvp_support::build_pvp_snapshot(world.get::<PlayerPvpState>(killer).unwrap());
    assert_eq!(snapshot.lifetime_honor_xp, 47);
    assert_eq!(snapshot.honor, 47);
    assert!(snapshot.honor_level.is_none());
    assert_eq!(world.get::<ClientId>(killer).unwrap().0, character);
    drop(world);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = crate::persistence::load_pvp(&db, character);
    assert_eq!(saved.lifetime_honor_xp, 47);
    assert_eq!(saved.honor, 47);
    let wallet = crate::persistence::load_currencies(&db, character);
    assert_eq!(wallet.get(1792), 47);
    assert_eq!(wallet.week_earned(1792), 0);
    assert_eq!(wallet.get(1602), 0);
}

#[test]
fn honor_wallet_holding_cap_does_not_reduce_the_separate_xp_award() {
    let (mut world, players, file) = honor_kill_world();
    let (character, killer) = players[0];
    let victim = players[1].1;
    let mut wallet = world
        .get_mut::<crate::currency::PlayerCurrencies>(killer)
        .unwrap();
    wallet
        .0
        .add(
            1792,
            14_990,
            shared::currency::currency_by_id(1792).unwrap(),
        )
        .unwrap();
    drop(wallet);
    hit(&mut world, killer, victim, 1000.0);
    assert_eq!(serialized_progress(&world, killer)["lifetime_honor_xp"], 47);
    assert_eq!(
        honor_wallet(&world, killer),
        15_000,
        "authored holding cap still applies"
    );
    drop(world);
    let db = redb::Database::open(file.path()).unwrap();
    let progress = crate::persistence::load_pvp(&db, character);
    assert_eq!(progress.lifetime_honor_xp, 47);
    assert_eq!(progress.honor, 15_000);
    let saved = crate::persistence::load_currencies(&db, character);
    assert_eq!(saved.get(1792), 15_000);
    assert_eq!(saved.week_earned(1792), 0);
}

fn honor_kill_world() -> (World, Vec<(u64, Entity)>, tempfile::NamedTempFile) {
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
    let killer = players[0].1;
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
    (world, players, file)
}

fn honor_wallet(world: &World, player: Entity) -> u32 {
    world
        .get::<crate::currency::PlayerCurrencies>(player)
        .unwrap()
        .0
        .get(1792)
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
