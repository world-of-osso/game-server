//! Real-DB tests against data/world.db.

use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::AppMessageExt;
use rusqlite::{Connection, OpenFlags};

use super::*;
use crate::persistence::{self, temp_database};
use crate::persistence_progression::load_trait_configs;
use crate::player_create_info::PlayerCreateInfos;
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
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    Some(load_class_spells(&conn).expect("load class spells"))
}

/// A character created with the data/world.db kit (items and create actions).
fn create_with_kit(db: &redb::Database, name: &str, race: u8, class: u8) -> Option<u64> {
    let kit = crate::player_create_info::OwnedKit::from_world_db()?;
    let created = persistence::create_character_with_appearance(
        db,
        1,
        name,
        race,
        class,
        Default::default(),
        &kit.kit(),
    );
    Some(created.unwrap().0)
}

fn known_ids(classes: &ClassSpells, race: u8, class_id: u8, level: u8, spec: u32) -> Vec<u32> {
    let player = Player {
        name: "Probe".into(),
        race,
        class: class_id,
        appearance: default(),
    };
    evaluate(classes, &player, level, spec, |_| Vec::new())
        .known
        .iter()
        .map(|spell| spell.spell_id)
        .collect()
}

/// Summon Imp follows DB2: SkillLineAbility 24651 (Warlock skill 849, AcquireMethod 2)
/// auto-learns 688 at SpellLevels.SpellLevel 3.
#[test]
fn warlock_learns_summon_imp_at_level_three() {
    let classes = class_spells().expect("world.db");
    assert!(!known_ids(&classes, HUMAN, 9, 2, 0).contains(&688));
    assert!(known_ids(&classes, HUMAN, 9, 3, 0).contains(&688));
    assert!(!known_ids(&classes, HUMAN, WARRIOR, 3, 0).contains(&688));
}

/// Retail SkillLineAbility 35949 (Hunter 795, method 4) and SpellLevels 334 (level 5).
#[test]
fn pet_dismiss_is_a_hunter_level_five_spell_not_a_warlock_spell() {
    let classes = class_spells().expect("world.db");
    assert!(!known_ids(&classes, HUMAN, 3, 4, 0).contains(&2641));
    assert!(known_ids(&classes, HUMAN, 3, 5, 0).contains(&2641));
    assert!(known_ids(&classes, HUMAN, 3, 10, 0).contains(&2641));
    assert!(!known_ids(&classes, HUMAN, 9, 3, 0).contains(&2641));
    assert!(!known_ids(&classes, HUMAN, 9, 10, 0).contains(&2641));
}

#[test]
fn guild_perks_real_membership_grants_rebuilds_revokes_and_reopens_without_touching_class_spells() {
    use std::collections::BTreeSet;
    let perks: BTreeSet<u32> = [78633, 83944, 83958, 83950, 83951].into_iter().collect();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("guild-perks.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let owner = create_with_kit(&db, "Perkleader", HUMAN, PALADIN).unwrap();
    let member = create_with_kit(&db, "Perkmember", HUMAN, PALADIN).unwrap();
    crate::persistence_character::set_character_guild_by_id(&db, owner, 7, "Actual Perks").unwrap();
    let mut app = setup_progression_app(db, class_spells().unwrap(), false, true);
    let player = spawn_player(&mut app, member, HUMAN, PALADIN, 60);
    let connection = connection_of(&mut app, player);
    app.world_mut()
        .entity_mut(connection)
        .insert(crate::auth::InWorld {
            character_id: member,
        });
    advance(&mut app);
    drain(&mut app, player);
    received::<SpellsLearned>(&mut app, player);
    let baseline: BTreeSet<_> = app
        .world()
        .get::<KnownSpells>(player)
        .unwrap()
        .iter()
        .collect();
    assert!(!baseline.is_empty() && baseline.is_disjoint(&perks));
    {
        let db = &app.world().resource::<GameDatabase>().db;
        crate::persistence_character::set_pending_guild_invite(
            db,
            member,
            7,
            "Actual Perks",
            "Perkleader",
        );
        crate::persistence_character::accept_pending_guild_invite(db, member).unwrap();
        let guild = crate::persistence_character::load_character_guild(db, member).unwrap();
        app.world_mut().entity_mut(player).insert(guild);
    }
    advance(&mut app);
    let joined: BTreeSet<_> = app
        .world()
        .get::<KnownSpells>(player)
        .unwrap()
        .iter()
        .collect();
    assert_eq!(
        joined,
        baseline.union(&perks).copied().collect(),
        "Actual guild join must grant exactly the installed perks"
    );
    let learned: BTreeSet<_> = received::<SpellsLearned>(&mut app, player)
        .into_iter()
        .flat_map(|reply| reply.spells)
        .collect();
    assert_eq!(learned, perks);
    received::<SpellsUnlearned>(&mut app, player);
    let previous_spec = app.world().get::<ActiveSpec>(player).unwrap().0;
    let next_spec = if previous_spec == SPEC_HOLY {
        SPEC_RETRIBUTION
    } else {
        SPEC_HOLY
    };
    send_request(&mut app, player, SetSpecialization { spec_id: next_spec });
    assert_eq!(app.world().get::<ActiveSpec>(player).unwrap().0, next_spec);
    let rebuilt: BTreeSet<_> = app
        .world()
        .get::<KnownSpells>(player)
        .unwrap()
        .iter()
        .collect();
    assert!(
        perks.is_subset(&rebuilt),
        "Ordinary spec rebuild must preserve guild grants"
    );
    assert!(
        received::<SpellsUnlearned>(&mut app, player)
            .iter()
            .all(|reply| reply.spells.iter().all(|id| !perks.contains(id))),
        "Respec must not transiently unlearn guild perks"
    );
    let expected_after_leave: BTreeSet<_> = rebuilt.difference(&perks).copied().collect();
    crate::persistence_character::remove_character_guild(
        &app.world().resource::<GameDatabase>().db,
        member,
        member,
    );
    app.world_mut()
        .entity_mut(player)
        .remove::<shared::components::GuildMembership>();
    advance(&mut app);
    let departed: BTreeSet<_> = app
        .world()
        .get::<KnownSpells>(player)
        .unwrap()
        .iter()
        .collect();
    assert_eq!(
        departed, expected_after_leave,
        "Actual quit removes perks, not class spells"
    );
    let unlearned: BTreeSet<_> = received::<SpellsUnlearned>(&mut app, player)
        .into_iter()
        .flat_map(|reply| reply.spells)
        .collect();
    assert_eq!(unlearned, perks);
    drop(app);
    let db = redb::Database::open(&path).unwrap();
    assert!(crate::persistence_character::load_character_guild(&db, member).is_none());
    let mut reopened = setup_progression_app(db, class_spells().unwrap(), false, true);
    let player = spawn_player(&mut reopened, member, HUMAN, PALADIN, 60);
    let connection = connection_of(&mut reopened, player);
    reopened
        .world_mut()
        .entity_mut(connection)
        .insert(crate::auth::InWorld {
            character_id: member,
        });
    advance(&mut reopened);
    let after_restart: BTreeSet<_> = reopened
        .world()
        .get::<KnownSpells>(player)
        .unwrap()
        .iter()
        .collect();
    assert_eq!(after_restart, expected_after_leave);
}

#[test]
fn guild_perks_member_login_forced_removal_and_offline_removal_reconcile_after_reopen() {
    use std::collections::BTreeSet;
    let perks: BTreeSet<_> = [78633, 83944, 83958, 83950, 83951].into_iter().collect();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("guild-perks-removal.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let owner = create_with_kit(&db, "Perkowner", HUMAN, PALADIN).unwrap();
    let member = create_with_kit(&db, "Perklogin", HUMAN, PALADIN).unwrap();
    crate::persistence_character::set_character_guild_by_id(&db, owner, 7, "Actual Perks").unwrap();
    accept_perk_membership(&db, member);
    let mut app = setup_progression_app(db, class_spells().unwrap(), false, true);
    let player = spawn_persisted_perk_member(&mut app, member);
    let joined: BTreeSet<_> = app
        .world()
        .get::<KnownSpells>(player)
        .unwrap()
        .iter()
        .collect();
    assert!(
        perks.is_subset(&joined),
        "Existing member login derives all perks"
    );
    assert_eq!(
        app.world()
            .get::<KnownSpells>(player)
            .unwrap()
            .iter()
            .count(),
        joined.len()
    );
    let class_knowledge: BTreeSet<_> = joined.difference(&perks).copied().collect();
    received::<SpellsUnlearned>(&mut app, player);
    crate::persistence_character::remove_character_guild(
        &app.world().resource::<GameDatabase>().db,
        member,
        owner,
    );
    app.world_mut()
        .entity_mut(player)
        .remove::<shared::components::GuildMembership>();
    advance(&mut app);
    let after_kick: BTreeSet<_> = app
        .world()
        .get::<KnownSpells>(player)
        .unwrap()
        .iter()
        .collect();
    assert_eq!(after_kick, class_knowledge);
    let revoked: BTreeSet<_> = received::<SpellsUnlearned>(&mut app, player)
        .into_iter()
        .flat_map(|message| message.spells)
        .collect();
    assert_eq!(revoked, perks);
    accept_perk_membership(&app.world().resource::<GameDatabase>().db, member);
    let guild = crate::persistence_character::load_character_guild(
        &app.world().resource::<GameDatabase>().db,
        member,
    )
    .unwrap();
    app.world_mut().entity_mut(player).insert(guild);
    advance(&mut app);
    let rejoined: BTreeSet<_> = app
        .world()
        .get::<KnownSpells>(player)
        .unwrap()
        .iter()
        .collect();
    assert_eq!(rejoined, joined, "Rejoin restores perks exactly once");
    drop(app);
    let db = redb::Database::open(&path).unwrap();
    crate::persistence_character::remove_character_guild(&db, member, owner);
    drop(db);
    let db = redb::Database::open(&path).unwrap();
    assert!(crate::persistence_character::load_character_guild(&db, member).is_none());
    let mut app = setup_progression_app(db, class_spells().unwrap(), false, true);
    let player = spawn_persisted_perk_member(&mut app, member);
    let after_offline_kick: BTreeSet<_> = app
        .world()
        .get::<KnownSpells>(player)
        .unwrap()
        .iter()
        .collect();
    assert_eq!(after_offline_kick, class_knowledge);
}

fn accept_perk_membership(db: &redb::Database, member: u64) {
    crate::persistence_character::set_pending_guild_invite(
        db,
        member,
        7,
        "Actual Perks",
        "Perkowner",
    );
    crate::persistence_character::accept_pending_guild_invite(db, member).unwrap();
}

fn spawn_persisted_perk_member(app: &mut App, character_id: u64) -> Entity {
    let player = spawn_player(app, character_id, HUMAN, PALADIN, 60);
    let connection = connection_of(app, player);
    app.world_mut()
        .entity_mut(connection)
        .insert(crate::auth::InWorld { character_id });
    let guild = crate::persistence_character::load_character_guild(
        &app.world().resource::<GameDatabase>().db,
        character_id,
    );
    if let Some(guild) = guild {
        app.world_mut().entity_mut(player).insert(guild);
    }
    advance(app);
    player
}

#[test]
fn guild_perks_preserve_pvp_replacements_and_profession_spell_identity() {
    use std::collections::BTreeSet;
    // The same native210256→853 replacement used by pvp_talents' knowledge controls;
    //2259 is Alchemy profession knowledge, not a guild entitlement.
    let mut known = KnownSpells::fixture(vec![2259, 853, 35395]);
    let perks = [78633, 83944, 83958, 83950, 83951];
    known.set_guild_grants(&perks);
    known.set_pvp_grants(vec![(210256, 853)]);
    let active: BTreeSet<_> = known.iter().collect();
    let expected: BTreeSet<_> = [2259, 35395, 210256].into_iter().chain(perks).collect();
    assert_eq!(active, expected);
    assert_eq!(known.iter().count(), expected.len());
    known.set_guild_grants(&[]);
    assert_eq!(
        known.iter().collect::<BTreeSet<_>>(),
        [2259, 35395, 210256].into_iter().collect()
    );
    known.set_pvp_grants(Vec::new());
    assert_eq!(
        known.iter().collect::<BTreeSet<_>>(),
        [2259, 853, 35395].into_iter().collect()
    );
    known.set_guild_grants(&perks);
    assert!(perks.iter().all(|id| known.contains(*id)));
    assert!(known.contains(2259) && known.contains(853));
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
/// dropped on the next receive. Collected in `First`, before lightyear's `Last`
/// clear/send_local chain, so every locally sent message is read exactly once.
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
    app.add_systems(First, collect::<M>);
}

fn setup_app(db: redb::Database, classes: ClassSpells) -> App {
    setup_progression_app(db, classes, false, false)
}

fn setup_progression_app(
    db: redb::Database,
    classes: ClassSpells,
    achievements: bool,
    perks: bool,
) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    add_inbox::<KnownSpellsSnapshot>(&mut app);
    add_inbox::<shared::protocol::ProfessionSnapshot>(&mut app);
    add_inbox::<SpellsLearned>(&mut app);
    add_inbox::<SpellsUnlearned>(&mut app);
    add_inbox::<SpecializationChanged>(&mut app);
    add_inbox::<ActionBarSnapshot>(&mut app);
    add_inbox::<TraitConfigSnapshot>(&mut app);
    add_inbox::<TraitCommitResult>(&mut app);
    add_inbox::<shared::protocol::TraitLoadoutsSnapshot>(&mut app);
    add_inbox::<shared::protocol::LogXpGain>(&mut app);
    add_inbox::<shared::protocol::PlayerXpUpdate>(&mut app);
    add_inbox::<shared::protocol::RestStateUpdate>(&mut app);
    app.register_message::<SetSpecialization>();
    app.register_message::<SetActionButton>();
    app.register_message::<CommitTraitConfig>();
    app.register_message::<shared::protocol::TraitLoadoutRequest>();
    app.insert_resource(GameDatabase::new(db));
    app.insert_resource(classes);
    app.add_plugins((ClassProgressionPlugin, crate::profession::ProfessionPlugin));
    app.add_plugins(crate::experience::ExperiencePlugin);
    if achievements {
        app.add_plugins(crate::achievement::AchievementPlugin);
    }
    if perks {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
        app.add_plugins(crate::guild_perks::GuildPerksPlugin::load(&root).unwrap());
    }
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
            crate::maps::InMap::continent(0),
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
        (
            MessageSender::<TraitConfigSnapshot>::default(),
            MessageReceiver::<TraitConfigSnapshot>::default(),
            MessageSender::<TraitCommitResult>::default(),
            MessageReceiver::<TraitCommitResult>::default(),
            MessageSender::<CommitTraitConfig>::default(),
            MessageReceiver::<CommitTraitConfig>::default(),
            MessageSender::<shared::protocol::TraitLoadoutRequest>::default(),
            MessageReceiver::<shared::protocol::TraitLoadoutRequest>::default(),
            MessageSender::<shared::protocol::TraitLoadoutsSnapshot>::default(),
            MessageReceiver::<shared::protocol::TraitLoadoutsSnapshot>::default(),
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
    received::<TraitConfigSnapshot>(app, player);
    received::<shared::protocol::TraitLoadoutsSnapshot>(app, player);
}

fn stored(app: &App, character_id: u64) -> ProgressionData {
    load_progression(&app.world().resource::<GameDatabase>().db, character_id).unwrap()
}

/// TrinityCore `Player::LearnSpecializationSpells` skips spec spells whose
/// `SpellLevel` is above the player level: Arms gets Mastery: Master of Arms
/// 1258398 at 10, Seasoned Soldier 279423 at 11 and Plate Specialization 86101 at 27.
#[test]
fn spec_spells_wait_for_their_spell_level() {
    let Some(classes) = class_spells() else {
        return;
    };
    let level_10 = known_ids(&classes, HUMAN, WARRIOR, 10, SPEC_ARMS);
    assert!(level_10.contains(&260708), "Sweeping Strikes (level 0)");
    assert!(level_10.contains(&1258398), "Mastery: Master of Arms (10)");
    assert!(!level_10.contains(&279423), "Seasoned Soldier is level 11");
    assert!(
        !level_10.contains(&86101),
        "Plate Specialization is level 27"
    );

    let level_27 = known_ids(&classes, HUMAN, WARRIOR, 27, SPEC_ARMS);
    assert!(level_27.contains(&279423));
    assert!(level_27.contains(&86101));
}

/// Retail pushes each newly learned active spell to the first empty main
/// action bar slot (`SPELL_PUSHED_TO_ACTIONBAR`, `IconIntroTracker`).
#[test]
fn learned_spells_fill_first_empty_main_bar_slots() {
    const SLAM: u32 = 1464;
    const ATTACK: u32 = 88163;
    const CHARGE: u32 = 100;
    const SHIELD_SLAM: u32 = 23922;
    const HAMSTRING: u32 = 1715;
    const WILL_TO_SURVIVE: u32 = 59752;
    const RECUPERATE: u32 = 1231411;
    let Some(classes) = class_spells() else {
        return;
    };
    let db = temp_database();
    let Some(character_id) = create_with_kit(&db, "Varian", HUMAN, WARRIOR) else {
        return;
    };
    let mut app = setup_app(db, classes);
    let player = spawn_player(&mut app, character_id, HUMAN, WARRIOR, 1);
    advance(&mut app);
    drain(&mut app, player);
    // Occupy slot 2 so Charge skips it.
    send_request(
        &mut app,
        player,
        SetActionButton {
            slot: 2,
            action: Some(ActionRef::Item(6948)),
        },
    );

    set_live_level(app.world_mut(), character_id, 2);
    advance(&mut app);
    // playercreateinfo_action (1, 1): Slam, Attack, Will to Survive, Recuperate, all on
    // the main bar: Retail warrior stances page no bonus bar (retail_action_bars.py).
    let charge_bar = vec![
        (0, ActionRef::Spell(SLAM)),
        (1, ActionRef::Spell(ATTACK)),
        (2, ActionRef::Item(6948)),
        (3, ActionRef::Spell(CHARGE)),
        (9, ActionRef::Spell(WILL_TO_SURVIVE)),
        (10, ActionRef::Spell(RECUPERATE)),
    ];
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot {
            slots: charge_bar.clone(),
        }]
    );

    set_live_level(app.world_mut(), character_id, 4);
    advance(&mut app);
    let mut bar = charge_bar;
    bar.extend([
        (4, ActionRef::Spell(SHIELD_SLAM)),
        (5, ActionRef::Spell(HAMSTRING)),
    ]);
    bar.sort_by_key(|&(slot, _)| slot);
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot { slots: bar.clone() }]
    );
    let saved: Vec<_> = stored(&app, character_id)
        .action_bar
        .into_iter()
        .map(|(slot, action)| (slot, action_from_data(action)))
        .collect();
    assert_eq!(saved, bar);
}

/// Player::Create (Player.cpp:505-511): a new Orc Warrior knows its class and
/// default skill line spells and enters with its playercreateinfo_action buttons.
#[test]
fn new_orc_warrior_enters_world_with_default_skill_spells_and_create_actions() {
    let Some(classes) = class_spells() else {
        return;
    };
    let db = temp_database();
    let Some(character_id) = create_with_kit(&db, "Garrosh", ORC, WARRIOR) else {
        return;
    };
    let mut app = setup_app(db, classes);
    let player = spawn_player(&mut app, character_id, ORC, WARRIOR, 1);
    advance(&mut app);

    let known = received::<KnownSpellsSnapshot>(&mut app, player);
    assert_eq!(known.len(), 1);
    let known = &known[0].spells;
    // The class line first: Warrior 137047, Slam 1464, Parry 3127, Attack 88163, Block 123829.
    assert_eq!(known[..5], [137047, 1464, 3127, 88163, 123829]);
    // Default skill lines: Racial - Orc (Blood Fury 20572, Hardiness 20573, Command
    // 21563, Languages 79743), Language: Orcish 669, One-Handed Swords 201, GENERIC
    // Auto Attack 6603, All Classes Recuperate 1231411.
    for spell in [20572, 20573, 21563, 79743, 669, 201, 6603, 1231411] {
        assert!(known.contains(&spell), "Orc Warrior lacks {spell}");
    }
    // Garrison Enchanting 161211 (line 1830, AutomaticSkillRank 35) needs a skill value
    // the level 1 starting value 1 does not reach; Language Common 668 is not Orcish.
    for spell in [161211, 668] {
        assert!(!known.contains(&spell), "Orc Warrior knows {spell}");
    }
    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged {
            spec_id: SPEC_WARRIOR_INITIAL,
        }]
    );
    // playercreateinfo_action (2, 1): Slam, Attack, Blood Fury, Recuperate.
    let create_actions = [(0, 1464), (1, 88163), (9, 20572), (10, 1231411)];
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot {
            slots: create_actions
                .iter()
                .map(|&(slot, spell)| (slot, ActionRef::Spell(spell)))
                .collect(),
        }]
    );
    assert_eq!(
        stored(&app, character_id),
        ProgressionData {
            spec_id: SPEC_WARRIOR_INITIAL,
            action_bar: create_actions
                .iter()
                .map(|&(slot, spell)| (slot, ActionRefData::Spell(spell)))
                .collect(),
        }
    );
}

/// Create action buttons that entering the world prunes (`init_player_progression`:
/// spell overrides first, then unknown spells) for each playable race/class at its
/// start level.
fn pruned_create_actions(
    classes: &ClassSpells,
    create_info: &PlayerCreateInfos,
) -> Vec<(u8, u8, u8, u32)> {
    let mut pruned = Vec::new();
    for (race, class) in crate::race_class::playable_pairs() {
        let level = crate::race_class::start_level(race, class);
        let player = Player {
            name: "Probe".into(),
            race,
            class,
            appearance: Default::default(),
        };
        let evaluated = evaluate(classes, &player, level, 0, |_| Vec::new());
        let known = KnownSpells(
            evaluated.known.iter().map(|spell| spell.spell_id).collect(),
            Vec::new(),
            Vec::new(),
        );
        let actions = &create_info.get(race, class).unwrap().actions;
        let mut bar: BTreeMap<u8, ActionRef> = actions
            .iter()
            .map(|&(slot, action)| (slot, action_from_data(action)))
            .collect();
        apply_spell_overrides(&mut bar, &[], &evaluated.traits.overrides, &known);
        prune_unknown_spells(&mut bar, &known);
        for &(slot, action) in actions {
            if let (ActionRefData::Spell(spell), None) = (action, bar.get(&slot)) {
                pruned.push((race, class, slot, spell));
            }
        }
    }
    pruned.sort();
    pruned
}

/// TrinityCore keeps a button whose spell the player does not know
/// (`IsActionButtonDataValid` only checks that the spell exists); the server prunes
/// it on entering the world. Below level 10 that drops: the Pandaren Warrior's
/// buttons 11 and 83 (TDB rows of type spell with the Small Rice Cake item id 77273,
/// spell Lava Eruption), every Death Knight's Blood Boil 50842 (SpellLevel 17, the
/// Death Knight starts at 8) and the Demon Hunter's Felsaber 200175 (a Mounts line
/// spell nothing teaches).
#[test]
fn create_action_spells_below_level_10_survive_entering_the_world() {
    let Some(classes) = class_spells() else {
        return;
    };
    let Some(kit) = crate::player_create_info::OwnedKit::from_world_db() else {
        return;
    };
    let pruned: Vec<_> = pruned_create_actions(&classes, &kit.create_info)
        .into_iter()
        .filter(|&(race, class, _, _)| crate::race_class::start_level(race, class) < 10)
        .collect();
    // Button 83's copy (the WotLK battle stance page) is dropped by retail_action_bars.py.
    let mut expected = vec![(24, 1, 11, 77273)];
    for race in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 22] {
        expected.push((race, 6, 4, 50842));
    }
    expected.extend([(4, 12, 11, 200175), (10, 12, 11, 200175)]);
    expected.sort();
    assert_eq!(pruned, expected);
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
    // The last two of each list are granted talents: Retribution's Hammer of
    // Wrath 1241288 and Greater Judgment 231663; Holy's Lay on Hands 633 and
    // Divine Steed 190784 (node group 8479). Plate Specialization 86539 is level 27.
    // Holy's SpecializationSpells override Shield of the Righteous 53600 (→ 415091)
    // and Judgment 20271 (→ 275773), so those baselines are unlearned first.
    assert_eq!(
        received::<SpellsUnlearned>(&mut app, player),
        vec![SpellsUnlearned {
            spells: vec![
                53600, 20271, 85256, 137027, 162698, 267316, 342348, 403876, 404752, 412314,
                462096, 404834, 384376, 1258011, 1241288, 231663,
            ],
        }]
    );
    // Holy's Cleanse 4987 overrides its Cleanse Toxins 440013, which is not learned.
    assert_eq!(
        received::<SpellsLearned>(&mut app, player),
        vec![SpellsLearned {
            spells: vec![
                53563, 137029, 162701, 183997, 275773, 4987, 289941, 53651, 82326, 415091, 428076,
                462093, 231642, 1258016, 633, 190784,
            ],
        }]
    );
    // Retribution buttons are pruned; Holy's new actives fill the empty slots in
    // learn order (Beacon of Light, Judgment, Cleanse, Holy Light, Shield of the
    // Righteous, Lay on Hands, Divine Steed); Cleanse replaces Cleanse Toxins.
    let holy_bar: Vec<(u8, u32)> = [
        CRUSADER_STRIKE,
        53563,
        275773,
        4987,
        82326,
        415091,
        633,
        190784,
    ]
    .into_iter()
    .enumerate()
    .map(|(slot, spell)| (slot as u8, spell))
    .collect();
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot {
            slots: holy_bar
                .iter()
                .map(|&(slot, spell)| (slot, ActionRef::Spell(spell)))
                .collect(),
        }]
    );
    assert_eq!(
        stored(&app, 7),
        ProgressionData {
            spec_id: SPEC_HOLY,
            action_bar: holy_bar
                .iter()
                .map(|&(slot, spell)| (slot, ActionRefData::Spell(spell)))
                .collect(),
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
    // A character without a progression record has an empty bar.
    let default_bar = Vec::new();

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
            slot: 180,
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
            slot: 179,
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
            (7, ActionRefData::Spell(1464)),
            (179, ActionRefData::Item(6948)),
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
    // Holy replaces Shield of the Righteous 53600 and Judgment 20271 (spec overrides).
    assert_eq!(
        received::<SpellsUnlearned>(&mut app, player),
        vec![SpellsUnlearned {
            spells: vec![53600, 20271, 417383],
        }]
    );
    let learned = received::<SpellsLearned>(&mut app, player);
    assert_eq!(learned.len(), 1);
    // Divine Shield 642 and Sense Undead 5502 are level 10, Mastery: Lightbringer 183997 is
    // Holy's; Holy's Beacon of Light 53563 has SpellLevel 16.
    for spell_id in [642, 5502, 183997] {
        assert!(learned[0].spells.contains(&spell_id), "missing {spell_id}");
    }
    assert!(!learned[0].spells.contains(&REDEMPTION));
    assert!(!learned[0].spells.contains(&53563));

    // Holy Light 82326 is a Holy spell of SpellLevel 11.
    set_live_level(app.world_mut(), 4, 13);
    advance(&mut app);
    assert_eq!(
        received::<SpellsLearned>(&mut app, player),
        vec![SpellsLearned {
            spells: vec![REDEMPTION, 82326],
        }]
    );
    assert!(received::<SpecializationChanged>(&mut app, player).is_empty());
    assert_eq!(stored(&app, 4).spec_id, SPEC_HOLY);
}

fn world_with_creature(entry: u32, min_level: u8) -> crate::world_data::WorldData {
    let template = crate::world_data::CreatureTemplate {
        entry,
        name: "Fixture wolf".into(),
        subname: String::new(),
        creature_type: 1,
        family: 0,
        type_flags: 0,
        flags_extra: 0,
        min_level,
        max_level: min_level,
        faction: 32,
        npc_flags: 0,
        speed_walk: 1.0,
        speed_run: 1.0,
        health_modifier: 1.0,
        unit_class: 1,
        expansion: 0,
        rank: 0,
        experience_modifier: 1.0,
        gives_experience: true,
        script_name: String::new(),
        aggro_mode: crate::creature_aggro::AggroMode::Aggressive,
        trigger: false,
        unit_flags: 0,
        models: Vec::new(),
        tuning: None,
        armor: 0.0,
        armor_modifier: 1.0,
        damage_modifier: 1.0,
        health_scaling_expansion: 0,
        mana_modifier: 1.0,
        regenerates_power: false,
        regenerates_health: true,
    };
    crate::world_data::WorldData {
        templates: std::collections::HashMap::from([(entry, template)]),
        spawns: Vec::new(),
        spawns_by_map: std::collections::HashMap::new(),
        waypoints: std::collections::HashMap::new(),
        ..Default::default()
    }
}

#[test]
fn kill_xp_crossing_level_10_levels_up_and_sends_learned_spells() {
    use crate::auth::InWorld;
    use crate::combat::DeathState;
    use crate::persistence_progression::{load_xp, save_xp};
    use shared::components::{Npc, Position};
    use shared::protocol::{LogXpGain, PlayerXpUpdate, XpGainReason};

    let Some(classes) = class_spells() else {
        return;
    };
    let db = temp_database();
    let (character_id, _) = persistence::create_character(&db, 1, "Uther", HUMAN, PALADIN).unwrap();
    // Level 9 needs 8770; a level 9 kill gives gtXp PerKill 60.
    save_xp(&db, character_id, 8720);
    let mut app = setup_app(db, classes);
    app.insert_resource(world_with_creature(299, 9));
    let player = spawn_player(&mut app, character_id, HUMAN, PALADIN, 9);
    let origin = Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    app.world_mut().entity_mut(player).insert(origin.clone());
    let connection = connection_of(&mut app, player);
    app.world_mut().entity_mut(connection).insert((
        InWorld { character_id },
        MessageSender::<LogXpGain>::default(),
        MessageReceiver::<LogXpGain>::default(),
        MessageSender::<PlayerXpUpdate>::default(),
        MessageReceiver::<PlayerXpUpdate>::default(),
    ));
    advance(&mut app);
    drain(&mut app, player);
    assert_eq!(
        received::<PlayerXpUpdate>(&mut app, player),
        vec![PlayerXpUpdate {
            xp: 8720,
            next_level_xp: 8770,
            rested_xp: 0,
        }]
    );

    let wolf = app
        .world_mut()
        .spawn((
            Npc {
                template_id: 299,
                name: "Fixture wolf".into(),
            },
            UnitLevel(9),
            origin,
            crate::maps::InMap::continent(0),
            crate::creature_tap::CreatureTap::tapped_by(character_id),
            shared::components::Health {
                current: 0.0,
                max: 100.0,
            },
            DeathState {
                respawn_timer: 60.0,
            },
        ))
        .id();
    advance(&mut app);

    assert_eq!(app.world().get::<Level>(player), Some(&Level(10)));
    assert_eq!(
        received::<LogXpGain>(&mut app, player),
        vec![LogXpGain {
            victim: Some(wolf.to_bits()),
            original: 60,
            amount: 60,
            group_bonus: 1.0,
            reason: XpGainReason::Kill,
        }]
    );
    assert_eq!(
        received::<PlayerXpUpdate>(&mut app, player),
        vec![PlayerXpUpdate {
            xp: 10,
            next_level_xp: 10_590,
            rested_xp: 0,
        }]
    );
    assert_eq!(app.world().get::<UnitLevel>(player), Some(&UnitLevel(10)));
    let db = &app.world().resource::<GameDatabase>().db;
    assert_eq!(load_xp(db, character_id), 10);
    assert_eq!(
        persistence::load_character(db, character_id).unwrap().level,
        10
    );
    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged { spec_id: SPEC_HOLY }]
    );
    let learned = received::<SpellsLearned>(&mut app, player);
    assert_eq!(learned.len(), 1);
    // Divine Shield 642 and Sense Undead 5502 are level 10.
    for spell_id in [642, 5502] {
        assert!(learned[0].spells.contains(&spell_id), "missing {spell_id}");
    }
}

#[test]
fn achievements_criteria_native_kill_xp_credits_only_earned_level_transitions() {
    use crate::achievement::{PlayerAchievementState, load_achievement_catalog};
    use crate::auth::InWorld;
    use crate::combat::DeathState;
    use crate::persistence_progression::save_xp;
    use shared::components::{Npc, Position};

    for starting_level in [9, 79, 89] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let db = redb::Database::create(file.path()).unwrap();
        persistence::create_tables(&db);
        let (character, _) =
            persistence::create_character(&db, 1, "Levelproof", HUMAN, PALADIN).unwrap();
        save_xp(
            &db,
            character,
            crate::experience::xp_to_next_level(starting_level) - 1,
        );
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        let mut app = setup_progression_app(db, class_spells().unwrap(), true, false);
        app.insert_resource(load_achievement_catalog(&conn).unwrap());
        app.insert_resource(world_with_creature(299, starting_level));
        let player = spawn_player(&mut app, character, HUMAN, PALADIN, starting_level);
        let origin = Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        app.world_mut()
            .entity_mut(player)
            .insert((origin.clone(), PlayerAchievementState::default()));
        let connection = connection_of(&mut app, player);
        app.world_mut().entity_mut(connection).insert(InWorld {
            character_id: character,
        });
        advance(&mut app);
        let state = app.world().get::<PlayerAchievementState>(player).unwrap();
        for criterion in [62399, 64498, 111909, 57642] {
            assert!(
                !state.criteria_progress.contains_key(&criterion),
                "login must not infer {criterion}"
            );
        }
        app.world_mut().spawn((
            Npc {
                template_id: 299,
                name: "Fixture wolf".into(),
            },
            UnitLevel(starting_level),
            origin,
            crate::maps::InMap::continent(0),
            crate::creature_tap::CreatureTap::tapped_by(character),
            shared::components::Health {
                current: 0.0,
                max: 100.0,
            },
            DeathState {
                respawn_timer: 60.0,
            },
        ));
        advance(&mut app);
        assert_eq!(
            app.world().get::<Level>(player),
            Some(&Level(starting_level + 1))
        );
        let state = app.world().get::<PlayerAchievementState>(player).unwrap();
        assert_eq!(state.criteria_progress.get(&62399), Some(&1));
        assert_eq!(
            state.criteria_progress.get(&64498).copied(),
            (starting_level == 79).then_some(1)
        );
        assert_eq!(
            state.criteria_progress.get(&111909).copied(),
            (starting_level == 89).then_some(1)
        );
        assert_eq!(
            state.criteria_progress.get(&57642).copied(),
            (starting_level == 89).then_some(1)
        );
        if starting_level == 89 {
            assert_eq!(state.criteria_progress.get(&64625), Some(&1));
            assert!(!state.criteria_progress.contains_key(&64623));
            assert!(!state.earned_ids.contains(&7382));
            assert_eq!(
                state.criteria_progress.get(&66145),
                Some(&1),
                "Alliance cap"
            );
            assert!(
                !state.criteria_progress.contains_key(&66146),
                "no Horde credit"
            );
            let db = &app.world().resource::<GameDatabase>().db;
            let (mage_character, _) =
                persistence::create_character(db, 1, "Mageproof", 2, 8).unwrap();
            save_xp(
                db,
                mage_character,
                crate::experience::xp_to_next_level(89) - 1,
            );
            let mage = spawn_player(&mut app, mage_character, 2, 8, 89); // Orc Mage
            app.world_mut().entity_mut(mage).insert((
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                PlayerAchievementState::default(),
            ));
            let connection = connection_of(&mut app, mage);
            app.world_mut().entity_mut(connection).insert((
                Authenticated { account_id: 1 },
                InWorld {
                    character_id: mage_character,
                },
            ));
            advance(&mut app);
            app.world_mut().spawn((
                Npc {
                    template_id: 299,
                    name: "Second fixture wolf".into(),
                },
                UnitLevel(89),
                Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                crate::maps::InMap::continent(0),
                crate::creature_tap::CreatureTap::tapped_by(mage_character),
                shared::components::Health {
                    current: 0.0,
                    max: 100.0,
                },
                DeathState {
                    respawn_timer: 60.0,
                },
            ));
            advance(&mut app);
            assert_eq!(app.world().get::<Level>(mage), Some(&Level(90)));
            let state = app.world().get::<PlayerAchievementState>(mage).unwrap();
            assert_eq!(state.criteria_progress.get(&64623), Some(&1));
            assert!(!state.criteria_progress.contains_key(&64625));
            assert!(state.account.as_ref().unwrap().earned_ids.contains(&7382));
            assert_eq!(state.criteria_progress.get(&66146), Some(&1), "Horde cap");
            assert!(
                !state.criteria_progress.contains_key(&66145),
                "no Alliance credit"
            );
            assert!(
                state.account.as_ref().unwrap().earned_ids.contains(&7380),
                "Double Agent"
            );
        }
        // Unrelated/admin level mutation is not an XP transition.
        app.world_mut().entity_mut(player).insert(Level(90));
        advance(&mut app);
        assert_eq!(
            app.world()
                .get::<PlayerAchievementState>(player)
                .unwrap()
                .criteria_progress
                .get(&62399),
            Some(&1)
        );
        drop(app);
        let reopened = redb::Database::open(file.path()).unwrap();
        assert_eq!(
            persistence::load_achievements(&reopened, character)
                .criteria_progress
                .get(&62399),
            Some(&1)
        );
        if starting_level == 89 {
            let saved = persistence::load_achievements(&reopened, character);
            let mut restored =
                setup_progression_app(reopened, class_spells().unwrap(), true, false);
            restored.insert_resource(load_achievement_catalog(&conn).unwrap());
            let player = spawn_player(&mut restored, character, HUMAN, PALADIN, 90);
            restored.world_mut().entity_mut(player).insert(saved);
            let connection = connection_of(&mut restored, player);
            restored.world_mut().entity_mut(connection).insert(InWorld {
                character_id: character,
            });
            advance(&mut restored);
            let state = restored
                .world()
                .get::<PlayerAchievementState>(player)
                .unwrap();
            assert_eq!(state.criteria_progress.get(&62399), Some(&1));
            assert_eq!(state.criteria_progress.get(&64625), Some(&1));
            assert!(state.account.as_ref().unwrap().earned_ids.contains(&7382));
            assert_eq!(state.criteria_progress.get(&66145), Some(&1));
            assert!(
                state.account.as_ref().unwrap().earned_ids.contains(&7380),
                "reopened Double Agent"
            );
        }
    }
}

#[test]
fn admin_set_level_persists_and_updates_online_player() {
    use bevy::ecs::system::RunSystemOnce;

    use crate::admin_handlers::handle_set_level;
    use crate::admin_types::AdminResponse;

    let db = temp_database();
    let (character_id, _) = persistence::create_character(&db, 1, "Uther", HUMAN, PALADIN).unwrap();
    let mut app = App::new();
    app.insert_resource(GameDatabase::new(db));
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

#[test]
fn admin_set_rested_offline_persists_and_clamps_pool() {
    use crate::admin_handlers::handle_set_rested;
    use crate::admin_types::AdminResponse;
    use crate::persistence_progression::load_rest;
    use bevy::ecs::system::RunSystemOnce;

    let db = temp_database();
    let (character_id, _) =
        persistence::create_character(&db, 1, "Restprobe", HUMAN, PALADIN).unwrap();
    let mut app = App::new();
    app.insert_resource(GameDatabase::new(db));
    for (requested, expected) in [(75, 75), (u32::MAX, 375), (0, 0)] {
        let response = app
            .world_mut()
            .run_system_once(move |db: Res<GameDatabase>, mut commands: Commands| {
                handle_set_rested(&db, &mut commands, "Restprobe", requested)
            })
            .unwrap();
        assert!(matches!(response, AdminResponse::RestedSet { amount, .. } if amount == expected));
        let db = &app.world().resource::<GameDatabase>().db;
        assert_eq!(load_rest(db, character_id).unwrap().rested_xp, expected);
    }
}

#[test]
fn admin_set_rested_online_updates_pool_and_sends_owner_messages() {
    use crate::admin_handlers::handle_set_rested;
    use crate::persistence_progression::load_rest;
    use crate::rest::PlayerRestState;
    use bevy::ecs::system::RunSystemOnce;
    use shared::protocol::{PlayerXpUpdate, RestStateUpdate};

    let db = temp_database();
    let (character_id, _) =
        persistence::create_character(&db, 1, "Restprobe", HUMAN, PALADIN).unwrap();
    let mut app = setup_app(db, class_spells().unwrap());
    let player = spawn_player(&mut app, character_id, HUMAN, PALADIN, 1);
    app.world_mut()
        .entity_mut(player)
        .insert(PlayerRestState::fixture(1, 0));
    let connection = connection_of(&mut app, player);
    app.world_mut().entity_mut(connection).insert((
        crate::auth::InWorld { character_id },
        MessageSender::<PlayerXpUpdate>::default(),
        MessageReceiver::<PlayerXpUpdate>::default(),
        MessageSender::<RestStateUpdate>::default(),
        MessageReceiver::<RestStateUpdate>::default(),
    ));
    advance(&mut app);
    received::<PlayerXpUpdate>(&mut app, player);
    for (requested, expected) in [(75, 75), (u32::MAX, 375), (0, 0)] {
        app.world_mut()
            .run_system_once(move |db: Res<GameDatabase>, mut commands: Commands| {
                handle_set_rested(&db, &mut commands, "Restprobe", requested)
            })
            .unwrap();
        advance(&mut app);
        assert_eq!(
            app.world()
                .get::<PlayerRestState>(player)
                .unwrap()
                .rested_xp(),
            expected
        );
        assert_eq!(
            load_rest(&app.world().resource::<GameDatabase>().db, character_id)
                .unwrap()
                .rested_xp,
            expected
        );
        assert_eq!(
            received::<PlayerXpUpdate>(&mut app, player),
            vec![PlayerXpUpdate {
                xp: 0,
                next_level_xp: 250,
                rested_xp: expected,
            }]
        );
        let messages = received::<RestStateUpdate>(&mut app, player);
        assert_eq!(messages.len(), 1);
        let snapshot = messages[0].snapshot.as_ref().unwrap();
        assert_eq!(snapshot.rested_xp, expected);
        assert_eq!(snapshot.rested_xp_max, 375);
    }
}

/// Retribution nodes (node, entry): Blade of Justice 184575 is a root, Divine
/// Storm 53385 and Expurgation 383344 need it, Avenging Wrath 31884 needs
/// Expurgation. Holy: Holy Shock 20473 root, Infusion of Light 53576 below it.
const BLADE_OF_JUSTICE: (u32, u32) = (81526, 102498);
const DIVINE_STORM: (u32, u32) = (81527, 102499);
const EXPURGATION: (u32, u32) = (92689, 114830);
const AVENGING_WRATH: (u32, u32) = (81544, 102519);
const HOLY_SHOCK: (u32, u32) = (81555, 102534);
const INFUSION_OF_LIGHT: (u32, u32) = (81569, 102550);
/// Nodes granted to Retribution at level 80 (Hammer of Wrath, Auras of the
/// Resolute, Greater Judgment, hero keystones Dawnlight and Light's Guidance).
const RET_GRANTED: [(u32, u32); 5] = [
    (81510, 133481),
    (81600, 102587),
    (81603, 102590),
    (95099, 117696),
    (95180, 117813),
];

fn selections(nodes: &[(u32, u32)]) -> Vec<TraitEntrySelection> {
    nodes
        .iter()
        .map(|&(node_id, entry_id)| TraitEntrySelection {
            node_id,
            entry_id,
            rank: 1,
        })
        .collect()
}

fn purchases(nodes: &[(u32, u32)]) -> Vec<Purchase> {
    nodes
        .iter()
        .map(|&(node_id, entry_id)| Purchase {
            node_id,
            entry_id,
            ranks: 1,
        })
        .collect()
}

fn spawn_level_80_paladin(
    app_db: redb::Database,
    classes: ClassSpells,
    spec: u32,
) -> (App, Entity) {
    save_progression(
        &app_db,
        9,
        &ProgressionData {
            spec_id: spec,
            action_bar: Vec::new(),
        },
    );
    let mut app = setup_app(app_db, classes);
    let player = spawn_player(&mut app, 9, HUMAN, PALADIN, 80);
    advance(&mut app);
    (app, player)
}

#[test]
fn enter_world_sends_granted_trait_config_with_level_budget() {
    let Some(classes) = class_spells() else {
        return;
    };
    let (mut app, player) = spawn_level_80_paladin(temp_database(), classes, SPEC_RETRIBUTION);

    assert_eq!(
        received::<TraitConfigSnapshot>(&mut app, player),
        vec![TraitConfigSnapshot {
            spec_id: SPEC_RETRIBUTION,
            tree_id: 790,
            entries: selections(&RET_GRANTED),
            unspent: vec![(2801, 31), (2800, 30), (2986, 10), (2987, 10), (2988, 10)],
        }]
    );
    let known = app.world().get::<KnownSpells>(player).unwrap();
    // Granted Hammer of Wrath 1241288 is known; no hero tree is chosen, so the
    // Dawnlight keystone 431377 is not.
    assert!(known.contains(1241288));
    assert!(!known.contains(431377));
}

#[test]
fn committed_ret_build_is_persisted_and_learns_talent_spells() {
    let Some(classes) = class_spells() else {
        return;
    };
    let (mut app, player) = spawn_level_80_paladin(temp_database(), classes, SPEC_RETRIBUTION);
    drain(&mut app, player);
    let build = [BLADE_OF_JUSTICE, DIVINE_STORM, EXPURGATION, AVENGING_WRATH];

    send_request(
        &mut app,
        player,
        CommitTraitConfig {
            spec_id: SPEC_RETRIBUTION,
            entries: selections(&build),
        },
    );

    assert_eq!(
        received::<TraitCommitResult>(&mut app, player),
        vec![TraitCommitResult {
            ok: true,
            reason: None
        }]
    );
    // Avenging Wrath 31884 is a trainer spell, so all four are new.
    assert_eq!(
        received::<SpellsLearned>(&mut app, player),
        vec![SpellsLearned {
            spells: vec![184575, 53385, 383344, 31884],
        }]
    );
    let mut entries = selections(&RET_GRANTED);
    entries.extend(selections(&build));
    assert_eq!(
        received::<TraitConfigSnapshot>(&mut app, player),
        vec![TraitConfigSnapshot {
            spec_id: SPEC_RETRIBUTION,
            tree_id: 790,
            entries,
            unspent: vec![(2801, 31), (2800, 26), (2986, 10), (2987, 10), (2988, 10)],
        }]
    );
    let loadout = app.world().get::<TraitLoadout>(player).unwrap();
    // Expurgation is passive, Avenging Wrath active.
    let talents: Vec<KnownSpell> = loadout.active_talent_spells().collect();
    for (spell_id, passive) in [(383344, true), (31884, false)] {
        assert!(
            talents.contains(&KnownSpell { spell_id, passive }),
            "{spell_id}"
        );
    }
    assert_eq!(
        load_trait_configs(&app.world().resource::<GameDatabase>().db, 9).spec(SPEC_RETRIBUTION),
        purchases(&build).as_slice()
    );
}

#[test]
fn rejected_commit_reports_reason_and_changes_nothing() {
    let Some(classes) = class_spells() else {
        return;
    };
    let (mut app, player) = spawn_level_80_paladin(temp_database(), classes, SPEC_RETRIBUTION);
    drain(&mut app, player);
    let known_before = app.world().get::<KnownSpells>(player).unwrap().clone();

    send_request(
        &mut app,
        player,
        CommitTraitConfig {
            spec_id: SPEC_RETRIBUTION,
            entries: selections(&[BLADE_OF_JUSTICE, AVENGING_WRATH]),
        },
    );
    send_request(
        &mut app,
        player,
        CommitTraitConfig {
            spec_id: SPEC_HOLY,
            entries: Vec::new(),
        },
    );

    assert_eq!(
        received::<TraitCommitResult>(&mut app, player),
        vec![
            TraitCommitResult {
                ok: false,
                reason: Some("node 81544 requires one of nodes [92689] fully ranked".into()),
            },
            TraitCommitResult {
                ok: false,
                reason: Some("spec 65 is not the active spec 70".into()),
            },
        ]
    );
    assert!(received::<SpellsLearned>(&mut app, player).is_empty());
    assert!(received::<TraitConfigSnapshot>(&mut app, player).is_empty());
    assert_eq!(app.world().get::<KnownSpells>(player), Some(&known_before));
    let saved = load_trait_configs(&app.world().resource::<GameDatabase>().db, 9);
    assert!(saved.spec(SPEC_RETRIBUTION).is_empty());
}

#[test]
fn spec_change_loads_the_other_specs_saved_config() {
    let Some(classes) = class_spells() else {
        return;
    };
    let db = temp_database();
    let ret_build = [BLADE_OF_JUSTICE, DIVINE_STORM, EXPURGATION];
    let holy_build = [HOLY_SHOCK, INFUSION_OF_LIGHT];
    save_trait_config(&db, 9, SPEC_RETRIBUTION, purchases(&ret_build));
    save_trait_config(&db, 9, SPEC_HOLY, purchases(&holy_build));
    let (mut app, player) = spawn_level_80_paladin(db, classes, SPEC_RETRIBUTION);
    let entered = received::<TraitConfigSnapshot>(&mut app, player);
    assert_eq!(entered.len(), 1);
    assert!(entered[0].entries.ends_with(&selections(&ret_build)));
    drain(&mut app, player);
    assert!(
        app.world()
            .get::<KnownSpells>(player)
            .unwrap()
            .contains(383344)
    );

    send_request(&mut app, player, SetSpecialization { spec_id: SPEC_HOLY });

    let snapshots = received::<TraitConfigSnapshot>(&mut app, player);
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].spec_id, SPEC_HOLY);
    assert!(snapshots[0].entries.ends_with(&selections(&holy_build)));
    assert_eq!(snapshots[0].unspent[1], (2800, 28));
    let known = app.world().get::<KnownSpells>(player).unwrap();
    assert!(known.contains(53576));
    assert!(!known.contains(383344));
    let learned = received::<SpellsLearned>(&mut app, player);
    assert!(learned[0].spells.contains(&53576));
    let unlearned = received::<SpellsUnlearned>(&mut app, player);
    assert!(unlearned[0].spells.contains(&383344));
}

#[test]
fn new_human_paladin_bar_is_its_create_actions_without_class_mounts() {
    let Some(classes) = class_spells() else {
        return;
    };
    let db = temp_database();
    let Some(character_id) = create_with_kit(&db, "Uther", HUMAN, PALADIN) else {
        return;
    };
    let mut app = setup_app(db, classes);
    let player = spawn_player(&mut app, character_id, HUMAN, PALADIN, 1);
    advance(&mut app);

    // Summon Warhorse 13819 and Summon Charger 23214 (EffectAura 78 MOUNTED) are
    // known from level 1 but no create action holds them.
    let known = app.world().get::<KnownSpells>(player).unwrap();
    assert!(known.contains(13819) && known.contains(23214));
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot {
            // playercreateinfo_action (1, 2): Crusader Strike, Will to Survive, Recuperate.
            slots: vec![
                (0, ActionRef::Spell(CRUSADER_STRIKE)),
                (9, ActionRef::Spell(59752)),
                (10, ActionRef::Spell(1231411)),
            ],
        }]
    );
}

/// Swift Justice (node 81521, entry 102491) needs Divine Storm; Final Verdict
/// 383328 (node 81532, entry 102504, TraitDefinition 107509) needs Swift
/// Justice and has `OverridesSpellID` Templar's Verdict 85256.
const SWIFT_JUSTICE: (u32, u32) = (81521, 102491);
const FINAL_VERDICT: (u32, u32) = (81532, 102504);
const FINAL_VERDICT_SPELL: u32 = 383328;

#[test]
fn final_verdict_replaces_templars_verdict_in_known_spells_and_bar() {
    let Some(classes) = class_spells() else {
        return;
    };
    let db = temp_database();
    save_progression(
        &db,
        9,
        &ProgressionData {
            spec_id: SPEC_RETRIBUTION,
            action_bar: vec![
                (0, ActionRefData::Spell(CRUSADER_STRIKE)),
                (5, ActionRefData::Spell(TEMPLARS_VERDICT)),
            ],
        },
    );
    let mut app = setup_app(db, classes);
    let player = spawn_player(&mut app, 9, HUMAN, PALADIN, 80);
    advance(&mut app);
    drain(&mut app, player);
    let without_verdict = [BLADE_OF_JUSTICE, DIVINE_STORM, SWIFT_JUSTICE];
    let mut with_verdict = without_verdict.to_vec();
    with_verdict.push(FINAL_VERDICT);

    send_request(
        &mut app,
        player,
        CommitTraitConfig {
            spec_id: SPEC_RETRIBUTION,
            entries: selections(&with_verdict),
        },
    );

    assert_eq!(
        received::<SpellsUnlearned>(&mut app, player),
        vec![SpellsUnlearned {
            spells: vec![TEMPLARS_VERDICT],
        }]
    );
    let learned = received::<SpellsLearned>(&mut app, player);
    assert!(learned[0].spells.contains(&FINAL_VERDICT_SPELL));
    // Blade of Justice 184575 and Divine Storm 53385 are pushed to the first empty
    // slots; Final Verdict takes Templar's Verdict's button.
    let replaced = vec![
        (0, ActionRef::Spell(CRUSADER_STRIKE)),
        (1, ActionRef::Spell(184575)),
        (2, ActionRef::Spell(53385)),
        (5, ActionRef::Spell(FINAL_VERDICT_SPELL)),
    ];
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot { slots: replaced }]
    );
    let known = app.world().get::<KnownSpells>(player).unwrap();
    assert!(known.contains(FINAL_VERDICT_SPELL) && !known.contains(TEMPLARS_VERDICT));
    assert_eq!(
        stored(&app, 9).action_bar,
        vec![
            (0, ActionRefData::Spell(CRUSADER_STRIKE)),
            (1, ActionRefData::Spell(184575)),
            (2, ActionRefData::Spell(53385)),
            (5, ActionRefData::Spell(FINAL_VERDICT_SPELL)),
        ]
    );

    send_request(
        &mut app,
        player,
        CommitTraitConfig {
            spec_id: SPEC_RETRIBUTION,
            entries: selections(&without_verdict),
        },
    );

    assert_eq!(
        received::<SpellsUnlearned>(&mut app, player),
        vec![SpellsUnlearned {
            spells: vec![FINAL_VERDICT_SPELL],
        }]
    );
    assert_eq!(
        received::<SpellsLearned>(&mut app, player),
        vec![SpellsLearned {
            spells: vec![TEMPLARS_VERDICT],
        }]
    );
    let restored = vec![
        (0, ActionRef::Spell(CRUSADER_STRIKE)),
        (1, ActionRef::Spell(184575)),
        (2, ActionRef::Spell(53385)),
        (5, ActionRef::Spell(TEMPLARS_VERDICT)),
    ];
    assert_eq!(
        received::<ActionBarSnapshot>(&mut app, player),
        vec![ActionBarSnapshot { slots: restored }]
    );
    assert_eq!(
        stored(&app, 9).action_bar,
        vec![
            (0, ActionRefData::Spell(CRUSADER_STRIKE)),
            (1, ActionRefData::Spell(184575)),
            (2, ActionRefData::Spell(53385)),
            (5, ActionRefData::Spell(TEMPLARS_VERDICT)),
        ]
    );
}

#[path = "class_progression_loadout_tests.rs"]
mod loadouts;

#[path = "class_progression_new_class_tests.rs"]
mod new_classes;

#[test]
fn paladin_learns_boundless_conviction_for_five_holy_power() {
    let Some(classes) = class_spells() else {
        return;
    };
    // Boundless Conviction 115675 (MOD_MAX_POWER +2 Holy Power): SkillLineAbility
    // 26101, AcquireMethod 2.
    assert!(known_ids(&classes, HUMAN, PALADIN, 10, SPEC_RETRIBUTION).contains(&115675));
}

/// `.character level` resets XP to 0 whether the character is online (SetXP(0)) or not
/// (CHAR_UPD_LEVEL `xp = 0`): cs_character.cpp:770-792.
#[test]
fn admin_set_level_resets_xp_online_and_offline() {
    use bevy::ecs::system::RunSystemOnce;

    use crate::admin_handlers::handle_set_level;
    use crate::persistence_progression::{load_xp, save_xp};

    let db = temp_database();
    let (online, _) = persistence::create_character(&db, 1, "Uther", HUMAN, PALADIN).unwrap();
    let (offline, _) = persistence::create_character(&db, 1, "Turalyon", HUMAN, PALADIN).unwrap();
    save_xp(&db, online, 500);
    save_xp(&db, offline, 700);
    let mut app = App::new();
    app.insert_resource(GameDatabase::new(db));
    app.world_mut()
        .spawn((ClientId(online), Level(1), UnitLevel(1)));
    for name in ["Uther", "Turalyon"] {
        app.world_mut()
            .run_system_once(move |db: Res<GameDatabase>, mut commands: Commands| {
                handle_set_level(&db, &mut commands, name, 5)
            })
            .unwrap();
    }
    let db = &app.world().resource::<GameDatabase>().db;
    assert_eq!((load_xp(db, online), load_xp(db, offline)), (0, 0));
}

/// Player::LearnDefaultSkills / LearnSkillRewardedSpells (Player.cpp:25275-25291,
/// 25407-25457): each race's racial skill line rewards its abilities at creation, by
/// race (and class) mask. Human 754: Will to Survive 59752, The Human Spirit 20598;
/// Orc 125: Blood Fury 20572 (Warrior class mask 45), Hardiness 20573; Haranir 2930:
/// Thorn Bloom 1237885, Lash Out 1238623, One With Nature 1238648. No race gets
/// another's racials.
#[test]
fn racial_skill_lines_reward_each_races_racials() {
    let Some(classes) = class_spells() else {
        return;
    };
    const ORC: u8 = 2;
    const HARANIR_HORDE: u8 = 91;
    const HARANIR_ALLIANCE: u8 = 86;
    const MONK: u8 = 10;
    let human = known_ids(&classes, HUMAN, WARRIOR, 1, 0);
    let orc = known_ids(&classes, ORC, WARRIOR, 1, 0);
    for spell in [59752, 20598] {
        assert!(human.contains(&spell), "Human Warrior lacks {spell}");
        assert!(!orc.contains(&spell), "Orc Warrior knows Human {spell}");
    }
    for spell in [20572, 20573] {
        assert!(orc.contains(&spell), "Orc Warrior lacks {spell}");
        assert!(!human.contains(&spell), "Human Warrior knows Orc {spell}");
    }
    for race in [HARANIR_HORDE, HARANIR_ALLIANCE] {
        let haranir = known_ids(&classes, race, MONK, 10, 0);
        for spell in [1237885, 1238623, 1238648] {
            assert!(
                haranir.contains(&spell),
                "Haranir {race} Monk lacks {spell}"
            );
        }
    }
}

/// Frostbolt 116 is the mage baseline (SkillLineAbility 43444, level 1);
/// SpecializationSpells 4 (Arcane 62: Arcane Blast 30451) and 5 (Fire 63:
/// Fireball 133) override it at level 10, while Frost 64 and Initial 1449 keep it.
#[test]
fn level_10_mage_specs_replace_frostbolt_like_retail() {
    let Some(classes) = class_spells() else {
        return;
    };
    const MAGE: u8 = 8;
    let bolts = |level, spec| {
        let known = known_ids(&classes, HUMAN, MAGE, level, spec);
        [116, 133, 30451]
            .into_iter()
            .filter(|id| known.contains(id))
            .collect::<Vec<u32>>()
    };
    assert_eq!(bolts(10, 62), vec![30451]);
    assert_eq!(bolts(10, 63), vec![133]);
    assert_eq!(bolts(10, 64), vec![116]);
    assert_eq!(bolts(9, 1449), vec![116]);
}

/// General (skill 183) content abilities carry SPELL_ATTR13_UNK16 and are not taught to a
/// fresh character; ordinary General spells still are
/// (docs/wiki/investigations/general-spell-grants.md).
#[test]
fn fresh_paladin_knows_general_basics_but_no_content_abilities() {
    let Some(classes) = class_spells() else {
        return;
    };
    let known = known_ids(&classes, HUMAN, PALADIN, 1, SPEC_RETRIBUTION);
    // Auto Attack, Armor Skills, Weapon Skills, The Human Spirit.
    for spell in [6603, 76271, 76294, 20598] {
        assert!(known.contains(&spell), "missing General spell {spell}");
    }
    // Garrison Ability, Covenant Ability, Sanity Restoration Orb, Summon Pocopoc,
    // Return to the Story, Exit Lorewalking, G-99 Breakneck, View Next Adventure, Throw.
    for spell in [
        161691, 313347, 314955, 360078, 466325, 467538, 1215279, 1236329, 385265,
    ] {
        assert!(
            !known.contains(&spell),
            "content ability {spell} was taught"
        );
    }
}

/// Online GM learning must cross the same Lightyear boundary as level/talent learning.
#[test]
fn online_learn_spell_admin_publishes_without_relog() {
    use crate::profession::{PlayerProfessions, load_profession_data};
    use bevy::ecs::system::RunSystemOnce;
    use shared::components::{MovementControl, Position};
    use shared::protocol::ProfessionSnapshot;

    let classes = class_spells().unwrap();
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let db = temp_database();
    let character_id = persistence::create_character(&db, 1, "Probe", HUMAN, WARRIOR)
        .unwrap()
        .0;
    save_progression(
        &db,
        character_id,
        &ProgressionData {
            spec_id: 72,
            action_bar: Vec::new(),
        },
    );
    let mut app = setup_app(db, classes);
    app.insert_resource(load_profession_data(&conn).unwrap());
    app.insert_resource(crate::spell_info::SpellStore::fixture(
        [1719, 85288]
            .into_iter()
            .map(|id| data.store.get(id).unwrap().clone())
            .collect(),
    ));
    let player = spawn_player(&mut app, character_id, HUMAN, WARRIOR, 10);
    app.world_mut().entity_mut(player).insert((
        Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        MovementControl::default(),
        PlayerProfessions::default(),
    ));
    let connection = connection_of(&mut app, player);
    app.world_mut().entity_mut(connection).insert((
        MessageSender::<ProfessionSnapshot>::default(),
        MessageReceiver::<ProfessionSnapshot>::default(),
    ));
    advance(&mut app);
    assert_eq!(app.world().get::<ActiveSpec>(player).unwrap().0, 72);
    let initial = received::<KnownSpellsSnapshot>(&mut app, player);
    assert_eq!(initial.len(), 1);
    assert!(!initial[0].spells.contains(&1719));
    assert!(!initial[0].spells.contains(&85288));
    drain(&mut app, player);
    received::<SpellsLearned>(&mut app, player);
    for spell in [1719, 85288] {
        let response =
            app.world_mut()
                .run_system_once(
                    move |db: Res<GameDatabase>,
                          mut commands: Commands,
                          players: Query<(
                        Entity,
                        &mut Position,
                        &ClientId,
                        &mut MovementControl,
                    )>| {
                        crate::admin_handlers::handle_learn_spell(
                            &db,
                            &mut commands,
                            &players,
                            "Probe",
                            spell,
                        )
                    },
                )
                .unwrap();
        assert!(
            matches!(response, crate::admin_types::AdminResponse::SpellLearned { spell: id, .. } if id == spell)
        );
        advance(&mut app);
        assert!(
            persistence::load_professions(&app.world().resource::<GameDatabase>().db, character_id)
                .knows(spell)
        );
        assert_eq!(
            received::<SpellsLearned>(&mut app, player),
            vec![SpellsLearned {
                spells: vec![spell]
            }]
        );
        assert!(
            app.world()
                .get::<KnownSpells>(player)
                .unwrap()
                .contains(spell)
        );
    }
    // Re-evaluation must not retract an explicitly learned spell.
    app.world_mut().get_mut::<Level>(player).unwrap().0 = 11;
    advance(&mut app);
    assert!(
        received::<SpellsUnlearned>(&mut app, player)
            .iter()
            .all(|delta| !delta.spells.contains(&1719) && !delta.spells.contains(&85288))
    );
    assert!(
        app.world()
            .get::<KnownSpells>(player)
            .unwrap()
            .contains(1719)
    );
    assert!(
        app.world()
            .get::<KnownSpells>(player)
            .unwrap()
            .contains(85288)
    );
    send_request(&mut app, player, SetSpecialization { spec_id: SPEC_ARMS });
    assert!(
        received::<SpellsUnlearned>(&mut app, player)
            .iter()
            .all(|delta| !delta.spells.contains(&1719) && !delta.spells.contains(&85288))
    );
    let response = app
        .world_mut()
        .run_system_once(
            |db: Res<GameDatabase>,
             mut commands: Commands,
             players: Query<(Entity, &mut Position, &ClientId, &mut MovementControl)>| {
                crate::admin_handlers::handle_learn_spell(
                    &db,
                    &mut commands,
                    &players,
                    "Probe",
                    1719,
                )
            },
        )
        .unwrap();
    assert!(matches!(
        response,
        crate::admin_types::AdminResponse::SpellLearned { .. }
    ));
    received::<SpellsLearned>(&mut app, player);
    advance(&mut app);
    assert!(
        received::<SpellsLearned>(&mut app, player).is_empty(),
        "duplicate learn must not republish"
    );
    // Auth loads the persisted learned spells before the initial progression snapshot.
    let profs =
        persistence::load_professions(&app.world().resource::<GameDatabase>().db, character_id);
    app.world_mut().despawn(connection);
    app.world_mut().despawn(player);
    let reconnected = spawn_player(&mut app, character_id, HUMAN, WARRIOR, 11);
    app.world_mut().entity_mut(reconnected).insert(profs);
    advance(&mut app);
    let snapshot = received::<KnownSpellsSnapshot>(&mut app, reconnected);
    assert_eq!(snapshot.len(), 1);
    for spell in [1719, 85288] {
        assert_eq!(
            snapshot[0].spells.iter().filter(|&&id| id == spell).count(),
            1
        );
    }
    assert!(received::<SpellsLearned>(&mut app, reconnected).is_empty());
}

/// Native skill_race_class_info: Dual Wield skill 118 is a default line for Rogues and
/// Demon Hunters, whose SkillLineAbility teaches Dual Wield 674 (effect 40); Paladins
/// have no such line, and Fury's own SpecializationSpells row adds 231842.
#[test]
fn dual_wield_comes_from_default_skill_lines_and_spec_spells() {
    const NIGHT_ELF: u8 = 4;
    const ROGUE: u8 = 4;
    const DEMON_HUNTER: u8 = 12;
    const SPEC_FURY: u32 = 72;
    let classes = class_spells().expect("world.db");
    assert!(known_ids(&classes, HUMAN, ROGUE, 10, 0).contains(&674));
    assert!(known_ids(&classes, NIGHT_ELF, DEMON_HUNTER, 10, 0).contains(&674));
    assert!(!known_ids(&classes, HUMAN, PALADIN, 80, SPEC_RETRIBUTION).contains(&674));
    assert!(known_ids(&classes, HUMAN, WARRIOR, 80, SPEC_FURY).contains(&231842));
}
