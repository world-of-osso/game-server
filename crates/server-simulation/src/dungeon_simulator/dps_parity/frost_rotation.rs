//! Fixture adapter over the production APL.
use super::{
    Run,
    literal_rotation::{self, PrecombatPoll},
};
use crate::simc_apl_native::{Encounter, NativeFacts};
use bevy::prelude::*;
use serde_json::Value;
use server::simc_apl::{Action, Facts};
pub(super) use server_follower_apl::frost_rotation::*;

#[cfg(test)]
mod proc_tests;
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
    // Howling Blast requires an earned Rime in this opener; missing Rime is
    // a separate mechanic, not a literal-driver admission failure.
    for spell in [49020, 49143] {
        assert!(
            actions.iter().any(|action| {
                action["spell_id"] == spell && action["count"].as_u64().unwrap() > 0
            }),
            "literal Obliterate/Frost Strike must execute: {spell}"
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

pub(super) fn prepare_driver(run: &mut Run) -> Result<Box<dyn super::SpecDriver>, String> {
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(run.app.world_mut(), run.actor, &run.profile, encounter)?;
    super::literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    for _ in 0..super::preparation::MAX_SETUP_TICKS {
        match rotation.poll_precombat(run.app.world_mut(), run.actor, &run.targets)? {
            PrecombatPoll::Waiting => super::preparation::finish_pending_setup(run)?,
            PrecombatPoll::Selected { action, execution } => {
                super::preparation::execute_precombat_plan(
                    run,
                    &mut rotation.driver,
                    action,
                    execution,
                )?;
            }
            PrecombatPoll::Finished => {
                rotation.start_combat_at(crate::spell_cast::now(run.app.world()));
                return Ok(Box::new(rotation));
            }
        }
    }
    Err("native Frost precombat did not finish within the setup bound".into())
}

pub(super) const REPORT_KEY: Option<&str> = Some("frost_rotation");

impl super::SpecDriver for Rotation {
    fn label(&self) -> &'static str {
        "LITERAL_SIMC_FROST_76f8118"
    }

    fn drive(
        &mut self,
        world: &mut World,
        actor: Entity,
        targets: &[Entity],
    ) -> Result<(), String> {
        self.drive(world, actor, targets)
    }

    fn report(&self) -> Value {
        self.report()
    }
}
