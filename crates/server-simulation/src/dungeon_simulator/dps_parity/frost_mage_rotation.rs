//! Thin Frost64 binding for the unchanged exported SimC source and shared driver.
mod bindings;
use super::literal_rotation::{
    self, Driver, PrecombatPoll, equipment::Equipment, execution::Prepared,
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts, reaction};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use std::collections::BTreeSet;

pub(super) struct Rotation {
    pub driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    tracked_targets: BTreeSet<Entity>,
}

fn program(profile: &super::inputs::Profile) -> Result<Program, String> {
    // SimC76f8118 class_modules/apl/mage.cpp264–339: preserve every Frost row.
    let source = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Frost64 profile lacks literal action rows")?;
    let rows = source
        .iter()
        .map(|row| {
            let line = row.as_str().ok_or("Frost64 action row is not text")?;
            let (head, body) = line
                .split_once('=')
                .ok_or("Frost64 action lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("invalid Frost64 action list")?
            };
            Ok((list, body.strip_prefix('/').unwrap_or(body)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

fn reaction_settings() -> reaction::ReactionSettings {
    // Pinned64.reference.single.json player reaction fields; player.cpp1017–1020.
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
        encounter: Encounter,
        profile: &super::inputs::Profile,
    ) -> Result<Self, String> {
        if profile.spec_id != 64 {
            return Err("literal Frost Mage requires spec64".into());
        }
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let spells: Vec<_> = bindings::BUFFS.iter().map(|&(_, id)| id).collect();
        reaction::configure(world, actor, reaction_settings(), &spells)?;
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver: Driver::new(program(profile)?),
            bindings,
            equipment,
            encounter,
            tracked_targets: BTreeSet::new(),
        })
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = Encounter::Fixture {
            started_at,
            duration: 300.0,
            style: crate::simc_apl_native::FightStyle::Patchwerk,
        };
    }

    fn configure_targets(&mut self, world: &mut World, targets: &[Entity]) -> Result<(), String> {
        for &target in targets {
            if self.tracked_targets.insert(target) {
                // buff.cpp481–500: a .react expression enables reaction on its
                // actual debuff owner, including newly spawned add-wave entities.
                reaction::configure(world, target, reaction_settings(), &[1221389])?;
            }
        }
        Ok(())
    }

    pub fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        self.configure_targets(world, targets)?;
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::actions(),
            self.driver.snapshot.is_none(),
        )?;
        let facts = FrostFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
        };
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.configure_targets(world, targets)?;
        self.interrupt_channel(world, actor, targets)?;
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::actions(),
            false,
        )?;
        let facts = FrostFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
        };
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }

    fn interrupt_channel(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if !world
            .get::<shared::casting::CastState>(actor)
            .is_some_and(|cast| cast.cast_type == shared::casting::CastType::Channel)
        {
            return Ok(());
        }
        let prepared = Prepared::read_after_channel(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::actions(),
        )?;
        let facts = FrostFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
        };
        if let Some(decision) =
            self.driver
                .channel_decision(world, actor, &facts, &prepared, &self.equipment)?
        {
            self.driver
                .execute_channel_decision(world, actor, decision)?;
        }
        Ok(())
    }

    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"driver": "LITERAL_SIMC_FROST_MAGE_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot})
    }
}

struct FrostFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}
impl Facts for FrostFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name.starts_with("equipped.")
            || name.starts_with("set_bonus.")
        {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}
impl ActionFacts for FrostFacts<'_> {
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        // SimC76f8118 sc_mage.cpp4101/4317: the Glacial Spike aura gates
        // both its own cast and the Frostbolt filler, independently of if_expr.
        match action.name.as_str() {
            "glacial_spike" => Ok(self.native.value("buff.glacial_spike.up")? != 0.0),
            "frostbolt" => Ok(self.native.value("buff.glacial_spike.up")? == 0.0),
            _ => self.native.target_ready(action),
        }
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name.starts_with("equipped.")
            || name.starts_with("set_bonus.")
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    use super::literal_rotation::PrecombatPoll;
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    for _ in 0..super::preparation::MAX_SETUP_TICKS {
        match rotation.poll_precombat(run.app.world_mut(), run.actor, &run.targets)? {
            PrecombatPoll::Waiting => super::preparation::finish_pending_setup(run)?,
            PrecombatPoll::Selected { action, execution } => {
                super::preparation::execute_precombat_plan(
                    run,
                    &mut rotation.driver,
                    action,
                    execution,
                )?;
            }
            PrecombatPoll::Finished => {
                rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
                return Ok(Box::new(rotation));
            }
        }
    }
    Err("native Frost Mage precombat exceeded setup bound".into())
}

pub(super) const REPORT_KEY: Option<&str> = Some("frost_mage_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_FROST_MAGE_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.drive(world, actor, targets)
    }

    fn report(&self) -> serde_json::Value {
        self.report()
    }
}
