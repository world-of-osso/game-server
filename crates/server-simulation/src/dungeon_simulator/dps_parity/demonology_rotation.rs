//! Thin literal266 adapter. Production followers retain their native kit.
use super::inputs::Profile;
use super::literal_rotation::{
    self, Driver,
    default_pet::ConfiguredPet,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::{Npc, PowerType};

mod bindings;

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    pet: Option<ConfiguredPet>,
    encounter: Encounter,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        profile: &Profile,
        encounter: Encounter,
    ) -> Result<Self, String> {
        if profile.spec_id != 266 || !matches!(encounter, Encounter::Fixture { .. }) {
            return Err("literal Demonology requires spec266 and an explicit fixture".into());
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
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(rotation)
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!()
        };
        self.encounter = Encounter::Fixture {
            started_at,
            duration,
            style,
        };
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

    fn select(
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
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL:"));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("demonology_rotation");

impl super::SpecDriver for Rotation {
    fn supports_actor(identity: &crate::followers::simulator::ParityActor) -> bool {
        matches!(identity, crate::followers::simulator::ParityActor::Follower)
    }

    fn label(&self) -> &'static str {
        "LITERAL_SIMC_DEMONOLOGY_76f8118"
    }

    fn report(&self) -> Value {
        self.report()
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

// SimC76f8118 pet.cpp66–68/143–148: awake pets use the owner's actual
// primary enemy. Literal capture replaces the kit that normally issues orders.
// Per-action target_if choices do not replace that default primary.
fn command_minions(world: &mut World, actor: Entity, targets: &[Entity]) {
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

fn configure_core_reactions(world: &mut World, actor: Entity) -> Result<(), String> {
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
fn read_program(profile: &Profile) -> Result<Program, String> {
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

fn parse_row(row: &Value) -> Result<(&str, &str), String> {
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
struct Summons {
    imps: usize,
    tyrants: usize,
}

fn read_summons(world: &mut World, owner: Entity) -> Summons {
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

struct DemoFacts<'a> {
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
