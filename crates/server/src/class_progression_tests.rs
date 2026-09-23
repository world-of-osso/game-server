//! Real-DB tests against data/world.db; each test skips when the DB is absent.

use std::path::PathBuf;
use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::AppMessageExt;
use rusqlite::{Connection, OpenFlags};

use super::*;
use crate::persistence::{self, temp_database};
use crate::spell_info::load_class_spells;

const HUMAN: u8 = 1;
const ORC: u8 = 2;
const BLOOD_ELF: u8 = 10;
const WARRIOR: u8 = 1;
const PALADIN: u8 = 2;

const CRUSADER_STRIKE: u32 = 35395;
const REDEMPTION: u32 = 7328;
const TEMPLARS_VERDICT: u32 = 85256;
const SPEC_HOLY: u32 = 65;
const SPEC_RETRIBUTION: u32 = 70;
const SPEC_ARMS: u32 = 71;
const SPEC_PALADIN_INITIAL: u32 = 1451;
const SPEC_WARRIOR_INITIAL: u32 = 1446;

fn class_spells() -> Option<ClassSpells> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
    if !path.exists() {
        eprintln!("skipping: {} not found", path.display());
        return None;
    }
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    Some(load_class_spells(&conn).expect("load class spells"))
}

fn known_ids(classes: &ClassSpells, race: u8, class_id: u8, level: u8, spec: u32) -> Vec<u32> {
    let player = Player {
        name: "Probe".into(),
        race,
        class: class_id,
        appearance: default(),
    };
    evaluate(classes, &player, level, spec)
        .1
        .iter()
        .map(|spell| spell.spell_id)
        .collect()
}

#[test]
fn paladin_known_spells_follow_required_level_and_race_mask() {
    let Some(classes) = class_spells() else {
        return;
    };
    let paladin = class_info(&classes, PALADIN).unwrap();
    let required = |spell_id| {
        paladin
            .learnable
            .iter()
            .find(|spell| spell.spell_id == spell_id)
            .unwrap()
            .required_level
    };
    assert_eq!(required(CRUSADER_STRIKE), 1);
    assert_eq!(required(REDEMPTION), 13);

    let level_10 = known_ids(&classes, HUMAN, PALADIN, 10, 0);
    assert!(level_10.contains(&CRUSADER_STRIKE));
    assert!(!level_10.contains(&REDEMPTION));
    assert!(known_ids(&classes, HUMAN, PALADIN, 13, 0).contains(&REDEMPTION));

    // Summon Warhorse 13819 is Human/Dwarf/Dark Iron; Thalassian Warhorse 34769 is Blood Elf.
    assert!(level_10.contains(&13819));
    assert!(!level_10.contains(&34769));
    let blood_elf = known_ids(&classes, BLOOD_ELF, PALADIN, 10, 0);
    assert!(blood_elf.contains(&34769));
    assert!(!blood_elf.contains(&13819));
}

/// Server-to-client messages drained every frame: unread receiver buffers are
/// dropped on the next receive.
#[derive(Resource)]
struct Inbox<M>(Vec<(Entity, M)>);

fn collect<M: Message>(
    mut receivers: Query<(Entity, &mut MessageReceiver<M>)>,
    mut inbox: ResMut<Inbox<M>>,
) {
    for (connection, mut receiver) in &mut receivers {
        inbox
            .0
            .extend(receiver.receive().map(|message| (connection, message)));
    }
}

fn add_inbox<M: Message + serde::Serialize + serde::de::DeserializeOwned>(app: &mut App) {
    app.register_message::<M>();
    app.insert_resource(Inbox::<M>(Vec::new()));
    app.add_systems(Last, collect::<M>);
}

fn setup_app(db: redb::Database, classes: ClassSpells) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    add_inbox::<KnownSpellsSnapshot>(&mut app);
    add_inbox::<SpellsLearned>(&mut app);
    add_inbox::<SpellsUnlearned>(&mut app);
    add_inbox::<SpecializationChanged>(&mut app);
    add_inbox::<ActionBarSnapshot>(&mut app);
    app.register_message::<SetSpecialization>();
    app.register_message::<SetActionButton>();
    app.insert_resource(GameDatabase { db });
    app.insert_resource(classes);
    app.add_plugins(ClassProgressionPlugin);
    app.finish();
    app.cleanup();
    app
}

fn spawn_player(app: &mut App, character_id: u64, race: u8, class: u8, level: u8) -> Entity {
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Probe".into(),
                race,
                class,
                appearance: default(),
            },
            Level(level),
            UnitLevel(level),
            ClientId(character_id),
        ))
        .id();
    app.world_mut().spawn((
        RemoteId(PeerId::Local(character_id)),
        Connected,
        HostClient { buffer: Vec::new() },
        Authenticated {
            account_id: character_id,
        },
        PlayerLink(player),
        (
            MessageSender::<KnownSpellsSnapshot>::default(),
            MessageReceiver::<KnownSpellsSnapshot>::default(),
            MessageSender::<SpellsLearned>::default(),
            MessageReceiver::<SpellsLearned>::default(),
            MessageSender::<SpellsUnlearned>::default(),
            MessageReceiver::<SpellsUnlearned>::default(),
        ),
        (
            MessageSender::<SpecializationChanged>::default(),
            MessageReceiver::<SpecializationChanged>::default(),
            MessageSender::<ActionBarSnapshot>::default(),
            MessageReceiver::<ActionBarSnapshot>::default(),
            MessageSender::<SetSpecialization>::default(),
            MessageReceiver::<SetSpecialization>::default(),
            MessageSender::<SetActionButton>::default(),
            MessageReceiver::<SetActionButton>::default(),
        ),
    ));
    player
}

fn advance(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

fn connection_of(app: &mut App, player: Entity) -> Entity {
    app.world_mut()
        .query::<(Entity, &PlayerLink)>()
        .iter(app.world())
        .find_map(|(entity, link)| (link.0 == player).then_some(entity))
        .unwrap()
}

fn received<M: Message>(app: &mut App, player: Entity) -> Vec<M> {
    let connection = connection_of(app, player);
    let mut inbox = app.world_mut().resource_mut::<Inbox<M>>();
    let (mine, others) = std::mem::take(&mut inbox.0)
        .into_iter()
        .partition(|(entity, _)| *entity == connection);
    inbox.0 = others;
    mine.into_iter().map(|(_, message)| message).collect()
}

fn send_request<M: Message>(app: &mut App, player: Entity, message: M) {
    let connection = connection_of(app, player);
    app.world_mut()
        .get_mut::<MessageSender<M>>(connection)
        .unwrap()
        .send::<TalentChannel>(message);
    advance(app);
}

/// Drain the enter-world messages.
fn drain(app: &mut App, player: Entity) {
    received::<KnownSpellsSnapshot>(app, player);
    received::<SpecializationChanged>(app, player);
    received::<ActionBarSnapshot>(app, player);
}

fn stored(app: &App, character_id: u64) -> ProgressionData {
    load_progression(&app.world().resource::<GameDatabase>().db, character_id).unwrap()
}

#[test]
fn new_warrior_enters_world_with_known_spells_and_default_bar() {
    let Some(classes) = class_spells() else {
        return;
    };
    let db = temp_database();
    let (character_id, _) = persistence::create_character(&db, 1, "Garrosh", ORC, WARRIOR).unwrap();
    let mut app = setup_app(db, classes);
    let player = spawn_player(&mut app, character_id, ORC, WARRIOR, 1);
    advance(&mut app);

    // Warrior 137047, Parry 3127, Block 123829 and Initial Warrior 325446 are passive.
    assert_eq!(
        received::<KnownSpellsSnapshot>(&mut app, player),
        vec![KnownSpellsSnapshot {
            spells: vec![137047, 1464, 3127, 88163, 123829, 325446],
        }]
    );
    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged {
            spec_id: SPEC_WARRIOR_INITIAL,
        }]
    );
    let default_bar = vec![(0, ActionRef::Spell(1464)), (1, ActionRef::Spell(88163))];
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot {
            slots: default_bar.clone(),
        }]
    );
    assert_eq!(
        stored(&app, character_id),
        ProgressionData {
            spec_id: SPEC_WARRIOR_INITIAL,
            action_bar: vec![
                (0, ActionRefData::Spell(1464)),
                (1, ActionRefData::Spell(88163))
            ],
        }
    );
    assert!(
        app.world()
            .get::<KnownSpells>(player)
            .unwrap()
            .contains(1464)
    );
}

#[test]
fn paladin_switch_retribution_to_holy_sends_deltas_and_prunes_bar() {
    let Some(classes) = class_spells() else {
        return;
    };
    let db = temp_database();
    save_progression(
        &db,
        7,
        &ProgressionData {
            spec_id: SPEC_RETRIBUTION,
            action_bar: vec![
                (0, ActionRefData::Spell(CRUSADER_STRIKE)),
                (5, ActionRefData::Spell(TEMPLARS_VERDICT)),
            ],
        },
    );
    let mut app = setup_app(db, classes);
    let player = spawn_player(&mut app, 7, HUMAN, PALADIN, 20);
    advance(&mut app);
    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged {
            spec_id: SPEC_RETRIBUTION,
        }]
    );
    drain(&mut app, player);

    send_request(&mut app, player, SetSpecialization { spec_id: SPEC_HOLY });

    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged { spec_id: SPEC_HOLY }]
    );
    assert_eq!(
        received::<SpellsUnlearned>(&mut app, player),
        vec![SpellsUnlearned {
            spells: vec![
                85256, 86539, 137027, 162698, 267316, 342348, 403876, 404752, 412314, 462096,
                404834, 384376, 1258011,
            ],
        }]
    );
    assert_eq!(
        received::<SpellsLearned>(&mut app, player),
        vec![SpellsLearned {
            spells: vec![
                53563, 86103, 137029, 162701, 183997, 212056, 275773, 4987, 289941, 53651, 498,
                82326, 415091, 428076, 440013, 462093, 231642, 1258016,
            ],
        }]
    );
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot {
            slots: vec![(0, ActionRef::Spell(CRUSADER_STRIKE))],
        }]
    );
    assert_eq!(
        stored(&app, 7),
        ProgressionData {
            spec_id: SPEC_HOLY,
            action_bar: vec![(0, ActionRefData::Spell(CRUSADER_STRIKE))],
        }
    );
}

#[test]
fn set_specialization_rejects_other_class_and_below_unlock_level() {
    let Some(classes) = class_spells() else {
        return;
    };
    let mut app = setup_app(temp_database(), classes);
    let low = spawn_player(&mut app, 1, HUMAN, PALADIN, 5);
    let high = spawn_player(&mut app, 2, HUMAN, PALADIN, 20);
    advance(&mut app);
    drain(&mut app, low);
    drain(&mut app, high);

    send_request(&mut app, low, SetSpecialization { spec_id: SPEC_HOLY });
    send_request(&mut app, high, SetSpecialization { spec_id: SPEC_ARMS });

    assert_eq!(
        received::<SpecializationChanged>(&mut app, low),
        vec![SpecializationChanged {
            spec_id: SPEC_PALADIN_INITIAL,
        }]
    );
    assert_eq!(
        received::<SpecializationChanged>(&mut app, high),
        vec![SpecializationChanged { spec_id: SPEC_HOLY }]
    );
    assert!(received::<SpellsLearned>(&mut app, low).is_empty());
    assert!(received::<SpellsLearned>(&mut app, high).is_empty());
    assert_eq!(
        app.world().get::<ActiveSpec>(high),
        Some(&ActiveSpec(SPEC_HOLY))
    );
}

#[test]
fn set_action_button_validates_slot_and_known_spell() {
    let Some(classes) = class_spells() else {
        return;
    };
    let mut app = setup_app(temp_database(), classes);
    let player = spawn_player(&mut app, 3, ORC, WARRIOR, 1);
    advance(&mut app);
    drain(&mut app, player);
    let default_bar = vec![(0, ActionRef::Spell(1464)), (1, ActionRef::Spell(88163))];

    send_request(
        &mut app,
        player,
        SetActionButton {
            slot: 3,
            action: Some(ActionRef::Spell(CRUSADER_STRIKE)),
        },
    );
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot {
            slots: default_bar.clone(),
        }]
    );
    send_request(
        &mut app,
        player,
        SetActionButton {
            slot: 120,
            action: Some(ActionRef::Spell(1464)),
        },
    );
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot { slots: default_bar }]
    );

    send_request(
        &mut app,
        player,
        SetActionButton {
            slot: 119,
            action: Some(ActionRef::Item(6948)),
        },
    );
    send_request(
        &mut app,
        player,
        SetActionButton {
            slot: 0,
            action: None,
        },
    );
    send_request(
        &mut app,
        player,
        SetActionButton {
            slot: 7,
            action: Some(ActionRef::Spell(1464)),
        },
    );
    assert!(received::<ActionBarSnapshot>(&mut app, player).is_empty());
    assert_eq!(
        stored(&app, 3).action_bar,
        vec![
            (1, ActionRefData::Spell(88163)),
            (7, ActionRefData::Spell(1464)),
            (119, ActionRefData::Item(6948)),
        ]
    );
}

#[test]
fn level_changes_send_learned_spells_and_leave_initial_spec_at_unlock() {
    let Some(classes) = class_spells() else {
        return;
    };
    let mut app = setup_app(temp_database(), classes);
    let player = spawn_player(&mut app, 4, HUMAN, PALADIN, 9);
    advance(&mut app);
    drain(&mut app, player);

    set_live_level(app.world_mut(), 4, 10);
    advance(&mut app);

    assert_eq!(app.world().get::<UnitLevel>(player), Some(&UnitLevel(10)));
    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged { spec_id: SPEC_HOLY }]
    );
    assert_eq!(
        received::<SpellsUnlearned>(&mut app, player),
        vec![SpellsUnlearned {
            spells: vec![417383],
        }]
    );
    let learned = received::<SpellsLearned>(&mut app, player);
    assert_eq!(learned.len(), 1);
    // Divine Shield 642 and Sense Undead 5502 are level 10; Beacon of Light 53563 is Holy.
    for spell_id in [642, 5502, 53563, 183997] {
        assert!(learned[0].spells.contains(&spell_id), "missing {spell_id}");
    }
    assert!(!learned[0].spells.contains(&REDEMPTION));

    set_live_level(app.world_mut(), 4, 13);
    advance(&mut app);
    assert_eq!(
        received::<SpellsLearned>(&mut app, player),
        vec![SpellsLearned {
            spells: vec![REDEMPTION],
        }]
    );
    assert!(received::<SpecializationChanged>(&mut app, player).is_empty());
    assert_eq!(stored(&app, 4).spec_id, SPEC_HOLY);
}

#[test]
fn admin_set_level_persists_and_updates_online_player() {
    use bevy::ecs::system::RunSystemOnce;

    use crate::admin_handlers::handle_set_level;
    use crate::admin_types::AdminResponse;

    let db = temp_database();
    let (character_id, _) = persistence::create_character(&db, 1, "Uther", HUMAN, PALADIN).unwrap();
    let mut app = App::new();
    app.insert_resource(GameDatabase { db });
    let player = app
        .world_mut()
        .spawn((ClientId(character_id), Level(1), UnitLevel(1)))
        .id();

    let set_level = |level| {
        move |db: Res<GameDatabase>, mut commands: Commands| {
            handle_set_level(&db, &mut commands, "Uther", level)
        }
    };
    let rejected = app.world_mut().run_system_once(set_level(91)).unwrap();
    assert!(matches!(rejected, AdminResponse::Error(message) if message == "level must be 1..=90"));
    let accepted = app.world_mut().run_system_once(set_level(12)).unwrap();
    assert!(matches!(
        accepted,
        AdminResponse::LevelSet { level: 12, .. }
    ));

    assert_eq!(app.world().get::<Level>(player), Some(&Level(12)));
    assert_eq!(app.world().get::<UnitLevel>(player), Some(&UnitLevel(12)));
    let db = &app.world().resource::<GameDatabase>().db;
    assert_eq!(
        persistence::load_character(db, character_id).unwrap().level,
        12
    );
}
