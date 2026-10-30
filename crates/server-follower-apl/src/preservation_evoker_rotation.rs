//! Preservation Evoker healing decisions over native readiness and party state.

use crate::literal_rotation::{
    Driver,
    equipment::Equipment,
    execution::{NativeAction, Prepared},
};
use crate::simc_apl_native::{Bindings, Encounter, NativeFacts};
use bevy::prelude::{Entity, World};
use server::simc_apl::{Action, ActionFacts, Facts, Program};
use shared::components::{Health, PowerType};

pub const SPELLS: &[(&str, u32, bool)] = &[
    ("emerald_communion", 370960, true),
    ("rewind", 363534, true),
    ("dream_breath", 355936, true),
    ("temporal_anomaly", 373861, true),
    ("verdant_embrace", 360995, false),
    ("spiritbloom", 367226, false),
    ("emerald_blossom", 355913, false),
    ("echo", 364343, false),
    ("reversion", 366155, false),
    ("living_flame", 361469, false),
    ("fire_breath", 357208, false),
    ("disintegrate", 356995, false),
];

pub struct Preservation {
    driver: Driver,
    bindings: Bindings,
    equipment: Equipment,
    encounter: Encounter,
}

impl Preservation {
    pub fn load(world: &mut World, healer: Entity, encounter: Encounter) -> Result<Self, String> {
        let mut bindings = Bindings {
            actions: SPELLS
                .iter()
                .map(|(name, spell, _)| ((*name).into(), *spell))
                .collect(),
            resources: [("mana".into(), PowerType::Mana)].into(),
            global_cooldown_spell: 361469,
            ..Default::default()
        };
        let equipment = Equipment::load(world, healer, &mut bindings)?;
        // SimC76f8118 apl_evoker.cpp160–162 has an empty Preservation default.
        let program = Program::parse(&[
            ("default", "emerald_communion,if=mana.pct<65"),
            ("default", "rewind,if=party.injured>=3"),
            ("default", "temporal_anomaly,if=party.injured>=2"),
            ("default", "echo,if=target.health.pct<95&target.echo_up=0"),
            ("default", "dream_breath,empower_to=1,if=party.injured>=1"),
            (
                "default",
                "spiritbloom,empower_to=1,if=target.health.pct<55",
            ),
            ("default", "verdant_embrace,if=target.health.pct<85"),
            ("default", "emerald_blossom,if=party.injured>=2"),
            (
                "default",
                "reversion,if=target.health.pct<95&target.reversion_up=0",
            ),
            ("default", "living_flame,if=target.health.pct<95"),
            ("damage", "fire_breath,empower_to=1"),
            ("damage", "disintegrate"),
            ("damage", "living_flame"),
        ])?;
        Ok(Preservation {
            driver: Driver::new(program),
            bindings,
            equipment,
            encounter,
        })
    }
}

pub struct PartyFacts<'a> {
    native: NativeFacts<'a>,
    injured: usize,
    reversion_up: bool,
    echo_up: bool,
}

impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" => Ok(self.injured as f64),
            "target.reversion_up" => Ok(f64::from(self.reversion_up)),
            "target.echo_up" => Ok(f64::from(self.echo_up)),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for PartyFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" | "target.reversion_up" | "target.echo_up" => self.value(name),
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

impl Preservation {
    /// Heal the party; damage `enemy` when no heal is due.
    pub fn drive(
        &mut self,
        world: &mut World,
        healer: Entity,
        party: &[Entity],
        enemy: Option<Entity>,
    ) -> Result<(), String> {
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
        let injured = living.iter().filter(|(_, ratio)| *ratio < 0.95).count();
        let reversion_up = crate::auras::has_aura(world, target, 366155)
            || crate::auras::has_aura(world, target, 367364);
        let echo_up = crate::auras::has_aura(world, target, 364343);
        let targets = [target];
        let actions = SPELLS
            .iter()
            .filter(|(name, _, _)| !matches!(*name, "disintegrate" | "fire_breath"))
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
            reversion_up,
            echo_up,
        };
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
            return Ok(());
        }
        if injured != 0 {
            return Ok(());
        }
        let Some(enemy) = enemy else {
            return Ok(());
        };
        let enemies = [enemy];
        let actions = SPELLS
            .iter()
            .filter(|(name, _, _)| matches!(*name, "fire_breath" | "disintegrate" | "living_flame"))
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, &enemies, &self.equipment, actions, false)?;
        let facts = NativeFacts::new(world, healer, &self.bindings, self.encounter, 1)
            .with_targets(Some(enemy), &enemies);
        if let Some((action, execution)) =
            self.driver
                .select("damage", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
        }
        Ok(())
    }
}

impl crate::follower::SpecRotation for Preservation {
    fn poll_precombat(
        &mut self,
        _world: &mut World,
        _actor: Entity,
        _targets: &[Entity],
    ) -> Result<crate::literal_rotation::PrecombatPoll, String> {
        // The triage program has no precombat list.
        Ok(crate::literal_rotation::PrecombatPoll::Finished)
    }

    fn execute(
        &mut self,
        world: &mut World,
        actor: Entity,
        action: Action,
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
        let party = crate::follower::party_of(world, actor);
        Preservation::drive(self, world, actor, &party, targets.first().copied())
    }
}
