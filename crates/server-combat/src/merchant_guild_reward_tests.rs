//! Shay Pressler (46602, Stormwind guild vendor) selling TDB1210 guild rewards.
use super::*;
use shared::reputation::{CharacterReputation, Standing, threshold_for_standing};

const SHAY_PRESSLER: u32 = 46602;
/// Worn Stoneskin Gargoyle Cape: Friendly, no achievement.
const GARGOYLE_CAPE: u32 = 62038;
/// Shroud of Cooperation: Honored and the guild achievement A Class Act.
const SHROUD_OF_COOPERATION: u32 = 63352;

fn guild_shopper() -> Harness {
    let mut h = harness().expect("authentic guild vendor data");
    h.app
        .insert_resource(crate::guild_rewards::tests::tdb_fixture());
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(CharacterReputation::default());
    h.set_gold(10_000_000);
    h
}

fn offer(h: &mut Harness, item_id: u32) -> (Entity, VendorItem) {
    let npc = h.open(SHAY_PRESSLER);
    let list = h
        .take::<VendorInventory>()
        .pop()
        .expect("guild vendor list");
    let item = list
        .items
        .into_iter()
        .find(|offer| offer.item_id == item_id)
        .expect("guild reward on Shay Pressler's list");
    (npc, item)
}

fn set_guild_standing(h: &mut Harness, standing: Standing) {
    h.app
        .world_mut()
        .get_mut::<CharacterReputation>(h.player)
        .unwrap()
        .set_rep(
            crate::guild_rewards::GUILD_FACTION,
            threshold_for_standing(standing),
        );
}

#[test]
fn guild_vendor_sells_a_reward_only_at_its_guild_standing() {
    let mut h = guild_shopper();
    let (npc, cape) = offer(&mut h, GARGOYLE_CAPE);
    assert!(
        !cape.usable,
        "Neutral guild standing lists the cape as unusable"
    );
    let before = h.inventory();
    h.buy(npc, cape.slot, cape.item_id, 1);
    assert_eq!(h.failures(), vec![MerchantError::ReputationRequired]);
    assert_eq!(h.inventory(), before);

    set_guild_standing(&mut h, Standing::Friendly);
    let (npc, cape) = offer(&mut h, GARGOYLE_CAPE);
    assert!(cape.usable);
    h.buy(npc, cape.slot, cape.item_id, 1);
    assert!(h.failures().is_empty());
    assert_eq!(h.count_of(GARGOYLE_CAPE), vec![1]);
}

#[test]
fn guild_vendor_refuses_a_reward_whose_guild_achievement_is_unearned() {
    let mut h = guild_shopper();
    set_guild_standing(&mut h, Standing::Exalted);
    let (npc, shroud) = offer(&mut h, SHROUD_OF_COOPERATION);
    assert!(!shroud.usable);
    let before = h.inventory();
    h.buy(npc, shroud.slot, shroud.item_id, 1);
    assert_eq!(h.failures(), vec![MerchantError::MissingTurnins]);
    assert_eq!(h.inventory(), before);
}

#[test]
fn admin_set_reputation_opens_the_friendly_reward_to_the_live_player() {
    use bevy::ecs::system::RunSystemOnce;

    use crate::admin_handlers::handle_set_reputation;
    use crate::admin_types::AdminResponse;

    let mut h = guild_shopper();
    let friendly = threshold_for_standing(Standing::Friendly);
    let set = move |faction_id| {
        move |db: Res<GameDatabase>, mut commands: Commands| {
            handle_set_reputation(&db, &mut commands, "Shopper", faction_id, friendly)
        }
    };
    let rejected = h.app.world_mut().run_system_once(set(999_999)).unwrap();
    assert!(
        matches!(rejected, AdminResponse::Error(message) if message == "faction 999999 not found")
    );

    let response = h
        .app
        .world_mut()
        .run_system_once(set(crate::guild_rewards::GUILD_FACTION))
        .unwrap();
    assert!(matches!(
        response,
        AdminResponse::ReputationSet {
            faction_id: 1168,
            value: 3000,
            ..
        }
    ));
    let (npc, cape) = offer(&mut h, GARGOYLE_CAPE);
    assert!(
        cape.usable,
        "Friendly guild standing lists the cape as usable"
    );
    h.buy(npc, cape.slot, cape.item_id, 1);
    assert!(h.failures().is_empty());
    assert_eq!(h.count_of(GARGOYLE_CAPE), vec![1]);
    let db = &h.app.world().resource::<GameDatabase>().db;
    let saved = persistence::load_reputation(db, h.character_id, || unreachable!());
    assert_eq!(
        saved.get_value(crate::guild_rewards::GUILD_FACTION),
        friendly
    );
}
