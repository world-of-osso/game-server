use super::*;
use lightyear::prelude::MessageSender;
use shared::reputation::threshold_for_standing;

/// TDB1210 `guild_rewards` / `guild_rewards_req_achievements` rows and their
/// Achievement.db2 titles/factions (12.1.5.70448), loaded through the real loader.
pub(crate) fn tdb_fixture() -> GuildRewardCatalog {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(include_str!("../../../scripts/guild_rewards_schema.sql"))
        .unwrap();
    conn.execute_batch(
        "INSERT INTO tdb_guild_rewards VALUES (62038,4,0,12000000),(63138,7,4294967295,3000000),\
         (63206,5,18875469,3000000),(63207,5,33555378,3000000),(63352,5,18875469,1500000);\
         INSERT INTO tdb_guild_rewards_req_achievements VALUES (63138,5812),(63138,5892),(63352,4989);\
         CREATE TABLE achievement (ID INTEGER, Title_lang TEXT, Faction INTEGER);\
         INSERT INTO achievement VALUES (4989,'A Class Act',-1),(5812,'United Nations',1),(5892,'United Nations',0);",
    )
    .unwrap();
    load_guild_reward_catalog(&conn).unwrap()
}

const HUMAN: u8 = 1;
const ORC: u8 = 2;

#[test]
fn guild_rewards_list_filters_race_mask_and_shows_only_the_requesters_team_achievement() {
    let catalog = tdb_fixture();
    let human = catalog.state(HUMAN);
    assert_eq!(
        human
            .rewards
            .iter()
            .map(|reward| reward.item_id)
            .collect::<Vec<_>>(),
        vec![62038, 63138, 63206, 63352],
        "Alliance-mask Wrap of Unity, not the Horde one"
    );
    let phoenix = &human.rewards[1];
    assert_eq!(
        (phoenix.min_standing, phoenix.cost),
        (Standing::Exalted, 3_000_000)
    );
    assert_eq!(
        phoenix.achievements,
        vec![GuildRewardAchievement {
            id: 5812,
            name: "United Nations".into()
        }]
    );
    let orc = catalog.state(ORC);
    assert!(orc.rewards.iter().any(|reward| reward.item_id == 63207));
    assert!(!orc.rewards.iter().any(|reward| reward.item_id == 63206));
    assert_eq!(
        orc.rewards
            .iter()
            .find(|reward| reward.item_id == 63138)
            .unwrap()
            .achievements[0]
            .id,
        5892
    );
}

#[test]
fn guild_reward_purchase_needs_standing_then_guild_achievement() {
    let catalog = tdb_fixture();
    let mut reputation = CharacterReputation::default();
    assert_eq!(
        catalog.check_purchase(62038, HUMAN, Some(&reputation)),
        Err(MerchantError::GuildReputationTooLow)
    );
    reputation.set_rep(GUILD_FACTION, threshold_for_standing(Standing::Friendly));
    assert_eq!(
        catalog.check_purchase(62038, HUMAN, Some(&reputation)),
        Ok(())
    );
    reputation.set_rep(GUILD_FACTION, threshold_for_standing(Standing::Exalted));
    assert_eq!(
        catalog.check_purchase(63352, HUMAN, Some(&reputation)),
        Err(MerchantError::MissingTurnins),
        "no guild has earned A Class Act"
    );
    assert_eq!(catalog.check_purchase(2379, HUMAN, None), Ok(()));
}

#[test]
fn guild_quest_reputation_carries_the_360_experience_remainder() {
    let (accumulated, first) = quest_reputation(0);
    let (accumulated, second) = quest_reputation(accumulated);
    let (accumulated, third) = quest_reputation(accumulated);
    assert_eq!((first, second, third), (166, 167, 167));
    assert_eq!(accumulated, 180_000);
}

struct Member {
    app: App,
    connection: Entity,
    player: Entity,
    character: u64,
}

fn member(in_guild: bool) -> Member {
    let db = crate::persistence::temp_database();
    let (character, _) = crate::persistence::create_character(&db, 1, "Guildie", 1, 1).unwrap();
    if in_guild {
        crate::persistence_character::set_character_guild_by_id(&db, character, 7, "Rewards")
            .unwrap();
    }
    let mut app = App::new();
    app.insert_resource(GameDatabase::new(db));
    let player = app.world_mut().spawn(CharacterReputation::default()).id();
    let connection = app
        .world_mut()
        .spawn((
            PlayerLink(player),
            MessageSender::<ReputationStateUpdate>::default(),
        ))
        .id();
    Member {
        app,
        connection,
        player,
        character,
    }
}

impl Member {
    fn award(&mut self, player_level: u8, quest_level: i32) {
        award_quest_reputation(
            self.app.world_mut(),
            self.connection,
            self.character,
            player_level,
            quest_level,
        );
    }

    fn live(&self) -> i32 {
        self.app
            .world()
            .get::<CharacterReputation>(self.player)
            .unwrap()
            .get_value(GUILD_FACTION)
    }

    fn saved(&self) -> i32 {
        crate::persistence::load_reputation(
            &self.app.world().resource::<GameDatabase>().db,
            self.character,
            || panic!("guild reputation must be persisted"),
        )
        .get_value(GUILD_FACTION)
    }
}

#[test]
fn guild_quest_reputation_skips_grey_quests_and_unguilded_characters() {
    let mut guildie = member(true);
    // GetGrayLevel(30) = 20: a level-20 quest is grey, a level-21 quest green.
    guildie.award(30, 20);
    assert_eq!(guildie.live(), 0);
    guildie.award(30, 21);
    guildie.award(30, 30);
    assert_eq!((guildie.live(), guildie.saved()), (333, 333));

    let mut loner = member(false);
    loner.award(30, 30);
    assert_eq!(loner.live(), 0);
}
