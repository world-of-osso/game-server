//! Thin literal266 adapter. Production followers retain their native kit.
use crate::literal_rotation::{
    self, Driver,
    default_pet::ConfiguredPet,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::followers::parity_profile::Profile;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::{Npc, PowerType};

pub mod bindings;

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub pet: Option<ConfiguredPet>,
    pub encounter: Encounter,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        profile: &Profile,
        encounter: Encounter,
    ) -> Result<Self, String> {
        if profile.spec_id != 266 {
            return Err("literal Demonology requires spec266".into());
        }
        let mut bindings = bindings::build();
        let pet = ConfiguredPet::read(world, actor, profile)?;
        if let Some(pet) = &pet {
            pet.bind(&mut bindings)?;
        }
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        configure_core_reactions(world, actor)?;
        let rotation = Self {
            driver: Driver::new(read_program(profile)?),
            bindings,
            equipment,
            pet,
            encounter,
        };
        Ok(rotation)
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        self.encounter = crate::literal_rotation::encounter_starting_at(self.encounter, started_at);
    }

    pub fn step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        precombat: bool,
    ) -> Result<bool, String> {
        if !precombat {
            command_minions(world, actor, targets);
        }
        let prepared = self.read_prepared(world, actor, targets)?;
        let Some((action, execution)) = self.select(world, actor, targets, precombat, &prepared)?
        else {
            return Ok(false);
        };
        self.driver.execute(world, actor, action, execution)?;
        Ok(true)
    }

    pub fn read_prepared(
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
        let actions = bindings::execution_actions().chain(pet_action);
        Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            actions,
            self.driver.snapshot.is_none(),
        )
    }

    pub fn select(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        precombat: bool,
        prepared: &Prepared,
    ) -> Result<Option<(Action, literal_rotation::execution::Execution)>, String> {
        let summons = read_summons(world, actor);
        let facts = DemoFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            summons,
            precombat,
        };
        let list = if precombat { "precombat" } else { "default" };
        self.driver.select(list, &facts, prepared, &self.equipment)
    }

    pub fn report(&self) -> Value {
        json!({"driver": "LITERAL_SIMC_DEMONOLOGY_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}

// SimC76f8118 pet.cpp66–68/143–148: awake pets use the owner's actual
// primary enemy. Literal capture replaces the kit that normally issues orders.
// Per-action target_if choices do not replace that default primary.
pub fn command_minions(world: &mut World, actor: Entity, targets: &[Entity]) {
    let target = targets.first().copied();
    let current = world
        .get::<crate::networking::CombatTarget>(actor)
        .and_then(|target| target.0);
    if current != target {
        world
            .entity_mut(actor)
            .insert(crate::networking::CombatTarget(target));
    }
    if let Some(pet) = crate::hunter_pet::summoned_pet(world, actor) {
        let current = world
            .get::<crate::networking::CombatTarget>(pet)
            .and_then(|target| target.0);
        if current != target {
            crate::pet_control::order_companion_pet(world, actor, target);
        }
    }
}

pub fn configure_core_reactions(world: &mut World, actor: Entity) -> Result<(), String> {
    // SimC76f8118 player.cpp1018–1020 and pinned266 report defaults, before acquisition.
    crate::simc_apl_native::reaction::configure(
        world,
        actor,
        crate::simc_apl_native::reaction::ReactionSettings {
            mean: 0.25,
            stddev: 0.05,
            offset: 0.1,
            nu: 0.15,
            maximum: 1.4,
        },
        &[264173],
    )
}

// SimC76f8118 apl/warlock.cpp200–281: preserve every exported row, not a
// native follower priority approximation or a second expression interpreter.
pub fn read_program(profile: &Profile) -> Result<Program, String> {
    let actions = profile
        .options
        .get("actions")
        .and_then(Value::as_array)
        .ok_or("266 profile lacks literal actions")?;
    let rows = actions
        .iter()
        .map(parse_row)
        .collect::<Result<Vec<_>, _>>()?;
    Program::parse(&rows)
}

pub fn parse_row(row: &Value) -> Result<(&str, &str), String> {
    let row = row.as_str().ok_or("266 action row must be text")?;
    let (head, literal) = row
        .split_once('=')
        .ok_or("266 action row lacks assignment")?;
    let head = head.trim_end_matches('+');
    let list = if head == "actions" {
        "default"
    } else {
        head.strip_prefix("actions.")
            .ok_or("266 row is not an action list")?
    };
    Ok((list, literal.trim_start_matches('/')))
}

#[derive(Default)]
pub struct Summons {
    imps: usize,
    tyrants: usize,
}

pub fn read_summons(world: &mut World, owner: Entity) -> Summons {
    let mut result = Summons::default();
    for (entity, summon, npc) in world
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature, &Npc)>()
        .iter(world)
    {
        if summon.owner != owner || !crate::spell_cast::targets::is_alive(world, entity) {
            continue;
        }
        // SimC76f8118 pets.cpp921/945 and1510–1520: real owned summons only.
        match npc.template_id {
            55659 | 143622 => result.imps += 1,
            135002 | 250289 => result.tyrants += 1,
            _ => {}
        }
    }
    result
}

pub struct DemoFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    summons: Summons,
    precombat: bool,
}

impl Facts for DemoFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "pet.demonic_tyrant.active" => Ok(if self.summons.tyrants > 0 { 1.0 } else { 0.0 }),
            "buff.wild_imps.stack" => Ok(self.summons.imps as f64),
            _ if name.starts_with("trinket.") => self.equipment.fact(name, &self.native),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for DemoFacts<'_> {
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        // SimC76f8118 actions.cpp3469–3477/3033–3034/5225–5229/5320–5324.
        match action.name.as_str() {
            "power_siphon" => Ok(self.summons.imps > 0
                || self.precombat && self.native.value("talent.inner_demons")? != 0.0),
            "implosion" => Ok(self.summons.imps > 0),
            "infernal_bolt" => Ok(self.native.value("buff.infernal_bolt.up")? != 0.0),
            "ruination" => Ok(self.native.value("buff.ruination.up")? != 0.0),
            // SimC76f8118 actions.cpp845–850: replacement makes Shadow Bolt unready.
            "shadow_bolt" => Ok(self.native.value("buff.infernal_bolt.up")? == 0.0),
            _ => Ok(true),
        }
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("target.")
            || name.starts_with("debuff.")
            || name.starts_with("dot.")
            || matches!(name, "gcd" | "gcd.max" | "gcd.min" | "gcd.remains")
        {
            self.native.value_for(action, name)
        } else {
            self.value(name)
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
        Ok(if self.step(world, actor, targets, true)? {
            crate::literal_rotation::PrecombatPoll::Waiting
        } else {
            crate::literal_rotation::PrecombatPoll::Finished
        })
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
        self.step(world, actor, targets, false).map(|_| ())
    }
}
