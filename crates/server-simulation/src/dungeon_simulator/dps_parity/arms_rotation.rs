//! Thin Arms binding for the unchanged profile APL and shared literal controller.
use super::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::PowerType;

// SimC76f8118 sc_warrior.cpp7300–7434/7446–7481: class and specialization lookups.
// Native knowledge/override resolution, costs, ranges and effects remain shared.
const ACTIONS: &[(&str, u32, bool)] = &[
    ("battle_stance", 386164, true),
    ("charge", 100, false),
    ("pummel", 6552, false),
    ("sweeping_strikes", 260708, true),
    ("ravager", 228920, false),
    ("avatar", 107574, true),
    ("colossus_smash", 167105, false),
    ("champions_spear", 376079, false),
    ("cleave", 845, false),
    ("mortal_strike", 12294, false),
    ("demolish", 436358, false),
    ("overpower", 7384, false),
    ("execute", 163201, false),
    ("heroic_strike", 1269383, false),
    ("slam", 1464, false),
    ("bladestorm", 227847, true),
    ("wrecking_throw", 384110, false),
    ("whirlwind", 1680, true),
    ("rend", 772, false),
    ("storm_bolt", 107570, false),
    ("arcane_torrent", 69179, true),
    ("lights_judgment", 255647, false),
    ("bag_of_tricks", 312411, false),
    ("berserking", 26297, true),
    ("blood_fury", 20572, true),
    ("fireblood", 265221, true),
    ("ancestral_call", 274738, true),
];

// sc_warrior.cpp7475–7481/8355–8524: buff and periodic aliases, not casts.
const BUFFS: &[(&str, u32)] = &[
    ("avatar", 107574),
    ("bladestorm", 446035),
    ("sweeping_strikes", 260708),
    ("colossus_smash", 208086),
    ("colossal_might", 440989),
    ("executioners_precision", 386633),
    ("sudden_death", 52437),
    ("collateral_damage", 334783),
    ("executioner", 445584),
    ("opportunist", 456120),
    ("imminent_demise", 445606),
    ("rend_dot", 388539),
];

const TALENTS: &[(&str, u32)] = &[
    ("avatar", 107574),
    ("demolish", 436358),
    ("slayers_dominance", 444767),
    ("deep_wounds", 1261060),
    ("dreadnaught", 262150),
    ("cleave", 845),
    ("rend", 772),
    ("tactical_edge", 1261051),
    ("fervor_of_battle", 202316),
    ("bloodletting", 383154),
    ("opportunist", 444774),
    ("massacre", 281001),
];

fn bindings() -> Bindings {
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
        global_cooldown_spell: 12294,
    }
}

fn program(profile: &super::inputs::Profile) -> Result<Program, String> {
    let actions = profile
        .options
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .ok_or("Arms reference has no literal actions")?;
    let rows = actions
        .iter()
        .map(|row| {
            let row = row.as_str().ok_or("Arms action row is not text")?;
            let (head, action) = row.split_once('=').ok_or("Arms action lacks assignment")?;
            let head = head.strip_suffix('+').unwrap_or(head);
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("Arms action is not an action list")?
            };
            Ok((list, action.strip_prefix('/').unwrap_or(action)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Program::parse(&rows)
}

pub(super) struct Rotation {
    driver: Driver,
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
        if profile.spec_id != 71 {
            return Err("literal Arms requires spec71".into());
        }
        let mut bindings = bindings();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        let driver = Driver::new(program(profile)?);
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver,
            bindings,
            equipment,
            encounter,
        })
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
        let prepared = self.read_prepared(world, actor, targets)?;
        let facts = ArmsFacts {
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

    fn read_prepared(
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
        serde_json::json!({"driver": "LITERAL_SIMC_ARMS_76f8118", "decisions": self.driver.decisions,
            "native_precombat_snapshot": self.driver.snapshot,
            "gaps": ["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}

fn precombat_selection(
    poll: literal_rotation::PrecombatPoll,
) -> Result<Option<(Action, literal_rotation::execution::Execution)>, String> {
    match poll {
        literal_rotation::PrecombatPoll::Selected { action, execution } => {
            Ok(Some((action, execution)))
        }
        literal_rotation::PrecombatPoll::Finished => Ok(None),
        literal_rotation::PrecombatPoll::Waiting => {
            Err("Arms precombat must finish pending native setup".into())
        }
    }
}

struct ArmsFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
}
impl Facts for ArmsFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.") || name.starts_with("equipped.") {
            self.equipment.fact(name, &self.native)
        } else {
            self.native.value(name)
        }
    }
}
impl ActionFacts for ArmsFacts<'_> {
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

pub(super) fn prepare_driver(run: &mut super::Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, encounter, &run.profile)?;
    while rotation.step(run.app.world_mut(), run.actor, &run.targets, true)? {
        super::preparation::finish_pending_setup(run)?;
    }
    rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
    Ok(Box::new(rotation))
}

pub(super) const REPORT_KEY: Option<&str> = Some("arms_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_ARMS_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.step(world, actor, targets, false).map(|_| ())
    }

    fn report(&self) -> serde_json::Value {
        self.report()
    }
}
