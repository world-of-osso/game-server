//! Mistweaver healing decisions over native readiness and party state.
#[cfg(test)]
#[path = "mistweaver_replay_tests.rs"]
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
    ("mana_tea", 115294, true),
    ("revival", 115310, true),
    ("life_cocoon", 116849, false),
    ("thunder_focus_tea", 116680, true),
    ("renewing_mist", 115151, false),
    ("enveloping_mist", 124682, false),
    ("sheiluns_gift", 205406, false),
    ("vivify", 116670, false),
];

struct Mistweaver {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

pub(super) fn prepare(run: &mut super::dps_parity::Run) -> Result<Box<dyn Policy>, String> {
    let mut bindings = Bindings {
        actions: SPELLS
            .iter()
            .map(|(name, spell, _)| ((*name).into(), *spell))
            .collect(),
        resources: [("mana".into(), PowerType::Mana)].into(),
        global_cooldown_spell: 116670,
        ..Default::default()
    };
    let equipment = Equipment::load(run.app.world_mut(), run.actor, &mut bindings)?;
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    // SimC76f8118 apl_monk.cpp supplies only Brewmaster and Windwalker defaults.
    let program = Program::parse(&[
        ("default", "life_cocoon,if=target.health.pct<35"),
        ("default", "revival,if=party.injured>=3"),
        ("default", "mana_tea,if=mana.pct<65"),
        ("default", "thunder_focus_tea,if=target.health.pct<75"),
        (
            "default",
            "renewing_mist,if=target.health.pct<95&target.renewing_mist_up=0",
        ),
        (
            "default",
            "enveloping_mist,if=target.health.pct<65&target.enveloping_mist_up=0",
        ),
        ("default", "sheiluns_gift,if=target.health.pct<65"),
        ("default", "vivify,if=target.health.pct<90"),
    ])?;
    Ok(Box::new(Mistweaver {
        driver: Driver::new(program),
        bindings,
        equipment,
        encounter,
    }))
}

struct PartyFacts<'a> {
    native: NativeFacts<'a>,
    injured: usize,
    renewing_up: bool,
    enveloping_up: bool,
}

impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" => Ok(self.injured as f64),
            "target.renewing_mist_up" => Ok(f64::from(self.renewing_up)),
            "target.enveloping_mist_up" => Ok(f64::from(self.enveloping_up)),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for PartyFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" | "target.renewing_mist_up" | "target.enveloping_mist_up" => {
                self.value(name)
            }
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

impl Policy for Mistweaver {
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
        let injured = living.iter().filter(|(_, ratio)| *ratio < 0.90).count();
        let renewing_up = crate::auras::has_aura(world, target, 119611)
            || crate::auras::has_aura(world, target, 115151);
        let enveloping_up = crate::auras::has_aura(world, target, 124682);
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
            injured,
            renewing_up,
            enveloping_up,
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
