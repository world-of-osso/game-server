use crate::literal_rotation::{
    Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

// SimC76f8118 sc_warrior.cpp4497–4518/7316/7450–7491: Fury variants,
// including Execute's specialization spell and Unrelenting Onslaught's Bladestorm.
pub const ACTIONS: &[(&str, u32, bool)] = &[
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

pub fn bindings() -> Bindings {
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

pub fn program() -> Result<Program, String> {
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

pub struct FuryFacts<'a> {
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
pub fn permits_bladestorm_action(facts: &dyn Facts, action: &Action) -> Result<bool, String> {
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

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
}

impl Rotation {
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot})
    }

    pub fn load(world: &mut World, actor: Entity, encounter: Encounter) -> Result<Self, String> {
        let mut bindings = bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        Ok(Self {
            driver: Driver::new(program()?),
            bindings,
            equipment,
            encounter,
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

    pub fn select(
        &mut self,
        world: &World,
        actor: Entity,
        targets: &[Entity],
        list: &str,
        prepared: &Prepared,
    ) -> Result<Option<(Action, crate::literal_rotation::execution::Execution)>, String> {
        let facts = FuryFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
        };
        self.driver.select(list, &facts, prepared, &self.equipment)
    }

    pub fn drive(
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

impl crate::follower::SpecRotation for Rotation {
    fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        let prepared = self.prepared(world, actor, targets, self.driver.snapshot.is_none())?;
        Ok(
            match self.select(world, actor, targets, "precombat", &prepared)? {
                Some((action, execution)) => {
                    crate::literal_rotation::PrecombatPoll::Selected { action, execution }
                }
                None => crate::literal_rotation::PrecombatPoll::Finished,
            },
        )
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
        self.drive(world, actor, targets)
    }
}
