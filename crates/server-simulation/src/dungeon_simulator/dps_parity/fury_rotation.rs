//! Literal pinned Fury adapter. Native world state owns readiness and all values.
#[cfg(test)]
mod deep_wounds_tests;
#[cfg(test)]
mod sudden_death_tests;
#[cfg(test)]
mod tests;
use super::{
    Run,
    literal_rotation::{
        self, Driver,
        equipment::Equipment,
        execution::{NativeAction, Prepared},
    },
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

// SimC76f8118 sc_warrior.cpp4497–4518/7316/7450–7491: Fury variants,
// including Execute's specialization spell and Unrelenting Onslaught's Bladestorm.
const ACTIONS: &[(&str, u32, bool)] = &[
    ("berserker_stance", 386196, true),
    ("charge", 100, false),
    ("heroic_leap", 6544, false),
    ("pummel", 6552, false),
    ("lights_judgment", 255647, false),
    ("bag_of_tricks", 312411, false),
    ("berserking", 26297, true),
    ("blood_fury", 20572, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
    ("recklessness", 1719, true),
    ("avatar", 107574, true),
    ("rampage", 184367, false),
    ("bladestorm", 227847, true),
    ("execute", 280735, false),
    ("crushing_blow", 335097, false),
    ("odyns_fury", 385059, true),
    ("bloodbath", 335096, false),
    ("raging_blow", 85288, false),
    ("bloodthirst", 23881, false),
    ("whirlwind", 190411, true),
    ("storm_bolt", 107570, false),
    ("thunder_blast", 435222, true),
    ("thunder_clap", 6343, true),
];

fn bindings() -> Bindings {
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.into(), id))
            .collect(),
        // SimC76f8118 sc_warrior.cpp8420–8460 uses these actual aura identities.
        buffs: [
            ("enrage", 184362),
            ("recklessness", 1719),
            ("avatar", 107574),
            ("hack_and_slash", 383873),
            ("sudden_death", 52437),
            ("whirlwind", 85739),
            ("bladestorm", 446035),
            ("thunder_blast", 435615),
        ]
        .into_iter()
        .map(|(name, id)| (name.into(), id))
        .collect(),
        talents: [
            ("slayers_dominance", 444767),
            ("unrelenting_onslaught", 444780),
            ("lightning_strikes", 434969),
            ("improved_whirlwind", 12950),
            ("massacre", 206315),
            ("deep_wounds", 1261060),
            ("wrath_and_fury", 392936),
            ("recklessness", 1719),
        ]
        .into_iter()
        .map(|(name, id)| (name.into(), id))
        .collect(),
        resources: [("rage".into(), PowerType::Rage)].into(),
        global_cooldown_spell: 23881,
        ..Default::default()
    }
}

fn program() -> Result<Program, String> {
    let rows = include_str!("fury_rotation/76f8118.simc")
        .lines()
        .filter(|row| !row.is_empty() && !row.starts_with('#'))
        .map(|row| {
            let (head, action) = row.split_once('=').ok_or("Fury APL lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.").ok_or("Fury APL lacks list")?
            };
            Ok((list, action.strip_prefix('/').unwrap_or(action)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

struct FuryFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}

impl Facts for FuryFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}
fn permits_bladestorm_action(facts: &dyn Facts, action: &Action) -> Result<bool, String> {
    const RACIALS: &[&str] = &[
        "lights_judgment",
        "bag_of_tricks",
        "berserking",
        "blood_fury",
        "fireblood",
        "ancestral_call",
    ];
    let warrior = ACTIONS.iter().any(|(name, _, _)| *name == action.name)
        && !RACIALS.contains(&action.name.as_str());
    if !warrior || facts.value("buff.bladestorm.up")? == 0.0 {
        return Ok(true);
    }
    // SimC76f8118 sc_warrior.cpp1343–1344/6409–6410: class actions wait
    // for Bladestorm; Unrelenting Onslaught admits Storm Bolt, not other casts.
    Ok(action.name == "storm_bolt" && facts.value("talent.unrelenting_onslaught")? != 0.0)
}

impl ActionFacts for FuryFacts<'_> {
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        Ok(permits_bladestorm_action(&self.native, action)? && self.native.target_ready(action)?)
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Rotation {
    pub(super) fn report(&self) -> serde_json::Value {
        serde_json::json!({"decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot})
    }

    fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        let mut bindings = bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver: Driver::new(program()?),
            bindings,
            equipment,
            encounter,
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
            ACTIONS
                .iter()
                .map(|&(name, spell, self_target)| NativeAction {
                    name,
                    spell,
                    self_target,
                    talent: None,
                }),
            snapshot,
        )
    }

    fn select(
        &mut self,
        world: &World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
        prepared: &Prepared,
    ) -> Result<Option<(Action, super::literal_rotation::execution::Execution)>, String> {
        let facts = FuryFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
        };
        self.driver.select(list, &facts, prepared, &self.equipment)
    }

    pub(super) fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let prepared = self.prepared(world, actor, targets, false)?;
        if let Some((action, execution)) =
            self.select(world, actor, targets, "default", &prepared)?
        {
            self.driver.execute(world, actor, action, execution)?;
        }
        Ok(())
    }
}

pub(super) fn prepare_precombat(run: &mut Run) -> Result<(), String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter)?;
    super::preparation::finish_pending_setup(run)?;
    // SimC76f8118 player.cpp6138–6183: source-ordered precombat, not a priority loop.
    loop {
        let prepared = rotation.prepared(
            run.app.world_mut(),
            run.actor,
            &run.targets,
            rotation.driver.snapshot.is_none(),
        )?;
        let Some((action, execution)) = rotation.select(
            run.app.world(),
            run.actor,
            &run.targets,
            "precombat",
            &prepared,
        )?
        else {
            break;
        };
        super::preparation::execute_precombat_plan(run, &mut rotation.driver, action, execution)?;
    }
    rotation.encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    run.gaps
        .retain(|gap| gap != "NATIVE_ASSISTED_COMBAT_70448_VS_SIMC_APL");
    run.fury_rotation = Some(rotation);
    Ok(())
}
