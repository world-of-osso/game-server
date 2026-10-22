//! Actual owned-journal RPCs plus physical learning/UseToy; no seeded CritterGUID.
//! Source removal is natural10s expiry. Native PetCancelAura is not exercised.
use super::*;
use crate::auras::ActiveAuras;
use crate::persistence::{InventoryData, toy_store};
use shared::components::{CombatStatus, Level, UnitAuras};
use shared::protocol::{
    InventoryChannel, InventoryError, InventoryItemData, ItemLocation, ToyCollectionUpdate,
    ToyOperation, ToyResult, UseItem, UseToy,
};

const TOY: u32 = 163705;
const SPELL: u32 = 279843;
const DURATION_MS: u32 = 10_000;
const RECOVERY_MS: u32 = 180_000;

fn load_toy_sources(app: &mut App) {
    static ITEMS: std::sync::OnceLock<crate::item_templates::ItemTemplates> =
        std::sync::OnceLock::new();
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    app.insert_resource(crate::toy_catalog::load(&conn).unwrap());
    app.insert_resource(crate::spell_cast::toy_scripts::load_toy_texts(&conn).unwrap());
    app.insert_resource(crate::game_objects::summoned::load_templates(&conn).unwrap());
    app.insert_resource(
        ITEMS
            .get_or_init(|| crate::item_templates::load_item_templates(&path).unwrap())
            .clone(),
    );
    app.init_resource::<crate::gold::GoldUpdateQueue>();
}

fn attach_toy_messages(h: &mut Harness) {
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<UseItem>::default(),
        MessageReceiver::<UseItem>::default(),
        MessageSender::<InventoryError>::default(),
        MessageReceiver::<InventoryError>::default(),
        MessageSender::<UseToy>::default(),
        MessageReceiver::<UseToy>::default(),
        MessageSender::<ToyResult>::default(),
        MessageReceiver::<ToyResult>::default(),
        MessageSender::<ToyCollectionUpdate>::default(),
        MessageReceiver::<ToyCollectionUpdate>::default(),
    ));
}

fn fixture(path: &std::path::Path) -> (Harness, OwnedPetIds, u64) {
    let (mut h, ids) = create_critter_fixture(path);
    load_toy_sources(&mut h.app);
    attach_toy_messages(&mut h);
    bevy::app::Plugin::build(&crate::toys::ToysPlugin, &mut h.app);
    h.app
        .add_systems(Update, crate::inventory::process_use_requests);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((CombatStatus(false), Level(80), UnitLevel(80)));
    let character = h
        .app
        .world()
        .get::<InWorld>(h.connection)
        .unwrap()
        .character_id;
    (h, ids, character)
}

fn collect_results(h: &mut Harness) -> Vec<ToyResult> {
    let mut results = Vec::new();
    for _ in 0..7 {
        advance_journal(h, 1);
        results.extend(drain::<ToyResult>(&mut h.app, h.connection));
        drain::<ToyCollectionUpdate>(&mut h.app, h.connection);
    }
    results
}

fn learn(h: &mut Harness, character: u64) {
    let db = &h.app.world().resource::<GameDatabase>().db;
    crate::persistence::save_inventory(
        db,
        character,
        &InventoryData {
            gold: 0,
            items: vec![InventoryItemData {
                item_guid: crate::persistence::next_id(db, "item_instance"),
                item_id: TOY,
                name: String::new(),
                quality: 1,
                required_level: 1,
                stack_count: 1,
                vendor_sell_price: 0,
                unique_auction: false,
            }],
        },
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseItem>>(h.connection)
        .unwrap()
        .send::<InventoryChannel>(UseItem {
            location: ItemLocation::Bag { bag: 0, slot: 0 },
            target: None,
        });
    let results = collect_results(h);
    assert!(
        results
            .iter()
            .any(|r| r.operation == ToyOperation::Learn && r.error.is_none()),
        "{results:?}; {:?}",
        h.inbox.inventory_errors
    );
    assert!(toy_store::load(&h.app.world().resource::<GameDatabase>().db, 1).contains_key(&TOY));
}

fn activate(h: &mut Harness) -> Vec<ToyResult> {
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseToy>>(h.connection)
        .unwrap()
        .send::<CollectionChannel>(UseToy {
            item_id: TOY,
            destination: None,
            witness: None,
        });
    collect_results(h)
}

fn assert_use_succeeds(h: &mut Harness) {
    let results = activate(h);
    assert!(
        results
            .iter()
            .any(|r| r.operation == ToyOperation::Use && r.error.is_none()),
        "{results:?}"
    );
}

fn assert_aura(h: &Harness, pet: Entity) {
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(pet)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SPELL)
        .expect("source companion aura applied");
    assert_eq!(aura.caster, h.caster);
    assert_eq!(aura.duration_ms, DURATION_MS);
    assert!(aura.expires_at.is_some());
    assert_eq!(aura.effects[0].amount, 0.0);
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(pet)
            .unwrap()
            .auras
            .iter()
            .any(|view| view.spell_id == SPELL && view.caster == Some(h.caster.to_bits()))
    );
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, SPELL));
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, SPELL));
    let go = h
        .inbox
        .go
        .iter()
        .rev()
        .find(|go| go.spell_id == SPELL)
        .unwrap();
    assert_eq!(go.caster, h.caster.to_bits());
    assert_eq!(go.hit_targets, vec![pet.to_bits()]);
}

#[test]
fn toy_coverage_imaginary_gun_journal_use_natural_expiry_and_source_recovery_reuse() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids, character) = fixture(&dir.path().join("gun-journal.redb"));
    let pet = summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
    learn(&mut h, character);
    let health = *h.app.world().get::<Health>(pet).unwrap();
    let model = *h.app.world().get::<ModelDisplay>(pet).unwrap();
    let position = *h.app.world().get::<Position>(pet).unwrap();
    let flags = h.app.world().get::<UnitFlags>(pet).unwrap().0;
    let owner_health = *h.app.world().get::<Health>(h.caster).unwrap();
    let owner_model = h.app.world().get::<ModelDisplay>(h.caster).copied();
    let owner_position = h.app.world().get::<Position>(h.caster).copied();
    let owner_flags = h
        .app
        .world()
        .get::<UnitFlags>(h.caster)
        .map(|flags| flags.0);
    let summoner = h.app.world().get::<UnitSummonedBy>(pet).unwrap().0;
    h.select(Some(h.npc));
    h.advance(2);
    assert_use_succeeds(&mut h);
    assert_aura(&h, pet);
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == SPELL && cd.duration_ms == RECOVERY_MS && !cd.is_gcd)
    );
    h.advance(180); // Existing50ms ticks: still before source10s expiry.
    assert_aura(&h, pet);
    h.advance(35);
    assert!(!crate::auras::has_aura(h.app.world(), pet, SPELL));
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(pet)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == SPELL)
    );
    assert_eq!(h.app.world().get::<Health>(pet), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(pet), Some(&model));
    assert_eq!(h.app.world().get::<Position>(pet), Some(&position));
    assert_eq!(h.app.world().get::<UnitFlags>(pet).unwrap().0, flags);
    assert_eq!(h.app.world().get::<Health>(h.caster), Some(&owner_health));
    assert_eq!(
        h.app.world().get::<ModelDisplay>(h.caster).copied(),
        owner_model
    );
    assert_eq!(
        h.app.world().get::<Position>(h.caster).copied(),
        owner_position
    );
    assert_eq!(
        h.app
            .world()
            .get::<UnitFlags>(h.caster)
            .map(|flags| flags.0),
        owner_flags
    );
    assert_eq!(
        h.app.world().get::<UnitSummonedBy>(pet).unwrap().0,
        summoner
    );
    let blocked = activate(&mut h);
    assert!(
        blocked
            .iter()
            .any(|r| r.error.as_deref() == Some("OnCooldown")),
        "{blocked:?}"
    );
    h.advance(3_400); // Cross authored180s recovery naturally; no cooldown rewrite.
    assert_use_succeeds(&mut h);
    assert_aura(&h, pet);
    assert_eq!(
        h.inbox.go.iter().filter(|go| go.spell_id == SPELL).count(),
        2
    );
}

#[test]
fn toy_coverage_imaginary_gun_legacy_current_critter_ignores_selected_foreign_journal_pet() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids, character) = fixture(&dir.path().join("gun-foreign.redb"));
    summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
    let own = cast_source_summon(&mut h, COYOTE_SPELL, COYOTE_ENTRY)[0];
    let foreign = spawn_player(&mut h.app, 8, 5.0);
    let connection = spawn_peer_connection(&mut h.app, foreign, 2);
    h.app
        .world_mut()
        .entity_mut(connection)
        .insert((Authenticated { account_id: 2 }, InWorld { character_id: 2 }));
    advance_journal(&mut h, 5);
    take_journal_replies(&mut h, connection);
    let replies = request_summon(&mut h, connection, ids.foreign);
    assert!(
        replies
            .iter()
            .any(|r| r.error.is_none() && r.summoned_pet_id == Some(ids.foreign)),
        "{replies:?}"
    );
    let entry = h
        .app
        .world()
        .resource::<PetCatalog>()
        .creature_id(MECHANICAL_SQUIRREL)
        .unwrap();
    let other = owned_actors(&mut h, foreign, entry)[0];
    learn(&mut h, character);
    h.select(Some(other));
    h.advance(2);
    assert_use_succeeds(&mut h);
    assert_aura(&h, own);
    assert!(!crate::auras::has_aura(h.app.world(), other, SPELL));
    assert!(!crate::auras::has_aura(h.app.world(), foreign, SPELL));
    h.advance(220);
    assert!(!crate::auras::has_aura(h.app.world(), own, SPELL));
    assert!(h.app.world().get_entity(other).is_ok());
}

#[test]
fn toy_coverage_imaginary_gun_no_critter_never_substitutes_owned_guardians_or_caster() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _, character) = fixture(&dir.path().join("gun-guardians.redb"));
    let guardians = cast_source_summon(&mut h, SKELETON_SPELL, SKELETON_ENTRY);
    assert!((2..=3).contains(&guardians.len()));
    learn(&mut h, character);
    h.select(Some(guardians[0]));
    h.advance(2);
    // Source flags contain no REQUIRE_ALL_TARGETS; the authored caster kit still
    // launches, but missing150 creates no recipient and no substituted aura.
    assert_use_succeeds(&mut h);
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.spell_id == SPELL && go.hit_targets.is_empty())
    );
    for actor in guardians.into_iter().chain([h.caster, h.npc]) {
        assert!(!crate::auras::has_aura(h.app.world(), actor, SPELL));
    }
}

#[test]
fn toy_coverage_imaginary_gun_noncompanion_journal_actor_is_not_a_critter_substitute() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids, character) = fixture(&dir.path().join("gun-ordinary-ally.redb"));
    let balloon = summon_journal_actor(&mut h, ids.ordinary_ally, HIDDEN_GREEN_BALLOON);
    learn(&mut h, character);
    h.select(Some(balloon));
    h.advance(2);
    assert_use_succeeds(&mut h);
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.spell_id == SPELL && go.hit_targets.is_empty())
    );
    assert!(!crate::auras::has_aura(h.app.world(), balloon, SPELL));
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, SPELL));
}

#[test]
fn toy_coverage_imaginary_gun_visual_payload_recipient_and_gameplay_sibling_drift_refuse() {
    for drift in 0..4 {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, ids, character) = fixture(&dir.path().join("gun-drift.redb"));
        let pet = summon_journal_actor(&mut h, ids.mechanical, MECHANICAL_SQUIRREL);
        learn(&mut h, character);
        let healths = [pet, h.caster, h.npc]
            .map(|entity| (entity, *h.app.world().get::<Health>(entity).unwrap()));
        let source = h.app.world().resource::<SpellStore>();
        let mut changed = source.get(SPELL).unwrap().clone();
        match drift {
            0 => changed.has_spell_visual = false,
            1 => changed.effects[0].base_points = 1.0,
            2 => changed.effects[0].implicit_targets = [1, 0],
            3 => {
                let mut damage = source.get(116).unwrap().effects[0].clone();
                damage.index = 1;
                changed.effects.push(damage);
            }
            _ => unreachable!(),
        }
        h.app.insert_resource(SpellStore::fixture(vec![changed]));
        h.select(Some(h.npc));
        h.advance(2);
        let results = activate(&mut h);
        assert!(
            results
                .iter()
                .any(|r| r.error.as_deref() == Some("unsupported toy spell: aura 4")),
            "drift{drift}: {results:?}"
        );
        assert!(!crate::auras::has_aura(h.app.world(), pet, SPELL));
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, SPELL));
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == SPELL));
        assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == SPELL));
        for (entity, health) in healths {
            assert_eq!(h.app.world().get::<Health>(entity), Some(&health));
        }
    }
}
