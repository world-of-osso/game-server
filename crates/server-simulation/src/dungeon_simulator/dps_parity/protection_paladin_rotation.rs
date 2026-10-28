//! Literal MID2 Protection rows driven by the shared parity APL engine.
use super::super::tank_replay::{
    protection_program,
    protection_rotation::{self, Context},
};
use super::{
    Run, SpecDriver,
    literal_rotation::{
        self, Driver, PrecombatPoll,
        equipment::Equipment,
        execution::{NativeAction, Prepared},
    },
};
use crate::simc_apl_native::{Bindings, Encounter, FightStyle};
use bevy::prelude::*;
use serde_json::{Value, json};

pub(super) const REPORT_KEY: Option<&str> = Some("protection_rotation");

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    potion: String,
}

impl Rotation {
    fn load(run: &mut Run) -> Result<Self, String> {
        let program = protection_program(&run.profile.options["actions"])?;
        let potion = run
            .profile
            .consumables
            .get("potion")
            .cloned()
            .ok_or("MID2_POTION_ABSENT")?;
        let actor = run.actor;
        let world = run.app.world_mut();
        let encounter = Encounter::Fixture {
            started_at: crate::spell_cast::now(world),
            duration: 300.0,
            style: FightStyle::Patchwerk,
        };
        let mut bindings = protection_rotation::bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        // SimC76f8118 sc_paladin.hpp781 and cpp3497–3504 default to Holy Bulwark;
        // only an unrecognized explicit starting_armament option uses a random choice.
        Ok(Self {
            driver: Driver::new(program),
            bindings,
            equipment,
            encounter,
            potion,
        })
    }

    fn refresh_armament(&mut self, world: &World, actor: Entity) -> Result<u8, String> {
        let resolved = literal_rotation::execution::resolve_action(world, actor, 432459)?
            .ok_or("NATIVE_HOLY_ARMAMENT_ACTION_UNAVAILABLE")?;
        self.bindings
            .actions
            .insert("holy_armaments".into(), resolved);
        Ok(u8::from(crate::auras::has_aura(world, actor, 432478)))
    }

    fn actions() -> impl Iterator<Item = NativeAction<'static>> {
        protection_rotation::ACTIONS.iter().map(|&(name, spell)| {
            let self_target = matches!(
                name,
                "rite_of_sanctification"
                    | "rite_of_adjuration"
                    | "devotion_aura"
                    | "consecration"
                    | "holy_armaments"
                    | "ardent_defender"
                    | "fireblood"
                    | "avenging_wrath"
                    | "arcane_torrent"
                    | "word_of_glory"
            );
            NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            }
        })
    }

    fn prepared(
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
            Self::actions(),
            snapshot,
        )
    }

    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        let armament = self.refresh_armament(world, actor)?;
        let active = crate::spell_cast::scripts::has_active_consecration(world, actor);
        let prepared = self.prepared(world, actor, targets, self.driver.snapshot.is_none())?;
        let facts = Context::new(
            world,
            actor,
            targets,
            &self.bindings,
            &self.equipment,
            self.encounter,
            active,
            Some(armament),
            &self.potion,
        );
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    fn start_combat(&mut self, at: f64) {
        if let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        {
            self.encounter = Encounter::Fixture {
                started_at: at,
                duration,
                style,
            };
        }
    }
}

pub(super) fn prepare_driver(run: &mut Run) -> Result<Box<dyn SpecDriver>, String> {
    let mut rotation = Rotation::load(run)?;
    for _ in 0..super::preparation::MAX_SETUP_TICKS {
        match rotation.poll_precombat(run.app.world_mut(), run.actor, &run.targets)? {
            PrecombatPoll::Waiting => super::preparation::finish_pending_setup(run)?,
            PrecombatPoll::Selected { action, execution } => {
                super::preparation::execute_precombat_plan(
                    run,
                    &mut rotation.driver,
                    action,
                    execution,
                )?
            }
            PrecombatPoll::Finished => {
                rotation.start_combat(crate::spell_cast::now(run.app.world()));
                return Ok(Box::new(rotation));
            }
        }
    }
    Err("PROTECTION_PRECOMBAT_EXCEEDED_30_SECONDS".into())
}

impl SpecDriver for Rotation {
    fn supports_actor(identity: &crate::followers::simulator::ParityActor) -> bool {
        matches!(
            identity,
            crate::followers::simulator::ParityActor::Player { .. }
        )
    }
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_PROTECTION_76f8118"
    }
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        Some(&[132403])
    }
    fn report(&self) -> Value {
        json!({"driver":self.label(),"decisions":self.driver.decisions})
    }
    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let armament = self.refresh_armament(world, actor)?;
        let active = crate::spell_cast::scripts::has_active_consecration(world, actor);
        let prepared = self.prepared(world, actor, targets, false)?;
        let facts = Context::new(
            world,
            actor,
            targets,
            &self.bindings,
            &self.equipment,
            self.encounter,
            active,
            Some(armament),
            &self.potion,
        );
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }
}
