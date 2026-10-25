//! Literal pinned BM source; decisions use the shared evaluator and native execution.
mod bindings;
use super::literal_rotation::{self, Driver, equipment::Equipment, execution::Prepared};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts, Facts, Program};

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    potion: String,
}

fn program() -> Result<Program, String> {
    // SimC76f8118 ActionPriorityLists/default/hunter_beast_mastery.simc, unchanged.
    let rows = include_str!("beast_mastery_rotation/76f8118.simc")
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (head, row) = line.split_once('=').ok_or("BM source assignment missing")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("BM source list missing")?
            };
            Ok((list, row.strip_prefix('/').unwrap_or(row)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        profile: &super::inputs::Profile,
    ) -> Result<Self, String> {
        // SimC76f8118 player.cpp1017–1020: this pinned profile supplies no
        // reaction overrides; native acquisition samples the source defaults.
        crate::simc_apl_native::reaction::configure(
            world,
            actor,
            crate::simc_apl_native::reaction::ReactionSettings {
                mean: 0.25,
                stddev: 0.0,
                offset: 0.1,
                nu: 0.15,
                maximum: 1.4,
            },
            &[1276720],
        )?;
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver: Driver::new(program()?),
            bindings,
            equipment,
            encounter,
            potion: profile
                .consumables
                .get("potion")
                .ok_or("BM profile potion option missing")?
                .clone(),
        })
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!("literal fixture validated at load")
        };
        self.encounter = Encounter::Fixture {
            started_at,
            duration,
            style,
        };
    }

    pub fn precombat_step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<bool, String> {
        self.start_combat_at(crate::spell_cast::now(world));
        self.step(world, actor, targets, "precombat")
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        // SimC76f8118 sc_hunter.cpp1706–1711: an awake pet schedules its
        // melee independently. Literal capture replaces follower AI's attack order.
        if let Some(pet) = crate::hunter_pet::summoned_pet(world, actor) {
            let current = world
                .get::<crate::networking::CombatTarget>(pet)
                .and_then(|target| target.0);
            if current != targets.first().copied() {
                crate::pet_control::order_companion_pet(world, actor, targets.first().copied());
            }
        }
        self.step(world, actor, targets, "default").map(|_| ())
    }

    fn step(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
    ) -> Result<bool, String> {
        let prepared = Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            bindings::actions(),
            self.driver.snapshot.is_none(),
        )?;
        let native = NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
            .with_targets(targets.first().copied(), targets);
        let facts = BmFacts {
            native,
            equipment: &self.equipment,
            potion: &self.potion,
        };
        let Some((action, execution)) =
            self.driver
                .select(list, &facts, &prepared, &self.equipment)?
        else {
            return Ok(false);
        };
        self.driver.execute(world, actor, action, execution)?;
        Ok(true)
    }

    pub fn report(&self) -> Value {
        json!({"driver":"LITERAL_SIMC_BEAST_MASTERY_76f8118","decisions":self.driver.decisions,
            "native_precombat_snapshot":self.driver.snapshot,
            "gaps":["NATIVE_POTION_INVENTORY_UNAVAILABLE","EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}

struct BmFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    potion: &'a str,
}

fn equipment_operand(name: &str) -> bool {
    name.starts_with("trinket.") || name.starts_with("equipped.") || name.starts_with("set_bonus.")
}

impl Facts for BmFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if equipment_operand(name) {
            return self.equipment.fact(name, &self.native);
        }
        // SimC76f8118 sc_hunter.cpp7340–7342,1055: configured default is true.
        if name == "max_prio_damage" {
            return Ok(1.0);
        }
        // SimC76f8118 player.cpp12317–12335: the third BM tiered entry.
        if name == "apex.3" {
            return self.native.value("talent.natures_ally_3");
        }
        // SimC76f8118 sc_hunter.cpp7368–7374: any actual ready-beast buff.
        if name == "howl_summon.ready" {
            let ready =
                ["wyvern", "boar", "bear"]
                    .into_iter()
                    .try_fold(false, |ready, beast| {
                        self.native
                            .value(&format!("buff.howl_{beast}.up"))
                            .map(|value| ready || value != 0.0)
                    })?;
            return Ok(if ready { 1.0 } else { 0.0 });
        }
        if let Some(alias) = name.strip_prefix("potion.") {
            return Ok(configured_potion(self.potion, alias));
        }
        self.native.value(name)
    }
}

fn configured_potion(potion: &str, alias: &str) -> f64 {
    // SimC76f8118 player.cpp12376–12409 uses the configured potion,
    // irrespective of whether the current inventory admits a use.
    let configured = potion
        .rsplit_once('_')
        .filter(|(_, suffix)| suffix.parse::<u8>().is_ok())
        .map_or(potion, |(base, _)| base);
    if configured.eq_ignore_ascii_case(alias) || potion.eq_ignore_ascii_case(alias) {
        1.0
    } else {
        0.0
    }
}

impl ActionFacts for BmFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if equipment_operand(name)
            || name.starts_with("potion.")
            || matches!(name, "max_prio_damage" | "apex.3" | "howl_summon.ready")
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    // SimC76f8118 sc_hunter.cpp1052: summon_pet defaults to duck. Literal
    // capture suppresses follower AI, so provision its existing native stable policy here.
    crate::hunter_pet::provision_follower_duck(run.app.world_mut(), run.actor)?;
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    while rotation.precombat_step(run.app.world_mut(), run.actor, &run.targets)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    run.gaps
        .retain(|gap| !gap.starts_with("NATIVE_FOLLOWER_KIT_VS_SIMC_APL"));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("beast_mastery_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_BEAST_MASTERY_76f8118"
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
