//! Summon Imp through real DB2 spell data, pet messages, AI and reopened redb.
use std::path::Path;

use lightyear::prelude::{MessageReceiver, MessageSender};
use rusqlite::{Connection, OpenFlags};
use shared::components::{Level, Mana, ModelDisplay, Npc, UnitSummonedBy};
use shared::protocol::{CombatChannel, CombatLogKind};

use super::*;
use crate::hunter_pet::HunterPetPlugin;
use crate::persistence::{self, GameDatabase};
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};
use crate::world_data::load_world_data;

const SUMMON_IMP: u32 = 688;
const FIREBOLT: u32 = 3110;
const DISMISS_PET: u32 = 2641;
const WARLOCK: u8 = 9;
const IMP: u32 = 416;

struct ImpHarness {
    h: Harness,
    bars: Vec<PetSpells>,
    clears: usize,
}

impl ImpHarness {
    fn login(path: &Path) -> Self {
        let conn = Connection::open_with_flags(
            crate::test_data::world_db_path(),
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let spells = load_pet_levelup_spells(&conn).unwrap();
        let mut data = load_world_data(&crate::test_data::world_db_path());
        data.templates.retain(|entry, _| *entry == IMP);
        let db = redb::Database::create(path).unwrap();
        persistence::create_tables(&db);
        let mut h = harness_with_plugins(
            Vec::new(),
            &[SUMMON_IMP, FIREBOLT, DISMISS_PET],
            WARLOCK,
            false,
            |app| {
                app.insert_resource(data);
                app.insert_resource(spells);
                app.insert_resource(GameDatabase { db });
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
            },
        )
        .expect("real spell data");
        h.app.world_mut().entity_mut(h.caster).insert(Level(1));
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
        let mut result = Self {
            h,
            bars: Vec::new(),
            clears: 0,
        };
        result.advance(2);
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
        self.h.cast(DISMISS_PET, self.pet());
        self.advance(70);
        assert!(self.h.inbox.failed.is_empty(), "{:?}", self.h.inbox.failed);
    }
}

#[test]
fn warlock_summon_imp_spawns_owned_pet_and_firebolt_bar() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = ImpHarness::login(&dir.path().join("pets.redb"));
    let pet = h.summon();
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
    assert!(world.get::<Mana>(pet).unwrap().current >= 40.0);
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
    let pet = h.summon();
    h.order(COMMAND_ATTACK, Some(h.h.npc));
    h.advance(100);
    assert!(
        h.h.inbox.log.iter().any(|event| {
            event.source == Some(pet.to_bits())
                && event.target == Some(h.h.npc.to_bits())
                && event.spell_id == Some(FIREBOLT)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "imp dealt no Firebolt damage: {:?}",
        h.h.inbox.log
    );
    h.h.app
        .world_mut()
        .get_mut::<Position>(h.h.caster)
        .unwrap()
        .x = -30.0;
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
    assert_eq!(h.h.app.world().get::<Npc>(pet).unwrap().template_id, IMP);
    assert!(
        h.bars
            .last()
            .unwrap()
            .action_buttons
            .contains(&pet_action_button(FIREBOLT, ACT_ENABLED))
    );
}

#[test]
fn warlock_imp_dismiss_removes_pet_and_does_not_restore_on_relog() {
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
