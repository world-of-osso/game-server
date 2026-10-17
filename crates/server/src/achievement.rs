use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};
use shared::components::{Level, Npc, Player};
use shared::protocol::{
    AchievementCatalogPage, AchievementChannel, AchievementSnapshot, AchievementStateUpdate,
    AchievementToastSnapshot, ChallengePhase, ChallengeRun, QueryAchievementCatalog,
};

use crate::auth::{Authenticated, InWorld};
use crate::maps::InMap;
use crate::networking::PlayerLink;
use crate::persistence::{self, GameDatabase};

#[path = "achievement_catalog.rs"]
mod catalog;
pub(crate) use catalog::{
    AchievementCatalog, CriteriaEvent, PlayerConditions, ReferencePlayer, load_achievement_catalog,
};
#[path = "achievement_account.rs"]
mod account_store;
#[path = "dungeon_progress.rs"]
mod dungeon;
#[path = "achievement_kills.rs"]
mod kills;
#[path = "scenario.rs"]
mod scenario;
pub(crate) use scenario::load_scenarios;

#[derive(Component, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlayerAchievementState {
    pub earned_ids: BTreeSet<u32>,
    #[serde(default)]
    pub criteria_progress: BTreeMap<u32, u64>,
    #[serde(default)]
    pub earned_at: BTreeMap<u32, i64>,
    /// Pre-support reputation quantities were not valid signed Retail progress.
    #[serde(default)]
    pub signed_reputation: bool,
    /// Retail AreaTable.AreaBit facts; distinct AreaIDs can share one explored bit.
    #[serde(default)]
    pub explored_area_bits: BTreeSet<u32>,
    #[serde(default)]
    pub(crate) monthly: Option<crate::adventure_guide::MonthlyCriteriaState>,
    /// Presentation-only aggregate; never serialized into character contributions.
    #[serde(skip)]
    pub account: Option<Box<PlayerAchievementState>>,
    /// Disposable derived tree results; excluded from persistence and logical equality.
    #[serde(skip)]
    pub(crate) evaluation: catalog::EvaluationCache,
}

/// One-time migration of the six former level placeholders. Counter-bearing rows
/// already use Retail IDs and are never remapped.
pub(crate) fn decode_saved_achievement_state(bytes: &[u8]) -> (PlayerAchievementState, bool) {
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("decode achievement JSON");
    let legacy = value.get("criteria_progress").is_none();
    let mut state: PlayerAchievementState =
        serde_json::from_value(value).expect("decode achievements");
    if legacy {
        const RETAIL_IDS: [u32; 6] = [6, 7, 9, 14783, 15805, 19459];
        state.earned_ids = state
            .earned_ids
            .into_iter()
            .map(|id| {
                if (1..=6).contains(&id) {
                    RETAIL_IDS[(id - 1) as usize]
                } else {
                    id
                }
            })
            .collect();
    }
    (state, legacy)
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct AchievementTracker {
    last_processed_level: u8,
    last_processed_zone: Option<u32>,
    last_processed_area: Option<u32>,
}

/// Committed gameplay credit from systems that do not own an exclusive World.
#[derive(Message, Clone, Copy)]
pub(crate) struct CriteriaCredit {
    pub player: Entity,
    pub event: CriteriaEvent,
}

/// Native class/profession merge and account mount load must precede once-only Login74.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct InitializeSpellKnowledge;

pub struct AchievementPlugin;

impl Plugin for AchievementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::maps::ServerClock>();
        app.add_message::<CriteriaCredit>();
        app.configure_sets(
            Update,
            InitializeSpellKnowledge.after(crate::auth::process_select_character),
        );
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        app.init_resource::<scenario::ScenarioRuns>();
        app.add_systems(PostUpdate, kills::publish_rewarded_creature_kills);
        app.add_systems(
            Update,
            (
                send_initial_achievement_snapshot.after(InitializeSpellKnowledge),
                process_level_achievement_updates,
                process_skill_achievement_updates,
                process_known_spell_achievement_updates,
                process_owned_pet_achievement_updates,
                process_equipped_achievement_updates,
                process_reputation_achievement_updates,
                process_exploration_achievement_updates,
                process_top_level_area_achievement_updates,
                process_committed_criteria_credits,
                process_inventory_acquisition_credits,
                dungeon::send_entry_progress,
                scenario::send_entry_progress,
                send_requested_catalog_pages,
            )
                .chain(),
        );
    }
}

fn send_initial_achievement_snapshot(world: &mut World) {
    let connections: Vec<_> = world
        .query_filtered::<(Entity, &PlayerLink, &InWorld, &Authenticated), Added<InWorld>>()
        .iter(world)
        .map(|(connection, link, in_world, auth)| {
            (connection, link.0, in_world.character_id, auth.account_id)
        })
        .collect();
    for (connection, player, character, account) in connections {
        let Some(level) = world.get::<Level>(player).map(|level| level.0) else {
            continue;
        };
        let events = load_login_events(world, character, account, level);
        let Some((snapshot, toasts)) = update_player_events(world, player, &events, None) else {
            continue;
        };
        crate::adventure_guide::ignore_login_achievements(
            world,
            player,
            toasts.iter().map(|toast| toast.achievement_id),
        );
        send_updates(world, connection, snapshot, Vec::new());
        world.entity_mut(player).insert(AchievementTracker {
            last_processed_level: level,
            last_processed_zone: None,
            last_processed_area: None,
        });
    }
}

fn load_login_events(world: &World, character: u64, account: u64, level: u8) -> Vec<CriteriaEvent> {
    let mut events = vec![CriteriaEvent::Login, CriteriaEvent::ReachLevel(level)];
    if let Some(db) = world.get_resource::<GameDatabase>() {
        // TC BankTabPurchased login case: current tab counts of both banks.
        let tabs = [
            (
                0,
                persistence::banks::load_character_bank(&db.db, character),
            ),
            (2, persistence::banks::load_account_bank(&db.db, account)),
        ];
        events.extend(
            tabs.map(|(bank, data)| CriteriaEvent::BankTabsOwned(bank, data.tabs.len() as u32)),
        );
        let rewarded = persistence::quest_store::load_quest_log(&db.db, character).rewarded;
        events.extend(rewarded_quest_facts(world, &rewarded, true));
        events.extend(rewarded.into_iter().map(CriteriaEvent::CompleteQuest));
    }
    events
}

/// TC GetRewardedQuestCount and the CompleteQuestsInZone login case (CriteriaHandler.cpp
/// 627-629, 666-678) count only quests that increase rewarded counters.
fn rewarded_quest_facts(world: &World, rewarded: &[u32], zones: bool) -> Vec<CriteriaEvent> {
    let Some(quests) = world.get_resource::<crate::quest_data::QuestDb>() else {
        return Vec::new();
    };
    let counted: Vec<_> = rewarded
        .iter()
        .filter_map(|id| quests.quests.get(id))
        .filter(|quest| quest.increases_rewarded_counters())
        .collect();
    let mut by_zone = BTreeMap::<u32, u32>::new();
    for quest in counted.iter().filter(|quest| quest.sort_id > 0 && zones) {
        *by_zone.entry(quest.sort_id as u32).or_default() += 1;
    }
    let count = u32::try_from(counted.len()).expect("rewarded quest count fits u32");
    std::iter::once(CriteriaEvent::RewardedQuestCount(count))
        .chain(
            by_zone
                .into_iter()
                .map(|(zone, count)| CriteriaEvent::RewardedQuestsInZone(zone, count)),
        )
        .collect()
}

fn process_level_achievement_updates(world: &mut World) {
    let players: Vec<_> = world
        .query_filtered::<(Entity, &Level, &AchievementTracker), Changed<Level>>()
        .iter(world)
        .filter(|(_, level, tracker)| level.0 != tracker.last_processed_level)
        .map(|(player, level, _)| (player, level.0))
        .collect();
    for (player, level) in players {
        let Some((snapshot, toasts)) =
            update_player_events(world, player, &[CriteriaEvent::ReachLevel(level)], None)
        else {
            continue;
        };
        crate::adventure_guide::record(world, player, CriteriaEvent::ReachLevel(level));
        world
            .get_mut::<AchievementTracker>(player)
            .unwrap()
            .last_processed_level = level;
        if let Some(connection) = connection_for_player(world, player) {
            send_updates(world, connection, snapshot, toasts);
        }
    }
}

/// TC a352b1fa CriteriaHandler.cpp:601-604 reads GetBaseSkillValue and 623-626
/// GetPureMaxSkillValue; Changed includes newly loaded skill lines, so login restores them.
fn process_skill_achievement_updates(world: &mut World) {
    let updates: Vec<_> = world.query_filtered::<(Entity, &crate::profession::PlayerProfessions), Changed<crate::profession::PlayerProfessions>>()
        .iter(world).flat_map(|(player, professions)| professions.lines.iter()
            .filter(|line| line.rank != 0)
            .flat_map(move |line| [
                (player, CriteriaEvent::SkillRank(line.skill_line, line.rank)),
                (player, CriteriaEvent::SkillMaxRank(line.skill_line, line.max_rank)),
            ])).collect();
    for (player, event) in updates {
        record_criteria_event(world, player, event);
    }
}

/// TC CriteriaHandler.cpp687/1530-1535: knowing the authored spell is a boolean fact.
/// Owned account mounts are known spells on every character: TC CollectionMgr::AddMount
/// (CollectionMgr.cpp:414-443) LearnSpells each one at login and when learned.
fn process_known_spell_achievement_updates(world: &mut World) {
    let mut query = world.query_filtered::<(
        Entity,
        Option<&crate::class_progression::KnownSpells>,
        Option<&crate::mount_journal::AccountMounts>,
    ), (
        With<Player>,
        Or<(
            Changed<crate::class_progression::KnownSpells>,
            Changed<crate::mount_journal::AccountMounts>,
        )>,
    )>();
    let Some(catalog) = world.get_resource::<AchievementCatalog>() else {
        return;
    };
    let updates: Vec<_> = query
        .iter(world)
        .map(|(player, known, mounts)| {
            let spells = known
                .into_iter()
                .flat_map(|known| known.iter())
                .chain(crate::mount_journal::owned_spells(world, mounts));
            (player, catalog.known_spell_events(spells))
        })
        .collect();
    for (player, events) in updates {
        record_criteria_events(world, player, &events);
    }
}

/// Persisted physical equipment from InventoryModel, never bag ownership or display art.
fn process_equipped_achievement_updates(world: &mut World) {
    let updates: Vec<_> = world.query_filtered::<(Entity, &crate::stats::EquippedItems), (With<Player>, Changed<crate::stats::EquippedItems>)>()
        .iter(world).map(|(player, equipped)| {
            let items: BTreeSet<_> = equipped.items.iter().filter_map(|(_, source, item)|
                (*source == shared::item_data::ItemDefinitionSource::Retail).then_some(*item)).collect();
            (player, items.into_iter().map(CriteriaEvent::EquipItem).collect::<Vec<_>>())
        }).collect();
    for (player, events) in updates {
        record_criteria_events(world, player, &events);
    }
}

/// Account journal facts, never the old cosmetic collection's pre-filled known flags.
fn process_owned_pet_achievement_updates(world: &mut World) {
    let mut query = world.query_filtered::<(Entity, &crate::battle_pets::AccountPetJournal), Changed<crate::battle_pets::AccountPetJournal>>();
    let Some(catalog) = world.get_resource::<crate::battle_pets::PetCatalog>() else {
        return;
    };
    let updates: Vec<_> = query
        .iter(world)
        .map(|(player, journal)| {
            let creatures: BTreeSet<_> = journal
                .journal
                .pets
                .iter()
                .filter_map(|pet| catalog.creature_id(pet.species_id))
                .collect();
            let events: Vec<_> = creatures
                .into_iter()
                .map(CriteriaEvent::KnowPetCreature)
                .chain(std::iter::once(CriteriaEvent::OwnUniquePets(
                    u32::try_from(journal.journal.unique_species())
                        .expect("journal count fits u32"),
                )))
                .collect();
            (player, events)
        })
        .collect();
    for (player, events) in updates {
        record_criteria_events(world, player, &events);
    }
}

#[path = "achievement_rank_factions.rs"]
mod rank_factions;

/// Current traditional ranks, not visible/tracked factions or historical maxima.
fn reputation_rank_events(
    reputation: &shared::reputation::CharacterReputation,
) -> Vec<CriteriaEvent> {
    let values: Vec<_> = reputation
        .values()
        .filter(|(id, _)| {
            rank_factions::TRADITIONAL_RANK_FACTIONS
                .binary_search(id)
                .is_ok()
        })
        .map(|(_, standing)| standing)
        .collect();
    use shared::reputation::{Standing, threshold_for_standing};
    let count = |rank| {
        let threshold = threshold_for_standing(rank);
        u32::try_from(values.iter().filter(|&&value| value >= threshold).count())
            .expect("faction count fits u32")
    };
    vec![
        CriteriaEvent::TotalExaltedFactions(count(Standing::Exalted)),
        CriteriaEvent::TotalReveredFactions(count(Standing::Revered)),
        CriteriaEvent::TotalHonoredFactions(count(Standing::Honored)),
    ]
}

/// TC CriteriaHandler709-715/737-745: positive standing and current rank totals.
fn process_reputation_achievement_updates(world: &mut World) {
    let updates: Vec<_> = world.query_filtered::<(Entity, &shared::reputation::CharacterReputation), Changed<shared::reputation::CharacterReputation>>()
        .iter(world).map(|(player, reputation)| {
            let events = reputation.values().filter(|(_, value)| *value > 0)
                .map(|(faction, value)| CriteriaEvent::ReputationGained(faction, value as u32))
                .chain(reputation_rank_events(reputation)).collect::<Vec<_>>();
            (player, events)
        }).collect();
    for (player, events) in updates {
        record_criteria_events(world, player, &events);
    }
}

/// TC Player::CheckAreaExplore: living, non-taxi players discover the authored terrain area.
fn process_exploration_achievement_updates(world: &mut World) {
    if !world.contains_resource::<crate::ground::GroundMap>() {
        return;
    }
    let players: Vec<_> = world
        .query::<(&Authenticated, &PlayerLink, &InWorld)>()
        .iter(world)
        .map(|(_, link, _)| link.0)
        .collect();
    for player in players {
        let Some(bit) = detect_explorable_area_bit(world, player) else {
            continue;
        };
        let Some(mut state) = world.get_mut::<PlayerAchievementState>(player) else {
            continue;
        };
        if !state.explored_area_bits.insert(bit) {
            continue;
        }
        if let Some((snapshot, toasts)) =
            update_player_events(world, player, &[CriteriaEvent::ExploreAreaBit(bit)], None)
            && let Some(connection) = connection_for_player(world, player)
        {
            send_updates(world, connection, snapshot, toasts);
        }
    }
}

fn detect_explorable_area_bit(world: &mut World, player: Entity) -> Option<u32> {
    let is_dead = world
        .get::<crate::death::PlayerDeathState>(player)
        .is_some_and(|life| life.state != shared::death::DeathState::Alive);
    let on_taxi = world.get::<crate::taxi::TaxiFlight>(player).is_some();
    if is_dead || on_taxi {
        return None;
    }
    let (area, _) = load_player_terrain_area(world, player)?;
    world.get_resource::<AchievementCatalog>()?.area_bit(area)
}

fn load_player_terrain_area(world: &mut World, player: Entity) -> Option<(u32, u32)> {
    let position = *world.get::<shared::components::Position>(player)?;
    let map = *world.get::<InMap>(player)?;
    let area = world
        .resource_mut::<crate::ground::GroundMap>()
        .area_at(map.map_id, Vec3::new(position.x, position.y, position.z))?;
    Some((area, map.map_id))
}

fn process_top_level_area_achievement_updates(world: &mut World) {
    if !world.contains_resource::<crate::ground::GroundMap>() {
        return;
    }
    if !world.contains_resource::<crate::area_table::AreaTable>() {
        return;
    }
    let players: Vec<_> = world
        .query::<(&Authenticated, &PlayerLink, &InWorld)>()
        .iter(world)
        .map(|(_, link, _)| link.0)
        .collect();
    for player in players {
        update_top_level_area_credit(world, player);
    }
}

fn update_top_level_area_credit(world: &mut World, player: Entity) {
    let zone = load_player_terrain_area(world, player).and_then(|(area, map)| {
        let zone = world
            .resource::<crate::area_table::AreaTable>()
            .zone_id(area, map)?;
        record_terrain_area_transition(world, player, area);
        Some(zone)
    });
    let Some(tracker) = world.get::<AchievementTracker>(player) else {
        return;
    };
    if tracker.last_processed_zone == zone {
        return;
    }
    world
        .get_mut::<AchievementTracker>(player)
        .unwrap()
        .last_processed_zone = zone;
    if let Some(zone) = zone {
        // TC Player::UpdateZone7590 does not impose exploration's alive/non-taxi restriction.
        record_criteria_event(world, player, CriteriaEvent::EnterTopLevelArea(zone));
    }
}

/// TC Player::UpdateArea7507-7510: actual authenticated terrain changes only.
fn record_terrain_area_transition(world: &mut World, player: Entity, area: u32) {
    let Some(tracker) = world.get::<AchievementTracker>(player).copied() else {
        return;
    };
    if tracker.last_processed_area == Some(area) {
        return;
    }
    let events: Vec<_> = std::iter::once(CriteriaEvent::EnterArea(area))
        .chain(tracker.last_processed_area.map(CriteriaEvent::LeaveArea))
        .collect();
    world
        .get_mut::<AchievementTracker>(player)
        .unwrap()
        .last_processed_area = Some(area);
    record_criteria_events(world, player, &events);
}

/// Runs only after the quest reward transaction commits. Failed/incomplete turn-ins
/// never reach this boundary; login reconstructs missed credit from rewarded quests.
pub(crate) fn record_quest_reward(
    world: &mut World,
    character: u64,
    quest: u32,
    daily: bool,
    money: i32,
) {
    let owner = world
        .query_filtered::<(Entity, &PlayerLink, &InWorld), With<Authenticated>>()
        .iter(world)
        .find_map(|(connection, link, in_world)| {
            (in_world.character_id == character).then_some((connection, link.0))
        });
    let Some((connection, player)) = owner else {
        return;
    };
    let mut events = vec![CriteriaEvent::CompleteQuest(quest)];
    if daily {
        events.push(CriteriaEvent::CompleteDailyQuest);
    }
    // TC Player.cpp:15211-15216 and 15274-15276.
    if money > 0 {
        events.push(CriteriaEvent::MoneyEarnedFromQuesting(money as u64));
    }
    let sort = world
        .get_resource::<crate::quest_data::QuestDb>()
        .and_then(|quests| quests.quests.get(&quest))
        .map_or(0, |quest| quest.sort_id);
    if sort > 0 {
        events.push(CriteriaEvent::CompleteQuestInZone(sort as u32));
    }
    if let Some(db) = world.get_resource::<GameDatabase>() {
        let rewarded = persistence::quest_store::load_quest_log(&db.db, character).rewarded;
        events.extend(rewarded_quest_facts(world, &rewarded, false));
    }
    for &event in &events {
        scenario::record(world, player, event);
        crate::adventure_guide::record(world, player, event);
    }
    if let Some((snapshot, toasts)) = update_player_events(world, player, &events, None) {
        send_updates(world, connection, snapshot, toasts);
    }
}

/// TC Player::setDeathState and Unit::Kill: only an accepted native death transition.
pub(crate) fn record_player_death(world: &mut World, player: Entity) {
    let Some(map) = world.get::<InMap>(player).copied() else {
        return;
    };
    let mut events = vec![
        CriteriaEvent::DieAnywhere,
        CriteriaEvent::DieOnMap(map.map_id),
    ];
    if let Some(capacity) = dungeon_copy_capacity(world, map) {
        events.push(CriteriaEvent::DieInInstance(capacity));
    }
    record_criteria_events(world, player, &events);
}

/// TC CriteriaHandler1481–1493: IsDungeon and the actual InstanceMap::GetMaxPlayers.
fn dungeon_copy_capacity(world: &World, map: InMap) -> Option<u32> {
    let entry = world
        .get_resource::<crate::maps::MapStore>()?
        .map(map.map_id)?;
    if !entry.is_dungeon() {
        return None;
    }
    world
        .get_resource::<crate::maps::InstanceManager>()?
        .copy_capacity(map)
}

/// TC Player::EnvironmentalDamage credits the actual lethal environmental type.
pub(crate) fn record_environmental_death(world: &mut World, player: Entity, kind: u32) {
    record_criteria_event(
        world,
        player,
        CriteriaEvent::DieFromEnvironmentalDamage(kind),
    );
}

/// TC Player::HandleFall27461–27509. Called only after an actual damaging landing.
/// Do not infer eligibility from the replicated Environmental(Falling) label.
pub(crate) fn record_survived_fall(world: &mut World, player: Entity, distance: f32) {
    if !survived_fall_qualifies(world, player, distance) {
        return;
    }
    record_criteria_event(
        world,
        player,
        CriteriaEvent::FallWithoutDying((distance * 100.0) as u32),
    );
}

fn survived_fall_qualifies(world: &World, player: Entity, distance: f32) -> bool {
    const MIN_FALL_YARDS: f32 = 14.57;
    const PHYSICAL_SCHOOL: u32 = 1;
    const HOVER_AURA: u32 = 106;
    const SAFE_FALL_AURA: u32 = 144;
    let distance_valid = distance.is_finite() && distance >= MIN_FALL_YARDS;
    let alive = player_conditions(world, player)
        .player
        .is_some_and(|player| player.alive == Some(true));
    let immune = crate::auras::is_immune(world, player, PHYSICAL_SCHOOL);
    if !distance_valid || !alive || immune {
        return false;
    }
    let auras = world.get::<crate::auras::ActiveAuras>(player);
    if auras.is_some_and(|auras| {
        auras.has_type(crate::auras::handlers::FEATHER_FALL)
            || auras.has_type(crate::auras::handlers::FLY)
            || auras.has_type(HOVER_AURA)
    }) {
        return false;
    }
    let safe_fall = auras.map_or(0.0, |auras| auras.modifier(SAFE_FALL_AURA));
    0.018 * (distance - safe_fall) - 0.2426 > 0.0
}

/// TC Unit::Kill11453: direct player or genuine HunterPet owner, not all tap recipients.
pub(crate) fn record_delivered_killing_blow(world: &mut World, killer: Entity) {
    let player = world
        .get::<crate::hunter_pet::HunterPet>(killer)
        .map_or(killer, |pet| pet.owner);
    let in_world = world
        .query_filtered::<(&PlayerLink, &InWorld), With<Authenticated>>()
        .iter(world)
        .any(|(link, _)| link.0 == player);
    if in_world && world.get::<Player>(player).is_some() {
        record_criteria_event(world, player, CriteriaEvent::DeliveredKillingBlow);
    }
}

/// TC Unit::Kill uses the actual lethal unit, not a previous nonlethal LastDamager.
pub(crate) fn record_player_killed_by(world: &mut World, player: Entity, killer: Entity) {
    if world.get::<Player>(player).is_none() {
        return;
    }
    let event = if let Some(npc) = world.get::<Npc>(killer) {
        CriteriaEvent::KilledByCreature(npc.template_id)
    } else if killer != player && world.get::<Player>(killer).is_some() {
        CriteriaEvent::KilledByPlayer
    } else {
        return;
    };
    record_criteria_event(world, player, event);
}

/// Pinned CriteriaType245/246 and Retail CraftingOrderType enum: completed delivery only.
pub(crate) fn record_order_fulfillment(world: &mut World, player: Entity, kind: u8) {
    record_criteria_events(
        world,
        player,
        &[
            CriteriaEvent::FulfillAnyCraftingOrder,
            CriteriaEvent::FulfillCraftingOrderType(kind),
        ],
    );
}

/// Committed credit for the in-world player of `character`; offline characters get none.
pub(crate) fn record_character_event(world: &mut World, character: u64, event: CriteriaEvent) {
    let player = world
        .query_filtered::<(&PlayerLink, &InWorld), With<Authenticated>>()
        .iter(world)
        .find_map(|(link, in_world)| (in_world.character_id == character).then_some(link.0));
    if let Some(player) = player {
        record_criteria_event(world, player, event);
    }
}

pub(crate) fn record_toy_ownership(world: &mut World, player: Entity, account: u64) {
    let toys = crate::persistence::toy_store::load(&world.resource::<GameDatabase>().db, account);
    let events: Vec<_> = toys
        .keys()
        .copied()
        .map(CriteriaEvent::LearnToy)
        .chain(std::iter::once(CriteriaEvent::OwnToys(
            u32::try_from(toys.len()).expect("toy count fits u32"),
        )))
        .collect();
    record_criteria_events(world, player, &events);
}

/// Actual XP-driven level transitions of existing owned pets, never seeded journal levels.
pub(crate) fn record_pet_level_changes(
    world: &mut World,
    player: Entity,
    before: &shared::pet_battle::PetJournal,
    after: &shared::pet_battle::PetJournal,
) {
    let catalog = world.resource::<crate::battle_pets::PetCatalog>();
    let events: Vec<_> = after
        .pets
        .iter()
        .filter_map(|pet| {
            let old = before.get(pet.id)?;
            let creature = catalog.creature_id(pet.species_id)?;
            (creature != 0 && pet.level > old.level).then_some((
                pet.species_id,
                old.level,
                pet.level,
            ))
        })
        .flat_map(|(species, old, new)| {
            ((old + 1)..=new).flat_map(move |level| {
                [
                    CriteriaEvent::BattlePetReachedLevel { species, level },
                    CriteriaEvent::ActivelyEarnedPetLevel { species, level },
                ]
            })
        })
        .collect();
    record_criteria_events(world, player, &events);
}

/// Retail CriteriaType157 AccountObtainPetThroughBattle: committed trap reward, not request.
pub(crate) fn record_pet_capture(world: &mut World, player: Entity) {
    record_criteria_event(world, player, CriteriaEvent::CaptureBattlePet);
}

/// TC BattlegroundScore::UpdatePvpStat:80 TrackedWorldStateUIModified, keyed by PvpStat,
/// completed in the battleground's map.
pub(crate) fn record_pvp_stat(world: &mut World, player: Entity, map: u32, stat: u32) {
    let events = [CriteriaEvent::TrackedWorldStateUIModified(stat)];
    if let Some((snapshot, toasts)) = update_player_events(world, player, &events, Some(map))
        && let Some(connection) = connection_for_player(world, player)
    {
        send_updates(world, connection, snapshot, toasts);
    }
}

/// TC Battleground::HandleKillPlayer: one native reward in this match's map.
pub(crate) fn record_battleground_kill(world: &mut World, player: Entity, map: u32) {
    let event = CriteriaEvent::KillPlayer;
    scenario::record(world, player, event);
    crate::adventure_guide::record(world, player, event);
    if let Some((snapshot, toasts)) = update_player_events(world, player, &[event], Some(map))
        && let Some(connection) = connection_for_player(world, player)
    {
        send_updates(world, connection, snapshot, toasts);
    }
}

/// TC BattlePetMgr::AddPet:483 after a new GUID is saved; never journal loads or changes.
pub(crate) fn record_learned_pet(world: &mut World, player: Entity, species: u32) {
    record_criteria_event(world, player, CriteriaEvent::LearnedNewPet(species));
}

/// TC Spell.cpp3543-3546 credits the casting Retail item after successful cast checks.
pub(crate) fn record_item_use(world: &mut World, player: Entity, item: u32) {
    record_criteria_event(world, player, CriteriaEvent::UseItem(item));
}

/// TC GameObject::Use credits a validated successful use by authored entry, not entity ID.
pub(crate) fn record_game_object_use(world: &mut World, player: Entity, entry: u32) {
    record_criteria_event(world, player, CriteriaEvent::UseGameObject(entry));
}

/// TC Unit::_ApplyAura credits the recipient when a new application is installed.
pub(crate) fn record_gain_aura(world: &mut World, target: Entity, spell: u32) {
    if world.get::<Player>(target).is_some() {
        record_criteria_event(world, target, CriteriaEvent::GainAura(spell));
    }
}

/// TC GameEventSender.cpp:43-58 credits the player source, never an NPC/target.
pub(crate) fn record_player_game_event(world: &mut World, source: Entity, event: u32) {
    if event != 0 && world.get::<Player>(source).is_some() {
        record_criteria_event(world, source, CriteriaEvent::TriggerGameEvent(event));
    }
}

/// TC a352b1fa Spell.cpp:3541-3550: successful regular and triggered casts.
pub(crate) fn record_spell_cast(world: &mut World, caster: Entity, spell: u32) {
    record_criteria_event(world, caster, CriteriaEvent::CastSpell(spell));
}

/// TC a352b1fa Spell::PreprocessSpellHit credits the recipient28 and caster110.
pub(crate) fn record_targeted_spell_hit(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: u32,
) {
    record_criteria_event(world, target, CriteriaEvent::BeSpellTarget(spell));
    let target_player = world
        .get::<Player>(target)
        .map(|player| catalog::SpellHitPlayer {
            race: player.race,
            class: player.class,
        });
    // TC CriteriaHandler1998–2001 reads the hit unit before spell effects.
    let target_health_percent = world
        .get::<shared::components::Health>(target)
        .filter(|health| health.max.is_finite() && health.max > 0.0)
        .map(|health| health.current / health.max * 100.0)
        .filter(|percent| percent.is_finite() && *percent >= 0.0);
    record_criteria_event(
        world,
        caster,
        CriteriaEvent::LandTargetedSpell(spell, target_player, target_health_percent, Some(target)),
    );
}

/// Called only after durable M+ settlement; players must belong to the finished copy.
pub(crate) fn record_mythic_completion(world: &mut World, run: &ChallengeRun) {
    if run.phase != ChallengePhase::Completed {
        return;
    }
    for (player, player_level) in find_mythic_finishers(world, run) {
        let completion = catalog::MythicCompletion {
            key_level: run.key.level,
            challenge_map: run.key.challenge_id,
            player_level,
            in_time: run.elapsed_ms <= run.time_limit_ms,
        };
        let event = CriteriaEvent::CompleteMythicPlus(completion);
        scenario::record(world, player, event);
        if let Some((snapshot, toasts)) =
            update_player_events(world, player, &[event], Some(run.map_id))
            && let Some(connection) = connection_for_player(world, player)
        {
            send_updates(world, connection, snapshot, toasts);
        }
    }
}

fn find_mythic_finishers(world: &mut World, run: &ChallengeRun) -> Vec<(Entity, u8)> {
    world
        .query_filtered::<(&PlayerLink, &InWorld), With<Authenticated>>()
        .iter(world)
        .filter_map(|(link, _)| {
            let map = world.get::<InMap>(link.0)?;
            let level = world.get::<Level>(link.0)?;
            let finished_copy = map.map_id == run.map_id && map.instance_id == run.instance_id;
            finished_copy.then_some((link.0, level.0))
        })
        .collect()
}

/// TC Battleground::EndBattleground credits winners, then every participant, with MapID.
pub(crate) fn record_battleground_end(world: &mut World, player: Entity, map: u32, won: bool) {
    let win = [
        CriteriaEvent::WinBattleground(map),
        CriteriaEvent::WinAnyBattleground,
    ];
    let mut events = if won { win.to_vec() } else { Vec::new() };
    events.push(CriteriaEvent::ParticipateInBattleground(map));
    if let Some((snapshot, toasts)) = update_player_events(world, player, &events, Some(map))
        && let Some(connection) = connection_for_player(world, player)
    {
        send_updates(world, connection, snapshot, toasts);
    }
}

/// TC Scenario.cpp:88-96 CompleteScenario credits every player in the finished copy once.
fn record_scenario_completion(world: &mut World, map: InMap, scenario: u32) {
    let events = [
        CriteriaEvent::CompleteAnyScenario,
        CriteriaEvent::CompleteScenario(scenario),
    ];
    for player in crate::scripted_ai::instance_script::players_in_map(world, map) {
        for &event in &events {
            crate::adventure_guide::record(world, player, event);
        }
        if let Some((snapshot, toasts)) =
            update_player_events(world, player, &events, Some(map.map_id))
            && let Some(connection) = connection_for_player(world, player)
        {
            send_updates(world, connection, snapshot, toasts);
        }
    }
}

/// Terminal authoritative winner only; conditional PvP/streak rows remain unevaluated.
pub(crate) fn record_pet_battle_win(world: &mut World, player: Entity) {
    record_criteria_event(world, player, CriteriaEvent::WinPetBattle);
}

/// TC Arena.cpp241–250: online rated participants/winners, after committed settlement.
pub(crate) fn record_ranked_arena_result(
    world: &mut World,
    player: Entity,
    before_rating: u32,
    after_rating: u32,
    map: u32,
    won: bool,
) {
    let connection = world
        .query::<(Entity, &Authenticated, &PlayerLink, &InWorld)>()
        .iter(world)
        .find_map(|(connection, _, link, _)| (link.0 == player).then_some(connection));
    let Some(connection) = connection else { return };
    let mut events = vec![CriteriaEvent::ParticipateInArena(map)];
    events.extend(personal_arena_rating_event(world, player, after_rating));
    if won {
        events.extend([
            CriteriaEvent::WinAnyRankedArena(before_rating.max(1)),
            CriteriaEvent::WinArena(map),
        ]);
    }
    for &event in &events {
        scenario::record(world, player, event);
        crate::adventure_guide::record(world, player, event);
    }
    if let Some((snapshot, toasts)) = update_player_events(world, player, &events, Some(map)) {
        send_updates(world, connection, snapshot, toasts);
    }
}

/// Current data's0/1 refer to2v2/3v3; read the actual accepted rated copy.
fn personal_arena_rating_event(
    world: &World,
    player: Entity,
    rating: u32,
) -> Option<CriteriaEvent> {
    let bracket = match crate::pvp::player_arena_conditions(world, player) {
        Some((2, true)) => 0,
        Some((3, true)) => 1,
        _ => return None,
    };
    Some(CriteriaEvent::PersonalArenaRating { bracket, rating })
}

pub(crate) fn record_player_event(world: &mut World, player: Entity, event: CriteriaEvent) {
    record_criteria_event(world, player, event);
}

fn record_criteria_event(world: &mut World, player: Entity, event: CriteriaEvent) {
    record_criteria_events(world, player, &[event]);
}

fn record_criteria_events(world: &mut World, player: Entity, events: &[CriteriaEvent]) {
    for &event in events {
        scenario::record(world, player, event);
        crate::adventure_guide::record(world, player, event);
    }
    let Some(catalog) = world.get_resource::<AchievementCatalog>() else {
        return;
    };
    let tracked: Vec<_> = events
        .iter()
        .copied()
        .filter(|event| catalog.tracks(*event))
        .collect();
    if tracked.is_empty() {
        return;
    }
    if let Some((snapshot, toasts)) = update_player_events(world, player, &tracked, None)
        && let Some(connection) = connection_for_player(world, player)
    {
        send_updates(world, connection, snapshot, toasts);
    }
}

fn process_inventory_acquisition_credits(world: &mut World) {
    let (acquired, looted, money, criteria) = {
        let mut updates = world.resource_mut::<crate::gold::GoldUpdateQueue>();
        (
            updates.take_acquisitions(),
            updates.take_looted_items(),
            updates.take_looted_money(),
            updates.take_criteria(),
        )
    };
    let owners: BTreeMap<_, _> = world
        .query_filtered::<(&PlayerLink, &InWorld), With<Authenticated>>()
        .iter(world)
        .map(|(link, owner)| (owner.character_id, link.0))
        .collect();
    for (character, item, count) in acquired {
        if let Some(&player) = owners.get(&character) {
            record_criteria_events(
                world,
                player,
                &[
                    CriteriaEvent::AcquireItem(item, count),
                    CriteriaEvent::ObtainAnyItem(item),
                ],
            );
        }
    }
    for (character, item, count, kind) in looted {
        if let Some(&player) = owners.get(&character) {
            // TC Player::StoreLootItem (Player.cpp:27202-27203).
            record_criteria_events(
                world,
                player,
                &[
                    CriteriaEvent::LootItem(item, count),
                    CriteriaEvent::GetLootByType(kind, count),
                    CriteriaEvent::LootAnyItem(item),
                ],
            );
        }
    }
    for (character, copper) in money {
        if let Some(&player) = owners.get(&character) {
            record_criteria_event(world, player, CriteriaEvent::LootMoney(copper));
        }
    }
    for (character, event) in criteria {
        if let Some(&player) = owners.get(&character) {
            record_criteria_event(world, player, event);
        }
    }
}

fn process_committed_criteria_credits(
    mut credits: MessageReader<CriteriaCredit>,
    mut commands: Commands,
) {
    for credit in credits.read().copied() {
        commands.queue(move |world: &mut World| {
            record_criteria_event(world, credit.player, credit.event);
        });
    }
}

fn connection_for_player(world: &mut World, player: Entity) -> Option<Entity> {
    world
        .query_filtered::<(Entity, &PlayerLink), With<Authenticated>>()
        .iter(world)
        .find_map(|(connection, link)| (link.0 == player).then_some(connection))
}

/// TC Map::GetDifficultyID of a map: its copy's difficulty, else 0.
fn map_difficulty(world: &World, map: Option<InMap>) -> u32 {
    let instances = world.get_resource::<crate::maps::InstanceManager>();
    map.and_then(|map| instances?.copy_difficulty(map))
        .map_or(0, |difficulty| difficulty.difficulty_id)
}

/// The reference player's ModifierTree facts at this criteria update.
pub(crate) fn player_conditions(world: &World, player: Entity) -> PlayerConditions {
    let map = world.get::<InMap>(player).copied();
    PlayerConditions {
        difficulty: map_difficulty(world, map),
        player: reference_player(world, player, map),
        ..Default::default()
    }
}

/// Read actual hit-unit aura authority while live events are consumed before effects.
/// Hit identities never enter persisted counters or login reconstruction.
pub(crate) fn event_conditions(
    world: &World,
    player: Entity,
    events: &[CriteriaEvent],
) -> PlayerConditions {
    let mut conditions = player_conditions(world, player);
    if let Some(reference) = conditions.player.as_mut() {
        // Login74 already rebuilds these facts from this character's persisted rewards.
        reference.rewarded_quests = events
            .iter()
            .filter_map(|event| match event {
                CriteriaEvent::CompleteQuest(quest) => Some(*quest),
                _ => None,
            })
            .collect();
    }
    for event in events {
        match event {
            CriteriaEvent::LandTargetedSpell(_, _, _, Some(target)) => {
                conditions
                    .hit_units
                    .insert(*target, read_spell_hit_unit(world, *target));
            }
            CriteriaEvent::DoEmote(_, Some(target)) => {
                if let Some(unit) = read_emote_target(world, player, *target) {
                    conditions.hit_units.insert(*target, unit);
                }
            }
            _ => {}
        }
    }
    conditions
}

fn read_spell_hit_unit(world: &World, target: Entity) -> catalog::CriteriaTargetUnit {
    catalog::CriteriaTargetUnit {
        creature_entry: world
            .get::<shared::components::Npc>(target)
            .map(|npc| npc.template_id),
        auras: world
            .get::<crate::auras::ActiveAuras>(target)
            .map(|auras| auras.iter().map(|aura| aura.spell_id).collect()),
        ..Default::default()
    }
}

/// TC GetUnit resolves the requested entity on the reference player's exact map copy.
fn read_emote_target(
    world: &World,
    player: Entity,
    target: Entity,
) -> Option<catalog::CriteriaTargetUnit> {
    let map = world.get::<InMap>(player)?;
    if world.get::<InMap>(target) != Some(map) {
        return None;
    }
    let is_player = world.get::<Player>(target).is_some();
    let npc = world.get::<shared::components::Npc>(target);
    if !is_player && npc.is_none() {
        return None;
    }
    let hostile = emote_target_hostility(world, player, target);
    Some(catalog::CriteriaTargetUnit {
        creature_entry: npc.map(|npc| npc.template_id),
        is_player,
        dead: world
            .get::<shared::components::Health>(target)
            .map(|health| health.current <= 0.0),
        hostile,
        ..Default::default()
    })
}

/// Missing faction authority is unknown, not a hostile opponent inferred from race.
fn emote_target_hostility(world: &World, player: Entity, target: Entity) -> Option<bool> {
    let templates = world.get_resource::<crate::faction::FactionTemplates>()?;
    let actor = world.get::<shared::components::UnitFactionTemplate>(player)?;
    let target_faction = world.get::<shared::components::UnitFactionTemplate>(target)?;
    templates.0.get(&actor.0)?;
    templates.0.get(&target_faction.0)?;
    Some(crate::spell_cast::targets::is_hostile(
        world, player, target,
    ))
}

/// Same native learned spell sources used by type34, never SpellStore membership.
fn known_player_spell_ids(world: &World, player: Entity) -> BTreeSet<u32> {
    let class_spells = world
        .get::<crate::class_progression::KnownSpells>(player)
        .into_iter()
        .flat_map(|known| known.iter());
    let profession_spells = world
        .get::<crate::profession::PlayerProfessions>(player)
        .into_iter()
        .flat_map(|known| known.spells.iter().copied());
    let mounts = world.get::<crate::mount_journal::AccountMounts>(player);
    class_spells
        .chain(profession_spells)
        .chain(crate::mount_journal::owned_spells(world, mounts))
        .collect()
}

fn spell_knowledge_complete(world: &World, player: Entity) -> bool {
    let class_known = world
        .get::<crate::class_progression::KnownSpells>(player)
        .is_some();
    let professions_known = world
        .get::<crate::profession::PlayerProfessions>(player)
        .is_some();
    let mounts_known = world
        .get::<crate::mount_journal::AccountMounts>(player)
        .zip(world.get_resource::<crate::mount_journal::MountCatalog>())
        .is_some_and(|(mounts, catalog)| {
            mounts.0.keys().all(|id| catalog.entries.contains_key(id))
        });
    class_known && professions_known && mounts_known
}

/// TC BattlePetTeamLevel1935–1939 uses the equipped journal slots, not every owned pet.
fn battle_pet_team_min_level(world: &World, player: Entity) -> Option<u8> {
    let journal = &world
        .get::<crate::battle_pets::AccountPetJournal>(player)?
        .journal;
    journal
        .battle_slots
        .iter()
        .try_fold(shared::pet_battle::MAX_PET_LEVEL, |minimum, guid| {
            let pet = journal.get((*guid)?)?;
            Some(minimum.min(pet.level))
        })
}

fn earned_player_achievement_ids(world: &World, player: Entity) -> Option<BTreeSet<u32>> {
    let (state, catalog) = world
        .get::<PlayerAchievementState>(player)
        .zip(world.get_resource::<AchievementCatalog>())?;
    Some(catalog.iter_earned_achievement_ids(state).collect())
}

fn reference_player(world: &World, player: Entity, map: Option<InMap>) -> Option<ReferencePlayer> {
    let identity = world.get::<Player>(player)?;
    let arena = crate::pvp::player_arena_conditions(world, player);
    Some(ReferencePlayer {
        auras: world
            .get::<crate::auras::ActiveAuras>(player)
            .into_iter()
            .flat_map(|auras| auras.iter().map(|aura| aura.spell_id))
            .collect(),
        auras_known: world.get::<crate::auras::ActiveAuras>(player).is_some(),
        known_spells: known_player_spell_ids(world, player),
        spell_knowledge_complete: spell_knowledge_complete(world, player),
        rewarded_quests: BTreeSet::new(),
        earned_achievements: earned_player_achievement_ids(world, player),
        account_achievements_known: world
            .get::<PlayerAchievementState>(player)
            .is_some_and(|state| state.account.is_some()),
        battle_pet_team_min_level: battle_pet_team_min_level(world, player),
        scenario_step: map.and_then(|map| scenario::current_step_id(world, map)),
        area: world
            .get::<AchievementTracker>(player)
            .and_then(|tracker| tracker.last_processed_area),
        map: map?.map_id,
        race: identity.race,
        class: identity.class,
        level: world.get::<Level>(player)?.0,
        arena_team_size: arena.map(|(size, _)| size),
        in_ranked_arena: arena.is_some_and(|(_, ranked)| ranked),
        controlled_node_counts: crate::pvp::player_controlled_node_counts(world, player),
        alive: world
            .get::<shared::components::Health>(player)
            .map(|health| {
                health.current > 0.0
                    && world
                        .get::<crate::death::PlayerDeathState>(player)
                        .is_none_or(|state| state.state == shared::death::DeathState::Alive)
            }),
        group_size: world
            .get_resource::<crate::group::GroupRegistry>()
            .zip(world.get::<crate::persistence::ClientId>(player))
            .map(|(groups, character)| groups.group_members(character.0).len()),
    })
}

/// Map-wide credit has no reference player: only the copy's difficulty is known,
/// so player-fact criteria cannot hold.
pub(crate) fn map_conditions(world: &World, map: InMap) -> PlayerConditions {
    PlayerConditions {
        difficulty: map_difficulty(world, Some(map)),
        player: None,
        ..Default::default()
    }
}

fn update_player_events(
    world: &mut World,
    player: Entity,
    events: &[CriteriaEvent],
    completed_map: Option<u32>,
) -> Option<(AchievementSnapshot, Vec<AchievementToastSnapshot>)> {
    if !world.contains_resource::<AchievementCatalog>() {
        return None;
    }
    let race = world.get::<Player>(player)?.race;
    let conditions = event_conditions(world, player, events);
    let now = world.resource::<crate::maps::ServerClock>().unix();
    let owner = world
        .query::<(&Authenticated, &PlayerLink, &InWorld)>()
        .iter(world)
        .find_map(|(auth, link, in_world)| {
            (link.0 == player).then_some((auth.account_id, in_world.character_id))
        });
    world.resource_scope(|world, catalog: Mut<AchievementCatalog>| {
        let mut state = world.get::<PlayerAchievementState>(player)?.clone();
        let mut toasts: Vec<_> = events
            .iter()
            .flat_map(|event| {
                catalog.update_at(&mut state, *event, completed_map, race, &conditions)
            })
            .collect();
        for toast in &toasts {
            state.earned_at.insert(toast.achievement_id, now);
        }
        if let Some((account, character)) = owner
            && let Some(db) = world.get_resource::<GameDatabase>()
        {
            toasts.extend(account_store::sync_account_progress(
                &db.db,
                &mut state,
                &catalog,
                account_store::AccountUpdate {
                    account,
                    character,
                    completed_map,
                    race,
                    now,
                },
            ));
        }
        if !toasts.is_empty()
            && let Some((_, character)) = owner
        {
            world.init_resource::<crate::gold::GoldUpdateQueue>();
            world
                .resource_mut::<crate::gold::GoldUpdateQueue>()
                .record_news(character);
        }
        let snapshot = catalog.snapshot(&state, race);
        world.entity_mut(player).insert(state);
        Some((snapshot, toasts))
    })
}

pub(crate) fn save_monthly_contribution(
    db: &redb::Database,
    account: u64,
    character: u64,
    state: &PlayerAchievementState,
) {
    account_store::save_monthly_contribution(db, account, character, state);
}
pub(crate) fn monthly_contributions(
    db: &redb::Database,
    account: u64,
) -> Vec<PlayerAchievementState> {
    account_store::monthly_contributions(db, account)
}

fn send_updates(
    world: &mut World,
    connection: Entity,
    snapshot: AchievementSnapshot,
    toasts: Vec<AchievementToastSnapshot>,
) {
    crate::adventure_guide::refresh_for_connection(world, connection);
    let Some(mut sender) = world.get_mut::<MessageSender<AchievementStateUpdate>>(connection)
    else {
        return;
    };
    if toasts.is_empty() {
        sender.send::<AchievementChannel>(AchievementStateUpdate {
            snapshot: Some(snapshot),
            completed: None,
            message: Some("achievement progress updated".into()),
            error: None,
        });
        return;
    }
    for toast in toasts {
        sender.send::<AchievementChannel>(AchievementStateUpdate {
            snapshot: Some(snapshot.clone()),
            completed: Some(toast),
            message: Some("achievement completed".into()),
            error: None,
        });
    }
}

/// TC Unit.cpp977-1001/1078: effective totals, raw peaks, no self-hit totals.
pub(crate) fn record_damage(
    world: &mut World,
    attacker: Entity,
    victim: Entity,
    health_before: f32,
    damage: f32,
    loss: f32,
) {
    if attacker != victim && world.get::<Player>(attacker).is_some() {
        record_criteria_events(
            world,
            attacker,
            &[
                CriteriaEvent::DamageDealt(health_before.min(damage) as u64),
                CriteriaEvent::HighestDamageDone(damage as u64),
            ],
        );
    }
    if world.get::<Player>(victim).is_some() {
        record_criteria_event(
            world,
            victim,
            CriteriaEvent::HighestDamageTaken(loss as u64),
        );
        if attacker != victim {
            record_criteria_event(
                world,
                victim,
                CriteriaEvent::TotalDamageTaken(health_before.min(loss) as u64),
            );
        }
    }
}

/// TC Unit.cpp6572-6583: totals exclude overheal; peaks retain the full heal.
pub(crate) fn record_healing(
    world: &mut World,
    caster: Entity,
    target: Entity,
    amount: f32,
    effective: f32,
) {
    if world.get::<Player>(caster).is_some() {
        record_criteria_events(
            world,
            caster,
            &[
                CriteriaEvent::HealingDone(effective as u64),
                CriteriaEvent::HighestHealCast(amount as u64),
            ],
        );
    }
    if world.get::<Player>(target).is_some() {
        record_criteria_events(
            world,
            target,
            &[
                CriteriaEvent::TotalHealReceived(effective as u64),
                CriteriaEvent::HighestHealReceived(amount as u64),
            ],
        );
    }
}

/// TC Player::SummonIfPossible26094 credits a valid acceptance before TeleportTo.
pub(crate) fn record_accepted_summon(world: &mut World, player: Entity) {
    record_criteria_event(world, player, CriteriaEvent::AcceptSummon);
}

/// TC InstanceScript435–441/Player21915–21925: read active loot locks before DONE saves them.
pub(crate) fn record_encounter_loot_defeat(
    world: &mut World,
    map: InMap,
    encounter: u32,
    difficulty: u32,
) {
    let eligible = encounter_loot_eligible_players(world, map, encounter, difficulty);
    for player in eligible {
        record_criteria_event(
            world,
            player,
            CriteriaEvent::DefeatEncounterEligibleForLoot(encounter),
        );
    }
}

fn encounter_loot_eligible_players(
    world: &mut World,
    map: InMap,
    encounter: u32,
    difficulty: u32,
) -> Vec<Entity> {
    let players = authenticated_encounter_players(world, map);
    let Some(instances) = world.get_resource::<crate::maps::InstanceManager>() else {
        return Vec::new();
    };
    let Some(copy) = instances
        .copy_difficulty(map)
        .filter(|copy| copy.difficulty_id == difficulty)
    else {
        return Vec::new();
    };
    let Some(row) = world
        .get_resource::<crate::maps::MapStore>()
        .and_then(|store| store.dungeon_encounter(encounter, map.map_id, difficulty))
    else {
        return Vec::new();
    };
    let now = crate::maps::difficulty::now(world).unix;
    players
        .into_iter()
        .filter_map(|(player, character)| {
            let locked = instances
                .locks
                .find_active_lock(character, copy, now)
                .is_some_and(|lock| lock.data.completed_encounters & (1 << row.bit) != 0);
            (!locked).then_some(player)
        })
        .collect()
}

fn authenticated_encounter_players(world: &mut World, map: InMap) -> Vec<(Entity, u64)> {
    world
        .query::<(&Authenticated, &InWorld, &PlayerLink)>()
        .iter(world)
        .filter_map(|(_, owner, link)| {
            let player = link.0;
            if world.get::<Player>(player).is_none() || world.get::<InMap>(player) != Some(&map) {
                return None;
            }
            let character = world.get::<crate::persistence::ClientId>(player)?.0;
            (character == owner.character_id).then_some((player, character))
        })
        .collect()
}

/// Successful scripted encounter boundary. A scripted defeat (Hogger's surrender
/// included) credits its creature criteria even when no corpse is produced.
pub(crate) fn record_encounter_defeat(
    world: &mut World,
    map: InMap,
    encounter_id: u32,
    difficulty: u32,
) {
    let Some(store) = world.get_resource::<crate::maps::MapStore>() else {
        return;
    };
    if store
        .dungeon_encounter(encounter_id, map.map_id, difficulty)
        .is_none()
    {
        return;
    }
    dungeon::send_map_progress(world, map);
    let store = world.resource::<crate::maps::MapStore>();
    let encounters = store.encounters_for_map(map.map_id, difficulty);
    let mask = world
        .get_resource::<crate::maps::InstanceManager>()
        .map_or(0, |instances| instances.completed_encounters(map));
    let complete = !encounters.is_empty()
        && encounters
            .iter()
            .all(|encounter| mask & (1 << encounter.bit) != 0);
    let creature = world
        .get_resource::<crate::scripted_ai::instance_script::InstanceScripts>()
        .and_then(|scripts| scripts.0.get(&map))
        .and_then(|script| {
            script.boss_creatures.iter().find_map(|&(entry, boss_id)| {
                (script.bosses[boss_id as usize].encounter_id == encounter_id).then_some(entry)
            })
        });
    let mut events = vec![CriteriaEvent::DefeatEncounter(encounter_id)];
    if let Some(creature) = creature {
        events.push(CriteriaEvent::KillCreature(creature));
    }
    let conditions = map_conditions(world, map);
    for &event in &events {
        scenario::record_map(world, map, event, &conditions);
    }
    for player in crate::scripted_ai::instance_script::players_in_map(world, map) {
        for &event in &events {
            crate::adventure_guide::record(world, player, event);
        }
        let Some((snapshot, toasts)) =
            update_player_events(world, player, &events, complete.then_some(map.map_id))
        else {
            continue;
        };
        if let Some(connection) = connection_for_player(world, player) {
            send_updates(world, connection, snapshot, toasts);
        }
    }
}

/// Ordinary NPC kills use the actual player killer. Scripted boss deaths are
/// already credited by SetBossState(DONE), so are not counted twice.
pub(crate) fn record_creature_death(world: &mut World, creature: Entity, killer: Option<Entity>) {
    let Some(killer) = killer else { return };
    let Some(npc) = world.get::<Npc>(creature) else {
        return;
    };
    let entry = npc.template_id;
    let map = world.get::<InMap>(creature).copied();
    let scripted_boss = map
        .and_then(|map| {
            world
                .get_resource::<crate::scripted_ai::instance_script::InstanceScripts>()?
                .0
                .get(&map)
        })
        .is_some_and(|script| script.boss_creatures.iter().any(|&(boss, _)| boss == entry));
    if scripted_boss {
        return;
    }
    scenario::record(world, killer, CriteriaEvent::KillCreature(entry));
    crate::adventure_guide::record(world, killer, CriteriaEvent::KillCreature(entry));
    let Some((snapshot, toasts)) =
        update_player_events(world, killer, &[CriteriaEvent::KillCreature(entry)], None)
    else {
        return;
    };
    if let Some(connection) = connection_for_player(world, killer) {
        send_updates(world, connection, snapshot, toasts);
    }
}

fn send_requested_catalog_pages(world: &mut World) {
    let requests: Vec<_> = world
        .query_filtered::<(
            Entity,
            &PlayerLink,
            &mut MessageReceiver<QueryAchievementCatalog>,
        ), (With<Authenticated>, With<InWorld>)>()
        .iter_mut(world)
        .flat_map(|(connection, link, mut receiver)| {
            receiver
                .receive()
                .map(|request| (connection, link.0, request))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, player, request) in requests {
        let Some(catalog) = world.get_resource::<AchievementCatalog>() else {
            continue;
        };
        let Some(state) = world.get::<PlayerAchievementState>(player) else {
            continue;
        };
        let Some(owner) = world.get::<Player>(player) else {
            continue;
        };
        let page = match request {
            QueryAchievementCatalog::Comparison {
                target_entity,
                query,
            } => comparison_page(world, player, target_entity, (*query).into()),
            request => catalog.page(state, request, owner.race),
        };
        if let Some(mut sender) = world.get_mut::<MessageSender<AchievementCatalogPage>>(connection)
        {
            sender.send::<AchievementChannel>(page);
        }
    }
}

fn comparison_page(
    world: &mut World,
    viewer: Entity,
    target_entity: u64,
    query: QueryAchievementCatalog,
) -> AchievementCatalogPage {
    let result = query_comparison_target(world, viewer, target_entity, query);
    match result {
        Ok((target_name, page)) => AchievementCatalogPage::Comparison {
            target_entity,
            target_name,
            page: Some(Box::new(page.try_into().expect("ordinary comparison page"))),
            error: None,
        },
        Err(error) => AchievementCatalogPage::Comparison {
            target_entity,
            target_name: String::new(),
            page: None,
            error: Some(error.into()),
        },
    }
}

fn query_comparison_target(
    world: &mut World,
    viewer: Entity,
    target_bits: u64,
    query: QueryAchievementCatalog,
) -> Result<(String, AchievementCatalogPage), &'static str> {
    if matches!(query, QueryAchievementCatalog::Comparison { .. }) {
        return Err("nested achievement comparison is invalid");
    }
    let target = Entity::try_from_bits(target_bits).ok_or("invalid comparison target")?;
    if target == viewer {
        return Err("comparison target must be another player");
    }
    let online = world
        .query::<(&Authenticated, &PlayerLink, &InWorld)>()
        .iter(world)
        .any(|(_, link, _)| link.0 == target);
    if !online {
        return Err("comparison target is not an online player");
    }
    let player = world
        .get::<Player>(target)
        .ok_or("comparison target is not a player")?;
    let state = world
        .get::<PlayerAchievementState>(target)
        .ok_or("comparison progress unavailable")?;
    let catalog = world
        .get_resource::<AchievementCatalog>()
        .ok_or("achievement catalog unavailable")?;
    let name = catalog::bound_text(&player.name, shared::protocol::ACHIEVEMENT_NAME_MAX_BYTES);
    Ok((name, catalog.page(state, query, player.race)))
}

#[cfg(test)]
#[path = "achievement_absent_aura_tests.rs"]
mod absent_aura_tests;
#[cfg(test)]
#[path = "achievement_completed_quest_tests.rs"]
mod completed_quest_tests;
#[cfg(test)]
#[path = "achievement_creature_type_tests.rs"]
mod creature_type_tests;
#[cfg(test)]
#[path = "achievement_nested_all_tests.rs"]
mod nested_all_tests;
#[cfg(test)]
#[path = "achievement_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "achievement_unknown_spell_tests.rs"]
mod unknown_spell_tests;

#[cfg(test)]
#[path = "achievement_send_perf_tests.rs"]
mod send_perf_tests;
