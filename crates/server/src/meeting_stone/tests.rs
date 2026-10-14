//! Meeting stone summons with real world.db data: the
//! Deadmines-area stone (178834, no ContentTuning), spells 59782 / 7720 and the
//! Meeting Stone Summoning Portal (179944, a two-caster ritual).

use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::*;
use lightyear::prelude::{Connected, PeerId, RemoteId};
use rusqlite::{Connection, OpenFlags};
use shared::casting::{CastState, CastType};
use shared::components::{
    CharacterAppearance, CombatStatus, Health, Level, MovementControl, Player, Position, Rotation,
    UnitFactionTemplate, Zone,
};
use shared::protocol::*;
use shared::spell_data::CastFailReason;

use super::*;
use crate::auth::{Authenticated, InWorld};
use crate::game_objects::{GameObjectSpawn, game_object_bundle, load_game_object_spawns};
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::networking::{CombatTarget, Moving, PlayerLink};
use crate::persistence::ClientId;
use crate::spell_cast::test_harness::{class_spells, power_table, world_db};
use crate::spell_info::SpellStore;

/// Meeting Stone near the Deadmines (Westfall), `ContentTuningId` 0.
const DEADMINES_STONE: u32 = 178_834;
/// Meeting Stone at Blackrock Depths, ContentTuning 212 (levels 15-30).
const LEVELED_STONE: u32 = 178_832;
/// Meeting Stone Summoning Portal, TRANS_DOOR MiscValue of 59782.
const MEETING_STONE_PORTAL: u32 = 179_944;
/// The portal's ritual spell: SUMMON_PLAYER.
const PORTAL_RITUAL_SPELL: u32 = 7720;
const WESTFALL: u32 = 40;
const ELWYNN_FOREST: u32 = 12;

struct Member {
    player: Entity,
    connection: Entity,
}

struct Harness {
    app: App,
    stone: Entity,
    stone_position: Position,
    summoner: Member,
    helper: Member,
    target: Member,
    stranger: Member,
    requests: Vec<SummonRequest>,
    failed: Vec<(Entity, CastFailed)>,
    new_worlds: Vec<(Entity, NewWorld)>,
}

fn pair<M: lightyear::prelude::Message>() -> (MessageSender<M>, MessageReceiver<M>) {
    (MessageSender::default(), MessageReceiver::default())
}

fn stone_spawn(conn: &Connection, entry: u32) -> GameObjectSpawn {
    load_game_object_spawns(conn)
        .unwrap()
        .0
        .into_iter()
        .find(|spawn| spawn.info.entry == entry)
        .expect("meeting stone spawn on map 0")
}

fn harness(stone_entry: u32) -> Option<Harness> {
    let data = world_db()?;
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    app.add_plugins((
        crate::npc_interaction::NpcInteractionPlugin,
        crate::spell_cast::SpellCastPlugin,
        crate::auras::AuraPlugin,
        MeetingStonePlugin,
        crate::maps::MapPlugin,
    ));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    app.insert_resource(crate::faction::world_db_faction_templates()?);
    app.finish();
    app.cleanup();
    app.update();
    let spells = [SUMMONING_STONE_EFFECT, PORTAL_RITUAL_SPELL, 65451]
        .iter()
        .map(|&id| data.store.get(id).expect("spell in world.db").clone())
        .collect();
    app.insert_resource(SpellStore::fixture(spells));
    app.insert_resource(power_table(data));
    app.insert_resource(class_spells(data));
    app.insert_resource(load_meeting_stone_data(&conn).unwrap());
    let maps = crate::maps::store::load_map_store(&conn).unwrap();
    app.insert_resource(crate::ground::GroundMap::new(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../game-engine/data"),
        maps.maps
            .iter()
            .map(|(&id, map)| (id, map.wdt_file_data_id))
            .collect(),
    ));
    app.insert_resource(crate::area_table::load_area_table(&conn).unwrap());
    app.insert_resource(maps);
    let spawn = stone_spawn(&conn, stone_entry);
    let stone = app.world_mut().spawn(game_object_bundle(&spawn)).id();
    let stone_position = *app.world().get::<Position>(stone).unwrap();
    let beside = |dx: f32| Position {
        x: stone_position.x + dx,
        ..stone_position
    };
    // Goldshire, Elwynn Forest.
    let (x, y, z) = crate::terrain_height::wow_to_bevy(-9464.0, 62.0, 56.0);
    let far = Position { x, y, z };
    let summoner = spawn_member(&mut app, 1, "Stonecaller", beside(1.0), WESTFALL);
    let helper = spawn_member(&mut app, 2, "Ritualist", beside(-1.0), WESTFALL);
    let target = spawn_member(&mut app, 3, "Farwalker", far, ELWYNN_FOREST);
    let stranger = spawn_member(&mut app, 4, "Stranger", far, ELWYNN_FOREST);
    app.insert_resource(party(&[
        (1, "Stonecaller", summoner.player),
        (2, "Ritualist", helper.player),
        (3, "Farwalker", target.player),
    ]));
    app.update();
    Some(Harness {
        app,
        stone,
        stone_position,
        summoner,
        helper,
        target,
        stranger,
        requests: Vec::new(),
        failed: Vec::new(),
        new_worlds: Vec::new(),
    })
}

fn spawn_member(app: &mut App, character_id: u64, name: &str, at: Position, zone: u32) -> Member {
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: name.into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            ClientId(character_id),
            at,
            Rotation {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            Level(20),
            UnitFactionTemplate(1),
            Moving(false),
            CombatTarget(None),
            MovementControl::default(),
            Zone { id: zone },
            crate::maps::InMap::continent(0),
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(character_id)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated {
                account_id: character_id,
            },
            InWorld { character_id },
            PlayerLink(player),
            (
                pair::<UseGameObject>(),
                pair::<InteractionOpened>(),
                pair::<InteractionClosed>(),
                pair::<InteractionFailed>(),
                pair::<SummonRequest>(),
                pair::<SummonResponse>(),
                pair::<CastFailed>(),
                pair::<SpellCastIntent>(),
                pair::<StopSpellCast>(),
            ),
            (
                pair::<NewWorld>(),
                pair::<WorldPortAck>(),
                pair::<TransferAborted>(),
            ),
        ))
        .id();
    Member { player, connection }
}

/// One party of the given members, grouped through the registry's own invite flow.
fn party(members: &[(u64, &str, Entity)]) -> GroupRegistry {
    let online: OnlineView = members
        .iter()
        .map(|&(id, name, entity)| {
            (
                id,
                OnlineMember {
                    entity: entity.to_bits(),
                    name: name.into(),
                    class: 1,
                    level: 20,
                    portrait: Default::default(),
                    dungeon_difficulty: crate::maps::store::DIFFICULTY_NORMAL,
                    raid_difficulty: crate::maps::RaidDifficulty::default(),
                },
            )
        })
        .collect();
    let mut registry = GroupRegistry::default();
    registry.sync_presence(&online);
    for &(id, name, _) in &members[1..] {
        registry.invite(members[0].0, name, &online);
        registry.respond_invite(id, true);
    }
    registry
}

impl Harness {
    fn advance(&mut self, frames: usize) {
        for _ in 0..frames {
            self.app.update();
            for member in [&self.summoner, &self.helper, &self.target, &self.stranger] {
                let world = self.app.world_mut();
                let requests: Vec<SummonRequest> = world
                    .get_mut::<MessageReceiver<SummonRequest>>(member.connection)
                    .unwrap()
                    .receive()
                    .collect();
                if member.player == self.target.player {
                    self.requests.extend(requests);
                }
                let failed: Vec<CastFailed> = world
                    .get_mut::<MessageReceiver<CastFailed>>(member.connection)
                    .unwrap()
                    .receive()
                    .collect();
                self.failed
                    .extend(failed.into_iter().map(|failure| (member.player, failure)));
                let new_worlds: Vec<NewWorld> = world
                    .get_mut::<MessageReceiver<NewWorld>>(member.connection)
                    .unwrap()
                    .receive()
                    .collect();
                self.new_worlds.extend(
                    new_worlds
                        .into_iter()
                        .map(|new_world| (member.player, new_world)),
                );
            }
        }
    }

    fn advance_secs(&mut self, secs: f32) {
        self.advance((secs / 0.05).round() as usize);
    }

    fn target(&mut self, who: Entity, target: Entity) {
        self.app.world_mut().get_mut::<CombatTarget>(who).unwrap().0 = Some(target);
    }

    fn use_object(&mut self, connection: Entity, object: Entity) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<UseGameObject>>(connection)
            .unwrap()
            .send::<InteractionChannel>(UseGameObject {
                object: object.to_bits(),
            });
        self.advance(3);
    }

    fn respond(&mut self, accept: bool) {
        let connection = self.target.connection;
        self.app
            .world_mut()
            .get_mut::<MessageSender<SummonResponse>>(connection)
            .unwrap()
            .send::<InteractionChannel>(SummonResponse { accept });
        self.advance(3);
    }

    fn cast(&self, who: Entity) -> Option<CastState> {
        self.app.world().get::<CastState>(who).cloned()
    }

    fn position(&self, who: Entity) -> Position {
        *self.app.world().get::<Position>(who).unwrap()
    }

    fn portals(&mut self) -> Vec<(Entity, Position)> {
        let mut query = self
            .app
            .world_mut()
            .query::<(Entity, &GameObjectInfo, &Position)>();
        query
            .iter(self.app.world())
            .filter(|(_, info, _)| info.entry == MEETING_STONE_PORTAL)
            .map(|(entity, _, position)| (entity, *position))
            .collect()
    }

    /// Summoner targets `target`, uses the stone and finishes the 5 s cast.
    fn open_portal(&mut self) -> Entity {
        let (summoner, target) = (self.summoner.player, self.target.player);
        self.target(summoner, target);
        self.use_object(self.summoner.connection, self.stone);
        self.advance_secs(5.1);
        self.portals().first().expect("summoning portal").0
    }

    /// Portal opened and clicked by the second member: the target has an offer.
    fn offer_summon(&mut self) {
        let portal = self.open_portal();
        self.use_object(self.helper.connection, portal);
        assert_eq!(self.requests.len(), 1, "summon offer sent");
    }
}

#[test]
fn toyfx2_effect_immunity_blocks_summon_player_and_reverts() {
    // MiniZep Controller46709 supplies authentic immunity37 to effect85.
    // Puppet summon/no-actions remain unsupported; isolate its real aura effect.
    let mut h = harness(DEADMINES_STONE).unwrap();
    let target = h.target.player;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: target,
            target,
            spell_id: 65451,
            effect_mask: 1 << 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(3);
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(target)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 65451)
    );
    let portal = h.open_portal();
    h.use_object(h.helper.connection, portal);
    h.advance(5);
    assert!(
        h.requests.is_empty(),
        "summon effect must not send an offer while immune"
    );
    assert!(
        h.app
            .world()
            .get::<super::summon::PendingSummon>(target)
            .is_none()
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |a| a.spell_id == 65451,
    );
    h.advance(3);
    assert!(
        !h.app
            .world()
            .get::<shared::components::UnitAuras>(target)
            .unwrap()
            .auras
            .iter()
            .any(|a| a.spell_id == 65451)
    );
    h.offer_summon();
    assert!(
        h.app
            .world()
            .get::<super::summon::PendingSummon>(target)
            .is_some()
    );
}

#[test]
fn toyfx2_effect_no_immunity_summon_bypasses_toy_immunity() {
    let mut h = harness(DEADMINES_STONE).unwrap();
    let mut spells: Vec<_> = [SUMMONING_STONE_EFFECT, PORTAL_RITUAL_SPELL, 65451]
        .into_iter()
        .map(|id| {
            h.app
                .world()
                .resource::<SpellStore>()
                .get(id)
                .unwrap()
                .clone()
        })
        .collect();
    spells
        .iter_mut()
        .find(|s| s.id == PORTAL_RITUAL_SPELL)
        .unwrap()
        .effects[0]
        .attributes |= 1;
    h.app.insert_resource(SpellStore::fixture(spells));
    let target = h.target.player;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: target,
            target,
            spell_id: 65451,
            effect_mask: 1 << 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.offer_summon();
    assert!(
        h.app
            .world()
            .get::<super::summon::PendingSummon>(target)
            .is_some()
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 65451)
    );
}

#[test]
fn meetingstones_summon_from_stormwind_subzone_sends_zone_without_player_zone_component() {
    let mut h = harness(DEADMINES_STONE).expect("meeting stone world fixture");
    let summoner = h.summoner.player;
    let (x, y, z) = crate::terrain_height::wow_to_bevy(-8850.0, 650.0, 60.0);
    h.app
        .world_mut()
        .entity_mut(summoner)
        .remove::<Zone>()
        .insert(Position { x, y, z });
    assert!(h.app.world().get::<Zone>(summoner).is_none());
    assert_eq!(
        h.app
            .world_mut()
            .resource_mut::<crate::ground::GroundMap>()
            .area_at(0, Vec3::new(x, y, z)),
        Some(5148)
    );
    super::summon::send_summon_request(h.app.world_mut(), summoner, h.target.player);
    h.advance(3);
    assert_eq!(
        h.requests,
        vec![SummonRequest {
            summoner: "Stonecaller".into(),
            zone_id: 1519,
            time_left_ms: 120_000,
        }]
    );
}

#[test]
fn stone_use_with_a_party_member_targeted_starts_the_summoning_stone_cast() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    let (summoner, target) = (h.summoner.player, h.target.player);
    h.target(summoner, target);
    h.use_object(h.summoner.connection, h.stone);

    let cast = h.cast(summoner).expect("Summoning Stone Effect cast");
    assert_eq!(cast.spell_id, SUMMONING_STONE_EFFECT);
    assert_eq!(cast.cast_type, CastType::Normal);
    assert_eq!(cast.duration, 5.0);
    assert!(h.failed.is_empty(), "{:?}", h.failed);
}

#[test]
fn stone_use_with_a_target_outside_the_party_is_refused_as_invalid_target() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    let (summoner, stranger) = (h.summoner.player, h.stranger.player);
    h.target(summoner, stranger);
    h.use_object(h.summoner.connection, h.stone);

    assert!(h.cast(summoner).is_none());
    assert_eq!(
        h.failed,
        vec![(
            summoner,
            CastFailed {
                spell_id: SUMMONING_STONE_EFFECT,
                reason: CastFailReason::InvalidTarget,
                detail: None,
            }
        )]
    );
}

#[test]
fn stone_use_targeting_yourself_is_refused_as_invalid_target() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    let summoner = h.summoner.player;
    h.target(summoner, summoner);
    h.use_object(h.summoner.connection, h.stone);

    assert!(h.cast(summoner).is_none());
    assert_eq!(h.failed.len(), 1);
    assert_eq!(h.failed[0].1.reason, CastFailReason::InvalidTarget);
}

#[test]
fn stone_below_its_content_tuning_max_level_is_refused() {
    let Some(mut h) = harness(LEVELED_STONE) else {
        return;
    };
    let (summoner, target) = (h.summoner.player, h.target.player);
    h.target(summoner, target);
    // Level 20 < ContentTuning 212 max level 30 (GameObject.cpp:3204-3206).
    h.use_object(h.summoner.connection, h.stone);
    assert!(h.cast(summoner).is_none());
    assert_eq!(h.failed[0].1.reason, CastFailReason::LevelRequirement);

    h.failed.clear();
    h.app.world_mut().get_mut::<Level>(summoner).unwrap().0 = 30;
    h.use_object(h.summoner.connection, h.stone);
    assert!(h.cast(summoner).is_none());
    assert_eq!(h.failed[0].1.reason, CastFailReason::TargetTooLowLevel);

    h.failed.clear();
    h.app.world_mut().get_mut::<Level>(target).unwrap().0 = 30;
    h.use_object(h.summoner.connection, h.stone);
    assert_eq!(
        h.cast(summoner).map(|cast| cast.spell_id),
        Some(SUMMONING_STONE_EFFECT)
    );
}

#[test]
fn finished_cast_opens_the_portal_at_the_summoner_and_channels_for_two_minutes() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    h.open_portal();

    let summoner = h.summoner.player;
    let portals = h.portals();
    assert_eq!(portals.len(), 1);
    assert_eq!(portals[0].1, h.position(summoner));
    // In the summoner's map copy, so the helper can see and click it.
    assert_eq!(
        h.app.world().get::<crate::maps::InMap>(portals[0].0),
        Some(&crate::maps::InMap::continent(0))
    );
    // GO_STATE_READY: the client draws no game object without this replicated state.
    assert_eq!(
        h.app
            .world()
            .get::<shared::protocol::GameObjectCollision>(portals[0].0),
        Some(&shared::protocol::GameObjectCollision(true))
    );
    let channel = h.cast(summoner).expect("summoning channel");
    assert_eq!(channel.spell_id, SUMMONING_STONE_EFFECT);
    assert_eq!(channel.cast_type, CastType::Channel);
    assert_eq!(channel.duration, 120.0);
    assert!(h.requests.is_empty(), "one clicker does not summon");
}

/// 59782's `InterruptFlags` lack Movement: walking does not break the 5 s cast.
#[test]
fn moving_during_the_cast_still_opens_the_portal() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    let (summoner, target) = (h.summoner.player, h.target.player);
    h.target(summoner, target);
    h.use_object(h.summoner.connection, h.stone);
    h.advance_secs(2.0);
    h.app.world_mut().get_mut::<Moving>(summoner).unwrap().0 = true;
    h.advance_secs(2.0);
    h.app.world_mut().get_mut::<Moving>(summoner).unwrap().0 = false;
    h.advance_secs(1.1);

    assert_eq!(h.portals().len(), 1);
    assert_eq!(h.cast(summoner).unwrap().cast_type, CastType::Channel);
}

/// The channel's `ChannelInterruptFlags` hold Moving: walking ends it.
#[test]
fn stopping_the_channel_closes_the_portal() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    h.open_portal();
    h.app
        .world_mut()
        .get_mut::<Moving>(h.summoner.player)
        .unwrap()
        .0 = true;
    h.advance(2);

    assert!(h.portals().is_empty());
}

#[test]
fn second_party_member_at_the_portal_sends_the_target_a_summon_offer() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    let portal = h.open_portal();
    h.use_object(h.helper.connection, portal);

    assert_eq!(
        h.requests,
        vec![SummonRequest {
            summoner: "Stonecaller".into(),
            zone_id: WESTFALL,
            time_left_ms: 120_000,
        }]
    );
    assert!(h.portals().is_empty(), "ritual is not persistent");
    assert!(h.cast(h.summoner.player).is_none(), "channel finished");
}

#[test]
fn the_summoner_clicking_its_own_portal_does_not_complete_the_ritual() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    let portal = h.open_portal();
    h.use_object(h.summoner.connection, portal);

    assert!(h.requests.is_empty());
    assert_eq!(h.portals().len(), 1);
}

#[test]
fn a_helper_outside_the_party_cannot_complete_the_ritual() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    let portal = h.open_portal();
    let at_portal = h.position(h.helper.player);
    let stranger = h.stranger.player;
    *h.app.world_mut().get_mut::<Position>(stranger).unwrap() = at_portal;
    h.use_object(h.stranger.connection, portal);

    assert!(h.requests.is_empty());
    assert_eq!(h.portals().len(), 1);
}

#[test]
fn achievements_criteria_accept_summon_credits_valid_native_acceptance_once_and_persists() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState};
    use crate::persistence::{self, GameDatabase};
    let mut h = harness(DEADMINES_STONE).expect("native Deadmines meeting stone");
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    persistence::create_tables(&db);
    let mut characters = Vec::new();
    for (account, name, member) in [
        (1, "Stonecaller", &h.summoner),
        (2, "Ritualist", &h.helper),
        (3, "Farwalker", &h.target),
        (4, "Stranger", &h.stranger),
    ] {
        let (character, _) = persistence::create_character(&db, account, name, 1, 1).unwrap();
        h.app
            .world_mut()
            .entity_mut(member.connection)
            .insert(InWorld {
                character_id: character,
            });
        h.app
            .world_mut()
            .entity_mut(member.player)
            .insert((ClientId(character), PlayerAchievementState::default()));
        characters.push(character);
    }
    h.app.insert_resource(party(&[
        (characters[0], "Stonecaller", h.summoner.player),
        (characters[1], "Ritualist", h.helper.player),
        (characters[2], "Farwalker", h.target.player),
    ]));
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    h.app.insert_resource(GameDatabase::new(db));
    bevy::app::Plugin::build(&AchievementPlugin, &mut h.app);
    let target = h.target.player;
    h.offer_summon();
    assert!(!h.requests.is_empty());
    let before = h.position(target);
    h.respond(false);
    h.respond(true);
    assert_eq!(h.position(target), before);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(target)
            .unwrap()
            .criteria_progress
            .contains_key(&8586)
    );
    h.requests.clear();
    h.offer_summon();
    h.app
        .world_mut()
        .entity_mut(target)
        .insert(CombatStatus(true));
    h.respond(true);
    assert_eq!(h.position(target), before);
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(target)
            .unwrap()
            .criteria_progress
            .contains_key(&8586)
    );
    h.app
        .world_mut()
        .entity_mut(target)
        .insert(CombatStatus(false));
    h.respond(true);
    assert_eq!(h.position(target), h.position(h.summoner.player));
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(target)
            .unwrap()
            .criteria_progress
            .get(&8586),
        Some(&1)
    );
    h.respond(true);
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(target)
            .unwrap()
            .criteria_progress
            .get(&8586),
        Some(&1)
    );
    assert!(
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.summoner.player)
            .unwrap()
            .criteria_progress
            .contains_key(&8586)
    );
    h.requests.clear();
    h.offer_summon();
    h.advance_secs(121.0);
    h.respond(true);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerAchievementState>(target)
            .unwrap()
            .criteria_progress
            .get(&8586),
        Some(&1)
    );
    let character = characters[2];
    drop(h);
    let reopened = redb::Database::open(file.path()).unwrap();
    assert_eq!(
        persistence::load_achievements(&reopened, character)
            .criteria_progress
            .get(&8586),
        Some(&1)
    );
}

#[test]
fn accepting_moves_the_target_to_where_the_summoner_stood() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    h.offer_summon();
    let summoned_to = h.position(h.summoner.player);
    let epoch = h
        .app
        .world()
        .get::<MovementControl>(h.target.player)
        .unwrap()
        .epoch;
    h.respond(true);

    assert_eq!(h.position(h.target.player), summoned_to);
    let control = h
        .app
        .world()
        .get::<MovementControl>(h.target.player)
        .unwrap();
    assert_eq!(
        control.epoch,
        epoch.wrapping_add(1),
        "client adopts the move"
    );
    assert_ne!(summoned_to, h.stone_position);
}

#[test]
fn declining_leaves_the_target_in_place_and_ends_the_offer() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    h.offer_summon();
    let before = h.position(h.target.player);
    h.respond(false);
    h.respond(true);

    assert_eq!(h.position(h.target.player), before);
}

#[test]
fn accepting_after_the_two_minute_timeout_leaves_the_target_in_place() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    h.offer_summon();
    let before = h.position(h.target.player);
    h.advance_secs(121.0);
    h.respond(true);

    assert_eq!(h.position(h.target.player), before);
}

#[test]
fn accepting_in_combat_is_ignored_and_the_offer_stays_open() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    h.offer_summon();
    let target = h.target.player;
    let before = h.position(target);
    h.app
        .world_mut()
        .entity_mut(target)
        .insert(CombatStatus(true));
    h.respond(true);
    assert_eq!(h.position(target), before);

    h.app
        .world_mut()
        .entity_mut(target)
        .insert(CombatStatus(false));
    h.respond(true);
    assert_eq!(h.position(target), h.position(h.summoner.player));
}

#[test]
fn a_second_summon_while_one_is_pending_is_refused() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    h.offer_summon();
    let (summoner, target) = (h.summoner.player, h.target.player);
    h.target(summoner, target);
    h.use_object(h.summoner.connection, h.stone);

    assert!(h.cast(summoner).is_none());
    assert_eq!(
        h.failed.last().map(|(_, failure)| failure.reason),
        Some(CastFailReason::SummonPending)
    );
}

/// Ratchet, Northern Barrens (Kalimdor, map 1).
const RATCHET: (f32, f32, f32) = (-956.0, -3754.0, 5.0);

#[test]
fn accepting_on_kalimdor_transfers_the_target_to_the_summoner_in_eastern_kingdoms() {
    let Some(mut h) = harness(DEADMINES_STONE) else {
        return;
    };
    let (x, y, z) = crate::terrain_height::wow_to_bevy(RATCHET.0, RATCHET.1, RATCHET.2);
    h.app
        .world_mut()
        .entity_mut(h.target.player)
        .insert((Position { x, y, z }, crate::maps::InMap::continent(1)));
    h.offer_summon();
    let summoned_to = h.position(h.summoner.player);
    h.respond(true);

    // Out of Kalimdor, loading Eastern Kingdoms.
    assert_eq!(
        h.app.world().get::<crate::maps::InMap>(h.target.player),
        None
    );
    h.advance(2);
    let target = h.target.player;
    let (_, new_world) = h
        .new_worlds
        .iter()
        .find(|(player, _)| *player == target)
        .expect("NewWorld for map 0");
    assert_eq!(new_world.map_id, 0);
    assert_eq!(new_world.map_directory, "azeroth");

    h.app
        .world_mut()
        .get_mut::<MessageSender<WorldPortAck>>(h.target.connection)
        .unwrap()
        .send::<TransferChannel>(WorldPortAck);
    h.advance(3);
    assert_eq!(
        h.app.world().get::<crate::maps::InMap>(h.target.player),
        Some(&crate::maps::InMap::continent(0))
    );
    assert_eq!(h.position(h.target.player), summoned_to);
}
