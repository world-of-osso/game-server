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

fn apply_initial_spheres(run: &mut super::Run) {
    let selected = run.app.world().get::<crate::class_progression::KnownSpells>(run.actor)
        .is_some_and(|known| known.contains(448601));
    if selected {
        // SimC76f8118 sc_mage.cpp366/6893–6894: the shipped profile keeps
        // default initial5 applications; the native aura enforces its own stack cap.
        for _ in 0..5 {
            crate::spell_cast::cast_triggered(run.app.world_mut(), run.actor, 448604, Some(run.actor), 1);
        }
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    use super::literal_rotation::PrecombatPoll;
    apply_initial_spheres(run);
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    loop {
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
            PrecombatPoll::Finished => break,
        }
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.gaps.retain(|gap| {
        !gap.starts_with("NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL")
            && !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL")
    });
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("fire_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_FIRE_76f8118"
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
