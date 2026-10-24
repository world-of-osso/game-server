//! Pinned Elemental fixture adapter; ordinary followers retain their native kit.
mod bindings;
mod facts;
mod source;
#[cfg(test)]
mod tests;

use super::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{Execution, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, FightStyle};
use bevy::prelude::*;
use facts::EleFacts;
use serde_json::{Value, json};
use server::simc_apl::Action;

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Rotation {
    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        if !matches!(encounter, Encounter::Fixture { .. }) {
            return Err(
                "DUNGEON_TTD_UNAVAILABLE: literal Elemental requires an explicit Fixture encounter"
                    .into(),
            );
        }
        if !world
            .get::<crate::followers::Follower>(actor)
            .is_some_and(|follower| follower.profile.spec_id == 262)
        {
            return Err("literal Elemental fixture requires its native spec262 follower".into());
        }
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let rotation = Self {
            driver: Driver::new(source::load()?),
            bindings,
            equipment,
            encounter,
        };
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(rotation)
    }

    pub fn precombat_step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<bool, String> {
        self.start_combat_at(crate::spell_cast::now(world));
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
            self.driver.snapshot.is_none(),
        )?;
        let Some((action, execution)) =
            self.select(world, actor, targets, "precombat", &prepared)?
        else {
            return Ok(false);
        };
        self.driver.execute(world, actor, action, execution)?;
        Ok(true)
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!("Elemental fixture validated at load");
        };
        self.encounter = Encounter::Fixture {
            started_at,
            duration,
            style,
        };
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        if targets.is_empty() {
            return Err("literal Elemental fixture has no native target".into());
        }
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
            false,
        )?;
        if let Some((action, execution)) =
            self.select(world, actor, targets, "default", &prepared)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }

    fn select(
        &mut self,
        world: &World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
        prepared: &Prepared,
    ) -> Result<Option<(Action, Execution)>, String> {
        let facts = EleFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
        );
        self.driver.select(list, &facts, prepared, &self.equipment)
    }

    pub fn report(&self) -> Value {
        let Encounter::Fixture { started_at, .. } = self.encounter else {
            unreachable!("Elemental fixture validated at load");
        };
        json!({"driver": "LITERAL_SIMC_ELEMENTAL_76f8118", "decisions": self.driver.decisions,
            "source": "MID2_Shaman_Elemental.simc@76f8118", "started_at": started_at,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion",
                "NATIVE_ELEMENTAL_PER_ABILITY_QUALIFICATION_OPEN", "NATIVE_ITEM_PROC_QUALIFICATION_OPEN"]})
    }
}

/// Consume the shared capture-before-startup and real cast/GCD setup lifecycle.
/// These APIs are supplied by the separately owned literal-precombat change.
pub(super) fn prepare_measurement(count: usize) -> Result<super::Run, String> {
    let (profile, traits) = super::read_reference(262)?;
    let identity = super::inputs::read_actor(&profile)?;
    if !matches!(
        &identity,
        crate::followers::simulator::ParityActor::Follower
    ) {
        return super::prepare_reference(profile, &traits, count, identity);
    }
    let mut run = super::prepare_literal_reference(profile, &traits, count, identity)?;
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: f64::from(super::TICKS) / 20.0,
        style: FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        super::preparation::finish_pending_setup(&mut run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.elemental_rotation = Some(rotation);
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL:"));
    Ok(run)
}
