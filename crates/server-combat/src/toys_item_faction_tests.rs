//! Native item flags must constrain both physical learning and account-owned use.
use super::*;
use shared::components::{Player, UnitAuras, UnitFactionTemplate, UnitLevel};
use shared::protocol::{SpellCooldownUpdate, SpellGo};

#[derive(Clone, Copy)]
struct FactionToy {
    item: u32,
    spell: u32,
    eligible_race: u8,
    opposed_race: u8,
}

const ALLIANCE_BANNER: FactionToy = FactionToy {
    item: 169275,
    spell: 300104,
    eligible_race: 1,
    opposed_race: 2,
};
const HORDE_BANNER: FactionToy = FactionToy {
    item: 169276,
    spell: 300144,
    eligible_race: 2,
    opposed_race: 1,
};

fn insert_racial_identity(h: &mut Harness, actor: Entity, race: u8) {
    h.app.world_mut().get_mut::<Player>(actor).unwrap().race = race;
    h.app.world_mut().entity_mut(actor).insert((
        UnitFactionTemplate(crate::faction::faction_template_for_race(race)),
        crate::faction::current_pvp_faction_for_race(race),
    ));
}

fn faction_fixture(path: &std::path::Path, race: u8) -> (Harness, u64) {
    let (mut h, character) = setup_for_race(path, race);
    let actor = h.caster;
    insert_racial_identity(&mut h, actor, race);
    (h, character)
}

fn spawn_account_character(h: &mut Harness, race: u8) -> (Entity, Entity) {
    let (character, _) = persistence::create_character(
        &h.app.world().resource::<GameDatabase>().db,
        1,
        "OpposedMage",
        race,
        8,
    )
    .unwrap();
    let actor = spawn_player(&mut h.app, 8, 6.0);
    insert_racial_identity(h, actor, race);
    h.app.world_mut().entity_mut(actor).insert((
        crate::persistence::ClientId(character),
        Level(80),
        UnitLevel(80),
        CombatStatus(false),
    ));
    let connection = spawn_peer_connection(&mut h.app, actor, 2);
    h.app.world_mut().entity_mut(connection).insert((
        InWorld {
            character_id: character,
        },
        MessageSender::<UseToy>::default(),
        MessageReceiver::<UseToy>::default(),
        MessageSender::<ToyResult>::default(),
        MessageReceiver::<ToyResult>::default(),
        MessageSender::<ToyCollectionUpdate>::default(),
        MessageReceiver::<ToyCollectionUpdate>::default(),
    ));
    h.advance(3);
    drain::<ToyCollectionUpdate>(&mut h.app, connection);
    (connection, actor)
}

#[derive(Default)]
struct UseReplies {
    results: Vec<ToyResult>,
    cooldowns: Vec<SpellCooldownUpdate>,
    spells: Vec<SpellGo>,
}

fn request_peer_use(h: &mut Harness, connection: Entity, item: u32) -> UseReplies {
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseToy>>(connection)
        .unwrap()
        .send::<CollectionChannel>(UseToy {
            item_id: item,
            destination: None,
            witness: None,
        });
    let mut replies = UseReplies::default();
    for _ in 0..7 {
        h.advance(1);
        replies
            .results
            .extend(drain::<ToyResult>(&mut h.app, connection));
        replies
            .cooldowns
            .extend(drain::<SpellCooldownUpdate>(&mut h.app, connection));
        replies
            .spells
            .extend(drain::<SpellGo>(&mut h.app, connection));
        drain::<ToyCollectionUpdate>(&mut h.app, connection);
    }
    replies
}

fn prove_learning_refusal(toy: FactionToy) {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = faction_fixture(&dir.path().join("learning.redb"), toy.opposed_race);
    save_learning_inventory(&mut h, character, toy.item);
    let before =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character);
    let owned = toy_store::load(&h.app.world().resource::<GameDatabase>().db, 1);
    let results = learn(&mut h, 0);
    assert_eq!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, character),
        before,
        "wrong-faction learning consumed item{}: {results:?}",
        toy.item
    );
    assert_eq!(
        toy_store::load(&h.app.world().resource::<GameDatabase>().db, 1),
        owned,
        "wrong-faction learning granted account ownership"
    );
    assert!(
        h.inbox
            .inventory_errors
            .iter()
            .any(|error| error.detail.as_deref() == Some("toy race/class restriction")),
        "{:?}; {results:?}",
        h.inbox.inventory_errors
    );
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == toy.spell));
}

fn assert_peer_use_denied(h: &mut Harness, connection: Entity, actor: Entity, toy: FactionToy) {
    let replies = request_peer_use(h, connection, toy.item);
    assert!(
        replies
            .results
            .iter()
            .any(|result| result.operation == ToyOperation::Use
                && result.error.as_deref() == Some("toy race/class restriction")),
        "wrong-faction use results: {:?}; cooldowns{:?}",
        replies.results,
        replies.cooldowns
    );
    assert!(!crate::auras::has_aura(h.app.world(), actor, toy.spell));
    assert!(!replies.spells.iter().any(|go| go.spell_id == toy.spell));
    assert!(
        !replies
            .cooldowns
            .iter()
            .any(|clock| clock.spell_id == toy.spell && clock.duration_ms > 0)
    );
}

fn assert_eligible_use(h: &mut Harness, toy: FactionToy) {
    assert_eq!(
        activate(h, toy.item)[0].error,
        None,
        "eligible original character still uses owned toy"
    );
    h.advance(40);
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(h.caster)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == toy.spell && aura.caster == Some(h.caster.to_bits()))
    );
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|clock| clock.spell_id == toy.spell
                && !clock.is_gcd
                && clock.duration_ms == 10_000)
    );
}

fn prove_owned_use_refusal(toy: FactionToy) {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = faction_fixture(&dir.path().join("owned.redb"), toy.eligible_race);
    learn_effect_toy(&mut h, character, toy.item);
    let owned = toy_store::load(&h.app.world().resource::<GameDatabase>().db, 1);
    assert!(owned.contains_key(&toy.item));
    let (connection, other) = spawn_account_character(&mut h, toy.opposed_race);
    assert_peer_use_denied(&mut h, connection, other, toy);
    assert_eq!(
        toy_store::load(&h.app.world().resource::<GameDatabase>().db, 1),
        owned
    );
    assert_eligible_use(&mut h, toy);
}

#[test]
#[ignore = "item faction qualification; run explicitly"]
fn toy_coverage_item_faction_all54_native_metadata_respect_faction_without_race_mask_rewrites() {
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let templates = crate::item_templates::load_item_templates(&path).unwrap();
    let mut query = conn.prepare("SELECT t.ItemID,i.Flags_1,i.AllowableClass,i.AllowableRaces_0,i.AllowableRaces_1 FROM toy t JOIN item_sparse i ON i.ID=t.ItemID WHERE (i.Flags_1 & 3)!=0 ORDER BY t.ItemID").unwrap();
    let rows: Vec<(u32, i64, i64, i64, i64)> = query
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(rows.iter().any(|row| row.0 == ALLIANCE_BANNER.item));
    assert!(rows.iter().any(|row| row.0 == HORDE_BANNER.item));
    println!(
        "native faction toy matrix: {} rows, both character sides",
        rows.len()
    );
    for (item, flags, classes, low, high) in rows {
        assert_eq!(
            (classes, low, high),
            (-1, -1, -1),
            "source cohort masks item{item}"
        );
        let template = &templates.retail[&item];
        assert_eq!(
            template.allows(8, 1),
            flags & 1 == 0,
            "Alliance character item{item}, flags{flags}"
        );
        assert_eq!(
            template.allows(8, 2),
            flags & 2 == 0,
            "Horde character item{item}, flags{flags}"
        );
    }
    let unrestricted = &templates.retail[&163705];
    assert_eq!(unrestricted.scaling.flags2 & 3, 0);
    assert!(unrestricted.allows(8, 1) && unrestricted.allows(8, 2));
}

#[test]
#[ignore = "item faction qualification; run explicitly"]
fn toy_coverage_item_faction_alliance_learning_refuses_horde_without_consumption() {
    prove_learning_refusal(ALLIANCE_BANNER);
}

#[test]
#[ignore = "item faction qualification; run explicitly"]
fn toy_coverage_item_faction_horde_learning_refuses_alliance_without_consumption() {
    prove_learning_refusal(HORDE_BANNER);
}

#[test]
#[ignore = "item faction qualification; run explicitly"]
fn toy_coverage_item_faction_owned_alliance_toy_refuses_same_account_horde_character() {
    prove_owned_use_refusal(ALLIANCE_BANNER);
}

#[test]
#[ignore = "item faction qualification; run explicitly"]
fn toy_coverage_item_faction_owned_horde_toy_refuses_same_account_alliance_character() {
    prove_owned_use_refusal(HORDE_BANNER);
}
