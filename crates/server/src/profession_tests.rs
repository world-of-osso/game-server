//! Real-DB profession tests against data/world.db (TDB1210 trainers, Retail DB2
//! recipes).
//!
//! Georgio Bolero (1346) is the Stormwind tailoring trainer (TDB trainer 163):
//! Tailoring (264617) costs 10 copper at level 5 and learns 3908 Tailoring,
//! 264616 (Classic Tailoring skill) and SKILL_STEP 1 of Classic Tailoring (2540,
//! cap 300 from tier 336). Bolt of Linen Cloth (2963) is learned with the line,
//! takes 2 Linen Cloth (2589), casts 1.5 s, and is orange below rank 25.

#[path = "crafting_orders/tests.rs"]
mod crafting_order_tests;
#[path = "profession/crafting_origin_tests.rs"]
mod crafting_origin_tests;
#[path = "profession/spell_cooldown_persistence_tests.rs"]
mod spell_cooldown_persistence_tests;

#[test]
fn achievements_criteria_native_known_spell_marker_gates_recipe_facts_and_reopens() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let cases = [
        (
            406106,
            432962,
            18805,
            vec![
                (370465, 61271),
                (370460, 61272),
                (370438, 61273),
                (370463, 61274),
                (370453, 61275),
                (370462, 61276),
                (370456, 61277),
                (370504, 61278),
                (370457, 61279),
                (370476, 61280),
            ],
        ),
        (
            432962,
            406106,
            19704,
            vec![
                (430601, 65250),
                (430602, 65251),
                (430603, 65252),
                (430604, 65253),
                (430605, 65254),
                (430612, 65256),
            ],
        ),
        (
            432963,
            406106,
            19716,
            vec![
                (430591, 65274),
                (430592, 65275),
                (430593, 65276),
                (430594, 65277),
                (430595, 65278),
                (430596, 65279),
                (430597, 65280),
                (430598, 65281),
                (430599, 65282),
                (430600, 65283),
            ],
        ),
    ];
    for (marker, wrong_marker, achievement, learned) in cases {
        let recipes: Vec<_> = learned.iter().map(|&(spell, _)| spell).collect();
        let criteria: Vec<_> = learned.iter().map(|&(_, criterion)| criterion).collect();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("known-marker.redb");
        let db = redb::Database::create(&path).unwrap();
        persistence::create_tables(&db);
        let mut h = harness_with(90, |app| {
            app.add_plugins((
                AchievementPlugin,
                crate::class_progression::ClassProgressionPlugin,
            ));
            app.insert_resource(GameDatabase::new(db));
        })
        .unwrap();
        h.app
            .insert_resource(load_achievement_catalog(&h.conn).unwrap());
        let data = world_db().unwrap();
        let spells = recipes
            .iter()
            .copied()
            .chain([marker, wrong_marker])
            .map(|id| data.store.get(id).unwrap().clone())
            .collect();
        h.app.insert_resource(SpellStore::fixture(spells));
        h.app
            .world_mut()
            .entity_mut(h.player)
            .insert(PlayerAchievementState::default());
        h.app.update();
        // Controlled starting profession knowledge uses the production learner and persistence,
        // never writes criteria; no claim about acquiring these passive markers through traits.
        with_learner(h.app.world_mut(), h.player, |learner, profs| {
            for &spell in &recipes {
                learner.learn_spell(profs, spell, 0);
            }
        })
        .unwrap();
        h.advance(2);
        assert!(criteria.iter().copied().all(|id| {
            !h.app
                .world()
                .get::<PlayerAchievementState>(h.player)
                .unwrap()
                .criteria_progress
                .contains_key(&id)
        }));
        with_learner(h.app.world_mut(), h.player, |learner, profs| {
            learner.learn_spell(profs, wrong_marker, 0)
        })
        .unwrap();
        h.advance(2);
        assert!(
            criteria.iter().copied().all(|id| !h
                .app
                .world()
                .get::<PlayerAchievementState>(h.player)
                .unwrap()
                .criteria_progress
                .contains_key(&id)),
            "wrong marker cannot qualify these recipes"
        );
        with_learner(h.app.world_mut(), h.player, |learner, profs| {
            learner.learn_spell(profs, marker, 0)
        })
        .unwrap();
        h.advance(2);
        assert!(h.professions().knows(marker));
        assert!(
            h.app
                .world()
                .get::<crate::class_progression::KnownSpells>(h.player)
                .unwrap()
                .contains(marker)
        );
        let state = h
            .app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap();
        for &id in &criteria {
            assert_eq!(state.criteria_progress.get(&id), Some(&1));
        }
        assert!(
            state
                .account
                .as_deref()
                .unwrap()
                .earned_ids
                .contains(&achievement)
        );
        h.app.update();
        let character = h.character_id;
        drop(h.app.world_mut().remove_resource::<GameDatabase>().unwrap());
        let reopened = redb::Database::open(&path).unwrap();
        let profs = persistence::load_professions(&reopened, character);
        assert!(profs.knows(marker));
        let saved = persistence::load_achievements(&reopened, character);
        for &id in &criteria {
            assert_eq!(saved.criteria_progress.get(&id), Some(&1));
        }
        h.app.insert_resource(GameDatabase::new(reopened));
        h.app
            .world_mut()
            .entity_mut(h.player)
            .insert(PlayerAchievementState::default());
        h.app
            .world_mut()
            .entity_mut(h.connection)
            .remove::<InWorld>();
        h.app.world_mut().entity_mut(h.connection).insert(InWorld {
            character_id: character,
        });
        h.app.update();
        assert!(
            h.app
                .world()
                .get::<PlayerAchievementState>(h.player)
                .unwrap()
                .account
                .as_deref()
                .unwrap()
                .earned_ids
                .contains(&achievement)
        );
    }
}

#[test]
fn achievements_criteria_native_goggle_casts_accept_empty_all_and_store_real_items() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("goggle-casts.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let mut h = harness_with(80, |app| {
        app.add_plugins(AchievementPlugin);
        app.insert_resource(GameDatabase::new(db));
    })
    .unwrap();
    h.app
        .insert_resource(load_achievement_catalog(&h.conn).unwrap());
    // Project only pinned Retail rows in this test's owned snapshot.
    let fixture = dir.path().join("goggle-fixture.db");
    prepare_achievement_goggle_db(&fixture);
    let items = Connection::open_with_flags(&fixture, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app
        .insert_resource(load_item_templates(&fixture).unwrap());
    h.app.insert_resource(load_vendor_catalog(&items).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(PlayerAchievementState::default());
    let data = world_db().unwrap();
    let spells: Vec<_> = (127117..=127123)
        .map(|id| data.store.get(id).unwrap().clone())
        .collect();
    let goggles: Vec<_> = spells
        .iter()
        .map(|spell| {
            spell
                .effects
                .iter()
                .find(|effect| effect.effect == 24)
                .unwrap()
                .item_type
        })
        .collect();
    h.app.insert_resource(SpellStore::fixture(spells));
    h.app.update();
    for spell in 127117..=127123 {
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.player, spell, None, u8::MAX);
    }
    h.app.update();
    assert!((61781..=61787).all(|criterion| {
        !h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .criteria_progress
            .contains_key(&criterion)
    }));
    for spell in 127117..=127123 {
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.player, spell, None, 0);
        h.app.update();
    }
    let inventory =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, h.character_id);
    for item in goggles {
        assert_eq!(
            inventory
                .items
                .iter()
                .filter(|entry| entry.item_id == item)
                .map(|entry| entry.stack_count)
                .sum::<u32>(),
            1,
            "real crafted goggle {item}"
        );
    }
    let state = h
        .app
        .world()
        .get::<PlayerAchievementState>(h.player)
        .unwrap();
    for criterion in 61781..=61787 {
        assert_eq!(state.criteria_progress.get(&criterion), Some(&1));
    }
    assert!(
        state
            .account
            .as_deref()
            .unwrap()
            .earned_ids
            .contains(&18905)
    );
    h.app.update();
    let character = h.character_id;
    let closed = h.app.world_mut().remove_resource::<GameDatabase>().unwrap();
    drop(closed);
    let reopened = redb::Database::open(&path).unwrap();
    let saved = persistence::load_achievements(&reopened, character);
    for criterion in 61781..=61787 {
        assert_eq!(saved.criteria_progress.get(&criterion), Some(&1));
    }
    h.app.insert_resource(GameDatabase::new(reopened));
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(PlayerAchievementState::default());
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .remove::<InWorld>();
    h.app.world_mut().entity_mut(h.connection).insert(InWorld {
        character_id: character,
    });
    h.app.update();
    assert!(
        h.app
            .world()
            .get::<PlayerAchievementState>(h.player)
            .unwrap()
            .account
            .as_deref()
            .unwrap()
            .earned_ids
            .contains(&18905)
    );
}

fn prepare_achievement_goggle_db(destination: &std::path::Path) {
    let helper = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/prepare_achievement_goggle_db.py");
    let output = std::process::Command::new("python3")
        .arg(helper)
        .arg(crate::test_data::world_db_path())
        .arg(destination)
        .output()
        .expect("run sourced goggle fixture projection");
    assert!(
        output.status.success(),
        "goggle fixture projection failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn recraft_297_refuses_owned_item_before_inventory_currency_or_skill_mutation() {
    const RECRAFT: u32 = 389195; // ProfessionExpansion11, Leatherworking2830.
    let mut h = harness(80).unwrap();
    h.give(22658, "Glacial Cloak", 1);
    h.give(193222, "Lustrous Scaled Hide", 2);
    // Explicit test grants, not recipe acquisition, recharge-origin or item-eligibility proof.
    let professions = PlayerProfessions {
        lines: vec![shared::profession::ProfessionSkillLine {
            skill_line: 2830,
            step: 1,
            rank: 25,
            max_rank: 100,
        }],
        spells: vec![RECRAFT],
    };
    let wallet = shared::currency::CurrencyWallet {
        entries: vec![shared::currency::CurrencyEntry {
            id: 3049,
            amount: 137,
            week_earned: 0,
        }],
    };
    persistence::save_professions(h.db(), h.character_id, &professions);
    persistence::save_currencies(h.db(), h.character_id, &wallet);
    h.app.world_mut().entity_mut(h.player).insert((
        professions.clone(),
        crate::currency::PlayerCurrencies(wallet.clone()),
    ));
    h.app.update();
    let before = persistence::load_inventory(h.db(), h.character_id);
    let item_guid = before
        .items
        .iter()
        .find(|item| item.item_id == 22658)
        .unwrap()
        .item_guid;
    assert_ne!(item_guid, 0, "the cast targets a real owned instance");
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(RECRAFT)
        .unwrap()
        .clone();
    assert!(spell.effects.iter().any(|effect| effect.effect == 297));
    let Err(CastFailure::Unsupported(reason)) = check_cast_items(h.app.world(), h.player, &spell)
    else {
        panic!("unsourced recrafting must refuse before any owned-item mutation");
    };
    assert!(
        reason.contains("CRAFTING_OPERATION_COMPOSITION"),
        "{reason}"
    );
    h.send::<SpellCastIntent, CombatChannel>(SpellCastIntent {
        destination: None,
        spell_id: Some(RECRAFT),
        spell: String::new(),
        target_entity: None,
        target_item_guid: Some(item_guid),
        witness: None,
    });
    h.finish_crafting();
    let failed = h.take::<CastFailed>();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].spell_id, RECRAFT);
    assert_eq!(failed[0].reason, CastFailReason::NotSupported);
    assert!(
        failed[0]
            .detail
            .as_deref()
            .unwrap()
            .contains("CRAFTING_OPERATION_COMPOSITION")
    );
    assert!(!h.casting());
    assert_eq!(persistence::load_inventory(h.db(), h.character_id), before);
    assert_eq!(h.professions(), professions);
    assert_eq!(
        persistence::load_professions(h.db(), h.character_id),
        professions
    );
    assert_eq!(persistence::load_currencies(h.db(), h.character_id), wallet);
    assert_eq!(
        h.app
            .world()
            .get::<crate::currency::PlayerCurrencies>(h.player)
            .unwrap()
            .0,
        wallet
    );
}

#[test]
fn quality_crafting_refuses_before_any_reagent_or_skill_mutation() {
    let mut h = harness(80).unwrap();
    h.learn_tailoring();
    h.give(LINEN_CLOTH, "Linen Cloth", 4);
    let mut professions = h.professions();
    professions.spells.push(375110);
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(professions.clone());
    h.app.update();
    let before =
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, h.character_id);
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(375110)
        .unwrap()
        .clone();
    assert!(spell.effects.iter().any(|effect| effect.effect == 288));
    let Err(CastFailure::Unsupported(reason)) = check_cast_items(h.app.world(), h.player, &spell)
    else {
        panic!("unsourced quality operations must refuse before consuming reagents");
    };
    assert!(
        reason.contains("CRAFTING_OPERATION_COMPOSITION"),
        "{reason}"
    );
    h.send::<CraftRecipe, ProfessionChannel>(CraftRecipe {
        spell_id: 375110,
        casts: 1,
    });
    h.finish_crafting();
    assert_eq!(
        persistence::load_inventory(&h.app.world().resource::<GameDatabase>().db, h.character_id),
        before
    );
    assert_eq!(h.professions(), professions);
}

#[test]
fn quality_reagent_selection_is_authoritative_and_never_assigns_an_outcome() {
    use shared::crafting_reagents::{CraftingReagentCatalog, ReagentAllocation};
    use shared::protocol::{CraftingOperationRequest, CraftingOperationStateUpdate};
    let mut h = harness_with(80, |app| {
        add_inbox::<CraftingOperationStateUpdate>(app);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
        app.insert_resource(CraftingReagentCatalog::load(&dir).unwrap());
    })
    .unwrap();
    h.app.world_mut().entity_mut(h.connection).insert((
        pair::<CraftingOperationRequest>(),
        pair::<CraftingOperationStateUpdate>(),
    ));
    h.give(193222, "Lustrous Scaled Hide", 2);
    h.give(193208, "Resilient Leather", 6);
    h.give(193210, "Resilient Leather", 4);
    let mut professions = h.professions();
    professions.spells.push(375188);
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(professions.clone());
    h.app.update();
    let before = persistence::load_inventory(h.db(), h.character_id);
    let selected = vec![
        ReagentAllocation {
            slot_index: 0,
            item_id: 193222,
            quantity: 2,
        },
        ReagentAllocation {
            slot_index: 1,
            item_id: 193208,
            quantity: 6,
        },
        ReagentAllocation {
            slot_index: 1,
            item_id: 193210,
            quantity: 4,
        },
    ];
    h.send::<_, ProfessionChannel>(CraftingOperationRequest {
        spell_id: 375188,
        allocations: selected.clone(),
    });
    let update = h
        .take::<CraftingOperationStateUpdate>()
        .pop()
        .expect("server must validate owned selections");
    assert_eq!(update.validation_error, None);
    assert_eq!(update.allocations, selected);
    assert_eq!(
        update.quality,
        Err(shared::profession::CraftingOperationGap::CraftingOperationComposition)
    );
    let mut invalid = selected.clone();
    invalid[2].quantity = 5;
    h.send::<_, ProfessionChannel>(CraftingOperationRequest {
        spell_id: 375188,
        allocations: invalid,
    });
    let update = h.take::<CraftingOperationStateUpdate>().pop().unwrap();
    assert!(update.validation_error.is_some());
    assert!(update.allocations.is_empty());
    assert!(update.quality.is_err());
    assert_eq!(persistence::load_inventory(h.db(), h.character_id), before);
    assert_eq!(h.professions(), professions);
}

use std::path::PathBuf;
use std::time::Duration;

use lightyear::connection::host::HostClient;
use lightyear::prelude::{Channel, Connected, PeerId, RemoteId};
use rusqlite::{Connection, OpenFlags};
use shared::casting::CastState;
use shared::components::{CharacterAppearance, Health, Level, Npc, Position, UnitFactionTemplate};
use shared::protocol::{
    CastFailed, CombatChannel, CraftRecipe, InteractNpc, InteractionChannel, InteractionKind,
    InteractionOpened, NpcFlags, ProtocolPlugin, SelectGossipOption, SpellCastIntent,
    SpellCooldownUpdate, StopSpellCast, TrainerBuyFailed, TrainerBuySpell, TrainerChannel,
    TrainerFailReason, TrainerList, TrainerServiceState,
};
use shared::spell_data::CastFailReason;

use super::*;
use crate::faction::world_db_faction_templates;
use crate::item_templates::load_item_templates;
use crate::merchant::load_vendor_catalog;
use crate::networking::Moving;
use crate::npc_interaction::{NpcInteractionPlugin, load_gossip_catalog};
use crate::persistence::ClientId;
use crate::spell_cast::test_harness::{class_spells, power_table, world_db};

const HUMAN_TEMPLATE: u32 = 1;
const GEORGIO_BOLERO: u32 = 1346;
const TAILORING: u32 = 264617;
const TAILORING_SKILL: u32 = 197;
const CLASSIC_TAILORING: u32 = 2540;
const BOLT_OF_LINEN_CLOTH: u32 = 2963;
const WHITE_LINEN_SHIRT: u32 = 2393;
const LINEN_CLOTH: u32 = 2589;
const BOLT_ITEM: u32 = 2996;
/// Blacksmithing (264435) and Mining (265838): the two primaries of `two_primaries`.
const BLACKSMITHING: u32 = 264435;
const MINING: u32 = 265838;
/// Cooking (264633): a secondary profession.
const COOKING: u32 = 264633;

fn world_db_path() -> Option<PathBuf> {
    Some(crate::test_data::world_db_path())
}

/// Spells the tests touch: the trainer spells of Tailoring, Blacksmithing,
/// Mining and Cooking trainers, what they learn, and the abilities of their lines.
fn profession_spells(conn: &Connection) -> Vec<u32> {
    let mut statement = conn
        .prepare(
            "SELECT SpellId FROM tdb_trainer_spell WHERE TrainerId IN (163, 27, 91, 136, 56) \
             UNION SELECT EffectTriggerSpell FROM spell_effect WHERE Effect = 36 AND SpellID IN \
               (SELECT SpellId FROM tdb_trainer_spell WHERE TrainerId IN (163, 27, 91, 136, 56)) \
             UNION SELECT Spell FROM skill_line_ability WHERE SkillLine IN (197, 164, 186, 185, 165)",
        )
        .unwrap();
    statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[derive(Resource)]
struct Inbox<M>(Vec<M>);

fn collect<M: lightyear::prelude::Message>(
    mut receivers: Query<&mut MessageReceiver<M>>,
    mut inbox: ResMut<Inbox<M>>,
) {
    for mut receiver in &mut receivers {
        inbox.0.extend(receiver.receive());
    }
}

fn add_inbox<M: lightyear::prelude::Message>(app: &mut App) {
    app.insert_resource(Inbox::<M>(Vec::new()));
    app.add_systems(First, collect::<M>);
}

fn pair<M: lightyear::prelude::Message>() -> (MessageSender<M>, MessageReceiver<M>) {
    (MessageSender::default(), MessageReceiver::default())
}

struct Harness {
    app: App,
    conn: Connection,
    character_id: u64,
    player: Entity,
    connection: Entity,
    level: u8,
}

fn harness(level: u8) -> Option<Harness> {
    harness_with(level, |_| {})
}

/// The profession server app over `world.db`, without a character.
fn harness_app(extra: impl FnOnce(&mut App)) -> Option<(App, Connection)> {
    let path = world_db_path()?;
    let data = world_db()?;
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.add_plugins(lightyear::prelude::server::ServerPlugins {
        tick_duration: Duration::from_millis(50),
    });
    app.add_plugins(ProtocolPlugin);
    add_inbox::<InteractionOpened>(&mut app);
    add_inbox::<TrainerList>(&mut app);
    add_inbox::<TrainerBuyFailed>(&mut app);
    add_inbox::<ProfessionSnapshot>(&mut app);
    add_inbox::<CastFailed>(&mut app);
    add_inbox::<SpellCooldownUpdate>(&mut app);
    app.insert_resource(world_db_faction_templates()?);
    app.insert_resource(load_gossip_catalog(&conn).unwrap());
    app.insert_resource(load_item_templates(&path).unwrap());
    app.insert_resource(load_vendor_catalog(&conn).unwrap());
    app.insert_resource(load_profession_data(&conn).unwrap());
    crate::appearance_collection::tests::install_database(&mut app);
    app.init_resource::<GoldUpdateQueue>();
    app.add_plugins((
        NpcInteractionPlugin,
        crate::combat::CombatPlugin,
        crate::spell_cast::SpellCastPlugin,
        crate::auras::AuraPlugin,
        crate::creature_scripts::CreatureScriptsPlugin,
        ProfessionPlugin,
    ));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        Duration::from_millis(50),
    ));
    extra(&mut app);
    app.finish();
    app.cleanup();
    app.update();
    let spells = profession_spells(&conn)
        .into_iter()
        .filter_map(|id| data.store.get(id).cloned())
        .collect();
    app.insert_resource(SpellStore::fixture(spells));
    app.insert_resource(power_table(data));
    app.insert_resource(class_spells(data));
    Some((app, conn))
}

fn harness_with(level: u8, extra: impl FnOnce(&mut App)) -> Option<Harness> {
    let (mut app, conn) = harness_app(extra)?;
    let db = &app.world().resource::<GameDatabase>().db;
    let (character_id, _) =
        persistence::create_character(db, 1, "Crafter", 1, 1).expect("create character");
    let (player, connection) = enter_world(&mut app, character_id, level);
    Some(Harness {
        app,
        conn,
        character_id,
        player,
        connection,
        level,
    })
}

/// Spawn the character's player, with its saved professions, and an in-world connection.
fn enter_world(app: &mut App, character_id: u64, level: u8) -> (Entity, Entity) {
    let professions =
        persistence::load_professions(&app.world().resource::<GameDatabase>().db, character_id);
    let player = app
        .world_mut()
        .spawn((
            Player {
                name: "Crafter".into(),
                race: 1,
                class: 1,
                appearance: CharacterAppearance::default(),
            },
            Level(level),
            ClientId(character_id),
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            UnitFactionTemplate(HUMAN_TEMPLATE),
            crate::maps::InMap::continent(0),
            Moving(false),
            professions,
        ))
        .id();
    let connection = app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            InWorld { character_id },
            PlayerLink(player),
            (
                pair::<InteractNpc>(),
                pair::<InteractionOpened>(),
                pair::<SelectGossipOption>(),
                pair::<TrainerList>(),
                pair::<TrainerBuySpell>(),
                pair::<TrainerBuyFailed>(),
            ),
            (
                pair::<ProfessionSnapshot>(),
                pair::<CraftRecipe>(),
                pair::<CastFailed>(),
                pair::<SpellCastIntent>(),
                pair::<StopSpellCast>(),
                pair::<SpellCooldownUpdate>(),
            ),
        ))
        .id();
    app.update();
    (player, connection)
}

impl Harness {
    fn send<M: lightyear::prelude::Message, C: Channel>(&mut self, message: M) {
        self.app
            .world_mut()
            .get_mut::<MessageSender<M>>(self.connection)
            .unwrap()
            .send::<C>(message);
        self.advance(3);
    }

    fn advance(&mut self, frames: usize) {
        for _ in 0..frames {
            self.app.update();
        }
    }

    fn take<M: lightyear::prelude::Message>(&mut self) -> Vec<M> {
        std::mem::take(&mut self.app.world_mut().resource_mut::<Inbox<M>>().0)
    }

    /// Spawn a creature from its world.db template 2 yd from the player.
    fn spawn_npc(&mut self, entry: u32) -> Entity {
        let (name, faction, npcflag): (String, u32, u32) = self
            .conn
            .query_row(
                "SELECT name, faction, npcflag FROM content_creature_template WHERE entry = ?1",
                [entry],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        self.app
            .world_mut()
            .spawn((
                Npc {
                    template_id: entry,
                    name,
                },
                NpcFlags::from_azerothcore(npcflag),
                Position {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                UnitFactionTemplate(faction),
                crate::maps::InMap::continent(0),
            ))
            .id()
    }

    /// Talk to Georgio and pick "Train me." (gossip menu 4353 option 0, type 5).
    fn open_georgio(&mut self) -> Entity {
        self.open_trainer(GEORGIO_BOLERO)
    }

    fn open_trainer(&mut self, entry: u32) -> Entity {
        let npc = self.spawn_npc(entry);
        self.send::<_, InteractionChannel>(InteractNpc { npc: npc.to_bits() });
        let opened = self.take::<InteractionOpened>();
        assert!(
            matches!(
                opened.as_slice(),
                [InteractionOpened {
                    kind: InteractionKind::Gossip(_),
                    ..
                }]
            ),
            "{opened:?}"
        );
        self.send::<_, InteractionChannel>(SelectGossipOption {
            npc: npc.to_bits(),
            option_id: 0,
        });
        // The list is sent in the frame the option opens the role; deliver it.
        self.advance(3);
        npc
    }

    fn buy(&mut self, npc: Entity, spell_id: u32) {
        self.send::<_, TrainerChannel>(TrainerBuySpell {
            npc: npc.to_bits(),
            spell_id,
        });
    }

    fn craft(&mut self, spell_id: u32, casts: u16) {
        self.send::<_, ProfessionChannel>(CraftRecipe { spell_id, casts });
    }

    fn professions(&self) -> PlayerProfessions {
        self.app
            .world()
            .get::<PlayerProfessions>(self.player)
            .unwrap()
            .clone()
    }

    fn db(&self) -> &redb::Database {
        &self.app.world().resource::<GameDatabase>().db
    }

    fn gold(&self) -> u64 {
        persistence::load_inventory(self.db(), self.character_id).gold
    }

    fn set_gold(&mut self, gold: u64) {
        let mut inventory = persistence::load_inventory(self.db(), self.character_id);
        inventory.gold = gold;
        persistence::save_inventory(self.db(), self.character_id, &inventory);
    }

    fn give(&mut self, item_id: u32, name: &str, count: u32) {
        let mut inventory = persistence::load_inventory(self.db(), self.character_id);
        let item_guid = persistence::next_id(self.db(), "item_instance");
        inventory.items.push(InventoryItemData {
            item_guid,
            item_id,
            name: name.into(),
            quality: 1,
            required_level: 0,
            stack_count: count,
            vendor_sell_price: 0,
            unique_auction: false,
        });
        persistence::save_inventory(self.db(), self.character_id, &inventory);
    }

    fn bag_count(&self, item_id: u32) -> u32 {
        persistence::load_inventory(self.db(), self.character_id)
            .items
            .iter()
            .filter(|item| item.item_id == item_id)
            .map(|item| item.stack_count)
            .sum()
    }

    /// Learn Tailoring at Georgio with 1 silver; drains the messages.
    fn learn_tailoring(&mut self) -> Entity {
        self.set_gold(100);
        let npc = self.open_georgio();
        self.buy(npc, TAILORING);
        self.take::<TrainerList>();
        self.take::<ProfessionSnapshot>();
        npc
    }

    fn line(&self, skill: u32) -> Option<ProfessionSkillLine> {
        self.professions().line(skill).copied()
    }

    fn set_rank(&mut self, skill: u32, rank: u16) {
        let mut profs = self.professions();
        let line = profs.line(skill).copied().unwrap();
        profs.set_line(ProfessionSkillLine { rank, ..line });
        self.app.world_mut().entity_mut(self.player).insert(profs);
        self.app.update();
    }

    fn casting(&self) -> bool {
        self.app.world().get::<CastState>(self.player).is_some()
    }

    /// Advance until the player has no cast and no queued craft (max 20 s).
    fn finish_crafting(&mut self) {
        for _ in 0..400 {
            self.app.update();
            if !self.casting() && self.app.world().get::<CraftQueue>(self.player).is_none() {
                // Deliver the last snapshot and failures.
                self.advance(3);
                return;
            }
        }
        panic!("crafting did not finish");
    }
}

fn service_state(list: &TrainerList, spell_id: u32) -> Option<TrainerServiceState> {
    list.services
        .iter()
        .find(|service| service.spell_id == spell_id)
        .map(|service| service.state)
}

#[test]
fn georgio_lists_tailoring_for_ten_copper_at_level_five() {
    let Some(mut h) = harness(10) else { return };
    h.open_georgio();
    let lists = h.take::<TrainerList>();
    let [list] = lists.as_slice() else {
        panic!("{lists:?}")
    };
    assert_eq!(list.trainer_id, 163);
    let tailoring = list
        .services
        .iter()
        .find(|service| service.spell_id == TAILORING)
        .unwrap();
    assert_eq!(
        (
            tailoring.cost,
            tailoring.req_level,
            tailoring.state,
            tailoring.profession
        ),
        (10, 5, TrainerServiceState::Available, true)
    );
    let shirt = list
        .services
        .iter()
        .find(|service| service.spell_id == WHITE_LINEN_SHIRT)
        .unwrap();
    assert!(!shirt.profession);
    assert_eq!(
        service_state(list, WHITE_LINEN_SHIRT),
        Some(TrainerServiceState::Unavailable)
    );
    assert_eq!(
        service_state(list, 3911),
        None,
        "no AzerothCore Apprentice Tailor rank"
    );
}

#[test]
fn learning_tailoring_takes_gold_and_learns_classic_tailoring_with_bolt_of_linen() {
    let Some(mut h) = harness(10) else { return };
    h.set_gold(100);
    let npc = h.open_georgio();
    h.take::<TrainerList>();
    h.buy(npc, TAILORING);

    assert_eq!(h.gold(), 90);
    let classic = h.line(CLASSIC_TAILORING).unwrap();
    assert_eq!((classic.step, classic.rank, classic.max_rank), (1, 1, 300));
    assert!(h.line(TAILORING_SKILL).is_some());
    let profs = h.professions();
    for spell in [3908, 264616, BOLT_OF_LINEN_CLOTH] {
        assert!(
            profs.knows(spell),
            "{spell} not learned: {:?}",
            profs.spells
        );
    }
    assert_eq!(
        h.take::<ProfessionSnapshot>().last(),
        Some(&profs.snapshot(h.app.world().resource::<ProfessionData>()))
    );
    let lists = h.take::<TrainerList>();
    let list = lists.last().expect("list re-sent after the purchase");
    assert_eq!(
        service_state(list, TAILORING),
        Some(TrainerServiceState::Known)
    );
    assert_eq!(
        service_state(list, WHITE_LINEN_SHIRT),
        Some(TrainerServiceState::Available)
    );
    assert_eq!(
        persistence::load_professions(h.db(), h.character_id),
        profs,
        "saved"
    );
}

#[test]
fn learning_tailoring_sets_parent_line_maximum_from_lower_authored_tier() {
    let Some(mut h) = harness(10) else { return };
    h.learn_tailoring();
    // Tailoring197 is enabled at ParentTierIndex4; SkillTiers462 authors only Value1=75,
    // so TC GetValueForTierIndex(3) falls back to 75.
    let parent = h.line(TAILORING_SKILL).unwrap();
    assert_eq!((parent.step, parent.rank, parent.max_rank), (4, 1, 75));
}

#[test]
fn tailoring_purchase_without_ten_copper_fails_and_changes_nothing() {
    let Some(mut h) = harness(10) else { return };
    h.set_gold(9);
    let npc = h.open_georgio();
    h.buy(npc, TAILORING);

    let failures = h.take::<TrainerBuyFailed>();
    assert_eq!(
        failures.iter().map(|f| f.reason).collect::<Vec<_>>(),
        vec![TrainerFailReason::NotEnoughMoney]
    );
    assert_eq!(h.gold(), 9);
    assert_eq!(h.professions(), PlayerProfessions::default());
}

#[test]
fn level_four_character_cannot_learn_tailoring() {
    let Some(mut h) = harness(4) else { return };
    h.set_gold(100);
    let npc = h.open_georgio();
    let lists = h.take::<TrainerList>();
    assert_eq!(
        service_state(&lists[0], TAILORING),
        Some(TrainerServiceState::Unavailable)
    );
    h.buy(npc, TAILORING);
    assert_eq!(
        h.take::<TrainerBuyFailed>()
            .iter()
            .map(|f| f.reason)
            .collect::<Vec<_>>(),
        vec![TrainerFailReason::Unavailable]
    );
    assert_eq!(h.gold(), 100);
}

#[test]
fn buying_from_a_trainer_whose_frame_is_not_open_fails() {
    let Some(mut h) = harness(10) else { return };
    h.set_gold(100);
    let npc = h.spawn_npc(GEORGIO_BOLERO);
    h.buy(npc, TAILORING);
    assert_eq!(
        h.take::<TrainerBuyFailed>()
            .iter()
            .map(|f| f.reason)
            .collect::<Vec<_>>(),
        vec![TrainerFailReason::Unavailable]
    );
    assert_eq!(h.gold(), 100);
}

/// Learn `spell` as the trainer purchase would (cast its effects on the player).
fn learn_by_cast(h: &mut Harness, spell: u32) {
    let player = h.player;
    crate::spell_cast::cast_triggered(h.app.world_mut(), player, spell, Some(player), 0);
    h.app.update();
}

#[test]
fn third_primary_profession_is_refused_but_cooking_is_not() {
    let Some(mut h) = harness(10) else { return };
    learn_by_cast(&mut h, BLACKSMITHING);
    learn_by_cast(&mut h, MINING);
    assert!(h.line(164).is_some() && h.line(186).is_some());
    h.set_gold(100);
    let npc = h.open_georgio();
    h.buy(npc, TAILORING);

    assert_eq!(
        h.take::<TrainerBuyFailed>()
            .iter()
            .map(|f| f.reason)
            .collect::<Vec<_>>(),
        vec![TrainerFailReason::Unavailable]
    );
    assert_eq!(h.gold(), 100);
    assert!(h.line(TAILORING_SKILL).is_none());

    let world = h.app.world_mut();
    let blocked = with_learner(world, h.player, |learner, profs| {
        learner.blocked_by_primary_limit(profs, COOKING)
    });
    assert_eq!(blocked, Some(false));
}

#[test]
fn professions_favorites_require_known_recipe_and_reopen_per_character_without_craft_mutation() {
    use shared::protocol::SetRecipeFavorite;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("recipe-favorites.redb");
    let db = redb::Database::create(&path).unwrap();
    persistence::create_tables(&db);
    let mut h = harness_with(10, |app| {
        app.insert_resource(GameDatabase::new(db));
    })
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert(pair::<SetRecipeFavorite>());
    h.set_gold(1000);
    let npc = h.open_trainer(5564);
    h.buy(npc, 264578);
    assert!(h.take::<TrainerBuyFailed>().is_empty());
    // Controlled rank prerequisite; only the real trainer purchase below teaches2153.
    with_learner(h.app.world_mut(), h.player, |learner, profs| {
        learner.set_skill(profs, 2532, 1, 15, 300);
    });
    h.give(2318, "Light Leather", 4);
    h.give(2320, "Coarse Thread", 1);
    h.advance(3);
    let initial = h.take::<ProfessionSnapshot>().pop().unwrap();
    assert!(!initial.spells.contains(&2153));
    assert!(initial.favorites.is_empty());
    let inventory = persistence::load_inventory(h.db(), h.character_id);
    let professions = h.professions();
    let currencies = persistence::load_currencies(h.db(), h.character_id);
    for favorite in [true, false] {
        h.send::<SetRecipeFavorite, ProfessionChannel>(SetRecipeFavorite {
            spell_id: 2153,
            favorite,
        });
        assert_eq!(
            h.take::<ProfessionSnapshot>().last(),
            Some(&initial),
            "unlearned favorite must return unchanged owner state"
        );
        assert_eq!(
            persistence::load_inventory(h.db(), h.character_id),
            inventory
        );
        assert_eq!(h.professions(), professions);
        assert_eq!(
            persistence::load_currencies(h.db(), h.character_id),
            currencies
        );
        assert!(!h.casting());
    }
    h.buy(npc, 2153);
    assert!(h.take::<TrainerBuyFailed>().is_empty());
    assert!(h.professions().knows(2153));
    assert_eq!(h.gold(), inventory.gold - 50);
    h.take::<ProfessionSnapshot>();
    let learned = h.professions();
    let inventory = persistence::load_inventory(h.db(), h.character_id);
    for favorite in [true, false, true] {
        h.send::<SetRecipeFavorite, ProfessionChannel>(SetRecipeFavorite {
            spell_id: 2153,
            favorite,
        });
        let reply = h
            .take::<ProfessionSnapshot>()
            .pop()
            .expect("favorite owner update");
        assert_eq!(reply.favorites, if favorite { vec![2153] } else { vec![] });
        assert_eq!(h.professions(), learned);
        assert_eq!(
            persistence::load_professions(h.db(), h.character_id),
            learned
        );
        assert_eq!(
            persistence::load_inventory(h.db(), h.character_id),
            inventory
        );
        assert_eq!(
            persistence::load_currencies(h.db(), h.character_id),
            currencies
        );
        assert!(!h.casting());
    }
    // Close the actual database, discard the old owner/connection, then open fresh state.
    let owner = h.character_id;
    assert!(h.app.world_mut().despawn(h.connection));
    assert!(h.app.world_mut().despawn(h.player));
    drop(
        h.app
            .world_mut()
            .remove_resource::<GameDatabase>()
            .unwrap()
            .into_database(),
    );
    let reopened = redb::Database::open(&path).unwrap();
    let saved = persistence::load_professions(&reopened, owner);
    h.app.insert_resource(GameDatabase::new(reopened));
    h.player = h.app.world_mut().spawn((ClientId(owner), saved)).id();
    h.connection = h
        .app
        .world_mut()
        .spawn((
            RemoteId(PeerId::Local(1)),
            Connected,
            HostClient { buffer: Vec::new() },
            Authenticated { account_id: 1 },
            InWorld {
                character_id: owner,
            },
            PlayerLink(h.player),
            pair::<ProfessionSnapshot>(),
            pair::<SetRecipeFavorite>(),
        ))
        .id();
    h.advance(4);
    let reopened = h
        .take::<ProfessionSnapshot>()
        .pop()
        .expect("fresh owner snapshot after reopen");
    assert_eq!(reopened.favorites, [2153]);
    assert_eq!(reopened.spells, learned.spells);
    assert_eq!(persistence::load_inventory(h.db(), owner), inventory);
    let (other, _) = persistence::create_character(h.db(), 1, "Othercrafter", 1, 1).unwrap();
    // Same-account isolation fixture; copied knowledge is not acquisition evidence.
    let other_player = h
        .app
        .world_mut()
        .spawn((ClientId(other), learned.clone()))
        .id();
    h.app.world_mut().entity_mut(h.connection).insert((
        PlayerLink(other_player),
        InWorld {
            character_id: other,
        },
    ));
    h.send::<SetRecipeFavorite, ProfessionChannel>(SetRecipeFavorite {
        spell_id: 2153,
        favorite: false,
    });
    assert!(
        h.take::<ProfessionSnapshot>()
            .last()
            .unwrap()
            .favorites
            .is_empty()
    );
    h.app.world_mut().entity_mut(h.connection).insert((
        PlayerLink(h.player),
        InWorld {
            character_id: owner,
        },
    ));
    // Fresh owner projection reads persistence without re-favoriting and masking an account-key bug.
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(learned.clone());
    h.advance(4);
    assert_eq!(
        h.take::<ProfessionSnapshot>().last().unwrap().favorites,
        [2153]
    );
    assert_eq!(persistence::load_inventory(h.db(), owner), inventory);
    assert_eq!(persistence::load_professions(h.db(), owner), learned);
}

#[test]
fn modern_crafting_unlearned_recipe_refuses_cast_until_real_trainer_purchase() {
    let mut h = harness(10).unwrap();
    h.set_gold(1000);
    let npc = h.open_trainer(5564);
    h.buy(npc, 264578);
    assert!(h.take::<TrainerBuyFailed>().is_empty());
    // Rank is an explicit fixture prerequisite, not evidence of acquiring skill points.
    with_learner(h.app.world_mut(), h.player, |learner, profs| {
        learner.set_skill(profs, 2532, 1, 15, 300);
    });
    h.advance(3);
    h.give(2318, "Light Leather", 4);
    h.give(2320, "Coarse Thread", 1);
    h.take::<CastFailed>();
    let inventory = persistence::load_inventory(h.db(), h.character_id);
    let professions = h.professions();
    assert!(!professions.knows(2153));
    let snapshot = h.take::<ProfessionSnapshot>().pop().unwrap();
    assert!(
        snapshot
            .recipe_sources
            .iter()
            .all(|source| professions.rank(source.req_skill_line) > 0)
    );
    assert_eq!(
        snapshot
            .recipe_sources
            .iter()
            .find(|source| source.spell_id == 2153 && source.trainer_id == 56),
        Some(&shared::protocol::ProfessionTrainerSource {
            trainer_id: 56,
            spell_id: 2153,
            cost: 50,
            req_skill_line: 2532,
            req_skill_rank: 15,
            req_abilities: vec![],
            req_level: 0,
        })
    );
    assert!(
        !snapshot
            .recipe_sources
            .iter()
            .any(|source| source.req_skill_line == 2540)
    );
    assert_eq!(
        persistence::load_inventory(h.db(), h.character_id),
        inventory
    );
    h.craft(2153, 1);
    assert!(!h.casting());
    assert!(!h.take::<CastFailed>().is_empty());
    assert_eq!(
        persistence::load_inventory(h.db(), h.character_id),
        inventory
    );
    assert_eq!(h.professions(), professions);
    assert_eq!(
        persistence::load_professions(h.db(), h.character_id),
        professions
    );
    h.buy(npc, 2153);
    assert!(h.take::<TrainerBuyFailed>().is_empty());
    assert!(h.professions().knows(2153));
    assert_eq!(h.gold(), inventory.gold - 50);
    assert_eq!(h.line(2532).unwrap().rank, 15);
    assert_eq!(h.bag_count(2318), 4);
    assert_eq!(h.bag_count(2320), 1);
    assert_eq!(h.bag_count(2303), 0);
    assert_eq!(
        h.take::<ProfessionSnapshot>().last(),
        Some(
            &h.professions()
                .snapshot(h.app.world().resource::<ProfessionData>())
        )
    );
}

#[test]
fn modern_crafting_leatherworking_trainer_cast_reagents_skillup_and_persistence() {
    let Some(mut h) = harness(10) else { return };
    h.set_gold(1000);
    let npc = h.open_trainer(5564); // Simon Tanner: trainer56, gossip4203/0.
    h.buy(npc, 264578);
    assert!(h.take::<TrainerBuyFailed>().is_empty());
    assert!(h.professions().knows(2108));
    assert!(h.professions().knows(2881));
    assert_eq!(h.line(2532).unwrap().rank, 1);
    assert_eq!(h.line(2532).unwrap().max_rank, 300);
    assert_eq!(h.gold(), 990);
    assert!(!h.professions().knows(2153));

    h.give(2934, "Ruined Leather Scraps", 42);
    h.craft(2881, 14);
    assert!(h.casting());
    h.advance(1000); // 50s budget for this authored multi-cast queue.
    h.finish_crafting();
    assert_eq!(h.bag_count(2934), 0);
    assert_eq!(h.bag_count(2318), 14);
    assert_eq!(h.line(2532).unwrap().rank, 15);

    h.buy(npc, 2153); // Handstitched Leather Pants, authored rank15/cost50.
    assert!(h.take::<TrainerBuyFailed>().is_empty());
    assert!(h.professions().knows(2153));
    assert_eq!(h.gold(), 940);
    h.give(2320, "Coarse Thread", 1);
    h.craft(2153, 1);
    assert!(h.casting());
    h.finish_crafting();
    assert_eq!(h.bag_count(2318), 10);
    assert_eq!(h.bag_count(2320), 0);
    assert_eq!(h.bag_count(2303), 1);
    assert_eq!(h.line(2532).unwrap().rank, 16);
    assert!(h.take::<CastFailed>().is_empty());
    assert_eq!(
        persistence::load_professions(h.db(), h.character_id),
        h.professions()
    );
    assert_eq!(
        h.take::<ProfessionSnapshot>().last(),
        Some(
            &h.professions()
                .snapshot(h.app.world().resource::<ProfessionData>())
        )
    );
}

#[test]
fn guild_news_real_sulfuras_craft_records_only_successful_created_item() {
    let mut h = harness(80).unwrap();
    crate::persistence_character::set_character_guild_by_id(h.db(), h.character_id, 7, "Crafters")
        .unwrap();
    let spell = world_db().unwrap().store.get(21160).unwrap().clone();
    h.app.insert_resource(SpellStore::fixture(vec![spell]));
    with_learner(h.app.world_mut(), h.player, |learner, profs| {
        learner.learn_spell(profs, 21160, 0);
    })
    .unwrap();
    h.craft(21160, 1);
    h.finish_crafting();
    assert_eq!(h.bag_count(17182), 0);
    assert!(
        crate::guild_news::query(h.db(), h.character_id)
            .entries
            .is_empty()
    );
    // Authored CREATE_ITEM24 Sulfuras recipe: Sulfuron Hammer + Eye of Sulfuras.
    h.give(17193, "Sulfuron Hammer", 1);
    h.give(17204, "Eye of Sulfuras", 1);
    h.craft(21160, 1);
    h.finish_crafting();
    assert_eq!(h.bag_count(17182), 1, "real recipe must finish before news");
    assert_eq!(h.bag_count(17193), 0);
    assert_eq!(h.bag_count(17204), 0);
    let entries = crate::guild_news::query(h.db(), h.character_id).entries;
    assert_eq!(entries.len(), 1);
    assert!(
        matches!(&entries[0].kind, shared::protocol::GuildNewsKind::ItemCrafted {
        member, item_id: 17182
    } if member == "Crafter")
    );
    h.craft(21160, 1);
    h.finish_crafting();
    assert_eq!(
        crate::guild_news::query(h.db(), h.character_id).entries,
        entries
    );
}

#[test]
fn crafting_two_bolts_consumes_four_linen_and_raises_classic_tailoring_twice() {
    let Some(mut h) = harness(10) else { return };
    h.learn_tailoring();
    h.give(LINEN_CLOTH, "Linen Cloth", 5);
    h.craft(BOLT_OF_LINEN_CLOTH, 2);
    assert!(h.casting(), "first cast started");
    h.finish_crafting();

    assert_eq!(h.bag_count(LINEN_CLOTH), 1);
    assert_eq!(h.bag_count(BOLT_ITEM), 2);
    // Orange below rank 25: every craft raises the skill (1000 permille).
    assert_eq!(h.line(CLASSIC_TAILORING).unwrap().rank, 3);
    assert!(h.take::<CastFailed>().is_empty());
    let snapshots = h.take::<ProfessionSnapshot>();
    assert_eq!(
        snapshots.last(),
        Some(
            &h.professions()
                .snapshot(h.app.world().resource::<ProfessionData>())
        )
    );
}

#[test]
fn crafting_without_linen_fails_with_missing_reagent() {
    let Some(mut h) = harness(10) else { return };
    h.learn_tailoring();
    h.give(LINEN_CLOTH, "Linen Cloth", 1);
    h.craft(BOLT_OF_LINEN_CLOTH, 1);

    let failed = h.take::<CastFailed>();
    assert_eq!(
        failed
            .iter()
            .map(|f| (f.spell_id, f.reason, f.detail.clone()))
            .collect::<Vec<_>>(),
        vec![(
            BOLT_OF_LINEN_CLOTH,
            CastFailReason::Reagents,
            Some("Missing reagent: Linen Cloth".into())
        )]
    );
    assert!(!h.casting());
    assert_eq!(h.bag_count(LINEN_CLOTH), 1);
    assert_eq!(h.line(CLASSIC_TAILORING).unwrap().rank, 1);
}

#[test]
fn queued_crafts_stop_when_the_linen_runs_out() {
    let Some(mut h) = harness(10) else { return };
    h.learn_tailoring();
    h.give(LINEN_CLOTH, "Linen Cloth", 4);
    h.craft(BOLT_OF_LINEN_CLOTH, 3);
    h.finish_crafting();

    assert_eq!(h.bag_count(BOLT_ITEM), 2);
    assert_eq!(h.bag_count(LINEN_CLOTH), 0);
    let failed = h.take::<CastFailed>();
    assert_eq!(
        failed.iter().map(|f| f.reason).collect::<Vec<_>>(),
        vec![CastFailReason::Reagents]
    );
}

/// One bank tab holding `count` Linen Cloth.
fn linen_bank(h: &Harness, count: u32) -> persistence::banks::BankData {
    let mut tab = crate::bank_storage::BankTabData::new("Reagents".into());
    tab.slots[0] = Some(InventoryItemData {
        item_guid: persistence::next_id(h.db(), "item_instance"),
        item_id: LINEN_CLOTH,
        name: "Linen Cloth".into(),
        quality: 1,
        required_level: 0,
        stack_count: count,
        vendor_sell_price: 0,
        unique_auction: false,
    });
    persistence::banks::BankData {
        tabs: vec![tab],
        ..Default::default()
    }
}

fn linen_in(bank: &persistence::banks::BankData) -> u32 {
    bank.tabs
        .iter()
        .flat_map(|tab| tab.slots.iter().flatten())
        .filter(|item| item.item_id == LINEN_CLOTH)
        .map(|item| item.stack_count)
        .sum()
}

/// Retail `ItemUtil.GetCraftingReagentCount` counts bags, the character bank
/// and the Warband bank; reagents are consumed in project policy
/// CRAFTING_REAGENT_CONSUMPTION_ORDER_RETAIL_ACCESSIBLE_BAGS.
#[test]
fn crafting_consumes_reagents_from_bags_then_character_bank_then_warband_bank() {
    use persistence::banks::{BankOwner, load_account_bank, load_character_bank, save_bank_move};
    let Some(mut h) = harness(10) else { return };
    h.learn_tailoring();
    h.give(LINEN_CLOTH, "Linen Cloth", 1);
    let character_bank = linen_bank(&h, 2);
    let warband_bank = linen_bank(&h, 5);
    save_bank_move(
        h.db(),
        h.character_id,
        BankOwner::Character(h.character_id, &character_bank),
        None,
    );
    // The harness connection authenticates account 1.
    save_bank_move(
        h.db(),
        h.character_id,
        BankOwner::Account(1, &warband_bank),
        None,
    );

    h.craft(BOLT_OF_LINEN_CLOTH, 1);
    h.finish_crafting();
    assert!(h.take::<CastFailed>().is_empty());
    assert_eq!(h.bag_count(BOLT_ITEM), 1);
    assert_eq!(h.bag_count(LINEN_CLOTH), 0);
    assert_eq!(linen_in(&load_character_bank(h.db(), h.character_id)), 1);
    assert_eq!(linen_in(&load_account_bank(h.db(), 1)), 5);

    h.craft(BOLT_OF_LINEN_CLOTH, 1);
    h.finish_crafting();
    assert!(h.take::<CastFailed>().is_empty());
    assert_eq!(h.bag_count(BOLT_ITEM), 2);
    assert_eq!(linen_in(&load_character_bank(h.db(), h.character_id)), 0);
    assert_eq!(linen_in(&load_account_bank(h.db(), 1)), 4);
}

/// The client counts bank reagents from `BankContents`, so a craft that takes
/// from a bank sends that bank's new contents, banker or not.
#[test]
#[ignore = "RED: CRAFTING_REAGENTS_FROM_BANK"]
fn crafting_from_banks_sends_their_new_contents() {
    use persistence::banks::{BankOwner, save_bank_move};
    use shared::protocol::{BankContents, BankType};
    let Some(mut h) = harness_with(10, |app| {
        add_inbox::<BankContents>(app);
        app.add_plugins(crate::bank::BankPlugin);
    }) else {
        return;
    };
    let connection = h.connection;
    h.app
        .world_mut()
        .entity_mut(connection)
        .insert(pair::<BankContents>());
    h.learn_tailoring();
    let character_bank = linen_bank(&h, 1);
    let warband_bank = linen_bank(&h, 5);
    save_bank_move(
        h.db(),
        h.character_id,
        BankOwner::Character(h.character_id, &character_bank),
        None,
    );
    save_bank_move(
        h.db(),
        h.character_id,
        BankOwner::Account(1, &warband_bank),
        None,
    );
    h.take::<BankContents>();

    h.craft(BOLT_OF_LINEN_CLOTH, 1);
    h.finish_crafting();
    assert!(h.take::<CastFailed>().is_empty());
    let sent = h.take::<BankContents>();
    let linen_sent = |bank| {
        sent.iter()
            .rev()
            .find(|contents| contents.bank == bank)
            .map(|contents| {
                contents
                    .tabs
                    .iter()
                    .flat_map(|tab| tab.slots.iter().flatten())
                    .filter(|item| item.item_id == LINEN_CLOTH)
                    .map(|item| item.count)
                    .sum::<u32>()
            })
    };
    assert_eq!(linen_sent(BankType::Character), Some(0));
    assert_eq!(linen_sent(BankType::Account), Some(4));
}

#[test]
fn stopping_the_cast_cancels_the_remaining_crafts() {
    let Some(mut h) = harness(10) else { return };
    h.learn_tailoring();
    h.give(LINEN_CLOTH, "Linen Cloth", 6);
    h.craft(BOLT_OF_LINEN_CLOTH, 3);
    h.send::<_, CombatChannel>(StopSpellCast);
    h.advance(60);

    assert!(!h.casting());
    assert_eq!(h.bag_count(BOLT_ITEM), 0);
    assert_eq!(h.bag_count(LINEN_CLOTH), 6);
}

#[test]
fn grey_bolt_of_linen_gives_no_skill() {
    let Some(mut h) = harness(10) else { return };
    h.learn_tailoring();
    h.set_rank(CLASSIC_TAILORING, 50);
    h.give(LINEN_CLOTH, "Linen Cloth", 2);
    h.craft(BOLT_OF_LINEN_CLOTH, 1);
    h.finish_crafting();

    assert_eq!(h.bag_count(BOLT_ITEM), 1);
    assert_eq!(h.line(CLASSIC_TAILORING).unwrap().rank, 50);
}

#[test]
fn unknown_recipe_cannot_be_crafted() {
    let Some(mut h) = harness(10) else { return };
    h.learn_tailoring();
    h.give(2996, "Bolt of Linen Cloth", 1);
    h.give(2320, "Coarse Thread", 1);
    h.give(2324, "Bleach", 1);
    h.craft(WHITE_LINEN_SHIRT, 1);

    let failed = h.take::<CastFailed>();
    assert_eq!(
        failed.iter().map(|f| f.detail.clone()).collect::<Vec<_>>(),
        vec![Some("spell not known".into())]
    );
    assert!(!h.casting());
}

#[test]
fn transmogacq_crafting_an_unbound_shirt_does_not_collect() {
    let Some(mut h) = harness(10) else { return };
    let npc = h.learn_tailoring();
    h.buy(npc, WHITE_LINEN_SHIRT);
    assert!(h.professions().knows(WHITE_LINEN_SHIRT));
    assert_eq!(h.gold(), 80);
    h.give(2996, "Bolt of Linen Cloth", 1);
    h.give(2320, "Coarse Thread", 1);
    h.give(2324, "Bleach", 1);
    h.craft(WHITE_LINEN_SHIRT, 1);
    h.finish_crafting();

    assert_eq!(h.bag_count(2576), 1, "White Linen Shirt");
    assert_eq!(h.bag_count(2996) + h.bag_count(2320) + h.bag_count(2324), 0);
    let templates = h
        .app
        .world()
        .resource::<crate::item_templates::ItemTemplates>();
    let appearance = templates
        .get(shared::item_data::ItemDefinitionSource::Retail, 2576)
        .unwrap()
        .appearance_id
        .expect("shirt has an appearance");
    assert!(
        !crate::persistence::appearance_store::load_account_appearances(h.db(), 1).has(appearance)
    );
    crate::appearance_collection::tests::assert_restart(h.app, 1, &[(appearance, false)]);
}

#[test]
fn online_learn_spell_trainer_publishes_without_relog() {
    use crate::class_progression::{ClassProgressionPlugin, KnownSpells};
    use shared::protocol::{KnownSpellsSnapshot, SpellsLearned};

    let mut h = harness(5).unwrap();
    ClassProgressionPlugin.build(&mut h.app);
    add_inbox::<KnownSpellsSnapshot>(&mut h.app);
    add_inbox::<SpellsLearned>(&mut h.app);
    h.app
        .world_mut()
        .entity_mut(h.connection)
        .insert((pair::<KnownSpellsSnapshot>(), pair::<SpellsLearned>()));
    h.advance(3);
    h.take::<KnownSpellsSnapshot>();
    h.take::<SpellsLearned>();
    h.set_gold(100);
    let npc = h.open_georgio();
    h.buy(npc, TAILORING);
    assert!(h.take::<TrainerBuyFailed>().is_empty());
    let profs = h.professions();
    assert!(profs.knows(BOLT_OF_LINEN_CLOTH));
    let learned: Vec<u32> = h
        .take::<SpellsLearned>()
        .into_iter()
        .flat_map(|delta| delta.spells)
        .collect();
    assert!(
        learned.contains(&BOLT_OF_LINEN_CLOTH),
        "client learned {learned:?}"
    );
    let known = h.app.world().get::<KnownSpells>(h.player).unwrap();
    for spell in profs.spells {
        assert!(
            known.contains(spell),
            "server does not know learned {spell}"
        );
    }
}

#[test]
fn achievements_criteria_skill_step_reads_learned_line_maximum_in_seventy_five_point_steps() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let Some(mut h) = harness_with(10, |app| {
        let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
        app.insert_resource(load_achievement_catalog(&conn).unwrap());
        app.add_plugins(AchievementPlugin);
    }) else {
        return;
    };
    let player = h.player;
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(PlayerAchievementState::default());
    h.advance(2);
    let counter = |h: &Harness| {
        h.app
            .world()
            .get::<PlayerAchievementState>(player)
            .unwrap()
            .criteria_progress
            .get(&5561)
            .copied()
    };
    // Professions learned1199, Tailoring197 leaf criterion5561: nothing before learning.
    assert_eq!(counter(&h), None);
    h.learn_tailoring();
    let tailoring = h.line(TAILORING_SKILL).unwrap();
    assert!(tailoring.max_rank >= 75, "{tailoring:?}");
    assert_eq!(counter(&h), Some(u64::from(tailoring.max_rank)));
    let catalog = h
        .app
        .world()
        .resource::<crate::achievement::AchievementCatalog>();
    let state = h.app.world().get::<PlayerAchievementState>(player).unwrap();
    // Tree1165 amount1 means one 75-point step (TC CriteriaHandler.cpp:1278-1279).
    let leaf = catalog.tree_progress(1165, state);
    assert_eq!((leaf.required, leaf.completed), (75, true));
}

#[path = "profession/concentration_tests.rs"]
mod concentration_tests;
