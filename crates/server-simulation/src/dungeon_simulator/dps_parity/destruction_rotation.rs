//! Literal267 adapter; native Warlock power and damage remain authoritative.
use super::inputs::Profile;
use super::literal_rotation::{
    self, Driver, default_pet::ConfiguredPet, equipment::Equipment, execution::Prepared,
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts, Facts, Program};

mod bindings;

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    pet: Option<ConfiguredPet>,
    encounter: Encounter,
}

impl Rotation {
    fn load(
        world: &mut World,
        actor: Entity,
        profile: &Profile,
        encounter: Encounter,
    ) -> Result<Self, String> {
        if profile.spec_id != 267 || !matches!(encounter, Encounter::Fixture { .. }) {
            return Err("literal Destruction requires spec267 and an explicit fixture".into());
        }
        let mut bindings = bindings::build();
        let pet = ConfiguredPet::read(world, actor, profile)?;
        if let Some(pet) = &pet {
            pet.bind(&mut bindings)?;
        }
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let rotation = Self {
            driver: Driver::new(read_program(profile)?),
            bindings,
            equipment,
            pet,
            encounter,
        };
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(rotation)
    }

    fn read_prepared(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<Prepared, String> {
        let pet_action = self
            .pet
            .as_ref()
            .map(|pet| pet.read_action(world, actor))
            .transpose()?
            .flatten();
        Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions().chain(pet_action),
            self.driver.snapshot.is_none(),
        )
    }

    fn step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        precombat: bool,
    ) -> Result<bool, String> {
        let prepared = self.read_prepared(world, actor, targets)?;
        let facts = DestructionFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            primary: targets.first().map(|target| target.to_bits()),
        };
        let list = if precombat { "precombat" } else { "default" };
        let Some((action, execution)) =
            self.driver
                .select(list, &facts, &prepared, &self.equipment)?
        else {
            return Ok(false);
        };
        self.driver.execute(world, actor, action, execution)?;
        Ok(true)
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, &run.profile, encounter)?;
    if let Some(pet) = ConfiguredPet::read(run.app.world(), run.actor, &run.profile)? {
        super::preparation::wait_for_configured_pet_action(run, &pet)?;
    }
    while rotation.step(run.app.world_mut(), run.actor, &run.targets, true)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL:"));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("destruction_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_DESTRUCTION_76f8118"
    }
    fn report(&self) -> Value {
        json!({"driver": self.label(), "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.step(world, actor, targets, false)?;
        Ok(())
    }
}

// SimC76f8118 apl/warlock.cpp Destruction: execute every exported row unchanged.
fn read_program(profile: &Profile) -> Result<Program, String> {
    let actions = profile
        .options
        .get("actions")
        .and_then(Value::as_array)
        .ok_or("267 profile lacks literal actions")?;
    let rows = actions
        .iter()
        .map(parse_row)
        .collect::<Result<Vec<_>, _>>()?;
    Program::parse(&rows)
}

fn parse_row(row: &Value) -> Result<(&str, &str), String> {
    let row = row.as_str().ok_or("267 action row must be text")?;
    let (head, literal) = row
        .split_once('=')
        .ok_or("267 action row lacks assignment")?;
    let head = head.trim_end_matches('+');
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("267 row is not an action list")?
    };
    Ok((list, literal.trim_start_matches('/')))
}

struct DestructionFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    primary: Option<u64>,
}

impl Facts for DestructionFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name == "demonic_art" {
            // SimC76f8118 sc_warlock.cpp876–881: any actual Demonic Art buff.
            for aura in [
                "demonic_art_overlord",
                "demonic_art_mother_of_chaos",
                "demonic_art_pit_lord",
            ] {
                if self.native.value(&format!("buff.{aura}.up"))? != 0.0 {
                    return Ok(1.0);
                }
            }
            return Ok(0.0);
        }
        if let Some(alias) = name.strip_prefix("dot_refreshable_count.") {
            // SimC76f8118 sc_warlock.cpp1002–1053: count refreshable actual recipients.
            let mut action = Action {
                name: alias.into(),
                source_row: None,
                target: None,
                options: Default::default(),
            };
            let mut count = 0.0;
            for target in self.native.target_ids(&action)? {
                action.target = Some(target);
                count += self
                    .native
                    .value_for(&action, &format!("dot.{alias}.refreshable"))?;
            }
            return Ok(count);
        }
        if name.starts_with("trinket.") {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}

impl ActionFacts for DestructionFacts<'_> {
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        match action.name.as_str() {
            "infernal_bolt" => Ok(self.native.value("buff.infernal_bolt.up")? != 0.0),
            "ruination" => Ok(self.native.value("buff.ruination.up")? != 0.0),
            "immolate" => Ok(self.native.value("talent.wither")? == 0.0),
            _ => Ok(true),
        }
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name == "refreshable" {
            return self
                .native
                .value_for(action, &format!("dot.{}.refreshable", action.name));
        }
        if name == "self.target" || name == "target" {
            // SimC76f8118 player.cpp self.target/target compare target identities.
            let target = if name == "self.target" {
                self.primary
            } else {
                action.target.or(self.primary)
            };
            return target
                .map(|target| target as f64)
                .ok_or("267 action has no target identity".into());
        }
        if let Some(alias) = name
            .strip_prefix("action.")
            .and_then(|value| value.strip_suffix(".usable"))
        {
            return self
                .native
                .value_for(action, &format!("action.{alias}.ready"));
        }
        if name == "demonic_art"
            || name.starts_with("dot_refreshable_count.")
            || name.starts_with("trinket.")
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}
