//! Native Discipline Priest priorities; the SimC 76f8118 healer APL is experimental.
use super::dps_parity::literal_rotation::{
    self, Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use super::healer_simc_replay::Policy;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::{ActiveSpec, Health, PowerType};

#[cfg(test)]
#[path = "discipline_priest_replay_tests.rs"]
mod tests;

const SPELLS: &[(&str, u32, bool)] = &[
    ("pain_suppression", 33206, false),
    ("barrier", 62618, true),
    ("radiance", 194509, false),
    ("void_shield", 17, false),
    ("shield", 17, false),
    ("evangelism", 246287, true),
    ("penance", 47540, false),
    ("shadowfiend", 34433, false),
    ("mind_blast", 8092, false),
    ("smite", 585, false),
];
struct Discipline {
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
        actions: SPELLS.iter().map(|(n, id, _)| ((*n).into(), *id)).collect(),
        resources: [("mana".into(), PowerType::Mana)].into(),
        global_cooldown_spell: 17,
        ..Default::default()
    };
    let equipment = Equipment::load(run.app.world_mut(), healer, &mut bindings)?;
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), healer, encounter)?;
    let program = Program::parse(&[
        ("default", "pain_suppression,if=target.health.pct<35"),
        ("default", "barrier,if=party.injured>=3"),
        ("default", "radiance,if=party.injured>=3"),
        ("default", "void_shield,if=party.darkness_up"),
        (
            "default",
            "shield,if=target.atonement_up=0&target.shield_up=0",
        ),
        ("default", "evangelism,if=party.injured>=3"),
        ("heal", "penance,if=target.health.pct<45"),
        ("damage", "shadowfiend,if=party.injured>=1"),
        ("damage", "penance"),
        ("damage", "mind_blast"),
        ("damage", "smite"),
    ])?;
    Ok(Box::new(Discipline {
        driver: Driver::new(program),
        bindings,
        equipment,
        encounter,
    }))
}
struct PartyFacts<'a> {
    native: NativeFacts<'a>,
    injured: usize,
    shield: bool,
    atonement: bool,
    darkness: bool,
}
impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" => Ok(self.injured as f64),
            "target.shield_up" => Ok(f64::from(self.shield)),
            "target.atonement_up" => Ok(f64::from(self.atonement)),
            "party.darkness_up" => Ok(f64::from(self.darkness)),
            _ => self.native.value(name),
        }
    }
}
impl ActionFacts for PartyFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" | "target.shield_up" | "target.atonement_up" | "party.darkness_up" => {
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
impl Discipline {
    fn select(
        &mut self,
        world: &mut World,
        healer: Entity,
        target: Entity,
        injured: usize,
        list: &str,
        names: &[&str],
    ) -> Result<bool, String> {
        let targets = [target];
        let actions = SPELLS
            .iter()
            .filter(|(name, _, _)| names.contains(name))
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, &targets, &self.equipment, actions, false)?;
        let atonement = world
            .get::<crate::auras::ActiveAuras>(target)
            .is_some_and(|auras| {
                auras
                    .iter()
                    .any(|a| a.spell_id == 194384 && a.caster == healer)
            });
        let facts = PartyFacts {
            native: NativeFacts::new(world, healer, &self.bindings, self.encounter, 1)
                .with_targets(Some(target), &targets),
            injured,
            shield: crate::auras::has_aura(world, target, 17),
            atonement,
            darkness: crate::auras::has_aura(world, healer, 1253591),
        };
        if let Some((action, execution)) =
            self.driver
                .select(list, &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
            return Ok(true);
        }
        Ok(false)
    }
}
impl Policy for Discipline {
    fn drive(&mut self, world: &mut World, healer: Entity, party: &[Entity]) -> Result<(), String> {
        let living: Vec<_> = party
            .iter()
            .copied()
            .filter_map(|unit| {
                let h = world.get::<Health>(unit)?;
                (h.current > 0.0 && h.max > 0.0).then_some((unit, h.current / h.max))
            })
            .collect();
        let Some((lowest, ratio)) = living.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            return Ok(());
        };
        let tank = party
            .iter()
            .copied()
            .find(|&unit| {
                world
                    .get::<ActiveSpec>(unit)
                    .is_some_and(|spec| spec.0 == 66)
            })
            .ok_or("Discipline party tank absent")?;
        let target = if ratio < 0.55 || !living.iter().any(|(unit, _)| *unit == tank) {
            lowest
        } else {
            tank
        };
        let injured = living.iter().filter(|(_, ratio)| *ratio < 0.85).count();
        if self.select(
            world,
            healer,
            target,
            injured,
            "default",
            &[
                "pain_suppression",
                "barrier",
                "radiance",
                "void_shield",
                "shield",
                "evangelism",
            ],
        )? {
            return Ok(());
        }
        if self.select(world, healer, lowest, injured, "heal", &["penance"])? {
            return Ok(());
        }
        let enemy = world
            .get::<crate::networking::CombatTarget>(tank)
            .and_then(|t| t.0);
        if let Some(enemy) = enemy {
            let _ = self.select(
                world,
                healer,
                enemy,
                injured,
                "damage",
                &["penance", "shadowfiend", "mind_blast", "smite"],
            )?;
        }
        Ok(())
    }
}
