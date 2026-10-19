//! Real-DB tests (data/world.db with `creature_family`): a level 10
//! Human hunter calls its Diseased Young Wolf 299 with Call Pet 1, gets the pet bar, and
//! orders the wolf around the harness's hostile Kobold Vermin.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lightyear::prelude::{MessageReceiver, MessageSender};
use rusqlite::{Connection, OpenFlags};
use shared::components::{PowerType, UnitPowers, UnitSummonedBy};
use shared::protocol::{CombatChannel, CombatLogKind};

use super::*;
use crate::hunter_pet::{HunterPetPlugin, PetStable};
use crate::persistence::pet_store::{StoredPet, StoredPetStable, save_pet_stable};
use crate::persistence::{self, GameDatabase};
use crate::spell_cast::test_harness::{
    Harness, SUNWELL_RADIANCE, apply_native_enemy_dodge_reduction_to, harness_with_plugins,
};
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
const BM_KILL_COMMAND: u32 = 34_026;
const BM_KILL_COMMAND_DAMAGE: u32 = 83_381;

fn world_db() -> Option<Connection> {
    let path = crate::test_data::world_db_path();
    Some(Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap())
}

fn wolf_template(conn: &Connection) -> HashMap<u32, CreatureTemplate> {
    let mut templates = load_templates(conn, &load_expected_stats(conn));
    // Keep native defense metadata for the harness's live Kobold target too.
    templates.retain(|entry, _| [6, DISEASED_YOUNG_WOLF].contains(entry));
    templates
}

fn db_path() -> PathBuf {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("pets.redb");
    std::mem::forget(dir);
    path
}

/// A stable holding a Diseased Young Wolf (level 10, Passive as tamed) in slot 1, not out.
fn seed_stable(path: &Path) {
    let db = open_db(path);
    let mut stored = StoredPetStable::default();
    stored.active[0] = Some(StoredPet {
        creature_entry: DISEASED_YOUNG_WOLF,
        display_id: WOLF_DISPLAY,
        name: "Diseased Young Wolf".into(),
        level: 10,
        family: 1,
        react_state: REACT_PASSIVE,
        health: None,
        specialization: None,
    });
    save_pet_stable(&db.db, CHARACTER_ID, &stored);
}

fn open_db(path: &Path) -> GameDatabase {
    let db = redb::Database::create(path).expect("redb");
    persistence::create_tables(&db);
    GameDatabase::new(db)
}

struct PetHarness {
    h: Harness,
    pet: Entity,
    bars: Vec<PetSpells>,
    clears: usize,
}

/// The level 10 hunter at the origin with its wolf called out and the Kobold Vermin
/// (hostile, 5000 health) moved 15 yards away; `PetSpells` received are collected.
fn hunter_with_wolf() -> Option<PetHarness> {
    let path = db_path();
    seed_stable(&path);
    hunter_logged_in(&path)
}

/// The hunter entering the world with the stable stored at `path`, then casting Call Pet 1
/// (which brings a pet already out back to it).
fn hunter_logged_in(path: &Path) -> Option<PetHarness> {
    let conn = world_db()?;
    let templates = wolf_template(&conn);
    let spells = load_pet_levelup_spells(&conn).expect("creature_family imported");
    let db = open_db(path);
    let ids = [
        CALL_PET_1,
        BITE,
        GROWL,
        DASH,
        BM_KILL_COMMAND,
        BM_KILL_COMMAND_DAMAGE,
        SUNWELL_RADIANCE,
        6590, // Native Increase Melee Hit Chance effect0: aura54 +100.
    ];
    let mut h = harness_with_plugins(Vec::new(), &ids, CLASS_HUNTER, false, |app| {
        app.insert_resource(WorldData {
            templates,
            ..Default::default()
        });
        app.insert_resource(db);
        app.insert_resource(spells);
        app.add_plugins((HunterPetPlugin, crate::creature_ai::CreatureAiPlugin));
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
    })?;
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<PetAction>::default(),
        MessageReceiver::<PetAction>::default(),
        MessageSender::<PetSpells>::default(),
        MessageReceiver::<PetSpells>::default(),
        MessageSender::<PetClearSpells>::default(),
        MessageReceiver::<PetClearSpells>::default(),
        MessageSender::<PetSpellAutocast>::default(),
        MessageReceiver::<PetSpellAutocast>::default(),
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
        clears: 0,
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

#[test]
fn pet_control_bm_kill_command_native_called_wolf_deals_named_damage_to_owner_target() {
    let mut p = hunter_with_wolf().expect("native hunter/wolf catalog");
    p.advance(35);
    let owner = p.h.caster;
    let target = p.h.npc;
    assert_eq!(
        p.h.app
            .world()
            .get::<crate::hunter_pet::HunterPet>(p.pet)
            .unwrap()
            .owner,
        owner
    );
    // Named pet-child damage must hit; creature miss/dodge have separate tests.
    apply_native_enemy_dodge_reduction_to(&mut p.h, p.pet);
    crate::auras::apply_aura(
        p.h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: p.pet,
            target: p.pet,
            spell_id: 6590,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert_eq!(
        p.h.app
            .world()
            .get::<crate::auras::ActiveAuras>(p.pet)
            .unwrap()
            .modifier(54),
        100.0
    );
    let health_before = p.h.health(target);
    p.h.set_power(PowerType::Focus, 100.0);
    p.h.inbox.log.clear();
    p.h.cast(BM_KILL_COMMAND, Some(target));
    p.advance(10);
    assert!(p.h.inbox.failed.is_empty(), "{:?}", p.h.inbox.failed);
    assert!(
        p.h.inbox.log.iter().any(|event| {
            event.spell_id == Some(BM_KILL_COMMAND)
                && event.source == Some(owner.to_bits())
                && event.kind == CombatLogKind::CastSuccess
        }),
        "ordinary owner Kill Command succeeds"
    );
    let damage = p.spell_damage(BM_KILL_COMMAND_DAMAGE, target);
    assert_eq!(
        damage.len(),
        1,
        "one83381 hit from the actual called wolf; owner log {:?}",
        p.h.inbox.log
    );
    assert!(damage[0] > 0);
    assert_eq!((health_before - p.h.health(target)) as i32, damage[0]);
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
            self.clears += self
                .h
                .app
                .world_mut()
                .get_mut::<MessageReceiver<PetClearSpells>>(connection)
                .unwrap()
                .receive()
                .count();
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

    /// `CMSG_PET_SPELL_AUTOCAST`; delivered 3 frames later.
    fn autocast(&mut self, spell: u32, enabled: bool) {
        let pet = self.pet.to_bits();
        self.h
            .app
            .world_mut()
            .get_mut::<MessageSender<PetSpellAutocast>>(self.h.connection)
            .unwrap()
            .send::<CombatChannel>(PetSpellAutocast {
                pet,
                spell,
                enabled,
            });
        self.advance(3);
    }

    /// The pet's melee swings at `target` in the owner's combat log, hits and misses alike
    /// (melee lines carry no spell id).
    fn melee_swings(&self, target: Entity) -> usize {
        self.h
            .inbox
            .log
            .iter()
            .filter(|event| {
                matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
                    && event.spell_id.is_none()
                    && event.source == Some(self.pet.to_bits())
                    && event.target == Some(target.to_bits())
            })
            .count()
    }

    fn assert_melee_attempts(&self, target: Entity) {
        assert!(self.melee_swings(target) > 0, "the wolf never swung");
        let damage: i32 = self
            .h
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.source == Some(self.pet.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.kind == CombatLogKind::Damage
            })
            .map(|event| {
                assert!(event.amount > 0, "{event:?}");
                event.amount
            })
            .sum();
        assert_eq!(5_000.0 - self.h.health(target), damage as f32);
    }

    /// The pet's completed casts of `spell` at `target`, landed or missed.
    fn spell_casts(&self, spell: u32, target: Entity) -> usize {
        self.h
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastSuccess
                    && event.spell_id == Some(spell)
                    && event.source == Some(self.pet.to_bits())
                    && event.target == Some(target.to_bits())
            })
            .count()
    }

    /// Damage the pet's `spell` did to `target`, as logged.
    fn spell_damage(&self, spell: u32, target: Entity) -> Vec<i32> {
        self.h
            .inbox
            .log
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(spell)
                    && event.source == Some(self.pet.to_bits())
                    && event.target == Some(target.to_bits())
            })
            .map(|event| event.amount)
            .collect()
    }

    fn focus(&self) -> i32 {
        self.h
            .app
            .world()
            .get::<UnitPowers>(self.pet)
            .and_then(|powers| powers.entries.first().map(|entry| entry.current))
            .expect("the pet has Focus")
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

/// TC a352b1fa PetHandler.cpp:217-238 deletes hunter pets for command 3.
#[test]
fn hunter_pet_dismiss_command_abandons_the_saved_wolf() {
    let path = db_path();
    seed_stable(&path);
    let mut p = hunter_logged_in(&path).expect("real hunter data");
    p.command(shared::protocol::COMMAND_ABANDON);
    assert!(p.h.app.world().get_entity(p.pet).is_err());
    let stable = p.h.app.world().get::<PetStable>(p.h.caster).unwrap();
    assert_eq!(stable.summoned, None);
    assert_eq!(stable.stored.current, None);
    assert!(stable.stored.active[0].is_none());
    assert_eq!(p.clears, 1);
    drop(p);
    let db = open_db(&path);
    let saved = crate::persistence::pet_store::load_pet_stable(&db.db, CHARACTER_ID).unwrap();
    assert!(saved.active[0].is_none());
    assert_eq!(saved.current, None);
}

#[test]
fn native_called_hunter_pet_carries_source_creature_flags_extra() {
    let p = hunter_with_wolf().expect("native called Hunter pet");
    let world = p.h.app.world();
    let entry = world
        .get::<shared::components::Npc>(p.pet)
        .unwrap()
        .template_id;
    let expected = world.resource::<WorldData>().templates[&entry].flags_extra;
    assert_eq!(
        world
            .get::<crate::zones::CreatureFlagsExtra>(p.pet)
            .map(|flags| flags.0),
        Some(expected),
        "ordinary Call Pet caches its authored template flags"
    );
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
        world.get::<shared::components::UnitIsPet>(p.pet),
        Some(&shared::components::UnitIsPet(true))
    );
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
    p.assert_melee_attempts(kobold);

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

/// Diseased Young Wolf's FactionTemplate 32: neutral to the Human hunter.
const NEUTRAL_BEAST: u32 = 32;

#[test]
fn attack_sends_the_wolf_at_a_neutral_beast() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    let kobold = p.h.npc;
    p.h.app
        .world_mut()
        .entity_mut(kobold)
        .insert(UnitFactionTemplate(NEUTRAL_BEAST));
    p.attack_kobold();
    p.advance(100);
    assert_eq!(p.pet_victim(), Some(kobold));
    assert!(p.pet_in_combat());
    p.assert_melee_attempts(kobold);
}

#[test]
fn the_stance_is_kept_in_the_stable_across_a_relog() {
    let path = db_path();
    seed_stable(&path);
    {
        let Some(mut p) = hunter_logged_in(&path) else {
            return;
        };
        p.react(REACT_DEFENSIVE);
        assert_eq!(p.last_bar().react_state, REACT_DEFENSIVE);
    }
    // Logging back in brings the wolf that was out back (`Player::LoadPet`), Defensive.
    let mut p = hunter_logged_in(&path).expect("world.db present");
    p.command(COMMAND_FOLLOW);
    assert_eq!(p.last_bar().react_state, REACT_DEFENSIVE);
    assert_eq!(
        p.h.app.world().get::<PetCharm>(p.pet).unwrap().react,
        REACT_DEFENSIVE
    );
}

#[test]
fn bite_ordered_from_the_bar_is_cast_by_the_wolf_at_the_target() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    let kobold = p.h.npc;
    let beside = p.position(p.pet);
    p.h.app.world_mut().entity_mut(kobold).insert(Position {
        x: beside.x + 2.0,
        ..beside
    });
    p.order(pet_action_button(BITE, ACT_ENABLED), Some(kobold), None);
    p.advance(2);
    // Bite rolls to hit like melee: the order is proven by the cast; damage only when it lands.
    assert_eq!(p.spell_casts(BITE, kobold), 1, "{:?}", p.h.inbox.log);
    if let [bite] = p.spell_damage(BITE, kobold)[..] {
        // 0.75 × the wolf's attack power (22% of its owner's 100), less the kobold's armor.
        assert!(bite > 0, "Bite did {bite} damage");
        assert!(p.h.health(kobold) <= 5_000.0 - bite as f32);
    }
    // Bite costs 25 Focus.
    assert_eq!(p.focus(), 75);
    // The wolf goes after the unit it bit (`_AttackStart`).
    assert_eq!(p.pet_victim(), Some(kobold));
}

#[test]
fn called_wolf_focus_regenerates_in_four_second_steps_without_resetting_at_full() {
    let mut p = hunter_with_wolf().expect("native hunter/wolf catalog");
    let kobold = p.h.npc;
    let beside = p.position(p.pet);
    p.h.app.world_mut().entity_mut(kobold).insert(Position {
        x: beside.x + 2.0,
        ..beside
    });
    p.order(pet_action_button(BITE, ACT_DISABLED), Some(kobold), None);
    p.command(COMMAND_FOLLOW);
    assert_eq!(p.spell_casts(BITE, kobold), 1);
    let focus = |p: &PetHarness| {
        crate::spell_cast::powers::power_value(p.h.app.world(), p.pet, PowerType::Focus)
            .unwrap()
            .0
    };
    // TC a352b1fa Pet.cpp64,671-687 / CreatureData.h423 / Creature.cpp964-998:
    // spawn-anchored 4-second timer, +24 whole Focus, even in combat.
    for _ in 0..60 {
        assert_eq!(focus(&p), 75.0, "no fractional player-style regeneration");
        p.advance(1);
    }
    let mut first_tick = false;
    for _ in 0..20 {
        p.advance(1);
        if focus(&p) != 75.0 {
            assert_eq!(focus(&p), 99.0);
            first_tick = true;
            break;
        }
    }
    assert!(first_tick, "first +24 tick within four seconds of spawn");
    for _ in 0..79 {
        p.advance(1);
        assert_eq!(focus(&p), 99.0);
    }
    p.advance(1);
    assert_eq!(focus(&p), 100.0, "next tick clamps to maximum");

    // A full pool does not pause/reset the timer. Spend three seconds into its
    // next interval: the next +24 is one second later, not four after spending.
    p.advance(60);
    let beside = p.position(p.pet);
    p.h.app.world_mut().entity_mut(kobold).insert(Position {
        x: beside.x + 2.0,
        ..beside
    });
    p.order(pet_action_button(BITE, ACT_DISABLED), Some(kobold), None);
    p.command(COMMAND_FOLLOW);
    assert_eq!(p.spell_casts(BITE, kobold), 2);
    assert_eq!(focus(&p), 75.0);
    p.advance(13);
    assert_eq!(focus(&p), 75.0);
    p.advance(1);
    assert_eq!(focus(&p), 99.0, "full-pool timer phase survives spending");
}

#[test]
fn an_attacking_wolf_autocasts_bite() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    let kobold = p.h.npc;
    p.attack_kobold();
    // 13 yards at 7 yards per second, then Bite every 3 s.
    p.advance(100);
    // Bite rolls to hit like melee, so count casts, not damage.
    assert!(
        p.spell_casts(BITE, kobold) > 0,
        "no Bite in {:?}",
        p.h.inbox.log
    );
}

#[test]
fn bite_with_autocast_turned_off_is_not_autocast() {
    let Some(mut p) = hunter_with_wolf() else {
        return;
    };
    let kobold = p.h.npc;
    p.autocast(BITE, false);
    assert_eq!(
        p.last_bar().action_buttons[4],
        pet_action_button(BITE, ACT_DISABLED)
    );
    p.attack_kobold();
    p.advance(100);
    // Swings, not damage: a run of misses, dodges or parries is a legitimate outcome.
    assert!(p.melee_swings(kobold) > 0, "the wolf never swung");
    assert_eq!(p.spell_damage(BITE, kobold), Vec::<i32>::new());

    p.autocast(BITE, true);
    assert_eq!(
        p.last_bar().action_buttons[4],
        pet_action_button(BITE, ACT_ENABLED)
    );
    p.advance(80);
    assert!(!p.spell_damage(BITE, kobold).is_empty());
}
