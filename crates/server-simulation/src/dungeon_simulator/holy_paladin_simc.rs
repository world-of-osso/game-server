//! Holy Paladin healing decisions over native readiness and party state.
#[cfg(test)]
#[path = "holy_paladin_simc_tests.rs"]
mod tests;
use super::dps_parity::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use super::healer_simc_replay::Policy;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::{Health, PowerType};

const SPELLS: &[(&str, u32, bool)] = &[
    ("beacon_of_light", 53563, false),
    ("lay_on_hands", 633, false),
    ("holy_shock", 20473, false),
    ("word_of_glory", 85673, false),
    ("flash_of_light", 19750, false),
    ("holy_light", 82326, false),
];

struct HolyPaladin {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

pub(super) fn prepare(
    run: &mut super::dps_parity::Run,
    healer: Entity,
) -> Result<Box<dyn Policy>, String> {
    let mut bindings = Bindings {
        actions: SPELLS
            .iter()
            .map(|(name, spell, _)| ((*name).into(), *spell))
            .collect(),
        resources: [
            ("mana".into(), PowerType::Mana),
            ("holy_power".into(), PowerType::HolyPower),
        ]
        .into(),
        global_cooldown_spell: 82326,
        ..Default::default()
    };
    let equipment = Equipment::load(run.app.world_mut(), healer, &mut bindings)?;
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), healer, encounter)?;
    // SimC has no default Holy healer APL; these are native triage thresholds.
    let program = Program::parse(&[
        ("default", "beacon_of_light,if=target.beacon_up=0"),
        ("default", "lay_on_hands,if=target.health.pct<20"),
        ("default", "holy_shock,if=target.health.pct<95"),
        (
            "default",
            "word_of_glory,if=target.health.pct<85&holy_power>=3",
        ),
        ("default", "flash_of_light,if=target.health.pct<50"),
        ("default", "holy_light,if=target.health.pct<90"),
    ])?;
    Ok(Box::new(HolyPaladin {
        driver: Driver::new(program),
        bindings,
        equipment,
        encounter,
    }))
}

struct PartyFacts<'a> {
    native: NativeFacts<'a>,
    beacon_up: bool,
}

impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "target.beacon_up" => Ok(f64::from(self.beacon_up)),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for PartyFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "target.beacon_up" => self.value(name),
            _ => self.native.value_for(action, name),
        }
    }
    fn target_ids(&self, action: &Action) -> Result<Vec<u64>, String> {
        self.native.target_ids(action)
    }
    fn target_ready(&self, action: &Action) -> Result<bool, String> {
        self.native.target_ready(action)
    }
    fn primary_resource(&self) -> Result<String, String> {
        Ok("mana".into())
    }
}

impl Policy for HolyPaladin {
    fn drive(&mut self, world: &mut World, healer: Entity, party: &[Entity]) -> Result<(), String> {
        let living: Vec<_> = party
            .iter()
            .copied()
            .filter_map(|unit| {
                let health = world.get::<Health>(unit)?;
                (health.current > 0.0 && health.max > 0.0)
                    .then_some((unit, health.current / health.max))
            })
            .collect();
        let Some((target, _)) = living.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1)) else {
            return Ok(());
        };
        let beacon_up = crate::auras::has_aura(world, target, 53563);
        let targets = [target];
        let actions = SPELLS
            .iter()
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, &targets, &self.equipment, actions, false)?;
        let facts = PartyFacts {
            native: NativeFacts::new(world, healer, &self.bindings, self.encounter, targets.len())
                .with_targets(Some(target), &targets),
            beacon_up,
        };
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
        }
        Ok(())
    }
}
