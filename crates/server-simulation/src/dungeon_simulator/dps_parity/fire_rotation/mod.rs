//! Fire fixture adapter; shared machinery owns scheduling and native execution.
mod bindings;
mod facts;
mod source;
use super::literal_rotation::{
    self, Driver, PrecombatPoll, equipment::Equipment, execution::Prepared,
};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use facts::FireFacts;
use serde_json::{Value, json};

fn configure_reactions(
    world: &mut World,
    actor: Entity,
    bindings: &Bindings,
) -> Result<(), String> {
    use crate::simc_apl_native::reaction::{ReactionSettings, configure};
    // SimC76f8118 player.cpp1017–1020/1225–1226: unchanged reference defaults.
    let settings = ReactionSettings {
        mean: 0.25,
        stddev: 0.05,
        offset: 0.1,
        nu: 0.15,
        maximum: 1.4,
    };
    let tracked = [
        "hot_streak",
        "heating_up",
        "pyroclasm",
        "heat_shimmer",
        "frostfire_empowerment",
    ]
    .map(|alias| bindings.buffs[alias]);
    configure(world, actor, settings, &tracked)
}

pub(super) struct Rotation {
    pub driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        profile: &super::inputs::Profile,
    ) -> Result<Self, String> {
        if !matches!(encounter, Encounter::Fixture { .. }) {
            return Err("literal Fire requires an explicit Fixture encounter".into());
        }
        let mut bindings = bindings::build(world, actor)?;
        configure_reactions(world, actor, &bindings)?;
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let rotation = Self {
            driver: Driver::new(source::load(profile)?),
            bindings,
            equipment,
            encounter,
        };
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(rotation)
    }

    pub fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        let prepared = self.prepare(world, actor, targets, self.driver.snapshot.is_none())?;
        let facts = FireFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
        );
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!("Fixture validated at load");
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
            return Err("literal Fire has no native target".into());
        }
        let prepared = self.prepare(world, actor, targets, false)?;
        let facts = FireFacts::new(
            world,
            actor,
            &self.bindings,
            &self.equipment,
            self.encounter,
            targets,
        );
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }

    fn prepare(
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
            bindings::execution_actions(&self.bindings),
            snapshot,
        )
    }

    pub fn report(&self) -> Value {
        json!({"driver": "LITERAL_SIMC_FIRE_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}
