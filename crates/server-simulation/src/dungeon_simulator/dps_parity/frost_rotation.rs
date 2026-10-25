//! Thin Frost player adapter over the shared literal SimC evaluator and dispatch.
mod bindings;
use super::{
    Run,
    inputs::Profile,
    literal_rotation::{
        self, Driver, PrecombatPoll,
        equipment::Equipment,
        execution::{Execution, Prepared},
    },
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::{Value, json};
use server::simc_apl::{Action, ActionFacts, Facts, Program};

pub(super) struct Rotation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
    first_ams_cast: f64,
}

impl Rotation {
    pub fn load(
        world: &mut World,
        actor: Entity,
        profile: &Profile,
        encounter: Encounter,
    ) -> Result<Self, String> {
        if !matches!(encounter, Encounter::Fixture { .. }) {
            return Err("literal Frost requires a Fixture encounter".into());
        }
        let mut bindings = bindings::build();
        let equipment = Equipment::load(world, actor, &mut bindings)?;
        // SimC76f8118 sc_death_knight.cpp1889/14580–14581: configured AMS delay,
        // default20s; no incoming damage or Runic Power grant is invented here.
        let first_ams_cast = profile
            .options
            .get("deathknight.first_ams_cast")
            .map(|value| {
                value
                    .as_str()
                    .ok_or("invalid deathknight.first_ams_cast")?
                    .parse::<f64>()
                    .map_err(|error| error.to_string())
            })
            .transpose()?
            .unwrap_or(20.0);
        literal_rotation::capture_fixture(world, actor, encounter)?;
        Ok(Self {
            driver: Driver::new(read_program(profile)?),
            bindings,
            equipment,
            encounter,
            first_ams_cast,
        })
    }

    fn read_prepared(
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
            bindings::execution_actions(),
            snapshot,
        )
    }

    pub fn poll_precombat(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<PrecombatPoll, String> {
        let prepared = self.read_prepared(world, actor, targets, self.driver.snapshot.is_none())?;
        let facts = FrostFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            first_ams_cast: self.first_ams_cast,
        };
        self.driver
            .poll_precombat(&facts, &prepared, &self.equipment)
    }

    pub fn execute_precombat(
        &mut self,
        run: &mut Run,
        action: Action,
        execution: Execution,
    ) -> Result<(), String> {
        super::preparation::execute_precombat_plan(run, &mut self.driver, action, execution)
    }

    pub fn start_combat_at(&mut self, started_at: f64) {
        let Encounter::Fixture {
            duration, style, ..
        } = self.encounter
        else {
            unreachable!("validated fixture")
        };
        self.encounter = Encounter::Fixture {
            started_at,
            duration,
            style,
        };
    }

    pub fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        let prepared = self.read_prepared(world, actor, targets, false)?;
        let facts = FrostFacts {
            native: NativeFacts::new(world, actor, &self.bindings, self.encounter, targets.len())
                .with_targets(targets.first().copied(), targets),
            equipment: &self.equipment,
            first_ams_cast: self.first_ams_cast,
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
        json!({"driver":"LITERAL_SIMC_FROST_76f8118", "decisions":self.driver.decisions,
            "native_precombat_snapshot":self.driver.snapshot,
            "gaps":["NATIVE_POTION_INVENTORY_UNAVAILABLE", "EXTERNAL_BUFF_PROVIDER_UNAVAILABLE:power_infusion"]})
    }
}

fn read_program(profile: &Profile) -> Result<Program, String> {
    let actions = profile
        .options
        .get("actions")
        .and_then(Value::as_array)
        .ok_or("Frost profile lacks literal action rows")?;
    let rows = actions
        .iter()
        .map(|row| {
            let row = row.as_str().ok_or("non-string Frost action row")?;
            let (head, action) = row
                .split_once('=')
                .ok_or("Frost action row lacks assignment")?;
            let head = head.trim_end_matches('+');
            let list = if head == "actions" {
                "default"
            } else {
                head.strip_prefix("actions.")
                    .ok_or("invalid Frost action list")?
            };
            Ok((list, action.trim_start_matches('/')))
        })
        .collect::<Result<Vec<_>, String>>()?;
    // Pinned MID2_Death_Knight_Frost.simc26–133, unmodified source-order APL.
    Program::parse(&rows)
}

#[test]
#[ignore = "requires private DB and pinned Frost profile; exercises real player literal casts"]
fn dps_parity_frost_player_executes_literal_profile() {
    let mut run = super::prepare_measurement(251, 1).expect("native Frost literal player");
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_some()
    );
    super::engage(&mut run);
    for _ in 0..400 {
        super::drive_player(&mut run).expect("literal Frost action");
        run.app.update();
    }
    let combat = super::report::combat(run.app.world(), &run.targets, 20.0);
    let actions = combat["actions"].as_array().unwrap();
    for spell in [49020, 49143, 49184] {
        assert!(
            actions.iter().any(|action| {
                action["spell_id"] == spell && action["count"].as_u64().unwrap() > 0
            }),
            "literal Obliterate/Frost Strike/Howling Blast must execute: {spell}"
        );
    }
    assert!(combat["dps"].as_f64().unwrap() > 0.0);
}

#[test]
#[ignore = "requires exact251 profile and private native Omnium catalogs"]
fn frost_named_and_numeric_expansion_entries_apply_native_auras() {
    const NAMED: &str = "rune_of_unleashed_fire/rune_of_lynxlike_reflexes/rune_of_lingering/rune_of_burning_haste/rune_of_overload";
    const NUMERIC: &str = "136822/136823/136817/136821/136814";
    for tokens in [NAMED, NUMERIC] {
        let (mut profile, traits) = super::read_reference(251).unwrap();
        assert_eq!(profile.omnium_talents.as_deref(), Some(NAMED));
        profile.omnium_talents = Some(tokens.into());
        let identity = super::inputs::read_actor(&profile).unwrap();
        let run = super::prepare_literal_reference(profile, &traits, 1, identity).unwrap();
        for spell in [1279599, 1279605, 1287555, 1279610, 1279614] {
            assert!(
                crate::auras::has_aura(run.app.world(), run.actor, spell),
                "{tokens}: native Omnium aura{spell} missing"
            );
        }
    }
}

struct FrostFacts<'a> {
    native: NativeFacts<'a>,
    equipment: &'a Equipment,
    first_ams_cast: f64,
}

impl Facts for FrostFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "death_knight.first_ams_cast" => Ok(self.first_ams_cast),
            _ if name.starts_with("trinket.") || name.starts_with("equipped.") => {
                self.equipment.fact(name, &self.native)
            }
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for FrostFacts<'_> {
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }

    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        if name.starts_with("trinket.")
            || name.starts_with("equipped.")
            || name == "death_knight.first_ams_cast"
        {
            self.value(name)
        } else {
            self.native.value_for(action, name)
        }
    }
}

#[test]
#[ignore = "native Frost literal ERW enemy dispatch, damage and actual400 raw RP grant"]
fn frost_literal_empower_rune_weapon_uses_hostile_primary() {
    let mut run = super::prepare_literal(251, 1).unwrap();
    super::engage(&mut run);
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let rotation = Rotation::load(run.app.world_mut(), run.actor, &run.profile, encounter).unwrap();
    let prepared = rotation
        .read_prepared(run.app.world_mut(), run.actor, &run.targets, false)
        .unwrap();
    let action = Action {
        name: "empower_rune_weapon".into(),
        source_row: None,
        target: None,
        options: Default::default(),
    };
    let execution = prepared
        .execution(&action, &rotation.equipment)
        .unwrap()
        .expect("native ERW must be admitted against the hostile primary");
    literal_rotation::execution::execute(run.app.world_mut(), run.actor, execution).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let combat = super::report::combat(run.app.world(), &run.targets, 2.0);
    assert!(
        combat["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["spell_id"] == 47568 && row["damage"].as_u64().unwrap() > 0)
    );
    assert!(
        combat["energize"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["spell_id"] == 1230959 && row["effective_gain"] == 400)
    );
}
