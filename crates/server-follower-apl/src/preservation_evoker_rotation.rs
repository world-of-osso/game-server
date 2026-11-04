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
    ("dream_breath", 355936, true),
    ("temporal_anomaly", 373861, true),
    ("verdant_embrace", 360995, false),
    ("spiritbloom", 367226, false),
    ("emerald_blossom", 355913, false),
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

pub(crate) fn bindings() -> Bindings {
    Bindings {
        actions: SPELLS
            .iter()
            .map(|(name, spell, _)| ((*name).into(), *spell))
            .collect(),
        resources: [("mana".into(), PowerType::Mana)].into(),
        global_cooldown_spell: 361469,
        ..Default::default()
    }
}

pub(crate) fn program() -> Result<Program, String> {
    // SimC76f8118 apl_evoker.cpp160–162 has an empty Preservation default.
    Program::parse(&[
        ("urgent", "emerald_communion,if=mana.pct<65"),
        // INFERRED_WCL: kept for use rate; timing unsupported. See healer-item-usage.md.
        (
            "default",
            "use_items,slots=trinket1:trinket2,if=in_combat&this_trinket.healer_rate_qualified",
        ),
        ("default", "temporal_anomaly,if=party.injured>=2"),
        ("default", "dream_breath,empower_to=1,if=party.injured>=1"),
        (
            "default",
            "spiritbloom,empower_to=1,if=target.health.pct<55",
        ),
        ("default", "verdant_embrace,if=target.health.pct<85"),
        // INFERRED_WCL: lowest slot that still spends retail's ~99% of procs; see preservation-merithra-policy.md.
        ("default", "reversion,if=merithra.active"),
        ("default", "emerald_blossom,if=party.injured>=2"),
        // INFERRED_WCL: first-cast delay median 26.862 seconds; see preservation-fire-breath.md.
        ("hybrid", "fire_breath,empower_to=1,if=time>=26.862"),
        ("damage", "disintegrate"),
        ("damage", "living_flame"),
    ])
}

pub fn action_spells() -> Result<Vec<u32>, String> {
    Ok(crate::literal_rotation::program_action_spells(
        &program()?,
        &bindings(),
    ))
}

impl Preservation {
    pub fn load(world: &mut World, healer: Entity, encounter: Encounter) -> Result<Self, String> {
        let mut bindings = bindings();
        let equipment = Equipment::load(world, healer, &mut bindings)?;
        let program = program()?;
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
    merithra_active: bool,
}

impl Facts for PartyFacts<'_> {
    fn value(&self, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" => Ok(self.injured as f64),
            "merithra.active" => Ok(f64::from(self.merithra_active)),
            _ => self.native.value(name),
        }
    }
}

impl ActionFacts for PartyFacts<'_> {
    fn value_for(&self, action: &Action, name: &str) -> Result<f64, String> {
        match name {
            "party.injured" | "merithra.active" => self.value(name),
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
    /// Heal the party; damage engaged enemies when no heal is due.
    pub fn drive(
        &mut self,
        world: &mut World,
        healer: Entity,
        party: &[Entity],
        enemies: &[Entity],
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
        let Some((target, _)) = crate::follower::lowest_health(world, living.iter().copied())
        else {
            return Ok(());
        };
        let injured = living.iter().filter(|(_, ratio)| *ratio < 0.95).count();
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
            merithra_active: crate::auras::has_aura(world, healer, 1256579),
        };
        if let Some((action, execution)) =
            self.driver
                .select("urgent", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
            return Ok(());
        }
        // Retail Fire Breath starts do not depend on party health; see preservation-fire-breath.md.
        if !enemies.is_empty() {
            let actions = [NativeAction {
                name: "fire_breath",
                spell: 357208,
                self_target: false,
                talent: None,
            }];
            let prepared = Prepared::read(world, healer, enemies, &self.equipment, actions, false)?;
            let facts =
                NativeFacts::new(world, healer, &self.bindings, self.encounter, enemies.len())
                    .with_targets(enemies.first().copied(), enemies);
            if let Some((action, execution)) =
                self.driver
                    .select("hybrid", &facts, &prepared, &self.equipment)?
            {
                self.driver.execute(world, healer, action, execution)?;
                return Ok(());
            }
        }
        let facts = PartyFacts {
            native: NativeFacts::new(world, healer, &self.bindings, self.encounter, targets.len())
                .with_targets(Some(target), &targets),
            injured,
            merithra_active: crate::auras::has_aura(world, healer, 1256579),
        };
        if let Some((action, execution)) =
            self.driver
                .select("default", &facts, &prepared, &self.equipment)?
        {
            self.driver.execute(world, healer, action, execution)?;
            return Ok(());
        }
        let Some(&enemy) = enemies.first() else {
            return Ok(());
        };

        let actions = SPELLS
            .iter()
            .filter(|(name, _, _)| matches!(*name, "fire_breath" | "disintegrate" | "living_flame"))
            .map(|&(name, spell, self_target)| NativeAction {
                name,
                spell,
                self_target,
                talent: None,
            });
        let prepared = Prepared::read(world, healer, enemies, &self.equipment, actions, false)?;
        let facts = NativeFacts::new(world, healer, &self.bindings, self.encounter, enemies.len())
            .with_targets(Some(enemy), enemies);
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
    fn drive_upkeep(&mut self, world: &mut World, actor: Entity) -> Result<(), String> {
        let party = crate::follower::healing_party(world, actor);
        Preservation::drive(self, world, actor, &party, &[])
    }

    fn action_spells(&self) -> Vec<u32> {
        self.driver.action_spells(&self.bindings)
    }

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
        let party = crate::follower::healing_party(world, actor);
        Preservation::drive(self, world, actor, &party, targets)
    }
}
