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
pub(crate) use catalog::{AchievementCatalog, CriteriaEvent, load_achievement_catalog};
#[path = "achievement_account.rs"]
mod account_store;
#[path = "dungeon_progress.rs"]
mod dungeon;
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
    /// Presentation-only aggregate; never serialized into character contributions.
    #[serde(skip)]
    pub account: Option<Box<PlayerAchievementState>>,
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
}

/// Committed gameplay credit from systems that do not own an exclusive World.
#[derive(Message, Clone, Copy)]
pub(crate) struct CriteriaCredit {
    pub player: Entity,
    pub event: CriteriaEvent,
}

pub struct AchievementPlugin;

impl Plugin for AchievementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::maps::ServerClock>();
        app.add_message::<CriteriaCredit>();
        app.init_resource::<crate::gold::GoldUpdateQueue>();
        app.init_resource::<scenario::ScenarioRuns>();
        app.add_systems(
            Update,
            (
                send_initial_achievement_snapshot,
                process_level_achievement_updates,
                process_skill_achievement_updates,
                process_known_spell_achievement_updates,
                process_reputation_achievement_updates,
                process_exploration_achievement_updates,
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
        .query_filtered::<(Entity, &PlayerLink, &InWorld), (With<Authenticated>, Added<InWorld>)>()
        .iter(world)
        .map(|(connection, link, in_world)| (connection, link.0, in_world.character_id))
        .collect();
    for (connection, player, character) in connections {
        let Some(level) = world.get::<Level>(player).map(|level| level.0) else {
            continue;
        };
        let events = load_login_events(world, character, level);
        let Some((snapshot, _)) = update_player_events(world, player, &events, None) else {
            continue;
        };
        send_updates(world, connection, snapshot, Vec::new());
        world.entity_mut(player).insert(AchievementTracker {
            last_processed_level: level,
        });
    }
}

fn load_login_events(world: &World, character: u64, level: u8) -> Vec<CriteriaEvent> {
    let mut events = vec![CriteriaEvent::Login, CriteriaEvent::ReachLevel(level)];
    if let Some(db) = world.get_resource::<GameDatabase>() {
        events.extend(
            persistence::quest_store::load_quest_log(&db.db, character)
                .rewarded
                .into_iter()
                .map(CriteriaEvent::CompleteQuest),
        );
    }
    events
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
        world.entity_mut(player).insert(AchievementTracker {
            last_processed_level: level,
        });
        if let Some(connection) = connection_for_player(world, player) {
            send_updates(world, connection, snapshot, toasts);
        }
    }
}

/// TC a352b1fa CriteriaHandler.cpp:601-604 reads GetBaseSkillValue;
/// Changed includes newly loaded skill lines, so login restores historical ranks.
fn process_skill_achievement_updates(world: &mut World) {
    let updates: Vec<_> = world.query_filtered::<(Entity, &crate::profession::PlayerProfessions), Changed<crate::profession::PlayerProfessions>>()
        .iter(world).flat_map(|(player, professions)| professions.lines.iter()
            .filter(|line| line.rank != 0)
            .map(move |line| (player, CriteriaEvent::SkillRank(line.skill_line, line.rank)))).collect();
    for (player, event) in updates {
        record_criteria_event(world, player, event);
    }
}

/// TC CriteriaHandler.cpp687/1530-1535: knowing the authored spell is a boolean fact.
fn process_known_spell_achievement_updates(world: &mut World) {
    let updates: Vec<_> = world.query_filtered::<(Entity, &crate::class_progression::KnownSpells), Changed<crate::class_progression::KnownSpells>>()
        .iter(world).flat_map(|(player, known)| known.iter().map(move |spell| (player, CriteriaEvent::LearnSpell(spell)))).collect();
    for (player, event) in updates {
        record_criteria_event(world, player, event);
    }
}

/// TC CriteriaHandler.cpp709-715 credits positive signed standing, not an unsigned tier offset.
fn process_reputation_achievement_updates(world: &mut World) {
    let updates: Vec<_> = world.query_filtered::<(Entity, &shared::reputation::CharacterReputation), Changed<shared::reputation::CharacterReputation>>()
        .iter(world).flat_map(|(player, reputation)| reputation.values()
            .filter(|(_, value)| *value > 0)
            .map(move |(faction, value)| (player, CriteriaEvent::ReputationGained(faction, value as u32)))).collect();
    for (player, event) in updates {
        record_criteria_event(world, player, event);
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
    let position = *world.get::<shared::components::Position>(player)?;
    let map = *world.get::<InMap>(player)?;
    let area = world
        .resource_mut::<crate::ground::GroundMap>()
        .area_at(map.map_id, Vec3::new(position.x, position.y, position.z))?;
    world.get_resource::<AchievementCatalog>()?.area_bit(area)
}

/// Runs only after the quest reward transaction commits. Failed/incomplete turn-ins
/// never reach this boundary; login reconstructs missed credit from rewarded quests.
pub(crate) fn record_quest_reward(world: &mut World, character: u64, quest: u32) {
    let owner = world
        .query_filtered::<(Entity, &PlayerLink, &InWorld), With<Authenticated>>()
        .iter(world)
        .find_map(|(connection, link, in_world)| {
            (in_world.character_id == character).then_some((connection, link.0))
        });
    let Some((connection, player)) = owner else {
        return;
    };
    scenario::record(world, player, CriteriaEvent::CompleteQuest(quest));
    if let Some((snapshot, toasts)) =
        update_player_events(world, player, &[CriteriaEvent::CompleteQuest(quest)], None)
    {
        send_updates(world, connection, snapshot, toasts);
    }
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

/// TC a352b1fa Spell.cpp:2876-2880 credits the targeted player, not caster.
pub(crate) fn record_spell_target(world: &mut World, target: Entity, spell: u32) {
    record_criteria_event(world, target, CriteriaEvent::BeSpellTarget(spell));
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

/// TC Battleground::EndBattleground credits winning participants with MapID.
pub(crate) fn record_battleground_win(world: &mut World, player: Entity, map: u32) {
    let events = [
        CriteriaEvent::WinBattleground(map),
        CriteriaEvent::WinAnyBattleground,
    ];
    if let Some((snapshot, toasts)) = update_player_events(world, player, &events, Some(map))
        && let Some(connection) = connection_for_player(world, player)
    {
        send_updates(world, connection, snapshot, toasts);
    }
}

/// Terminal authoritative winner only; conditional PvP/streak rows remain unevaluated.
pub(crate) fn record_pet_battle_win(world: &mut World, player: Entity) {
    record_criteria_event(world, player, CriteriaEvent::WinPetBattle);
}

fn record_criteria_event(world: &mut World, player: Entity, event: CriteriaEvent) {
    scenario::record(world, player, event);
    if !world
        .get_resource::<AchievementCatalog>()
        .is_some_and(|catalog| catalog.tracks(event))
    {
        return;
    }
    if let Some((snapshot, toasts)) = update_player_events(world, player, &[event], None)
        && let Some(connection) = connection_for_player(world, player)
    {
        send_updates(world, connection, snapshot, toasts);
    }
}

fn process_inventory_acquisition_credits(world: &mut World) {
    let (acquired, looted) = {
        let mut updates = world.resource_mut::<crate::gold::GoldUpdateQueue>();
        (updates.take_acquisitions(), updates.take_looted_items())
    };
    let owners: BTreeMap<_, _> = world
        .query_filtered::<(&PlayerLink, &InWorld), With<Authenticated>>()
        .iter(world)
        .map(|(link, owner)| (owner.character_id, link.0))
        .collect();
    for (character, item, count) in acquired {
        if let Some(&player) = owners.get(&character) {
            record_criteria_event(world, player, CriteriaEvent::AcquireItem(item, count));
        }
    }
    for (character, item, count) in looted {
        if let Some(&player) = owners.get(&character) {
            record_criteria_event(world, player, CriteriaEvent::LootItem(item, count));
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
            .flat_map(|event| catalog.update(&mut state, *event, completed_map, race))
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
        let snapshot = catalog.snapshot(&state, race);
        world.entity_mut(player).insert(state);
        Some((snapshot, toasts))
    })
}

fn send_updates(
    world: &mut World,
    connection: Entity,
    snapshot: AchievementSnapshot,
    toasts: Vec<AchievementToastSnapshot>,
) {
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
    for &event in &events {
        scenario::record_map(world, map, event);
    }
    for player in crate::scripted_ai::instance_script::players_in_map(world, map) {
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
#[path = "achievement_tests.rs"]
mod tests;
