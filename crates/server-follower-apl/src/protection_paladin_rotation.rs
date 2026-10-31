//! Literal MID2 Protection rows driven by the shared parity APL engine.
pub mod context;

use crate::literal_rotation::{
    self, Driver, PrecombatPoll,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter};
use bevy::prelude::*;
use context::Context;

pub fn program(actions: &serde_json::Value) -> Result<server::simc_apl::Program, String> {
    let lines = actions.as_array().ok_or("MID2_PROTECTION_ACTIONS_ABSENT")?;
    let mut rows = Vec::with_capacity(lines.len());
    for line in lines {
        let line = line.as_str().ok_or("MID2_PROTECTION_ACTION_NOT_TEXT")?;
        let (head, action) = line
            .split_once("+=")
            .or_else(|| line.split_once('='))
            .ok_or_else(|| format!("invalid MID2 Protection action: {line}"))?;
        let list = if head == "actions" {
            "default"
        } else {
            head.strip_prefix("actions.")
                .ok_or_else(|| format!("invalid MID2 Protection list: {head}"))?
        };
        rows.push((list, action.strip_prefix('/').unwrap_or(action)));
    }
    server::simc_apl::Program::parse(&rows)
}

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
    pub potion: String,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        profile: &server::followers::parity_profile::Profile,
    ) -> Result<Self, String> {
        let program = program(&profile.options["actions"])?;
        let potion = profile
            .consumables
            .get("potion")
            .cloned()
            .ok_or("MID2_POTION_ABSENT")?;
        let mut bindings = context::bindings();
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

    pub fn refresh_armament(&mut self, world: &World, actor: Entity) -> Result<u8, String> {
        // Without Lightsmith (e.g. below hero-talent level) the action is unknown;
        // Prepared skips it like any unlearned action and its base binding stays.
        if let Some(resolved) = literal_rotation::execution::resolve_action(world, actor, 432459)? {
            self.bindings
                .actions
                .insert("holy_armaments".into(), resolved);
        }
        // sc_paladin.hpp781: next_armament is armament state, not talent knowledge.
        Ok(u8::from(crate::auras::has_aura(world, actor, 432478)))
    }

    pub fn actions() -> impl Iterator<Item = NativeAction<'static>> {
        context::ACTIONS.iter().map(|&(name, spell)| {
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

    pub fn prepared(
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

    pub fn poll_precombat(
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
}

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        Rotation::poll_precombat(self, world, actor, targets)
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
        self.encounter = crate::literal_rotation::encounter_starting_at(self.encounter, started_at);
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
