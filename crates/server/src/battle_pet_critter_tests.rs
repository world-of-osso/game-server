//! Real journal RPC + authored legacy summons exercise the current-critter boundary.
#[path = "toys_imaginary_gun_tests.rs"]
mod imaginary_gun;
use super::*;
use crate::spell_cast::test_harness::*;
use crate::spell_info::SpellStore;

const MECHANICAL_SQUIRREL: u32 = 39;
const SNOWY_OWL: u32 = 69;
const HIDDEN_GREEN_BALLOON: u32 = 344;
const COYOTE_SPELL: u32 = 51149;
const COYOTE_ENTRY: u32 = 28267;
const GOLDEN_HARE_SPELL: u32 = 26571;
const GOLDEN_HARE_ENTRY: u32 = 15923;
const SKELETON_SPELL: u32 = 261;
const SKELETON_ENTRY: u32 = 48;
const EXPLICIT_MINIPET_SPELL: u32 = 48722;
const OWN_CRITTER_SPELL: u32 = 279843;

#[derive(Resource, Default)]
struct JournalReplies(HashMap<Entity, Vec<CollectionStateUpdate>>);

// HostClient replies survive only one frame; retain each real wire delivery.
fn advance_journal(h: &mut Harness, frames: usize) {
    for _ in 0..frames {
        h.advance(1);
        let connections: Vec<_> = h
            .app
            .world_mut()
            .query_filtered::<Entity, With<MessageReceiver<CollectionStateUpdate>>>()
            .iter(h.app.world())
            .collect();
        for connection in connections {
            let replies = drain::<CollectionStateUpdate>(&mut h.app, connection);
            h.app
                .world_mut()
                .resource_mut::<JournalReplies>()
                .0
                .entry(connection)
                .or_default()
                .extend(replies);
        }
    }
}

fn take_journal_replies(h: &mut Harness, connection: Entity) -> Vec<CollectionStateUpdate> {
    h.app
        .world_mut()
        .resource_mut::<JournalReplies>()
        .0
        .remove(&connection)
        .unwrap_or_default()
}

struct OwnedPetIds {
    mechanical: u64,
    snowy: u64,
    ordinary_ally: u64,
    foreign: u64,
}

fn store_owned_pet(db: &Database, account: u64, species: u32) -> u64 {
    let mut journal = load_journal(db, account);
    let id = crate::persistence::next_id(db, "battle_pet_instance");
    journal
        .add_with_guid(species, 1, shared::pet_battle::PetQuality::Common, 3, id)
        .unwrap();
    save_journal(db, account, &journal);
    id
}

fn seed_owned_journals(db: &Database) -> OwnedPetIds {
    OwnedPetIds {
        mechanical: store_owned_pet(db, 1, MECHANICAL_SQUIRREL),
        snowy: store_owned_pet(db, 1, SNOWY_OWL),
        ordinary_ally: store_owned_pet(db, 1, HIDDEN_GREEN_BALLOON),
        foreign: store_owned_pet(db, 2, MECHANICAL_SQUIRREL),
    }
}

fn initialize_fixture_sessions(world: &mut World) {
    let mut query = world.query_filtered::<(Entity, &PlayerLink, &InWorld), Added<InWorld>>();
    let sessions: Vec<_> = query
        .iter(world)
        .map(|(connection, link, state)| (connection, link.0, state.character_id))
        .collect();
    for (connection, owner, character) in sessions {
        world.entity_mut(owner).insert((
            crate::persistence::ClientId(character),
            Rotation {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            PlayerCollectionState {
                mounts: Vec::new(),
                pets: Vec::new(),
            },
        ));
        world.entity_mut(connection).insert((
            MessageSender::<SummonPet>::default(),
            MessageReceiver::<SummonPet>::default(),
            MessageSender::<DismissPet>::default(),
            MessageReceiver::<DismissPet>::default(),
            MessageSender::<CollectionStateUpdate>::default(),
            MessageReceiver::<CollectionStateUpdate>::default(),
        ));
    }
}

fn install_critter_runtime(app: &mut App, db: Database) {
    app.init_resource::<JournalReplies>();
    app.insert_resource(GameDatabase::new(db));
    app.insert_resource(load_test_catalog());
    app.insert_resource(crate::test_data::world_data().clone());
    app.init_resource::<crate::group::GroupRegistry>();
    app.add_systems(
        Update,
        (
            initialize_fixture_sessions,
            load_account_journals,
            receive_pet_requests,
            despawn_ownerless_companions,
        )
            .chain(),
    );
}

fn create_critter_fixture(path: &std::path::Path) -> (Harness, OwnedPetIds) {
    let db = Database::create(path).unwrap();
    crate::persistence::create_tables(&db);
    crate::persistence::create_character(&db, 1, "CritterOne", 1, 8).unwrap();
    crate::persistence::create_character(&db, 2, "CritterTwo", 1, 8).unwrap();
    let ids = seed_owned_journals(&db);
    let mut h = harness_with_plugins(Vec::new(), &[], 8, false, |app| {
        install_critter_runtime(app, db)
    })
    .unwrap();
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    advance_journal(&mut h, 5);
    let connection = h.connection;
    take_journal_replies(&mut h, connection);
    (h, ids)
}

fn request_summon(h: &mut Harness, connection: Entity, pet_id: u64) -> Vec<CollectionStateUpdate> {
    h.app
        .world_mut()
        .get_mut::<MessageSender<SummonPet>>(connection)
        .unwrap()
        .send::<CollectionChannel>(SummonPet { pet_id });
    advance_journal(h, 6);
    take_journal_replies(h, connection)
}

fn request_dismiss(h: &mut Harness) -> Vec<CollectionStateUpdate> {
    h.app
        .world_mut()
        .get_mut::<MessageSender<DismissPet>>(h.connection)
        .unwrap()
        .send::<CollectionChannel>(DismissPet);
    advance_journal(h, 6);
    let connection = h.connection;
    take_journal_replies(h, connection)
}

fn owned_actors(h: &mut Harness, owner: Entity, entry: u32) -> Vec<Entity> {
    let world = h.app.world_mut();
    world
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(world)
        .filter(|(_, npc, summoner)| npc.template_id == entry && summoner.0 == owner.to_bits())
        .map(|(entity, _, _)| entity)
        .collect()
}

fn summon_journal_actor(h: &mut Harness, pet_id: u64, species: u32) -> Entity {
    let connection = h.connection;
    let updates = request_summon(h, connection, pet_id);
    assert!(
        updates
            .iter()
            .any(|update| update.error.is_none() && update.summoned_pet_id == Some(pet_id)),
        "{updates:?}"
    );
    let entry = h
        .app
        .world()
        .resource::<PetCatalog>()
        .creature_id(species)
        .unwrap();
    let owner = h.caster;
    let actors = owned_actors(h, owner, entry);
    assert_eq!(actors.len(), 1);
    actors[0]
}

fn cast_source_summon(h: &mut Harness, spell: u32, entry: u32) -> Vec<Entity> {
    let owner = h.caster;
    let result =
        crate::spell_cast::creature_cast(h.app.world_mut(), owner, spell, Some(owner), false);
    assert!(result.is_ok(), "source{spell}: {result:?}");
    advance_journal(h, 5);
    let actors = owned_actors(h, owner, entry);
    assert!(
        !actors.is_empty(),
        "source{spell} actually creates entry{entry}"
    );
    actors
}

fn select_source_targets(
    h: &mut Harness,
    spell_id: u32,
    explicit: Option<Entity>,
) -> Result<Vec<Entity>, u32> {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    let effect = &spell.effects[0];
    let owner = h.caster;
    crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        owner,
        explicit,
        &spell,
        effect,
        0,
        false,
    )
}

#[test]
fn toy_coverage_critter_journal_rpc_registers_real_source_nonzero_and_generic_zero_spell() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("journal.redb"));
    for (id, species) in [
        (ids.mechanical, MECHANICAL_SQUIRREL),
        (ids.snowy, SNOWY_OWL),
    ] {
        let actor = summon_journal_actor(&mut h, id, species);
        assert_eq!(
            select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
            Ok(vec![actor])
        );
        assert_eq!(
            select_source_targets(&mut h, EXPLICIT_MINIPET_SPELL, Some(actor)),
            Ok(vec![actor])
        );
        let updates = request_dismiss(&mut h);
        assert!(
            updates
                .iter()
                .any(|update| update.error.is_none() && update.summoned_pet_id.is_none())
        );
        assert!(h.app.world().get_entity(actor).is_err());
        assert_eq!(
            select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
            Ok(Vec::new())
        );
    }
}

#[test]
fn toy_coverage_critter_same_source_slot_replaces_across_journal_and_legacy_both_orders() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("shared-slot.redb"));
    let journal = summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
    let coyotes = cast_source_summon(&mut h, COYOTE_SPELL, COYOTE_ENTRY);
    assert_eq!(coyotes.len(), 1);
    assert!(
        h.app.world().get_entity(journal).is_err(),
        "source41 and journal3221 both claim slot5"
    );
    let connection = h.connection;
    let updates = take_journal_replies(&mut h, connection);
    assert!(
        updates
            .iter()
            .any(|update| update.summoned_pet_id.is_none())
    );
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(coyotes.clone())
    );
    let replacement = summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
    assert!(h.app.world().get_entity(coyotes[0]).is_err());
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(vec![replacement])
    );
}

#[test]
fn toy_coverage_critter_unslotted_native_expiry_never_restores_an_older_owned_actor() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("no-restore.redb"));
    let older = summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
    let hares = cast_source_summon(&mut h, GOLDEN_HARE_SPELL, GOLDEN_HARE_ENTRY);
    assert_eq!(hares.len(), 1);
    assert!(
        h.app.world().get_entity(older).is_ok(),
        "slot0 does not replace slot5"
    );
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(hares.clone())
    );
    assert_eq!(
        select_source_targets(&mut h, EXPLICIT_MINIPET_SPELL, Some(older)),
        Ok(Vec::new())
    );
    h.advance(140); // Source26571 duration6000ms, existing50ms ticks.
    assert!(h.app.world().get_entity(hares[0]).is_err());
    assert!(h.app.world().get_entity(older).is_ok());
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(Vec::new())
    );
    assert_eq!(
        select_source_targets(&mut h, EXPLICIT_MINIPET_SPELL, Some(older)),
        Ok(Vec::new())
    );
}

#[test]
fn toy_coverage_critter_older_journal_dismissal_does_not_clear_latest_legacy_critter() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("old-remove.redb"));
    let older = summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
    let hares = cast_source_summon(&mut h, GOLDEN_HARE_SPELL, GOLDEN_HARE_ENTRY);
    request_dismiss(&mut h);
    assert!(h.app.world().get_entity(older).is_err());
    assert!(h.app.world().get_entity(hares[0]).is_ok());
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(hares)
    );
}

#[test]
fn toy_coverage_critter_guardians_and_source_noncompanion_journal_actor_are_not_substitutes() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("no-substitute.redb"));
    let guardians = cast_source_summon(&mut h, SKELETON_SPELL, SKELETON_ENTRY);
    assert!(
        (2..=3).contains(&guardians.len()),
        "source261 base3/variance0.25 produces two or three guardians: {guardians:?}"
    );
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(Vec::new())
    );
    for guardian in guardians {
        assert_eq!(
            select_source_targets(&mut h, EXPLICIT_MINIPET_SPELL, Some(guardian)),
            Ok(Vec::new())
        );
    }
    let ordinary_ally = summon_journal_actor(&mut h, ids.ordinary_ally, HIDDEN_GREEN_BALLOON);
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(Vec::new())
    );
    assert_eq!(
        select_source_targets(&mut h, EXPLICIT_MINIPET_SPELL, Some(ordinary_ally)),
        Ok(Vec::new())
    );
}

#[test]
fn toy_coverage_critter_target90_has_no_self_default_and_target150_ignores_foreign_selection() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("foreign.redb"));
    let own = summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
    let foreign = spawn_player(&mut h.app, 8, 5.0);
    let connection = spawn_peer_connection(&mut h.app, foreign, 2);
    h.app
        .world_mut()
        .entity_mut(connection)
        .insert((Authenticated { account_id: 2 }, InWorld { character_id: 2 }));
    advance_journal(&mut h, 5);
    take_journal_replies(&mut h, connection);
    let updates = request_summon(&mut h, connection, ids.foreign);
    assert!(
        updates
            .iter()
            .any(|update| update.error.is_none() && update.summoned_pet_id == Some(ids.foreign))
    );
    let entry = h
        .app
        .world()
        .resource::<PetCatalog>()
        .creature_id(MECHANICAL_SQUIRREL)
        .unwrap();
    let others = owned_actors(&mut h, foreign, entry);
    assert_eq!(others.len(), 1);
    assert_eq!(
        select_source_targets(&mut h, EXPLICIT_MINIPET_SPELL, None),
        Ok(Vec::new())
    );
    let owner = h.caster;
    assert_eq!(
        select_source_targets(&mut h, EXPLICIT_MINIPET_SPELL, Some(owner)),
        Ok(Vec::new())
    );
    assert_eq!(
        select_source_targets(&mut h, EXPLICIT_MINIPET_SPELL, Some(others[0])),
        Ok(Vec::new())
    );
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, Some(others[0])),
        Ok(vec![own])
    );
}

#[test]
fn toy_coverage_critter_owner_map_departure_removes_actor_without_rebinding_owned_data() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("map-exit.redb"));
    let actor = summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(InMap::continent(1));
    h.advance(6);
    assert!(h.app.world().get_entity(actor).is_err());
    assert_eq!(
        select_source_targets(&mut h, OWN_CRITTER_SPELL, None),
        Ok(Vec::new())
    );
    assert_eq!(
        h.app
            .world()
            .get::<AccountPetJournal>(h.caster)
            .unwrap()
            .journal
            .get(ids.mechanical)
            .unwrap()
            .species_id,
        MECHANICAL_SQUIRREL
    );
}

#[test]
fn toy_coverage_critter_owner_teardown_removes_both_journal_and_legacy_actors() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids) = create_critter_fixture(&dir.path().join("logout.redb"));
    let actor = summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
    let hares = cast_source_summon(&mut h, GOLDEN_HARE_SPELL, GOLDEN_HARE_ENTRY);
    let owner = h.caster;
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<InWorld>();
    h.app.world_mut().despawn(owner);
    h.advance(6);
    assert!(h.app.world().get_entity(actor).is_err());
    assert!(h.app.world().get_entity(hares[0]).is_err());
}
