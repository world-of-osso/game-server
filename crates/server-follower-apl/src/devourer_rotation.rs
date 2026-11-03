//! Thin pinned Devourer adapter; native casts and the shared interpreter do the work.
pub mod bindings;
pub mod facts;
use crate::literal_rotation::{Driver, PrecombatPoll, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use facts::DevourerFacts;
use serde_json::{Value, json};
use server::simc_apl::Program;

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
    entropy_initialized: bool,
}

pub fn configure_actor_reactions(world: &mut World, actor: Entity) -> Result<(), String> {
    // Pinned 1480 reference reaction fields; player.cpp1017–1020.
    let settings = crate::simc_apl_native::reaction::ReactionSettings {
        mean: 0.25,
        stddev: 0.05,
        offset: 0.1,
        nu: 0.15,
        maximum: 1.4,
    };
    crate::simc_apl_native::reaction::configure(world, actor, settings, &[1217607, 1256302])
}

pub fn load_program() -> Result<Program, String> {
    let rows = include_str!("devourer_rotation/76f8118.simc")
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (head, action) = line
                .split_once('=')
                .ok_or("Devourer action lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Devourer action lacks list")?
            };
            Ok((list, action.strip_prefix('/').unwrap_or(action)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

impl Rotation {
    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        if world
            .get::<shared::components::ActiveSpec>(actor)
            .map(|spec| spec.0)
            != Some(1480)
        {
            return Err("literal Devourer requires native spec1480".into());
        }
        configure_actor_reactions(world, actor)?;
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let driver = Driver::new(load_program()?);
        Ok(Self {
            driver,
            bindings,
            equipment,
            encounter,
            entropy_initialized: false,
        })
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = crate::literal_rotation::encounter_starting_at(self.encounter, started_at);
    }

    pub fn precombat_step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<bool, String> {
        match self.poll_precombat(world, actor, targets)? {
            PrecombatPoll::Waiting => Ok(true),
            PrecombatPoll::Finished => Ok(false),
            PrecombatPoll::Selected { action, execution } => {
                crate::follower::SpecRotation::execute(self, world, actor, action, execution)?;
                Ok(true)
            }
        }
    }

    pub fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        if !self.entropy_initialized {
            self.initialize_entropy(world, actor)?;
            self.entropy_initialized = true;
        }
        let prepared = self.prepare(world, actor, targets)?;
        let facts = DevourerFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            world,
            actor,
        };
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    fn initialize_entropy(&self, world: &mut World, actor: Entity) -> Result<(), String> {
        if !world
            .get::<server::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1261684))
        {
            return Ok(());
        }
        // SimC76f8118 cpp9824–9835 seeds Entropy's native effect2 at precombat begin.
        let count =
            server::spell_cast::scripts::script_effect_value(world, actor, 1261684, 1) as usize;
        for _ in 0..count {
            if !server::spell_cast::try_cast_triggered(world, actor, 1225789, Some(actor), 0) {
                return Err("Entropy precombat Meta stack cast refused".into());
            }
        }
        Ok(())
    }

    pub fn prepare(
        &self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<Prepared, String> {
        let mut prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::execution_actions(),
            self.driver.snapshot.is_none(),
        )?;
        // SimC76f8118 cpp6254–6296 selects Eradicate only while its buff is active.
        if server::auras::has_aura(world, actor, 1239524) {
            prepared.suppress_action("reap");
            prepared.suppress_action("cull");
        } else {
            prepared.suppress_action("eradicate");
        }
        Ok(prepared)
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let pickup = {
            let facts = DevourerFacts {
                native: NativeFacts::new(
                    world,
                    actor,
                    &self.bindings,
                    self.encounter,
                    targets.len(),
                )
                .with_targets(targets.first().copied(), targets),
                equipment: &self.equipment,
                world,
                actor,
            };
            self.driver
                .fragment_pickup_permitted(world, actor, &facts)?
        };
        if let Some(permitted) = pickup {
            server::soul_fragments::finish_pickup(world, actor, permitted)?;
        }
        let prepared = self.prepare(world, actor, targets)?;
        let facts = DevourerFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            world,
            actor,
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
        json!({"driver": "LITERAL_SIMC_DEVOURER_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_DEVOURER_PER_ABILITY_QUALIFICATION_OPEN", "NATIVE_DEVOURER_FRAGMENT_SIDE_EFFECTS_UNQUALIFIED"]})
    }
}

impl crate::follower::SpecRotation for Rotation {
    fn action_spells(&self) -> Vec<u32> {
        self.driver.action_spells(&self.bindings)
    }

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
