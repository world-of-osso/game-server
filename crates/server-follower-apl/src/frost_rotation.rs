//! Thin Frost player adapter over the shared literal SimC evaluator and dispatch.
pub mod bindings;
use crate::literal_rotation::{Driver, PrecombatPoll, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts, reaction};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::followers::parity_profile::Profile;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use std::collections::BTreeSet;

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
    pub first_ams_cast: f64,
    pub tracked_targets: BTreeSet<Entity>,
}

pub fn reaction_settings() -> reaction::ReactionSettings {
    // Pinned251.reference.single.json player fields; player.cpp1017–1020.
    reaction::ReactionSettings {
        mean: 0.25,
        stddev: 0.05,
        offset: 0.1,
        nu: 0.15,
        maximum: 1.4,
    }
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        profile: &Profile,
        encounter: Encounter,
    ) -> Result<Self, String> {
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        // SimC76f8118 sc_death_knight.cpp1889/14580–14581: configured AMS delay,
        // default20s; no incoming damage or Runic Power grant is invented here.
        let first_ams_cast = profile
            .options
            .get("deathknight.first_ams_cast")
            .map(|value| {
                value
                    .as_str()
                    .ok_or("invalid deathknight.first_ams_cast")?
                    .parse::<f64>()
                    .map_err(|error| error.to_string())
            })
            .transpose()?
            .unwrap_or(20.0);
        // Killing Machine, Rime and Frostbane react to actual acquisition.
        reaction::configure(world, actor, reaction_settings(), &[51124, 59052, 1229310])?;
        Ok(Self {
            driver: Driver::new(read_program(profile)?),
            bindings,
            equipment,
            encounter,
            first_ams_cast,
            tracked_targets: BTreeSet::new(),
        })
    }

    pub fn configure_targets(
        &mut self,
        world: &mut World,
        targets: &[Entity],
    ) -> Result<(), String> {
        for &target in targets {
            if self.tracked_targets.insert(target) {
                // Razorice and Trollbane retain real acquisition on every add wave.
                reaction::configure(world, target, reaction_settings(), &[51714, 444826])?;
            }
        }
        Ok(())
    }

    pub fn read_prepared(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        snapshot: bool,
    ) -> Result<Prepared, String> {
        Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
            snapshot,
        )
    }

    pub fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        self.configure_targets(world, targets)?;
        let prepared = self.read_prepared(world, actor, targets, self.driver.snapshot.is_none())?;
        let facts = FrostFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            first_ams_cast: self.first_ams_cast,
        };
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = crate::literal_rotation::encounter_starting_at(self.encounter, started_at);
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.configure_targets(world, targets)?;
        let prepared = self.read_prepared(world, actor, targets, false)?;
        let facts = FrostFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            first_ams_cast: self.first_ams_cast,
        };
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }

    pub fn report(&self) -> Value {
        json!({"driver":"LITERAL_SIMC_FROST_76f8118", "decisions":self.driver.decisions,
            "native_precombat_snapshot":self.driver.snapshot,
            "gaps":["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}

pub fn read_program(profile: &Profile) -> Result<Program, String> {
    let actions = profile
        .options
        .get("actions")
        .and_then(Value::as_array)
        .ok_or("Frost profile lacks literal action rows")?;
    let rows = actions
        .iter()
        .map(|row| {
            let row = row.as_str().ok_or("non-string Frost action row")?;
            let (head, action) = row
                .split_once('=')
                .ok_or("Frost action row lacks assignment")?;
            let head = head.trim_end_matches('+');
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("invalid Frost action list")?
            };
            Ok((list, action.trim_start_matches('/')))
        })
        .collect::<Result<Vec<_>, String>>()?;
    // Pinned MID2_Death_Knight_Frost.simc26–133, unmodified source-order APL.
    Program::parse(&rows)
}

pub struct FrostFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    first_ams_cast: f64,
}

impl Facts for FrostFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "death_knight.first_ams_cast" => Ok(self.first_ams_cast),
            _ if name.starts_with("trinket.") || name.starts_with("equipped.") => {
                self.equipment.fact(name, &self.native)
            }
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for FrostFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }

    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name.starts_with("equipped.")
            || name == "death_knight.first_ams_cast"
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        self.poll_precombat(world, actor, targets)
    }

    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: server::simc_apl::Action,
        execution: crate::literal_rotation::execution::Execution,
    ) -> Result<(), String> {
        self.driver.execute(world, actor, action, execution)
    }

    fn start_combat_at(&mut self, started_at: f64) {
        self.start_combat_at(started_at);
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.drive(world, actor, targets)
    }
}
