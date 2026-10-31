//! Thin Protection Warrior binding for the unchanged profile APL and shared literal controller.
use crate::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

// SimC 76f8118 Protection action, aura, and talent aliases.
pub const ACTIONS: &[(&str, u32, bool)] = &[
    ("battle_stance", 386164, true),
    ("charge", 100, false),
    ("avatar", 107574, true),
    ("shield_wall", 871, true),
    ("ignore_pain", 190456, true),
    ("shield_block", 2565, true),
    ("ravager", 228920, false),
    ("thunder_blast", 435222, false),
    ("demoralizing_shout", 1160, true),
    ("champions_leap", 1271985, false),
    ("champions_spear", 376079, false),
    ("demolish", 436358, false),
    ("shield_charge", 385952, false),
    ("thunder_clap", 6343, true),
    ("revenge", 6572, false),
    ("shield_slam", 23922, false),
    ("execute", 163201, false),
    ("wrecking_throw", 384110, false),
    ("shattering_throw", 64382, false),
    ("devastate", 20243, false),
    ("arcane_torrent", 69179, true),
    ("lights_judgment", 255647, false),
    ("bag_of_tricks", 312411, false),
    ("berserking", 26297, true),
    ("blood_fury", 20572, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
];
pub const BUFFS: &[(&str, u32)] = &[
    ("avatar", 107574),
    ("shield_block", 132404),
    ("ignore_pain", 190456),
    ("thunder_blast", 435615),
    ("violent_outburst", 386478),
    ("seeing_red", 386486),
    ("phalanx", 1278009),
    ("revenge", 5302),
    ("colossal_might", 440989),
    ("sudden_death", 52437),
    ("ravager", 156287),
    ("rend_dot", 388539),
];
pub const TALENTS: &[(&str, u32)] = &[
    ("booming_voice", 202743),
    ("snap_induction", 456270),
    ("heavy_repercussions", 203177),
    ("practiced_strikes", 429647),
    ("demolish", 436358),
    ("lightning_strikes", 434969),
    ("heavy_handed", 1235088),
    ("javelineer", 1271948),
    ("massacre", 281001),
];

pub fn bindings() -> Bindings {
    let names = |rows: &[(&str, u32)]| {
        rows.iter()
            .map(|&(name, id)| (name.to_owned(), id))
            .collect()
    };
    Bindings {
        actions: ACTIONS
            .iter()
            .map(|&(name, id, _)| (name.to_owned(), id))
            .collect(),
        buffs: names(BUFFS),
        talents: names(TALENTS),
        resources: [("rage".into(), PowerType::Rage)].into(),
        dot_refresh: Default::default(),
        global_cooldown_spell: 23922,
    }
}

pub fn program(profile: &server::followers::parity_profile::Profile) -> Result<Program, String> {
    let actions = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Protection Warrior reference has no literal actions")?;
    let rows = actions
        .iter()
        .map(|row| {
            let row = row
                .as_str()
                .ok_or("Protection Warrior action row is not text")?;
            let (head, action) = row
                .split_once('=')
                .ok_or("Protection Warrior action lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Protection Warrior action is not an action list")?
            };
            Ok((list, action.strip_prefix('/').unwrap_or(action)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

pub struct Rotation {
    pub driver: Driver,
    pub bindings: Bindings,
    pub equipment: Equipment,
    pub encounter: Encounter,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        encounter: Encounter,
        profile: &server::followers::parity_profile::Profile,
    ) -> Result<Self, String> {
        if profile.spec_id != 73 {
            return Err("literal Protection Warrior requires spec73".into());
        }
        let mut bindings = bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let driver = Driver::new(program(profile)?);
        Ok(Self {
            driver,
            bindings,
            equipment,
            encounter,
        })
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
        let prepared = self.read_prepared(world, actor, targets)?;
        let facts = ProtectionWarriorFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
        };
        let selected = if precombat {
            precombat_selection(
                self.driver
                    .poll_precombat(&facts, &prepared, &self.equipment)?,
            )?
        } else {
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        };
        let Some((action, execution)) = selected else {
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
        let actions = ACTIONS
            .iter()
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        Prepared::read(
            world,
            actor,
            targets,
            &self.equipment,
            actions,
            self.driver.snapshot.is_none(),
        )
    }

    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"driver": "LITERAL_SIMC_PROTECTION_WARRIOR_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}

pub fn precombat_selection(
    poll: literal_rotation::PrecombatPoll,
) -> Result<Option<(Action, literal_rotation::execution::Execution)>, String> {
    match poll {
        literal_rotation::PrecombatPoll::Selected { action, execution } => {
            Ok(Some((action, execution)))
        }
        literal_rotation::PrecombatPoll::Finished => Ok(None),
        literal_rotation::PrecombatPoll::Waiting => {
            Err("Protection Warrior precombat must finish pending native setup".into())
        }
    }
}

pub struct ProtectionWarriorFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}
impl Facts for ProtectionWarriorFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}
impl ActionFacts for ProtectionWarriorFacts<'_> {
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
