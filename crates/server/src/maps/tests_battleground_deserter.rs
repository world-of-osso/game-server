use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, RemoveReason};

#[path = "tests_battleground_deserter_port.rs"]
mod port;

#[path = "tests_battleground_lfg.rs"]
mod lfg_refusal;

// TC a352b1fa Player.h2594: IsDeserter() is HasAura(26013).
const DESERTER_SPELL_ID: u32 = 26013;
const DUNGEON_DESERTER_SPELL_ID: u32 = 71041;

#[test]
fn bg_deserter_solo_concrete_refuses_active_aura_and_allows_clear_aura() {
    assert_solo_deserter_refusal(1018);
}

#[test]
fn bg_deserter_solo_random_refuses_active_aura_and_allows_clear_aura() {
    assert_solo_deserter_refusal(32);
}

#[test]
fn bg_deserter_party_concrete_refuses_every_member_until_the_aura_is_cleared() {
    assert_party_deserter_refusal(1018);
}

#[test]
fn bg_deserter_party_random_refuses_every_member_until_the_aura_is_cleared() {
    assert_party_deserter_refusal(32);
}

#[test]
fn bg_deserter_dungeon_aura_does_not_refuse_battleground_admission() {
    let (mut h, players, _database) = deserter_harness(1);
    apply_sourced_aura(&mut h, &players[0], DUNGEON_DESERTER_SPELL_ID);
    let updates = request_queue(&mut h, &players[0], 1018);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "non-BG Deserter: {updates:?}"
    );
    assert_queued(&h, &players, 1018);
}

fn deserter_harness(count: u64) -> (Harness, Vec<Traveler>, tempfile::NamedTempFile) {
    let source = crate::spell_cast::test_harness::world_db().expect("native spell source");
    let spell = source
        .store
        .get(DESERTER_SPELL_ID)
        .expect("native Deserter")
        .clone();
    assert_eq!(spell.name, "Deserter");
    assert_eq!((spell.effects[0].effect, spell.effects[0].aura), (6, 4));
    let dungeon_deserter = source
        .store
        .get(DUNGEON_DESERTER_SPELL_ID)
        .expect("native Dungeon Deserter")
        .clone();
    let spells = crate::spell_info::SpellStore::fixture(vec![spell, dungeon_deserter]);
    let database_file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(database_file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let mut h = harness_with(move |app| {
        app.insert_resource(crate::persistence::GameDatabase::new(db));
        app.insert_resource(crate::pvp::random_bg::tests::source_catalog(&[1018]));
        app.insert_resource(spells);
        app.insert_resource(
            crate::spell_cast::toy_requirements::load(
                crate::test_data::world_db_path().parent().unwrap(),
            )
            .expect("authored aura area requirements"),
        );
        app.add_plugins((
            crate::group::GroupPlugin,
            crate::pvp::PvpPlugin,
            crate::experience::ExperiencePlugin,
            crate::death::DeathPlugin,
            crate::auras::AuraPlugin,
        ));
        app.add_plugins(crate::npc_interaction::NpcInteractionPlugin);
    })
    .expect("real map/channel fixture");
    let players: Vec<_> = (1..=count)
        .map(|id| h.spawn_at_level(id, 25, STORMWIND, STOCKADE_ENTRANCE_TRIGGER))
        .collect();
    for who in &players {
        h.app
            .world_mut()
            .entity_mut(who.player)
            .insert(crate::pvp::PlayerPvpState::default());
        h.app
            .world_mut()
            .entity_mut(who.connection)
            .insert((pair::<QueueForBattleground>(), pair::<PvpStateUpdate>()));
    }
    h.receive_pvp_updates(5);
    (h, players, database_file)
}

fn apply_sourced_aura(h: &mut Harness, who: &Traveler, spell_id: u32) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap();
    let application = AuraApplication {
        caster: who.player,
        target: who.player,
        spell_id,
        effect_mask: crate::auras::aura_effect_mask(spell),
        source: AuraSource::Cast,
    };
    crate::auras::apply_aura(h.app.world_mut(), &application);
    let active = h
        .app
        .world()
        .get::<ActiveAuras>(who.player)
        .expect("applied native aura");
    let aura = active
        .iter()
        .find(|aura| aura.spell_id == spell_id)
        .unwrap();
    assert!(
        aura.expires_at
            .is_some_and(|expires| expires > crate::auras::aura_now(h.app.world()))
    );
}

fn clear_deserter(h: &mut Harness, who: &Traveler) {
    let removed = crate::auras::remove_auras(
        h.app.world_mut(),
        who.player,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == DESERTER_SPELL_ID,
    );
    assert_eq!(removed.len(), 1);
    assert!(
        !h.app
            .world()
            .get::<ActiveAuras>(who.player)
            .unwrap()
            .has_spell(DESERTER_SPELL_ID)
    );
}

fn request_queue(h: &mut Harness, who: &Traveler, queue_id: u32) -> Vec<PvpStateUpdate> {
    h.app
        .world_mut()
        .get_mut::<MessageSender<QueueForBattleground>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(QueueForBattleground {
            battleground_id: queue_id,
        });
    h.receive_pvp_updates(5)
        .remove(&who.connection)
        .expect("normal queue reply")
}

fn assert_not_queued(h: &Harness, players: &[Traveler]) {
    for who in players {
        let state = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(who.player)
            .unwrap();
        assert!(state.queue.is_none(), "refused party member was queued");
        assert!(
            state.battleground.is_none(),
            "refused party member was invited"
        );
    }
}

fn assert_queued(h: &Harness, players: &[Traveler], queue_id: u32) {
    for who in players {
        let state = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(who.player)
            .unwrap();
        assert!(
            matches!(&state.queue, Some(crate::pvp::PlayerPvpQueue::Battleground {
            battleground_id, ..
        }) if *battleground_id == queue_id)
        );
        assert!(
            state.battleground.is_none(),
            "fewer than two teams remain queued"
        );
    }
}

fn assert_solo_deserter_refusal(queue_id: u32) {
    let (mut h, players, _database) = deserter_harness(1);
    let who = &players[0];
    apply_sourced_aura(&mut h, who, DESERTER_SPELL_ID);
    let updates = request_queue(&mut h, who, queue_id);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("deserter"))),
        "active native Deserter must refuse the normal queue request: {updates:?}"
    );
    assert_not_queued(&h, &players);
    clear_deserter(&mut h, who);
    let updates = request_queue(&mut h, who, queue_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "cleared aura: {updates:?}"
    );
    assert_queued(&h, &players, queue_id);
}

fn assert_party_deserter_refusal(queue_id: u32) {
    let (mut h, players, _database) = deserter_harness(5);
    form_source_five_player_parties(&mut h, &players, 25);
    let leader = &players[0];
    let last_member = &players[4];
    apply_sourced_aura(&mut h, last_member, DESERTER_SPELL_ID);
    let updates = request_queue(&mut h, leader, queue_id);
    assert!(
        updates.iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("deserter"))),
        "one deserter must refuse the whole ordinary party: {updates:?}"
    );
    assert_not_queued(&h, &players);
    clear_deserter(&mut h, last_member);
    let updates = request_queue(&mut h, leader, queue_id);
    assert!(
        updates.iter().all(|update| update.error.is_none()),
        "cleared party aura: {updates:?}"
    );
    assert_queued(&h, &players, queue_id);
}
