//! Real-DB tests (data/world.db with `creature_family`; skipped when absent): a level 10
//! Human hunter calls its Diseased Young Wolf 299 with Call Pet 1, gets the pet bar, and
//! orders the wolf around the harness's hostile Kobold Vermin.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lightyear::prelude::{MessageReceiver, MessageSender};
use rusqlite::{Connection, OpenFlags};
use shared::components::{PowerType, UnitPowers, UnitSummonedBy};
use shared::protocol::CombatChannel;

use super::*;
use crate::hunter_pet::{HunterPetPlugin, PetStable};
use crate::persistence::pet_store::{StoredPet, StoredPetStable, save_pet_stable};
use crate::persistence::{self, GameDatabase};
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};
use crate::world_data::{CreatureTemplate, WorldData, load_expected_stats, load_templates};

/// `CLASS_HUNTER`.
const CLASS_HUNTER: u8 = 3;
const CALL_PET_1: u32 = 883;
/// Diseased Young Wolf: family 1, Wolf (skill lines 208 Pet - Wolf, 270 Pet - Generic Hunter).
const DISEASED_YOUNG_WOLF: u32 = 299;
const WOLF_DISPLAY: u32 = 1_413;
/// The harness connection's `InWorld` character.
const CHARACTER_ID: u64 = 1;
const DASH: u32 = 61_684;
const BITE: u32 = 17_253;
const GROWL: u32 = 2_649;
const FURIOUS_BITE: u32 = 263_840;

fn world_db() -> Option<Connection> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db");
    path.exists()
        .then(|| Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap())
}

fn wolf_template(conn: &Connection) -> HashMap<u32, CreatureTemplate> {
    let mut templates = load_templates(conn, &load_expected_stats(conn));
    templates.retain(|entry, _| *entry == DISEASED_YOUNG_WOLF);
    templates
}

fn db_path() -> PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("pets.redb");
    std::mem::forget(dir);
    path
}

/// A stable holding a Diseased Young Wolf (level 10) in slot 1, not out.
fn seeded_stable(path: &Path) -> GameDatabase {
    let db = redb::Database::create(path).expect("redb");
    persistence::create_tables(&db);
    let mut stored = StoredPetStable::default();
    stored.active[0] = Some(StoredPet {
        creature_entry: DISEASED_YOUNG_WOLF,
        display_id: WOLF_DISPLAY,
        name: "Diseased Young Wolf".into(),
        level: 10,
        family: 1,
    });
    save_pet_stable(&db, CHARACTER_ID, &stored);
    GameDatabase { db }
}

struct PetHarness {
    h: Harness,
    pet: Entity,
    bars: Vec<PetSpells>,
}

/// The level 10 hunter at the origin with its wolf called out and the Kobold Vermin
/// (hostile, 5000 health) moved 15 yards away; `PetSpells` received are collected.
fn hunter_with_wolf() -> Option<PetHarness> {
    let conn = world_db()?;
    let templates = wolf_template(&conn);
    let spells = load_pet_levelup_spells(&conn).expect("creature_family imported");
    let db = seeded_stable(&db_path());
    let mut h = harness_with_plugins(Vec::new(), &[CALL_PET_1], CLASS_HUNTER, false, |app| {
        app.insert_resource(WorldData {
            templates,
            ..Default::default()
        });
        app.insert_resource(db);
        app.insert_resource(spells);
        app.add_plugins((HunterPetPlugin, crate::creature_ai::CreatureAiPlugin));
        app.add_systems(
            FixedUpdate,
            (process_pet_actions, pet_ai_system)
                .chain()
                .after(crate::combat::auto_attack_system),
        );
    })?;
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<PetAction>::default(),
        MessageReceiver::<PetAction>::default(),
        MessageSender::<PetSpells>::default(),
        MessageReceiver::<PetSpells>::default(),
        MessageSender::<PetClearSpells>::default(),
        MessageReceiver::<PetClearSpells>::default(),
    ));
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: 15.0,
        y: 0.0,
        z: 0.0,
    });
    let mut harness = PetHarness {
        pet: Entity::PLACEHOLDER,
        h,
        bars: Vec::new(),
    };
    harness.advance(2);
    harness.h.cast(CALL_PET_1, None);
    harness.advance(3);
    harness.pet = harness
        .h
        .app
        .world()
        .get::<PetStable>(harness.h.caster)
        .and_then(|stable| stable.summoned)
        .expect("Call Pet 1 brought the wolf out");
    Some(harness)
}

impl PetHarness {
    fn advance(&mut self, frames: usize) {
        for _ in 0..frames {
            self.h.advance(1);
            let connection = self.h.connection;
            self.bars.extend(
                self.h
                    .app
                    .world_mut()
                    .get_mut::<MessageReceiver<PetSpells>>(connection)
                    .unwrap()
                    .receive(),
            );
        }
    }

    /// `CMSG_PET_ACTION`; delivered 3 frames later.
    fn order(&mut self, action: u32, target: Option<Entity>, position: Option<[f32; 3]>) {
        let pet = self.pet.to_bits();
        self.h
            .app
            .world_mut()
            .get_mut::<MessageSender<PetAction>>(self.h.connection)
            .unwrap()
            .send::<CombatChannel>(PetAction {
                pet,
                action,
                target: target.map(Entity::to_bits),
                position,
            });
        self.advance(3);
    }

    fn command(&mut self, command: u32) {
        self.order(pet_action_button(command, ACT_COMMAND), None, None);
    }

    fn react(&mut self, react: u32) {
        self.order(pet_action_button(react, ACT_REACTION), None, None);
    }

    fn attack_kobold(&mut self) {
        let kobold = self.h.npc;
        self.order(
            pet_action_button(COMMAND_ATTACK, ACT_COMMAND),
            Some(kobold),
            None,
        );
    }

    fn position(&self, unit: Entity) -> Position {
        *self.h.app.world().get::<Position>(unit).unwrap()
    }

    fn gap(&self, a: Entity, b: Entity) -> f32 {
        planar_distance(&self.position(a), &self.position(b))
    }

    fn pet_victim(&self) -> Option<Entity> {
        self.h
            .app
            .world()
            .get::<AutoAttack>(self.pet)
            .map(|attack| attack.victim)
    }

    fn pet_in_combat(&self) -> bool {
        let flags = self.h.app.world().get::<UnitFlags>(self.pet).unwrap();
        flags.0 & UnitFlags::PET_IN_COMBAT != 0
    }

    fn move_owner(&mut self, x: f32, z: f32) {
        let caster = self.h.caster;
        self.h
            .app
            .world_mut()
            .entity_mut(caster)
            .insert(Position { x, y: 0.0, z });
    }

    /// The kobold auto-attacks `victim` (a creature's AI target).
    fn kobold_attacks(&mut self, victim: Entity) {
        let kobold = self.h.npc;
        self.h
            .app
            .world_mut()
            .entity_mut(kobold)
            .insert(CombatTarget(Some(victim)));
    }

    fn last_bar(&self) -> &PetSpells {
        self.bars.last().expect("a PetSpells was sent")
    }
}

#[test]
fn level_up_spells_of_the_wolf_family_in_trinitycore_order() {
    let Some(conn) = world_db() else {
        return;
    };
    let spells = load_pet_levelup_spells(&conn).unwrap();
    let wolf: Vec<(u8, u32)> = spells.0[&1]
        .iter()
        .map(|spell| (spell.level, spell.spell))
        .collect();
    // Growl (270) at 1; Bite (208) then Dash (270) at 10; Furious Bite (208) at 14.
    assert_eq!(
        wolf,
        vec![(1, GROWL), (10, BITE), (10, DASH), (14, FURIOUS_BITE)]
    );
    assert!(spells.0[&1].iter().all(|spell| spell.kind == ACT_ENABLED));
}

#[test]
fn a_called_wolf_sends_its_owner_the_retail_pet_bar() {
    let Some(p) = hunter_with_wolf() else {
        return;
    };
    let bar = p.last_bar();
    assert_eq!(bar.pet, p.pet.to_bits());
    assert_eq!(
        (bar.command_state, bar.react_state),
        (COMMAND_FOLLOW, REACT_PASSIVE)
    );
    // Highest level first into the free spell slots; Furious Bite (14) is not learned yet.
    assert_eq!(
        bar.action_buttons,
        [
            pet_action_button(COMMAND_ATTACK, ACT_COMMAND),
            pet_action_button(COMMAND_FOLLOW, ACT_COMMAND),
            pet_action_button(COMMAND_MOVE_TO, ACT_COMMAND),
            pet_action_button(DASH, ACT_ENABLED),
            pet_action_button(BITE, ACT_ENABLED),
            pet_action_button(GROWL, ACT_ENABLED),
            0,
            pet_action_button(REACT_ASSIST, ACT_REACTION),
            pet_action_button(REACT_DEFENSIVE, ACT_REACTION),
            pet_action_button(REACT_PASSIVE, ACT_REACTION),
        ]
    );
    let world = p.h.app.world();
    assert_eq!(
        world.get::<UnitSummonedBy>(p.pet),
        Some(&UnitSummonedBy(p.h.caster.to_bits()))
    );
    let focus = world
        .get::<UnitPowers>(p.pet)
        .and_then(|powers| powers.entries.first().cloned())
        .expect("the wolf has a power");
    assert_eq!(
        (focus.power, focus.current, focus.max),
        (PowerType::Focus, 100, 100)
    );
}

#[test]
fn attack_sends_the_wolf_at_the_target_and_follow_brings_it_back() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    let kobold = p.h.npc;
    p.attack_kobold();
    // 13 yards at 7 yards per second, then swings every 2 s.
    p.advance(100);
    assert_eq!(p.pet_victim(), Some(kobold));
    assert!(p.pet_in_combat());
    assert!(
        p.gap(p.pet, kobold) <= 5.0,
        "wolf {} yd from the kobold",
        p.gap(p.pet, kobold)
    );
    let hurt = p.h.health(kobold);
    assert!(hurt < 5_000.0, "the kobold took no swing");

    p.command(COMMAND_FOLLOW);
    assert_eq!(p.last_bar().command_state, COMMAND_FOLLOW);
    p.advance(60);
    assert_eq!(p.pet_victim(), None);
    assert!(!p.pet_in_combat());
    let back = p.gap(p.pet, p.h.caster);
    assert!(back <= 5.0, "wolf {back} yd from its owner");
    assert_eq!(
        p.h.health(kobold),
        hurt,
        "the wolf kept biting after Follow"
    );
}

#[test]
fn stay_holds_the_wolf_while_its_owner_runs_off() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    p.command(COMMAND_STAY);
    assert_eq!(p.last_bar().command_state, COMMAND_STAY);
    let held = p.position(p.pet);
    p.move_owner(-20.0, 0.0);
    p.advance(60);
    assert_eq!(p.position(p.pet), held);
}

#[test]
fn move_to_sends_the_wolf_to_the_point_where_it_stays() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    p.order(
        pet_action_button(COMMAND_MOVE_TO, ACT_COMMAND),
        None,
        Some([-6.0, 0.0, 8.0]),
    );
    assert_eq!(p.last_bar().command_state, COMMAND_MOVE_TO);
    // 10 yards at 7 yards per second.
    p.advance(40);
    let at = p.position(p.pet);
    assert!((at.x + 6.0).hypot(at.z - 8.0) < 0.01, "wolf at {at:?}");
    p.move_owner(20.0, -20.0);
    p.advance(40);
    let still = p.position(p.pet);
    assert!(
        (still.x + 6.0).hypot(still.z - 8.0) < 0.01,
        "wolf left for {still:?}"
    );
}

#[test]
fn a_passive_wolf_ignores_what_attacks_its_owner() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    let owner = p.h.caster;
    p.kobold_attacks(owner);
    p.advance(20);
    assert_eq!(p.pet_victim(), None);
    assert!(!p.pet_in_combat());
}

#[test]
fn a_defensive_wolf_attacks_what_attacks_its_owner() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    let (owner, kobold) = (p.h.caster, p.h.npc);
    p.react(REACT_DEFENSIVE);
    assert_eq!(p.last_bar().react_state, REACT_DEFENSIVE);
    p.kobold_attacks(owner);
    p.advance(20);
    assert_eq!(p.pet_victim(), Some(kobold));
    assert!(p.pet_in_combat());
}

#[test]
fn a_defensive_wolf_attacks_what_attacks_it() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    let (pet, kobold) = (p.pet, p.h.npc);
    p.react(REACT_DEFENSIVE);
    p.kobold_attacks(pet);
    p.advance(20);
    assert_eq!(p.pet_victim(), Some(kobold));
}

#[test]
fn a_defensive_wolf_leaves_its_owners_own_victim_alone() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    let kobold = p.h.npc;
    p.react(REACT_DEFENSIVE);
    p.h.attack(kobold);
    p.advance(20);
    assert_eq!(p.pet_victim(), None);
}

#[test]
fn an_assisting_wolf_attacks_its_owners_victim() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    let kobold = p.h.npc;
    p.react(REACT_ASSIST);
    assert_eq!(p.last_bar().react_state, REACT_ASSIST);
    p.h.attack(kobold);
    p.advance(20);
    assert_eq!(p.pet_victim(), Some(kobold));
    assert!(p.pet_in_combat());
}

#[test]
fn passive_stops_an_ordered_attack() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    p.react(REACT_DEFENSIVE);
    p.attack_kobold();
    p.advance(5);
    assert_eq!(p.pet_victim(), Some(p.h.npc));
    p.react(REACT_PASSIVE);
    p.advance(2);
    assert_eq!(p.pet_victim(), None);
    assert_eq!(p.last_bar().react_state, REACT_PASSIVE);
}
