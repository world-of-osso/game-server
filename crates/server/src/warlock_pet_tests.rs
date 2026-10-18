//! Summon Imp through real DB2 spell data, pet messages, AI and reopened redb.
use std::path::Path;

use lightyear::prelude::{MessageReceiver, MessageSender};
use rusqlite::{Connection, OpenFlags};
use shared::components::{
    Level, ModelDisplay, Npc, PowerType, UnitLevel, UnitPowers, UnitSummonedBy,
};
use shared::protocol::{CombatChannel, CombatLogKind};

use super::*;
use crate::auth::InWorld;
use crate::hunter_pet::HunterPetPlugin;
use crate::persistence::{self, GameDatabase};
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};
use crate::world_data::load_world_data;

const SUMMON_IMP: u32 = 688;
const FIREBOLT: u32 = 3110;
const DISMISS_PET: u32 = 2641;
const WARLOCK: u8 = 9;
const IMP: u32 = 416;

// The generic cast harness enters the world before returning. Defer that one
// entry until this fixture installs level-3 state (Summon Imp's DB2 SpellLevel) and pet message endpoints.
fn defer_initial_pet_login(
    mut commands: Commands,
    connections: Query<Entity, Added<InWorld>>,
    mut deferred: Local<bool>,
) {
    if *deferred {
        return;
    }
    for connection in &connections {
        commands.entity(connection).remove::<InWorld>();
        *deferred = true;
    }
}

struct ImpHarness {
    h: Harness,
    bars: Vec<PetSpells>,
    clears: usize,
}

impl ImpHarness {
    fn login(path: &Path) -> Self {
        Self::login_with_spells(path, 3, &[SUMMON_IMP, FIREBOLT])
    }

    fn login_with_spells(path: &Path, level: u8, known: &[u32]) -> Self {
        Self::login_with_class(path, level, known, WARLOCK)
    }

    fn login_with_class(path: &Path, level: u8, known: &[u32], class: u8) -> Self {
        let conn = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let spells = load_pet_levelup_spells(&conn).unwrap();
        let prepared = crate::spell_cast::test_harness::prepare_class_world();
        let mut data = prepared.world_data();
        data.templates
            .retain(|entry, _| [416, 417, 1860, 1863, 17252, 184600, 78116].contains(entry));
        let db = redb::Database::create(path).unwrap();
        persistence::create_tables(&db);
        let mut h = harness_with_plugins(Vec::new(), known, class, true, |app| {
            app.insert_resource(data);
            app.insert_resource(spells);
            app.insert_resource(GameDatabase::new(db));
            app.add_plugins((HunterPetPlugin, crate::creature_ai::CreatureAiPlugin));
            app.add_systems(PreUpdate, defer_initial_pet_login);
            app.add_systems(
                FixedUpdate,
                (
                    process_pet_actions,
                    process_pet_autocast_toggles,
                    pet_ai_system,
                )
                    .chain()
                    .after(crate::combat::auto_attack_system),
            );
        })
        .expect("real spell data");
        h.app.world_mut().entity_mut(h.caster).insert(Level(level));
        // Keep the full live spell catalog, not the generic harness's three-spell fixture.
        h.app.insert_resource(
            crate::spell_cast::test_harness::world_db()
                .unwrap()
                .store
                .clone(),
        );
        h.app.world_mut().entity_mut(h.connection).insert((
            MessageSender::<PetAction>::default(),
            MessageReceiver::<PetAction>::default(),
            MessageSender::<PetSpells>::default(),
            MessageReceiver::<PetSpells>::default(),
            MessageSender::<PetClearSpells>::default(),
            MessageReceiver::<PetClearSpells>::default(),
        ));
        // Install the real login boundary only after the level and pet-message
        // endpoints exist. The generic harness has already performed one update.
        h.app
            .world_mut()
            .entity_mut(h.connection)
            .insert(InWorld { character_id: 1 });
        h.app.world_mut().entity_mut(h.npc).insert(Position {
            x: 15.0,
            y: 0.0,
            z: 0.0,
        });
        let mut result = Self {
            h,
            bars: Vec::new(),
            clears: 0,
        };
        result.advance(3);
        result
    }

    fn advance(&mut self, frames: usize) {
        for _ in 0..frames {
            self.h.advance(1);
            self.bars.extend(
                self.h
                    .app
                    .world_mut()
                    .get_mut::<MessageReceiver<PetSpells>>(self.h.connection)
                    .unwrap()
                    .receive(),
            );
            self.clears += self
                .h
                .app
                .world_mut()
                .get_mut::<MessageReceiver<PetClearSpells>>(self.h.connection)
                .unwrap()
                .receive()
                .count();
        }
    }

    fn summon(&mut self) -> Entity {
        self.h.cast(SUMMON_IMP, None);
        self.advance(130); // six-second summon and message delivery
        assert!(self.h.inbox.failed.is_empty(), "{:?}", self.h.inbox.failed);
        self.pet().expect("Summon Imp must create the pet")
    }

    fn pet(&self) -> Option<Entity> {
        crate::hunter_pet::summoned_pet(self.h.app.world(), self.h.caster)
    }

    fn order(&mut self, command: u32, target: Option<Entity>) {
        let pet = self.pet().unwrap().to_bits();
        self.h
            .app
            .world_mut()
            .get_mut::<MessageSender<PetAction>>(self.h.connection)
            .unwrap()
            .send::<CombatChannel>(PetAction {
                pet,
                action: pet_action_button(command, ACT_COMMAND),
                target: target.map(Entity::to_bits),
                position: None,
            });
        self.advance(3);
    }

    fn dismiss(&mut self) {
        assert!(
            !self
                .h
                .app
                .world()
                .get::<crate::class_progression::KnownSpells>(self.h.caster)
                .unwrap()
                .contains(DISMISS_PET)
        );
        self.order(shared::protocol::COMMAND_ABANDON, None);
        assert!(self.h.inbox.failed.is_empty(), "{:?}", self.h.inbox.failed);
    }
}

#[test]
fn pet_lifecycle_warlock_authored_demons_summon_dismiss_and_reopen() {
    // Local SpellEffect SUMMON_PET MiscValue rows; Incubus184600 is not in the active
    // content template view and is recorded as a separate source-data boundary.
    for (spell, entry) in [
        (688, 416),
        (697, 1860),
        (712, 1863),
        (691, 417),
        (30146, 17252),
        (366222, 1863),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("demons.redb");
        let mut first = ImpHarness::login_with_spells(&path, 60, &[spell]);
        first.h.cast(spell, None);
        first.advance(150);
        assert!(
            first.h.inbox.failed.is_empty(),
            "spell{spell}: {:?}",
            first.h.inbox.failed
        );
        let pet = first.pet().expect("authored demon summoned");
        assert_eq!(
            first.h.app.world().get::<Npc>(pet).unwrap().template_id,
            entry
        );
        let max_health = first.h.app.world().get::<Health>(pet).unwrap().max;
        // Felhunter's legacy expansion base-health row is1; preserve a legal wound
        // within the computed maximum, not an over-max fixture.
        let wounded_health = 23.0_f32.min(max_health * 0.5);
        println!("spell{spell} entry{entry}: max={max_health}, wound={wounded_health}");
        first
            .h
            .app
            .world_mut()
            .get_mut::<Health>(pet)
            .unwrap()
            .current = wounded_health;
        first.advance(3);
        drop(first);
        let mut reloaded = ImpHarness::login_with_spells(&path, 60, &[spell]);
        let pet = reloaded.pet().expect("saved demon relogged");
        assert_eq!(
            reloaded.h.app.world().get::<Npc>(pet).unwrap().template_id,
            entry
        );
        assert_eq!(
            reloaded.h.app.world().get::<Health>(pet).unwrap().current,
            wounded_health,
            "spell{spell}: wounded health changed across storage reopen"
        );
        reloaded.dismiss();
        assert!(reloaded.pet().is_none());
        drop(reloaded);
        let mut dismissed = ImpHarness::login_with_spells(&path, 60, &[spell]);
        assert!(dismissed.pet().is_none(), "dismissed demon auto-restored");
        dismissed.h.cast(spell, None);
        dismissed.advance(150);
        assert!(
            dismissed.h.inbox.failed.is_empty(),
            "{:?}",
            dismissed.h.inbox.failed
        );
        let pet = dismissed.pet().expect("explicit resummon after dismiss");
        assert_eq!(
            dismissed.h.app.world().get::<Npc>(pet).unwrap().template_id,
            entry
        );
        let health = dismissed.h.app.world().get::<Health>(pet).unwrap();
        assert_eq!(
            health.current, health.max,
            "explicit summon starts living/full"
        );
    }
}

#[test]
fn pet_lifecycle_fel_domination_accelerates_one_authored_summon() {
    const FEL_DOMINATION: u32 = 333_889;
    let directory = tempfile::tempdir().unwrap();
    let mut normal =
        ImpHarness::login_with_spells(&directory.path().join("normal.redb"), 60, &[SUMMON_IMP]);
    normal.h.cast(SUMMON_IMP, None);
    normal.advance(20);
    assert!(
        normal.pet().is_none(),
        "ordinary six-second summon completed within one second"
    );
    assert!(
        normal.h.inbox.failed.is_empty(),
        "{:?}",
        normal.h.inbox.failed
    );

    let mut accelerated = ImpHarness::login_with_spells(
        &directory.path().join("accelerated.redb"),
        60,
        &[SUMMON_IMP, FEL_DOMINATION],
    );
    accelerated.h.cast(FEL_DOMINATION, None);
    accelerated.advance(3);
    assert!(
        accelerated.h.inbox.failed.is_empty(),
        "{:?}",
        accelerated.h.inbox.failed
    );
    assert!(
        accelerated
            .h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(accelerated.h.caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == FEL_DOMINATION)
    );
    accelerated.h.cast(SUMMON_IMP, None);
    accelerated.advance(20);
    assert!(
        accelerated.h.inbox.failed.is_empty(),
        "{:?}",
        accelerated.h.inbox.failed
    );
    assert!(
        accelerated.pet().is_some(),
        "90% faster summon did not complete within one second"
    );
    assert!(
        !accelerated
            .h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(accelerated.h.caster)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == FEL_DOMINATION),
        "next-summon charge was not consumed"
    );
}

#[test]
fn achievements_criteria_killing_blow_from_authentic_summoned_imp_credits_owner_and_persists() {
    use crate::achievement::{AchievementPlugin, PlayerAchievementState, load_achievement_catalog};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pet-killing-blow.redb");
    let mut h = ImpHarness::login(&path);
    let db = &h.h.app.world().resource::<GameDatabase>().db;
    let (character, _) = persistence::create_character(db, 1, "ImpKillproof", 1, WARLOCK).unwrap();
    assert_eq!(character, 1);
    h.h.app
        .world_mut()
        .entity_mut(h.h.caster)
        .insert(PlayerAchievementState::default());
    let conn = Connection::open(crate::test_data::world_db_path()).unwrap();
    h.h.app
        .insert_resource(load_achievement_catalog(&conn).unwrap());
    bevy::app::Plugin::build(&AchievementPlugin, &mut h.h.app);
    let pet = h.summon();
    assert!(
        !h.h.app
            .world()
            .get::<PlayerAchievementState>(h.h.caster)
            .unwrap()
            .criteria_progress
            .contains_key(&5529)
    );
    h.h.app
        .world_mut()
        .get_mut::<shared::components::Health>(h.h.npc)
        .unwrap()
        .current = 1.0;
    h.order(shared::protocol::COMMAND_ATTACK, Some(h.h.npc));
    h.advance(200);
    assert_eq!(h.h.health(h.h.npc), 0.0);
    assert!(
        h.h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Death
                && event.source == Some(pet.to_bits())
                && event.target == Some(h.h.npc.to_bits()))
    );
    assert_eq!(
        h.h.app
            .world()
            .get::<PlayerAchievementState>(h.h.caster)
            .unwrap()
            .criteria_progress
            .get(&5529),
        Some(&1)
    );
    h.advance(20);
    assert_eq!(
        h.h.app
            .world()
            .get::<PlayerAchievementState>(h.h.caster)
            .unwrap()
            .criteria_progress
            .get(&5529),
        Some(&1)
    );
    drop(h);
    let reopened = redb::Database::open(&path).unwrap();
    assert_eq!(
        persistence::load_achievements(&reopened, character)
            .criteria_progress
            .get(&5529),
        Some(&1)
    );
}

#[test]
fn class_completeness_native_water_elemental_recall_dismiss_and_record_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("elemental.redb");
    let known = [31687, 31707, 33395];
    let mut h = ImpHarness::login_with_class(&path, 80, &known, 8);
    h.h.cast(31687, None);
    h.advance(130);
    assert!(h.h.inbox.failed.is_empty(), "{:?}", h.h.inbox.failed);
    let pet = h.pet().expect("native Water Elemental");
    assert_eq!(h.h.app.world().get::<Npc>(pet).unwrap().template_id, 78116);
    assert_eq!(
        h.h.app.world().get::<UnitSummonedBy>(pet).unwrap().0,
        h.h.caster.to_bits()
    );
    assert!(
        h.h.app
            .world()
            .get::<UnitPowers>(pet)
            .unwrap()
            .entries
            .iter()
            .any(|pool| pool.power == PowerType::Mana)
    );
    assert!(
        h.h.app
            .world()
            .get::<shared::components::Mana>(pet)
            .unwrap()
            .max
            > 0.0
    );
    h.advance(310); // Native31687 has15000ms cooldown, not a fixture override.
    h.h.cast(31687, None);
    h.advance(130);
    assert!(h.h.inbox.failed.is_empty(), "{:?}", h.h.inbox.failed);
    assert_eq!(
        h.pet(),
        Some(pet),
        "living elemental recalls, not duplicates"
    );
    h.order(COMMAND_ABANDON, None);
    h.advance(4);
    assert!(h.pet().is_none());
    assert!(
        h.h.app
            .world()
            .get::<crate::hunter_pet::PetStable>(h.h.caster)
            .unwrap()
            .stored
            .active[0]
            .is_some(),
        "SUMMON_PET record retained, unlike abandoned hunter"
    );
    drop(h);
    let mut h = ImpHarness::login_with_class(&path, 80, &known, 8);
    assert!(h.pet().is_none(), "dismissed pet is not restored at login");
    h.h.cast(31687, None);
    h.advance(130);
    assert!(h.h.inbox.failed.is_empty(), "{:?}", h.h.inbox.failed);
    assert_eq!(
        h.h.app
            .world()
            .get::<Npc>(h.pet().unwrap())
            .unwrap()
            .template_id,
        78116
    );
    let dead = h.pet().unwrap();
    h.h.app.world_mut().get_mut::<Health>(dead).unwrap().current = 0.0;
    h.advance(6);
    assert!(
        h.pet().is_none(),
        "non-hunter corpse removed, record retained"
    );
    h.advance(310);
    h.h.cast(31687, None);
    h.advance(130);
    assert!(h.h.inbox.failed.is_empty(), "{:?}", h.h.inbox.failed);
    assert_ne!(h.pet(), Some(dead));
    assert!(h.h.health(h.pet().unwrap()) > 0.0);
}

#[test]
#[ignore = "RED: target32 must keep computed height when ground data is unavailable"]
fn warlock_summon_imp_without_ground_data_keeps_height_and_succeeds() {
    for empty_ground_resource in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut h = ImpHarness::login(&dir.path().join("pets.redb"));
        h.h.app
            .world_mut()
            .remove_resource::<crate::ground::GroundMap>();
        if empty_ground_resource {
            h.h.app.insert_resource(crate::ground::GroundMap::new(
                dir.path(),
                Default::default(),
            ));
        }
        let height = 123.25;
        h.h.app
            .world_mut()
            .get_mut::<Position>(h.h.caster)
            .unwrap()
            .y = height;
        let pet = h.summon();
        let world = h.h.app.world();
        assert_eq!(world.get::<Npc>(pet).unwrap().template_id, IMP);
        assert_eq!(world.get::<Position>(pet).unwrap().y, height);
        assert_eq!(
            world.get::<UnitSummonedBy>(pet).unwrap().0,
            h.h.caster.to_bits()
        );
    }
}

#[test]
fn warlock_summon_imp_spawns_owned_pet_and_firebolt_bar() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = ImpHarness::login(&dir.path().join("pets.redb"));
    let pet = h.summon();
    let summons: Vec<_> =
        h.h.inbox
            .log
            .iter()
            .filter(|event| event.kind == CombatLogKind::Summon)
            .collect();
    assert_eq!(summons.len(), 1, "spell-created pet emits one summon");
    assert_eq!(summons[0].source, Some(h.h.caster.to_bits()));
    assert_eq!(summons[0].target, Some(pet.to_bits()));
    assert_eq!(summons[0].spell_id, Some(SUMMON_IMP));
    let world = h.h.app.world();
    assert_eq!(world.get::<Npc>(pet).unwrap().template_id, IMP);
    assert_eq!(
        world.get::<UnitSummonedBy>(pet).unwrap().0,
        h.h.caster.to_bits()
    );
    assert_eq!(
        world.get::<UnitFactionTemplate>(pet),
        world.get::<UnitFactionTemplate>(h.h.caster)
    );
    assert!(world.get::<ModelDisplay>(pet).unwrap().display_id > 0);
    let at = world.get::<Position>(pet).unwrap();
    let owner_at = world.get::<Position>(h.h.caster).unwrap();
    assert!(planar_distance(at, owner_at) <= 3.0);
    assert_eq!(world.get::<UnitLevel>(pet), Some(&UnitLevel(3)));
    assert!(
        world
            .get::<UnitPowers>(pet)
            .unwrap()
            .entries
            .iter()
            .any(|pool| pool.power == PowerType::Energy && pool.current >= 40)
    );
    let bar = h.bars.last().expect("pet bar received by owner");
    assert_eq!(bar.pet, pet.to_bits());
    assert!(
        bar.action_buttons
            .contains(&pet_action_button(FIREBOLT, ACT_ENABLED))
    );
}

#[test]
fn warlock_imp_attack_autocasts_firebolt_and_follow_moves_to_owner() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = ImpHarness::login(&dir.path().join("pets.redb"));
    // The live target is Northshire's neutral Training Dummy (44548), not
    // the harness's hostile kobold. Include its real display and LOS bake.
    let data = load_world_data(&crate::test_data::world_db_path());
    let dummy = data.template_for(44548, 0).unwrap();
    h.h.app.world_mut().entity_mut(h.h.caster).insert(Position {
        x: -8949.0,
        y: 82.6,
        z: 132.0,
    });
    h.h.app.world_mut().entity_mut(h.h.npc).insert((
        Npc {
            template_id: 44548,
            name: dummy.name.clone(),
        },
        UnitFactionTemplate(u32::from(dummy.faction)),
        UnitFlags(dummy.unit_flags),
        ModelDisplay { display_id: 3019 },
        Position {
            x: -8965.57,
            y: 81.5936,
            z: 146.288,
        },
        crate::creature_scripts::TrainingDummy::default(),
    ));
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let wdt: u32 = conn
        .query_row("SELECT WdtFileDataID FROM map WHERE ID = 0", [], |row| {
            row.get(0)
        })
        .unwrap();
    h.h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(0, wdt)]),
    ));
    let pet = h.summon();
    let los_dir = crate::ground::ground_data_dir().join("los");
    crate::test_data::require(los_dir.join(los::eye::EYE_TABLE_FILE));
    h.h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(&los_dir).unwrap());
    h.order(COMMAND_ATTACK, Some(h.h.npc));
    h.advance(100);
    assert!(
        h.h.inbox.log.iter().any(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(h.h.npc.to_bits())
                && event.spell_id == Some(FIREBOLT)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0 // Real level-3 owner stats, not the harness's fixed 100 SP.
        }),
        "imp dealt no Firebolt damage: {:?}",
        h.h.inbox.log
    );
    h.h.app
        .world_mut()
        .get_mut::<Position>(h.h.caster)
        .unwrap()
        .x -= 30.0;
    h.order(COMMAND_FOLLOW, None);
    let before = h.h.app.world().get::<Position>(pet).unwrap().x;
    h.advance(20);
    assert!(h.h.app.world().get::<Position>(pet).unwrap().x < before);
    assert_eq!(h.h.app.world().get::<CombatTarget>(pet).unwrap().0, None);
}

#[test]
fn warlock_summon_imp_recalls_existing_pet_without_duplicating_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = ImpHarness::login(&dir.path().join("pets.redb"));
    let pet = h.summon();
    h.order(COMMAND_STAY, None);
    h.h.app
        .world_mut()
        .get_mut::<Position>(h.h.caster)
        .unwrap()
        .x = 30.0;
    assert_eq!(h.summon(), pet);
    let world = h.h.app.world();
    assert!(
        planar_distance(
            world.get::<Position>(pet).unwrap(),
            world.get::<Position>(h.h.caster).unwrap()
        ) <= 3.0
    );
    assert_eq!(h.bars.last().unwrap().pet, pet.to_bits());
}

#[test]
fn warlock_imp_current_pet_returns_after_reopening_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pets.redb");
    {
        let mut h = ImpHarness::login(&path);
        h.summon();
    }
    let h = ImpHarness::login(&path);
    let pet = h.pet().expect("permanent current imp restored at login");
    assert!(
        h.h.inbox
            .log
            .iter()
            .all(|event| event.kind != CombatLogKind::Summon),
        "login restoration is not a spell-driven summon"
    );
    assert_eq!(h.h.app.world().get::<Npc>(pet).unwrap().template_id, IMP);
    assert_eq!(h.h.app.world().get::<UnitLevel>(pet), Some(&UnitLevel(3)));
    assert!(
        h.bars
            .last()
            .unwrap()
            .action_buttons
            .contains(&pet_action_button(FIREBOLT, ACT_ENABLED))
    );
}

#[test]
fn warlock_pet_dismiss_removes_imp_and_does_not_restore_on_relog() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pets.redb");
    {
        let mut h = ImpHarness::login(&path);
        let pet = h.summon();
        h.dismiss();
        assert!(h.h.app.world().get_entity(pet).is_err());
        assert!(h.pet().is_none());
        assert_eq!(h.clears, 1);
    }
    let mut h = ImpHarness::login(&path);
    assert!(h.pet().is_none());
    assert!(h.bars.is_empty());
    let pet = h.summon();
    assert_eq!(h.h.app.world().get::<Npc>(pet).unwrap().template_id, IMP);
}

#[test]
fn warlock_imp_death_removes_pet_and_does_not_restore_on_relog() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pets.redb");
    {
        let mut h = ImpHarness::login(&path);
        let pet = h.summon();
        crate::combat::kill_unit(h.h.app.world_mut(), h.h.npc, pet);
        h.advance(4);
        assert!(h.h.app.world().get_entity(pet).is_err());
        assert!(h.pet().is_none());
        assert_eq!(h.clears, 1);
    }
    assert!(ImpHarness::login(&path).pet().is_none());
}
